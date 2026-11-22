//! Smoke tests for the doctor's Windows probes, run on the Windows CI runner.
#![cfg(windows)]

#[test]
fn network_profiles_can_be_read() {
    let profiles = portal_sys::windows::network_profiles().expect("network list manager");
    println!("{profiles:#?}");
}

#[test]
fn firewall_rules_can_be_read() {
    let exe = std::env::current_exe().unwrap();
    let rules = portal_sys::windows::firewall_rules_for(&exe).expect("firewall policy");
    println!("{rules:?}");
    assert!(rules.block.is_empty(), "a test binary has no block rules");
}

#[test]
fn controlled_folder_access_can_be_queried() {
    println!("{:?}", portal_sys::windows::controlled_folder_access());
}
