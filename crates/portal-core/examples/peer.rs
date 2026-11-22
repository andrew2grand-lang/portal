//! A headless Portal peer for testing against the app without a second
//! computer. Prints the core's log, snapshots and progress.
//!
//!   cargo run -p portal-core --example peer -- --name "Test PC" [--port 0]
//!       [--data DIR] [--receive DIR] [--connect 192.168.1.20] [--auto]
//!       [--send PATH]... [--no-mdns] [--no-broadcast]
//!       [--loopback] [--udp-port N] [--announce-to ADDR]... [--listen-after SECS]
//!
//! --auto          shows a pairing code when asked (printed as `PAIR CODE 048213`,
//!                 to type in the app) and accepts offers by itself
//! --send          sends PATH to the first paired, connected device (once)
//! --loopback      127.0.0.1 only, clipboard in memory (tests); announces go
//!                 only to --announce-to addresses (another peer's UDP port)
//! --listen-after  starts listening this many seconds late (needs --port)
//! --blind         ignores discovery: only ever dials stored addresses
//!
//! Prints `ONLINE <name>` / `OFFLINE <name>` when a paired device connects or drops.

use std::path::PathBuf;

use portal_core::events::{Event, PairStep, TransferPhase};
use portal_core::node::{PairAction, SettingsPatch};
use portal_core::{Config, Core};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let mut name = "Test peer".to_owned();
    let mut port = 0u16;
    let mut data: Option<PathBuf> = None;
    let mut receive: Option<PathBuf> = None;
    let mut connect: Option<String> = None;
    let mut sends: Vec<PathBuf> = Vec::new();
    let (mut mdns, mut broadcast, mut auto, mut loopback) = (true, true, false, false);
    let mut udp_port: Option<u16> = None;
    let mut announce_to: Vec<std::net::SocketAddr> = Vec::new();
    let mut listen_after: Option<f64> = None;
    let mut blind = false;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--name" => name = args.next().ok_or("--name needs a value")?,
            "--port" => port = args.next().ok_or("--port needs a value")?.parse()?,
            "--data" => data = Some(args.next().ok_or("--data needs a value")?.into()),
            "--receive" => receive = Some(args.next().ok_or("--receive needs a value")?.into()),
            "--connect" => connect = Some(args.next().ok_or("--connect needs a value")?),
            "--send" => sends.push(std::fs::canonicalize(args.next().ok_or("--send needs a value")?)?),
            "--auto" => auto = true,
            "--no-mdns" => mdns = false,
            "--no-broadcast" => broadcast = false,
            "--loopback" => loopback = true,
            "--blind" => blind = true,
            "--udp-port" => udp_port = Some(args.next().ok_or("--udp-port needs a value")?.parse()?),
            "--announce-to" => announce_to.push(args.next().ok_or("--announce-to needs a value")?.parse()?),
            "--listen-after" => listen_after = Some(args.next().ok_or("--listen-after needs a value")?.parse()?),
            other => return Err(format!("unknown argument {other}").into()),
        }
    }
    let dir = data.unwrap_or_else(|| std::env::temp_dir().join(format!("portal-peer-{}", name.replace(' ', "-"))));
    let mut cfg = Config::new(&dir, env!("CARGO_PKG_VERSION"));
    cfg.device_name = Some(name);
    cfg.port = port;
    cfg.mdns = mdns;
    cfg.broadcast = broadcast;
    if loopback {
        cfg.loopback_only = true;
        cfg.clipboard = std::sync::Arc::new(portal_core::clip::MemoryClipboard::default());
    }
    if let Some(p) = udp_port {
        cfg.udp_port = p;
    }
    cfg.extra_targets = announce_to;
    cfg.fault_listen_after = listen_after.map(std::time::Duration::from_secs_f64);
    cfg.fault_blind = blind;
    let (core, mut events) = Core::start(cfg).await?;
    if let Some(r) = receive {
        core.update_settings(SettingsPatch { receive_dir: Some(r), ..Default::default() }).await;
    }
    let me = core.snapshot().await?.me;
    println!("peer {} ({}) on port {} — data in {}", me.name, me.id, me.port, dir.display());
    if let Some(addr) = connect {
        match core.connect(&addr).await {
            Ok(p) => println!("connected to {} ({:?})", p.name, p.os),
            Err(e) => println!("connect failed: {e}"),
        }
    }
    let mut last = String::new();
    let mut sent = false;
    // The pairing code last printed (printed once per code)
    let mut shown: Option<String> = None;
    let mut online: std::collections::HashSet<String> = Default::default();
    while let Some(event) = events.recv().await {
        match event {
            Event::Log(l) => println!("{:>8} {}", l.tag, l.text),
            Event::Progress(p) => println!(
                "progress {:>5.1}%  {:.0} MB/s  {}",
                p.bytes as f64 * 100.0 / p.total.max(1) as f64,
                p.rate / 1e6,
                p.file
            ),
            Event::Notice(n) => println!("  notice {}: {}", n.title, n.body),
            Event::Snapshot(s) => {
                for p in s.peers.iter().filter(|p| p.paired) {
                    if p.online && online.insert(p.id.to_string()) {
                        println!("ONLINE {}", p.name);
                    } else if !p.online && online.remove(&p.id.to_string()) {
                        println!("OFFLINE {}", p.name);
                    }
                }
                if auto {
                    if let Some(p) = &s.pairing
                        && p.failed.is_none()
                    {
                        match (p.step, &p.code) {
                            (PairStep::Asked, _) => {
                                let _ = core.pairing(PairAction::ShowCode).await;
                            }
                            (PairStep::Code, Some(code)) if shown.as_deref() != Some(code.as_str()) => {
                                println!("PAIR CODE {code}  (type it on {})", p.peer_name);
                                shown = Some(code.clone());
                            }
                            _ => {}
                        }
                    }
                    for t in s.transfers.iter().filter(|t| {
                        t.phase == TransferPhase::Offered && t.direction == portal_core::events::Direction::In
                    }) {
                        core.accept(t.tid, false).await;
                    }
                }
                if !sent
                    && !sends.is_empty()
                    && let Some(p) = s.peers.iter().find(|p| p.paired && p.online)
                {
                    sent = true;
                    match core.send(&p.id, sends.clone()).await {
                        Ok(()) => println!("sending to {}", p.name),
                        Err(e) => println!("send failed: {e}"),
                    }
                }
                let peers: Vec<String> = s
                    .peers
                    .iter()
                    .map(|p| {
                        format!(
                            "{}[{}{}{:?}]",
                            p.name,
                            if p.paired { "paired " } else { "" },
                            if p.online { "online " } else { "" },
                            p.link
                        )
                    })
                    .collect();
                let t: String = s
                    .transfers
                    .iter()
                    .map(|t| {
                        format!(
                            " transfer {} {:?} {:?}{}",
                            t.peer_name,
                            t.direction,
                            t.phase,
                            t.failure.as_ref().map(|f| format!(": {f}")).unwrap_or_default()
                        )
                    })
                    .collect();
                let line = format!("peers: {}{t}", peers.join(", "));
                if line != last {
                    println!("{line}");
                    last = line;
                }
            }
        }
    }
    Ok(())
}
