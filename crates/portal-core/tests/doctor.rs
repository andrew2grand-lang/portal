//! The connection doctor's reasoning, from fake facts (brief §6.7).

use std::time::Duration;

use portal_core::doctor::{Category, Facts, Status, WindowsFacts, diagnose};
use portal_core::model::Os;
use portal_core::net::iface::{Iface, IfaceKind};

fn wifi() -> Iface {
    Iface {
        index: 5,
        name: "en0".into(),
        label: "Wi-Fi".into(),
        kind: IfaceKind::Wifi,
        v4: vec![("192.168.1.24".parse().unwrap(), 24)],
        v6: vec![],
        has_gateway: true,
        is_default: true,
    }
}

fn vpn() -> Iface {
    Iface {
        index: 20,
        name: "utun7".into(),
        label: "utun7".into(),
        kind: IfaceKind::Vpn,
        v4: vec![("10.8.0.2".parse().unwrap(), 24)],
        v6: vec![],
        has_gateway: false,
        is_default: false,
    }
}

fn facts(os: Os) -> Facts {
    Facts {
        os,
        ifaces: vec![wifi()],
        searching: Duration::from_secs(12),
        peers_seen: 0,
        paired_online: false,
        connected_names: vec![],
        known_addrs: vec![],
        known_addrs_silent: false,
        host_unreachable: false,
        local_dial_ok: false,
        windows: None,
        receive_dir: "/Users/sam/Downloads/SeamDrop".into(),
        write_denied: false,
        remote_disconnected: None,
        twin: false,
    }
}

fn check<'a>(checks: &'a [portal_core::doctor::Check], id: &str) -> Option<&'a portal_core::doctor::Check> {
    checks.iter().find(|c| c.id == id)
}

#[test]
fn no_network_at_all_is_a_plain_failure() {
    let mut f = facts(Os::Macos);
    f.ifaces.clear();
    let c = diagnose(&f);
    assert_eq!(check(&c, "network").unwrap().status, Status::Fail);
    assert_eq!(c[0].id, "network", "problems come first");
}

#[test]
fn being_on_wifi_passes() {
    let c = diagnose(&facts(Os::Macos));
    assert_eq!(check(&c, "network").unwrap().status, Status::Pass);
    assert!(check(&c, "network").unwrap().title.contains("Wi-Fi"));
}

#[test]
fn a_silent_network_with_a_router_is_flagged_as_likely_isolation_not_certain() {
    let mut f = facts(Os::Macos);
    f.known_addrs = vec!["192.168.1.31:47920".parse().unwrap()];
    f.known_addrs_silent = true;
    let c = diagnose(&f);
    let iso = check(&c, "isolation").unwrap();
    assert_eq!(iso.status, Status::Likely);
    assert!(iso.fix.as_ref().unwrap().contains("Connect by IP"));
}

#[test]
fn isolation_is_not_suggested_before_ten_seconds_or_once_something_is_seen() {
    let mut f = facts(Os::Macos);
    f.searching = Duration::from_secs(4);
    assert!(check(&diagnose(&f), "isolation").is_none());
    let mut f = facts(Os::Macos);
    f.peers_seen = 1;
    assert!(check(&diagnose(&f), "isolation").is_none());
}

/// Review 04 (beginner): a computer that answered earlier and now doesn't is
/// most likely asleep; the network isn't blamed for it
#[test]
fn isolation_is_not_blamed_once_a_local_connection_worked() {
    let mut f = facts(Os::Macos);
    f.known_addrs = vec!["192.168.1.31:47920".parse().unwrap()];
    f.known_addrs_silent = true;
    f.local_dial_ok = true;
    assert!(check(&diagnose(&f), "isolation").is_none());
}

#[test]
fn macos_local_network_denial_is_inferred_from_host_unreachable() {
    let mut f = facts(Os::Macos);
    f.host_unreachable = true;
    let c = diagnose(&f);
    let ln = check(&c, "local-network").unwrap();
    assert_eq!(ln.status, Status::Likely);
    assert!(ln.action.as_ref().unwrap().url.starts_with("x-apple.systempreferences:"));
    let fix = ln.fix.as_ref().unwrap();
    assert!(fix.contains("Connect by IP"), "says the PC can still reach this Mac");
    let ip = f.ifaces[0].v4[0].0.to_string();
    assert!(fix.contains(&format!("this Mac’s address: {ip}")), "and what to type there: {fix}");
}

