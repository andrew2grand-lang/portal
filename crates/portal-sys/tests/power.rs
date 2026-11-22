//! The keep-awake guard can be taken and released, repeatedly and on any thread.

use portal_sys::power::KeepAwake;

#[test]
fn keeping_awake_starts_and_stops_cleanly() {
    for _ in 0..3 {
        let guard = KeepAwake::new("Portal test: sending files").expect("the OS allows it");
        drop(guard);
    }
}

#[test]
fn the_guard_can_be_released_on_another_thread() {
    let guard = KeepAwake::new("Portal test: receiving files").expect("the OS allows it");
    std::thread::spawn(move || drop(guard)).join().unwrap();
}

/// On macOS the activity shows up as a power assertion while held.
#[cfg(target_os = "macos")]
#[test]
fn macos_lists_the_assertion_while_it_is_held() {
    let reason = format!("Portal test {}", std::process::id());
    let listed = || {
        let out = std::process::Command::new("pmset").args(["-g", "assertions"]).output().unwrap();
        String::from_utf8_lossy(&out.stdout).contains(&reason)
    };
    let guard = KeepAwake::new(&reason).unwrap();
    assert!(listed(), "assertion missing while held");
    drop(guard);
    assert!(!listed(), "assertion still there after release");
}
