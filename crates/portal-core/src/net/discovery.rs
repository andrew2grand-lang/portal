//! Finding the other machine (brief §6.1).
//!
//! 1. mDNS / DNS-SD: `_seamdrop._tcp` (the old `_portal._tcp` went in Mission
//!    05: protocol 4 can't talk to Portal), instance = device id, unique host name
//!    `seamdrop-<id>.local.` (never the machine's own hostname, which macOS's
//!    mDNSResponder and Windows' own mDNS already answer for).
//! 2. UDP announce on every interface's *directed* broadcast address (incl.
//!    169.254.255.255 on link-local interfaces), 255.255.255.255 and IPv6
//!    ff02::1 per interface. A device that hears a new announce replies by
//!    unicast at once.
//! 3. Sweep: the same announce sent by unicast to every address of a small
//!    private network (a /24 at most). Some routers pass neither multicast
//!    nor broadcast between their wired and wireless sides, but unicast
//!    always gets through, and the reply comes straight back. The node only
//!    asks for a sweep while it's alone, and less often as time goes by.
//!
//! Sightings are hints only: identity is always proven by TLS afterwards.

use std::collections::HashMap;
use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV6};
use std::sync::Arc;
use std::time::{Duration, Instant};

use mdns_sd::{IfKind, ScopedIp, ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::{Deserialize, Serialize};
use socket2::{Domain, Protocol, Socket, Type};
use tokio::net::UdpSocket;
use tokio::sync::{Mutex, mpsc};

use crate::consts::{HOST_PREFIX, PROTOCOL_VERSION, SERVICE_TYPE};
use crate::model::{DeviceId, Os};
use crate::net::iface::{self, Iface, IfaceKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Via {
    Mdns,
    Broadcast,
    /// A trusted peer's last-known address answered.
    Known,
    /// Typed into "Connect by IP".
    Manual,
    /// The peer connected to us.
    Inbound,
}

/// "Device X can be reached at this address." A hint until TLS proves it.
#[derive(Clone, Debug, PartialEq)]
pub struct Sighting {
    pub id: DeviceId,
    pub name: String,
    pub os: Os,
    pub proto: u32,
    /// TCP address to dial (IPv6 link-local carries our interface index).
    pub addr: SocketAddr,
    pub via: Via,
    /// Our interface it was seen on, when known.
    pub if_index: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct DiscoveryConfig {
    pub id: DeviceId,
    pub name: String,
    pub os: Os,
    /// TCP port we listen on (advertised).
    pub port: u16,
    /// UDP port for announces.
    pub udp_port: u16,
    pub mdns: bool,
    pub broadcast: bool,
    /// Extra unicast announce targets (tests; could hold known peers).
    pub extra_targets: Vec<SocketAddr>,
    /// Tests: bind to loopback only and skip real interfaces.
    pub loopback: bool,
}

const MAGIC: &[u8] = b"PORTAL1\n";
const MAX_DATAGRAM: usize = 1200;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Announce {
    pub proto: u32,
    pub id: DeviceId,
    pub name: String,
    pub os: Os,
    /// TCP port to dial.
    pub port: u16,
    /// UDP port to reply to.
    pub uport: u16,
    pub reply: bool,
}

pub fn encode_announce(a: &Announce) -> Vec<u8> {
    let mut out = MAGIC.to_vec();
    out.extend(serde_json::to_vec(a).expect("announce serialises"));
    out
}

pub fn decode_announce(bytes: &[u8]) -> Option<Announce> {
    if bytes.len() > MAX_DATAGRAM {
        return None;
    }
    serde_json::from_slice(bytes.strip_prefix(MAGIC)?).ok()
}

enum Cmd {
    Rename(String),
    AnnounceNow,
    Sweep,
}

/// Running discovery. Dropping it stops everything.
pub struct Discovery {
    cmd: mpsc::Sender<Cmd>,
    mdns: Option<ServiceDaemon>,
    tasks: Vec<tokio::task::JoinHandle<()>>,
}

impl Discovery {
    /// Updates the advertised device name.
    pub fn rename(&self, name: &str) {
        let _ = self.cmd.try_send(Cmd::Rename(name.to_owned()));
    }

    /// Announces again now and for the next few seconds (network changed,
    /// "Try again").
    pub fn announce_now(&self) {
        let _ = self.cmd.try_send(Cmd::AnnounceNow);
    }

    /// Announces by unicast to every address on the local network (see the
    /// module notes). Does nothing without the UDP announcer.
    pub fn sweep(&self) {
        let _ = self.cmd.try_send(Cmd::Sweep);
    }
}

/// Small private networks only: a /24 is 254 datagrams. Bigger networks
/// (offices, schools) are left alone.
const SWEEP_MIN_PREFIX: u8 = 24;

/// Every other address on our small private Wi-Fi and wired networks.
pub fn sweep_targets(ifaces: &[Iface]) -> Vec<Ipv4Addr> {
    let mut out = Vec::new();
    for i in ifaces.iter().filter(|i| matches!(i.kind, IfaceKind::Wifi | IfaceKind::Ethernet) && i.has_gateway) {
        for (ip, prefix) in &i.v4 {
            if !ip.is_private() || !(SWEEP_MIN_PREFIX..=30).contains(prefix) {
                continue;
            }
            let host_bits = 32 - *prefix as u32;
            let network = u32::from(*ip) & (u32::MAX << host_bits);
            for host in 1..(1u32 << host_bits) - 1 {
                let target = Ipv4Addr::from(network | host);
                if target != *ip && !out.contains(&target) {
                    out.push(target);
                }
            }
        }
    }
    out
}

impl Drop for Discovery {
    fn drop(&mut self) {
        for t in &self.tasks {
            t.abort();
        }
        if let Some(d) = self.mdns.take() {
            let _ = d.shutdown();
        }
    }
}

pub async fn start(cfg: DiscoveryConfig, tx: mpsc::Sender<Sighting>) -> io::Result<Discovery> {
    let (cmd_tx, cmd_rx) = mpsc::channel(16);
    let mut tasks = Vec::new();
    let mdns = if cfg.mdns {
        match start_mdns(&cfg, tx.clone(), &mut tasks) {
            Ok(d) => Some(d),
            Err(e) => {
                tracing::warn!("mDNS unavailable: {e}");
                None
            }
        }
    } else {
        None
    };
    let mdns_for_rename = mdns.clone();
    let state = Arc::new(Mutex::new(cfg.clone()));
    if cfg.broadcast {
        let sockets = Arc::new(BroadcastSockets::bind(&cfg)?);
        tasks.push(tokio::spawn(recv_loop(sockets.v4.clone(), sockets.clone(), state.clone(), tx.clone())));
        if let Some(v6) = sockets.v6.clone() {
            tasks.push(tokio::spawn(recv_loop(v6, sockets.clone(), state.clone(), tx.clone())));
        }
        tasks.push(tokio::spawn(announce_loop(sockets, state.clone(), cmd_rx, mdns_for_rename)));
    } else {
        tasks.push(tokio::spawn(mdns_only_loop(state, cmd_rx, mdns_for_rename)));
    }
    Ok(Discovery { cmd: cmd_tx, mdns, tasks })
}

// ---- mDNS -----------------------------------------------------------------

/// Advertised and browsed (Mission 05 dropped `_portal._tcp`, decision 121).
pub const SERVICE_TYPES: [&str; 1] = [SERVICE_TYPE];

/// Another copy of this app on this computer (same device id, another port
/// or protocol): the old app still running after the rename, say. Our own
/// announcements come back with our own port and protocol and aren't one.
pub fn is_twin(cfg: &DiscoveryConfig, id: &DeviceId, port: u16, proto: u32) -> bool {
    id == &cfg.id && (port != cfg.port || proto != PROTOCOL_VERSION)
}

fn service_info(cfg: &DiscoveryConfig, service_type: &str) -> Result<ServiceInfo, mdns_sd::Error> {
    let host = format!("{HOST_PREFIX}{}.local.", cfg.id);
    let os = serde_json::to_value(cfg.os).ok().and_then(|v| v.as_str().map(str::to_owned)).unwrap_or_default();
    let name: String = cfg.name.chars().take(63).collect();
    let props = [
        ("id", cfg.id.to_string()),
        ("name", name),
        ("os", os),
        ("proto", PROTOCOL_VERSION.to_string()),
        ("port", cfg.port.to_string()),
    ];
    Ok(ServiceInfo::new(service_type, cfg.id.as_str(), &host, "", cfg.port, &props[..])?.enable_addr_auto())
}

fn start_mdns(
    cfg: &DiscoveryConfig,
    tx: mpsc::Sender<Sighting>,
    tasks: &mut Vec<tokio::task::JoinHandle<()>>,
) -> Result<ServiceDaemon, mdns_sd::Error> {
    let daemon = ServiceDaemon::new()?;
    if !cfg.loopback {
        daemon.disable_interface(vec![IfKind::LoopbackV4, IfKind::LoopbackV6])?;
        // Apple peer-to-peer and access-point interfaces, idle tunnels, and the
        // loopback interface's link-local fe80::1 (not reachable from outside).
        daemon.disable_interface(IfKind::Predicate(mdns_sd::IfPredicate::new(|i| {
            iface::is_ignored_name(&i.name) || i.name == "lo0" || i.name == "lo"
        })))?;
    }
    for ty in SERVICE_TYPES {
        daemon.register(service_info(cfg, ty)?)?;
        let browse = daemon.browse(ty)?;
        let (cfg, tx) = (cfg.clone(), tx.clone());
        tasks.push(tokio::spawn(async move {
            while let Ok(event) = browse.recv_async().await {
                if let ServiceEvent::ServiceResolved(service) = event {
                    for s in mdns_sightings(&service, &cfg) {
                        if tx.send(s).await.is_err() {
                            return;
                        }
                    }
                }
            }
        }));
    }
    Ok(daemon)
}

fn mdns_sightings(service: &mdns_sd::ResolvedService, cfg: &DiscoveryConfig) -> Vec<Sighting> {
    let txt = |k: &str| service.get_property_val_str(k).map(str::to_owned);
    let Some(id) = txt("id").as_deref().and_then(DeviceId::parse) else { return vec![] };
    let name = txt("name").unwrap_or_else(|| "Unknown device".into());
    let os = txt("os").map(|o| Os::parse(&o)).unwrap_or(Os::Other);
    let proto = txt("proto").and_then(|p| p.parse().ok()).unwrap_or(0);
    let port = service.get_port();
    if id == cfg.id && !is_twin(cfg, &id, port, proto) {
        return vec![];
    }
    service
        .get_addresses()
        .iter()
        .filter_map(|ip| {
            let (addr, if_index) = match ip {
                ScopedIp::V4(v4) => {
                    (SocketAddr::new(IpAddr::V4(*v4.addr()), port), v4.interface_ids().first().map(|i| i.index))
                }
                ScopedIp::V6(v6) => {
                    let index = v6.scope_id().index;
                    let scope = if iface::is_link_local(&IpAddr::V6(*v6.addr())) { index } else { 0 };
                    (SocketAddr::V6(SocketAddrV6::new(*v6.addr(), port, 0, scope)), Some(index))
                }
                #[allow(unreachable_patterns)]
                _ => return None,
            };
            Some(Sighting { id: id.clone(), name: name.clone(), os, proto, addr, via: Via::Mdns, if_index })
        })
        .collect()
}

fn rename_mdns(daemon: &ServiceDaemon, cfg: &DiscoveryConfig) {
    for ty in SERVICE_TYPES {
        match service_info(cfg, ty) {
            Ok(info) => {
                let _ = daemon.unregister(info.get_fullname());
                if let Err(e) = daemon.register(info) {
                    tracing::warn!("mDNS re-register failed: {e}");
                }
            }
            Err(e) => tracing::warn!("mDNS service info: {e}"),
        }
    }
}

async fn mdns_only_loop(state: Arc<Mutex<DiscoveryConfig>>, mut cmd: mpsc::Receiver<Cmd>, mdns: Option<ServiceDaemon>) {
    while let Some(c) = cmd.recv().await {
        if let (Cmd::Rename(name), Some(d)) = (c, &mdns) {
            let mut cfg = state.lock().await;
            cfg.name = name;
            rename_mdns(d, &cfg);
        }
    }
}

// ---- UDP announce ---------------------------------------------------------

struct BroadcastSockets {
    v4: Arc<UdpSocket>,
    v6: Option<Arc<UdpSocket>>,
}

fn udp(addr: SocketAddr) -> io::Result<UdpSocket> {
    let socket = Socket::new(Domain::for_address(addr), Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    #[cfg(all(unix, not(target_os = "solaris")))]
    socket.set_reuse_port(true)?;
    if addr.is_ipv6() {
        socket.set_only_v6(true)?;
    } else {
        socket.set_broadcast(true)?;
    }
    socket.bind(&addr.into())?;
    socket.set_nonblocking(true)?;
    UdpSocket::from_std(socket.into())
}

impl BroadcastSockets {
    fn bind(cfg: &DiscoveryConfig) -> io::Result<Self> {
        if cfg.loopback {
            return Ok(Self { v4: Arc::new(udp(SocketAddr::from((Ipv4Addr::LOCALHOST, cfg.udp_port)))?), v6: None });
        }
        let v4 = Arc::new(udp(SocketAddr::from((Ipv4Addr::UNSPECIFIED, cfg.udp_port)))?);
        let v6 = udp(SocketAddr::from((Ipv6Addr::UNSPECIFIED, cfg.udp_port)))
            .inspect_err(|e| tracing::warn!("no IPv6 announce socket: {e}"))
            .ok()
            .map(Arc::new);
        Ok(Self { v4, v6 })
    }
}

fn announce(cfg: &DiscoveryConfig, reply: bool) -> Vec<u8> {
    encode_announce(&Announce {
        proto: PROTOCOL_VERSION,
        id: cfg.id.clone(),
        name: cfg.name.chars().take(63).collect(),
        os: cfg.os,
        port: cfg.port,
        uport: cfg.udp_port,
        reply,
    })
}

/// Sends a datagram out of one specific IPv4 interface (macOS: IP_BOUND_IF),
/// so link-local broadcasts leave over the cable even when Wi-Fi is primary.
fn send_from_iface(src: Ipv4Addr, if_index: u32, dest: SocketAddr, payload: &[u8]) -> io::Result<()> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_broadcast(true)?;
    #[cfg(target_vendor = "apple")]
    socket.bind_device_by_index_v4(std::num::NonZeroU32::new(if_index))?;
    #[cfg(not(target_vendor = "apple"))]
    let _ = if_index;
    socket.bind(&SocketAddr::from((src, 0)).into())?;
    socket.send_to(payload, &dest.into())?;
    Ok(())
}

async fn send_all(sockets: &BroadcastSockets, cfg: &DiscoveryConfig) {
    let payload = announce(cfg, false);
    for target in &cfg.extra_targets {
        let _ = sockets.v4.send_to(&payload, target).await;
    }
    if cfg.loopback {
        return;
    }
    let dest_port = cfg.udp_port;
    let ifaces = tokio::task::spawn_blocking(iface::snapshot).await.unwrap_or_default();
    for i in ifaces.iter().filter(|i| !matches!(i.kind, IfaceKind::Loopback | IfaceKind::Vpn)) {
        for (addr, prefix) in &i.v4 {
            let dest = SocketAddr::from((iface::directed_broadcast(*addr, *prefix), dest_port));
            if let Err(e) = send_from_iface(*addr, i.index, dest, &payload) {
                tracing::debug!("announce to {dest} via {}: {e}", i.name);
            }
        }
        if let Some(v6) = &sockets.v6
            && i.v6.iter().any(|a| iface::is_link_local(&IpAddr::V6(*a)))
        {
            let dest =
                SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::new(0xff02, 0, 0, 0, 0, 0, 0, 1), dest_port, 0, i.index));
            if let Err(e) = v6.send_to(&payload, dest).await {
                tracing::debug!("announce to {dest}: {e}");
            }
        }
    }
    let _ = sockets.v4.send_to(&payload, SocketAddr::from((Ipv4Addr::BROADCAST, dest_port))).await;
}

/// Announces every 2 s for this long after starting, waking, a network
/// change and Try again (the grace before "isn't reachable" is 20 s).
const BURST: Duration = Duration::from_secs(30);

async fn announce_loop(
    sockets: Arc<BroadcastSockets>,
    state: Arc<Mutex<DiscoveryConfig>>,
    mut cmd: mpsc::Receiver<Cmd>,
    mdns: Option<ServiceDaemon>,
) {
    // Every 2 s for the first 30 s after a (re)start, then every 5 s (so a
    // device whose own announces don't get through still hears ours soon).
    let mut burst_until = Instant::now() + BURST;
    loop {
        let cfg = state.lock().await.clone();
        send_all(&sockets, &cfg).await;
        let wait = if Instant::now() < burst_until { Duration::from_secs(2) } else { Duration::from_secs(5) };
        tokio::select! {
            _ = tokio::time::sleep(wait) => {}
            c = cmd.recv() => match c {
                None => return,
                Some(Cmd::AnnounceNow) => burst_until = Instant::now() + BURST,
                Some(Cmd::Sweep) => send_sweep(&sockets, &cfg).await,
                Some(Cmd::Rename(name)) => {
                    let mut cfg = state.lock().await;
                    cfg.name = name;
                    if let Some(d) = &mdns {
                        rename_mdns(d, &cfg);
                    }
                }
            },
        }
    }
}

async fn send_sweep(sockets: &BroadcastSockets, cfg: &DiscoveryConfig) {
    if cfg.loopback {
        return;
    }
    let ifaces = tokio::task::spawn_blocking(iface::snapshot).await.unwrap_or_default();
    let targets = sweep_targets(&ifaces);
    let payload = announce(cfg, false);
    for (n, ip) in targets.iter().enumerate() {
        let _ = sockets.v4.send_to(&payload, SocketAddr::from((*ip, cfg.udp_port))).await;
        // Paced: a burst of hundreds of datagrams can overrun a Wi-Fi queue.
        if n % 32 == 31 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }
    tracing::debug!("swept {} addresses", targets.len());
}

async fn recv_loop(
    socket: Arc<UdpSocket>,
    sockets: Arc<BroadcastSockets>,
    state: Arc<Mutex<DiscoveryConfig>>,
    tx: mpsc::Sender<Sighting>,
) {
    let mut buf = vec![0u8; 2048];
    let mut replied: HashMap<DeviceId, Instant> = HashMap::new();
    loop {
        let Ok((len, src)) = socket.recv_from(&mut buf).await else { continue };
        let Some(a) = decode_announce(&buf[..len]) else { continue };
        let cfg = state.lock().await.clone();
        if a.id == cfg.id && !is_twin(&cfg, &a.id, a.port, a.proto) {
            continue;
        }
        let (addr, if_index) = match src {
            SocketAddr::V4(v4) => (SocketAddr::from((*v4.ip(), a.port)), None),
            SocketAddr::V6(v6) => {
                let scope = v6.scope_id();
                (SocketAddr::V6(SocketAddrV6::new(*v6.ip(), a.port, 0, scope)), Some(scope).filter(|s| *s != 0))
            }
        };
        if !a.reply && replied.get(&a.id).is_none_or(|t| t.elapsed() > Duration::from_secs(2)) {
            replied.insert(a.id.clone(), Instant::now());
            let reply_to = match src {
                SocketAddr::V4(v4) => SocketAddr::from((*v4.ip(), a.uport)),
                SocketAddr::V6(v6) => SocketAddr::V6(SocketAddrV6::new(*v6.ip(), a.uport, 0, v6.scope_id())),
            };
            send_reply(&sockets, &cfg, reply_to).await;
        }
        let sighting =
            Sighting { id: a.id, name: a.name, os: a.os, proto: a.proto, addr, via: Via::Broadcast, if_index };
        if tx.send(sighting).await.is_err() {
            return;
        }
    }
}

async fn send_reply(sockets: &BroadcastSockets, cfg: &DiscoveryConfig, to: SocketAddr) {
    let payload = announce(cfg, true);
    let result = match to {
        SocketAddr::V4(v4) if v4.ip().is_link_local() && !cfg.loopback => {
            // Leave through the interface that owns the matching 169.254 address.
            let ifaces = tokio::task::spawn_blocking(iface::snapshot).await.unwrap_or_default();
            match iface::iface_for(&ifaces, &to, None).and_then(|i| i.v4.first().map(|(ip, _)| (*ip, i.index))) {
                Some((src, index)) => send_from_iface(src, index, to, &payload),
                None => sockets.v4.send_to(&payload, to).await.map(|_| ()),
            }
        }
        SocketAddr::V4(_) => sockets.v4.send_to(&payload, to).await.map(|_| ()),
        SocketAddr::V6(_) => match &sockets.v6 {
            Some(v6) => v6.send_to(&payload, to).await.map(|_| ()),
            None => Ok(()),
        },
    };
    if let Err(e) = result {
        tracing::debug!("reply to {to}: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn net(kind: IfaceKind, ip: &str, prefix: u8, gateway: bool) -> Iface {
        Iface {
            index: 4,
            name: "en0".into(),
            label: "Wi-Fi".into(),
            kind,
            v4: vec![(ip.parse().unwrap(), prefix)],
            v6: vec![],
            has_gateway: gateway,
            is_default: gateway,
        }
    }

    #[test]
    fn both_the_new_and_the_old_service_type_are_used_during_the_migration() {
        assert_eq!(SERVICE_TYPES, ["_seamdrop._tcp.local."]);
    }

    #[test]
    fn our_own_echo_is_ignored_but_another_copy_of_the_app_is_not() {
        let me = DeviceId::random();
        let cfg = DiscoveryConfig {
            id: me.clone(),
            name: "Mac".into(),
            os: Os::Macos,
            port: 51000,
            udp_port: 47920,
            mdns: false,
            broadcast: false,
            extra_targets: vec![],
            loopback: true,
        };
        assert!(!is_twin(&cfg, &me, 51000, PROTOCOL_VERSION), "our own announcement");
        assert!(is_twin(&cfg, &me, 47920, 1), "the old app, same identity");
        assert!(is_twin(&cfg, &me, 47920, PROTOCOL_VERSION), "a second copy on another port");
        assert!(!is_twin(&cfg, &DeviceId::random(), 47920, 1), "someone else");
    }

    #[test]
    fn a_home_network_is_swept_except_for_ourselves() {
        let targets = sweep_targets(&[net(IfaceKind::Wifi, "192.168.1.23", 24, true)]);
        assert_eq!(targets.len(), 253);
        assert!(!targets.contains(&"192.168.1.23".parse().unwrap()));
        assert!(!targets.contains(&"192.168.1.0".parse().unwrap()), "network address");
        assert!(!targets.contains(&"192.168.1.255".parse().unwrap()), "broadcast address");
        assert!(targets.contains(&"192.168.1.1".parse().unwrap()));
        assert!(targets.contains(&"192.168.1.254".parse().unwrap()));
    }

    #[test]
    fn the_pc_on_ethernet_sweeps_too() {
        assert_eq!(sweep_targets(&[net(IfaceKind::Ethernet, "10.0.0.7", 24, true)]).len(), 253);
        assert_eq!(sweep_targets(&[net(IfaceKind::Wifi, "192.168.4.10", 28, true)]).len(), 13);
    }

    #[test]
    fn big_public_routerless_and_vpn_networks_are_not_swept() {
        assert!(sweep_targets(&[net(IfaceKind::Wifi, "10.20.30.40", 16, true)]).is_empty(), "school-sized");
        assert!(sweep_targets(&[net(IfaceKind::Wifi, "81.2.69.160", 24, true)]).is_empty(), "public");
        assert!(sweep_targets(&[net(IfaceKind::Ethernet, "169.254.3.4", 16, false)]).is_empty(), "direct link");
        assert!(sweep_targets(&[net(IfaceKind::Vpn, "10.8.0.2", 24, true)]).is_empty(), "VPN");
        assert!(sweep_targets(&[net(IfaceKind::Ethernet, "192.168.1.9", 24, false)]).is_empty(), "no router");
    }
}