#[test]
fn a_working_local_connection_clears_the_local_network_suspicion() {
    let mut f = facts(Os::Macos);
    f.host_unreachable = true;
    f.local_dial_ok = true;
    assert_eq!(check(&diagnose(&f), "local-network").unwrap().status, Status::Pass);
}

#[test]
fn the_local_network_check_is_macos_only() {
    assert!(check(&diagnose(&facts(Os::Windows)), "local-network").is_none());
}

fn windows(allow: Option<i32>, block: Vec<&str>, profiles: Vec<(&str, Category)>) -> WindowsFacts {
    WindowsFacts {
        profiles: profiles.into_iter().map(|(n, c)| (n.to_string(), c)).collect(),
        allow_profiles: allow,
        block_rules: block.into_iter().map(String::from).collect(),
        cfa: Some(false),
    }
}

#[test]
fn a_firewall_block_rule_is_a_failure_with_how_to_remove_it() {
    let mut f = facts(Os::Windows);
    f.windows = Some(windows(Some(7), vec!["SeamDrop"], vec![("Home", Category::Private)]));
    let c = diagnose(&f);
    let block = check(&c, "firewall-block").unwrap();
    assert_eq!(block.status, Status::Fail);
    // Likely from a Cancel on Windows' question; one button repairs it
    let fix = block.fix.as_ref().unwrap();
    assert!(fix.contains("Cancel") && fix.contains("administrator"), "{fix}");
    let action = block.action.as_ref().unwrap();
    assert_eq!((action.label.as_str(), action.url.as_str()), ("Fix it", "seamdrop:fix-firewall"));
}

#[test]
fn a_missing_allow_rule_fails_and_a_partial_one_is_likely() {
    let mut f = facts(Os::Windows);
    f.windows = Some(windows(Some(0), vec![], vec![]));
    let missing = diagnose(&f);
    assert_eq!(check(&missing, "firewall").unwrap().status, Status::Fail);
    assert_eq!(check(&missing, "firewall").unwrap().action.as_ref().unwrap().url, "seamdrop:fix-firewall");
    f.windows = Some(windows(Some(2), vec![], vec![]));
    let partial = diagnose(&f);
    assert_eq!(check(&partial, "firewall").unwrap().status, Status::Likely);
    assert_eq!(check(&partial, "firewall").unwrap().action.as_ref().unwrap().url, "seamdrop:fix-firewall");
    f.windows = Some(windows(Some(7), vec![], vec![]));
    assert_eq!(check(&diagnose(&f), "firewall").unwrap().status, Status::Pass);
}

#[test]
fn a_public_network_is_explained_and_only_a_problem_without_the_public_rule() {
    let mut f = facts(Os::Windows);
    f.windows = Some(windows(Some(7), vec![], vec![("Home", Category::Public)]));
    let ok = diagnose(&f);
    let p = check(&ok, "profile").unwrap();
    assert_eq!(p.status, Status::Pass);
    assert!(p.title.contains("Public"));
    f.windows = Some(windows(Some(3), vec![], vec![("Home", Category::Public)]));
    let bad = diagnose(&f);
    let p = check(&bad, "profile").unwrap();
    assert_eq!(p.status, Status::Fail);
    assert!(p.fix.as_ref().unwrap().contains("Private"));
}

#[test]
fn controlled_folder_access_is_flagged_only_for_protected_folders() {
    let mut f = facts(Os::Windows);
    f.receive_dir = "C:\\Users\\Sam\\Documents\\SeamDrop".into();
    f.windows = Some(WindowsFacts { cfa: Some(true), ..windows(Some(7), vec![], vec![]) });
    assert_eq!(check(&diagnose(&f), "cfa").unwrap().status, Status::Likely);
    f.receive_dir = "C:\\Users\\Sam\\Downloads\\SeamDrop".into();
    assert!(check(&diagnose(&f), "cfa").is_none());
}

#[test]
fn a_refused_write_is_a_failure_on_any_system() {
    let mut f = facts(Os::Macos);
    f.write_denied = true;
    assert_eq!(check(&diagnose(&f), "write").unwrap().status, Status::Fail);
}

#[test]
fn a_vpn_with_an_address_is_flagged_but_idle_tunnels_are_not() {
    let mut f = facts(Os::Macos);
    f.ifaces.push(vpn());
    assert_eq!(check(&diagnose(&f), "vpn").unwrap().status, Status::Likely);
    let f = facts(Os::Macos);
    assert_eq!(check(&diagnose(&f), "vpn").unwrap().status, Status::Pass);
}

