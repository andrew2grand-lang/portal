//! The connection doctor (brief §6.7): from what the node and the OS can
//! see, a short checklist with plain fixes. Every check says passed, failed,
//! likely or unknown, and never claims certainty it doesn't have. Pure
//! function, so each rule is tested with made-up facts.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use serde::Serialize;

use crate::consts::APP_NAME;
use crate::model::Os;
use crate::net::iface::{self, Iface, IfaceKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Fail,
    Likely,
    Unknown,
    Pass,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    pub label: String,
    pub url: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub id: &'static str,
    pub status: Status,
    pub title: String,
    pub fix: Option<String>,
    pub action: Option<Action>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Public,
    Private,
    Domain,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WindowsFacts {
    /// Connected networks: name and Windows' category.
    pub profiles: Vec<(String, Category)>,
    /// NET_FW_PROFILE2_* bits covered by enabled inbound Allow rules for
    /// the app (None: couldn't read the firewall).
    pub allow_profiles: Option<i32>,
    /// Enabled inbound Block rules for the app.
    pub block_rules: Vec<String>,
    /// Controlled folder access switched on.
    pub cfa: Option<bool>,
}

#[derive(Clone, Debug)]
pub struct Facts {
    pub os: Os,
    pub ifaces: Vec<Iface>,
    /// Since the search (re)started.
    pub searching: Duration,
    /// Devices seen at all (paired or not).
    pub peers_seen: usize,
    pub paired_online: bool,
    /// The paired devices connected right now, by name.
    pub connected_names: Vec<String>,
    /// Last-known addresses of paired devices.
    pub known_addrs: Vec<SocketAddr>,
    /// We dialled the last-known addresses and nothing answered.
    pub known_addrs_silent: bool,
    /// An outgoing connection to a local address failed with "no route to host".
    pub host_unreachable: bool,
    /// An outgoing connection to a local address worked recently.
    pub local_dial_ok: bool,
    pub windows: Option<WindowsFacts>,
    pub receive_dir: PathBuf,
    /// The last receive failed because writing was refused.
    pub write_denied: bool,
    /// A paired device pressed Disconnect (its name).
    pub remote_disconnected: Option<String>,
    /// Another copy of the app (the old one, before the rename) is open on
    /// this computer with the same identity.
    pub twin: bool,
}

const MAC_LOCAL_NETWORK: &str = "x-apple.systempreferences:com.apple.preference.security?Privacy_LocalNetwork";
const WIN_NETWORK: &str = "ms-settings:network-status";
const WIN_RANSOMWARE: &str = "windowsdefender://ransomwareprotection/";
/// Handled by the app: repairs SeamDrop's firewall rules (asks for an administrator).
pub const FIX_FIREWALL: &str = "seamdrop:fix-firewall";
const ALL_PROFILES: i32 = 1 | 2 | 4;
const PUBLIC: i32 = 4;
/// How long to look before suggesting the network is the problem (brief §6.1).
const PATIENCE: Duration = Duration::from_secs(10);

fn check(id: &'static str, status: Status, title: impl Into<String>, fix: Option<String>) -> Check {
    Check { id, status, title: title.into(), fix, action: None }
}

fn action(label: &str, url: &str) -> Option<Action> {
    Some(Action { label: label.into(), url: url.into() })
}

pub fn diagnose(f: &Facts) -> Vec<Check> {
    let mut out = Vec::new();
    let usable: Vec<&Iface> = f
        .ifaces
        .iter()
        .filter(|i| matches!(i.kind, IfaceKind::Wifi | IfaceKind::Ethernet | IfaceKind::Bridge))
        .collect();
    let connected = f.paired_online || !f.connected_names.is_empty();
    let long_search = f.searching >= PATIENCE && !connected;

    // What works right now comes first: a live connection rules out most causes
    if !f.connected_names.is_empty() {
        out.push(check(
            "connected",
            Status::Pass,
            format!("Connected to {} right now", crate::util::and_list(&f.connected_names)),
            Some("So this network and this computer’s settings work for SeamDrop.".into()),
        ));
    }

    // The other computer chose to disconnect: nothing here can fix that.
    if let Some(name) = &f.remote_disconnected
        && !connected
    {
        out.push(check(
            "remote-disconnected",
            Status::Fail,
            format!("{name} disconnected from this computer"),
            Some(format!("On {name}, press Connect. The two stay paired, so no code is needed.")),
        ));
    }

    // Two copies of the app answer for this computer: the other one usually
    // holds the port, and the other computer sees two versions at once.
    if f.twin {
        let (this, quit) = match f.os {
            Os::Macos => ("this Mac", "Quit it from the menu bar, then move Portal from Applications to the Trash."),
            Os::Windows => ("this PC", "Quit it from the system tray, then uninstall Portal in Settings → Apps."),
            _ => ("this computer", "Quit it, then remove it."),
        };
        out.push(check(
            "old-app",
            Status::Fail,
            format!("The old Portal app is still open on {this}"),
            Some(format!("{APP_NAME} replaces it and already has its settings and paired devices. {quit}")),
        ));
    }

    // On a network at all?
    if usable.is_empty() {
        out.push(check(
            "network",
            Status::Fail,
            "This computer isn’t on a network",
            Some("Join your Wi-Fi, or plug this computer into your router.".into()),
        ));
    } else {
        let mut kinds: Vec<&str> = Vec::new();
        for i in &usable {
            let word = if i.kind == IfaceKind::Wifi { "Wi-Fi" } else { "Ethernet" };
            if !kinds.contains(&word) {
                kinds.push(word);
            }
        }
        out.push(check("network", Status::Pass, format!("On {}", kinds.join(" and ")), None));
    }

    // Same network as the paired device was last time?
    let remote_v4: Vec<_> = f
        .known_addrs
        .iter()
        .filter_map(|a| match a {
            SocketAddr::V4(v4) if !v4.ip().is_link_local() => Some(*v4.ip()),
            _ => None,
        })
        .collect();
    if !connected && !remote_v4.is_empty() && !usable.is_empty() {
        let same =
            remote_v4.iter().any(|r| usable.iter().any(|i| i.v4.iter().any(|(a, p)| iface::same_subnet(*a, *r, *p))));
        out.push(if same {
            check("same-network", Status::Pass, "Same network as last time", None)
        } else {
            check(
                "same-network",
                Status::Likely,
                "Your other computer was last on a different network",
                Some("Put both on the same home network: its Wi-Fi, or plugged into its router.".into()),
            )
        });
    }

    // macOS Local Network permission: no API, so infer it.
    if f.os == Os::Macos {
        if f.local_dial_ok {
            out.push(check("local-network", Status::Pass, "Local network access is on", None));
        } else if f.host_unreachable && connected {
            // A paired device answers, so the permission can't be off for
            // everything: an asleep one also gives "no route to host"
            let mut c = check(
                "local-network",
                Status::Unknown,
                "Local network access",
                Some(format!(
                    "A device that’s asleep can’t be reached either. If one that’s awake can’t be, check that {APP_NAME} is on in System Settings → Privacy & Security → Local Network."
                )),
            );
            c.action = action("Open Settings", MAC_LOCAL_NETWORK);
            out.push(c);
        } else if f.host_unreachable {
            let here = crate::node::lan_addresses(&f.ifaces).first().map(|(_, ip)| ip.to_string());
            let fallback = match here {
                Some(ip) => format!("Until then, on the PC, use Connect by IP and type this Mac’s address: {ip}."),
                None => "Until then, Connect by IP from the PC to this Mac still works.".into(),
            };
            let mut c = check(
                "local-network",
                Status::Likely,
                format!("{APP_NAME} may need Local Network access on this Mac"),
                Some(format!("Turn {APP_NAME} on in System Settings → Privacy & Security → Local Network. {fallback}")),
            );
            c.action = action("Open Settings", MAC_LOCAL_NETWORK);
            out.push(c);
        } else if long_search {
            let mut c = check(
                "local-network",
                Status::Unknown,
                "Local network access",
                Some("If macOS asked to find devices on your network, choose Allow. You can check in System Settings → Privacy & Security → Local Network.".into()),
            );
            c.action = action("Open Settings", MAC_LOCAL_NETWORK);
            out.push(c);
        }
    }

    // Windows: firewall rules and network profiles.
    if let Some(w) = &f.windows {
        if !w.block_rules.is_empty() {
            let mut c = check(
                "firewall-block",
                Status::Fail,
                format!("A Windows Firewall rule blocks {APP_NAME}"),
                Some(format!(
                    "Usually left by pressing Cancel on a Windows Firewall question about {APP_NAME}. Fix it removes the block ({}) and allows {APP_NAME} again; Windows asks for an administrator first.",
                    w.block_rules.join(", ")
                )),
            );
            c.action = action("Fix it", FIX_FIREWALL);
            out.push(c);
        }
        match w.allow_profiles {
            Some(bits) if bits & ALL_PROFILES == ALL_PROFILES => {
                out.push(check("firewall", Status::Pass, format!("Windows Firewall allows {APP_NAME}"), None))
            }
            Some(0) => {
                let mut c = check(
                    "firewall",
                    Status::Fail,
                    format!("Windows Firewall has no rule for {APP_NAME}"),
                    Some(
                        "Fix it adds the rule the installer makes; Windows asks for an administrator first. Meanwhile this PC can still connect out to the Mac."
                            .to_owned(),
                    ),
                );
                c.action = action("Fix it", FIX_FIREWALL);
                out.push(c);
            }
            Some(_) => {
                let mut c = check(
                    "firewall",
                    Status::Likely,
                    format!("Windows Firewall allows {APP_NAME} on some networks only"),
                    Some(format!(
                        "Fix it restores {APP_NAME}'s rule for private and public networks; Windows asks for an administrator first."
                    )),
                );
                c.action = action("Fix it", FIX_FIREWALL);
                out.push(c);
            }
            None => out.push(check("firewall", Status::Unknown, "Couldn’t read Windows Firewall settings", None)),
        }
        let public_ok = w.allow_profiles.is_some_and(|b| b & PUBLIC != 0) && w.block_rules.is_empty();
        for (name, category) in &w.profiles {
            if *category != Category::Public {
                continue;
            }
            let what = format!("“{name}”");
            let c = if public_ok {
                check(
                    "profile",
                    Status::Pass,
                    format!("{what} is a Public network, and {APP_NAME} is allowed there"),
                    Some("If the Mac still can’t reach this PC, the PC connects out to the Mac instead.".into()),
                )
            } else {
                let mut c = check(
                    "profile",
                    Status::Fail,
                    format!("{what} is set to Public, and Windows blocks {APP_NAME} there"),
                    Some(format!(
                        "Set {name} to Private in Settings → Network & internet, or reinstall {APP_NAME} to restore its firewall rule."
                    )),
                );
                c.action = action("Open network settings", WIN_NETWORK);
                c
            };
            out.push(c);
        }
        // Controlled folder access protects Documents and Desktop.
        let dir = f.receive_dir.to_string_lossy().to_lowercase();
        let protected =
            ["\\documents", "\\desktop", "\\pictures", "\\videos", "\\music"].iter().any(|p| dir.contains(p));
        if protected && (w.cfa == Some(true) || f.write_denied) {
            let mut c = check(
                "cfa",
                Status::Likely,
                format!("Controlled folder access may stop {APP_NAME} saving files"),
                Some(format!(
                    "Choose a receive folder outside Documents and Desktop in Settings (Downloads works), or allow {APP_NAME} in Windows Security → Ransomware protection."
                )),
            );
            c.action = action("Open Windows Security", WIN_RANSOMWARE);
            out.push(c);
        }
    }

    if f.write_denied && !out.iter().any(|c| c.id == "cfa") {
        out.push(check(
            "write",
            Status::Fail,
            format!("{APP_NAME} couldn’t save to the receive folder"),
            Some(format!("Choose another folder in Settings. Current folder: {}", f.receive_dir.display())),
        ));
    }

    // VPN: only one that actually carries traffic counts.
    let vpns: Vec<&Iface> =
        f.ifaces.iter().filter(|i| i.kind == IfaceKind::Vpn && (!i.v4.is_empty() || i.is_default)).collect();
    if let Some(v) = vpns.first() {
        out.push(check(
            "vpn",
            Status::Likely,
            format!("A VPN is on ({})", v.label),
            Some(
                "A VPN can hide the computers on your network from each other. Pause it, or use Connect by IP.".into(),
            ),
        ));
    } else {
        out.push(check("vpn", Status::Pass, "No VPN in the way", None));
    }

    // Router / AP isolation: a likely cause, never a certainty.
    let has_router = usable.iter().any(|i| i.has_gateway);
    // Not once a local connection worked: then a silent computer is likelier asleep
    if long_search
        && has_router
        && f.peers_seen == 0
        && !f.local_dial_ok
        && (f.known_addrs.is_empty() || f.known_addrs_silent)
    {
        out.push(check(
            "isolation",
            Status::Likely,
            "Your network may be keeping devices apart",
            Some("Guest, school and office Wi-Fi often do. Use your home network, or Connect by IP.".into()),
        ));
    }

    if f.peers_seen == 0 && !connected {
        out.push(check(
            "peer-open",
            Status::Unknown,
            format!("Is {APP_NAME} open on the other computer?"),
            Some(format!("Open {APP_NAME} there too. It keeps running from the menu bar or system tray.")),
        ));
    }

    out.sort_by_key(|c| c.status);
    out
}