#[test]
fn a_peer_last_seen_on_another_subnet_is_pointed_out() {
    let mut f = facts(Os::Macos);
    f.known_addrs = vec!["10.0.0.31:47920".parse().unwrap()];
    assert_eq!(check(&diagnose(&f), "same-network").unwrap().status, Status::Likely);
    f.known_addrs = vec!["192.168.1.31:47920".parse().unwrap()];
    assert_eq!(check(&diagnose(&f), "same-network").unwrap().status, Status::Pass);
}

#[test]
fn once_connected_the_doctor_only_reports_real_problems() {
    let mut f = facts(Os::Macos);
    f.paired_online = true;
    f.peers_seen = 1;
    let c = diagnose(&f);
    assert!(check(&c, "isolation").is_none());
    assert!(check(&c, "peer-open").is_none());
}

/// Review 04 (developer): with paired devices connected right now, the check
/// says so first, and "no route" to an asleep one isn't blamed on macOS
#[test]
fn a_live_connection_is_the_first_fact_and_softens_local_network_doubts() {
    let mut f = facts(Os::Macos);
    f.paired_online = true;
    f.peers_seen = 3;
    f.connected_names = vec!["Desk PC".into(), "MacBook Air".into()];
    f.host_unreachable = true; // dialling the asleep laptop
    let c = diagnose(&f);
    let live = check(&c, "connected").expect("names what works");
    assert_eq!(live.status, Status::Pass);
    assert!(live.title.contains("Desk PC and MacBook Air"), "{}", live.title);
    assert_eq!(c.iter().find(|c| c.status == Status::Pass).map(|c| c.id), Some("connected"), "{c:#?}");
    assert_eq!(check(&c, "local-network").map(|c| c.status), Some(Status::Unknown));
}

#[test]
fn no_advice_mentions_cables_any_more() {
    let mut cases = vec![facts(Os::Macos), facts(Os::Windows)];
    let mut offline = facts(Os::Macos);
    offline.ifaces.clear();
    cases.push(offline);
    let mut moved = facts(Os::Macos);
    moved.known_addrs = vec!["10.9.9.9:47920".parse().unwrap()];
    cases.push(moved);
    let mut isolated = facts(Os::Macos);
    isolated.known_addrs = vec!["192.168.1.31:47920".parse().unwrap()];
    isolated.known_addrs_silent = true;
    cases.push(isolated);
    for f in cases {
        for c in diagnose(&f) {
            let text = format!("{} {}", c.title, c.fix.unwrap_or_default()).to_lowercase();
            assert!(!text.contains("cable"), "{text}");
        }
    }
}

#[test]
fn a_device_that_disconnected_from_us_is_named_with_the_way_back() {
    let mut f = facts(Os::Windows);
    f.remote_disconnected = Some("MacBook Pro".into());
    let c = diagnose(&f);
    let d = check(&c, "remote-disconnected").unwrap();
    assert_eq!(d.status, Status::Fail);
    assert!(d.title.contains("MacBook Pro"));
    assert!(d.fix.as_ref().unwrap().contains("press Connect"));
    assert_eq!(c[0].id, "remote-disconnected", "first: it explains everything else");
    f.paired_online = true;
    assert!(check(&diagnose(&f), "remote-disconnected").is_none(), "gone once connected");
}

#[test]
fn the_old_app_still_open_on_this_computer_is_named_with_how_to_remove_it() {
    for (os, this, how) in [(Os::Macos, "this Mac", "menu bar"), (Os::Windows, "this PC", "Settings → Apps")] {
        let mut f = facts(os);
        assert!(check(&diagnose(&f), "old-app").is_none());
        f.twin = true;
        let c = diagnose(&f);
        let d = check(&c, "old-app").unwrap();
        assert_eq!(d.status, Status::Fail);
        assert_eq!(d.title, format!("The old Portal app is still open on {this}"));
        let fix = d.fix.as_ref().unwrap();
        assert!(fix.contains("SeamDrop replaces it") && fix.contains(how), "{fix}");
    }
}

#[test]
fn every_check_names_the_app_by_its_current_name() {
    let mut f = facts(Os::Windows);
    f.windows = Some(WindowsFacts {
        profiles: vec![("Cafe".into(), Category::Public)],
        allow_profiles: Some(0),
        block_rules: vec![],
        cfa: Some(true),
    });
    f.receive_dir = "C:\\Users\\Sam\\Documents\\SeamDrop".into();
    f.write_denied = true;
    let all: String =
        diagnose(&f).iter().map(|c| format!("{} {} ", c.title, c.fix.clone().unwrap_or_default())).collect();
    assert!(all.contains("SeamDrop"), "{all}");
    assert!(!all.contains("Portal"), "{all}");
}
