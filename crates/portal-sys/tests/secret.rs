use portal_sys::fs::write_private_file;
use portal_sys::secret::{protect, unprotect};

#[test]
fn protected_secret_round_trips() {
    let secret = b"private key bytes \x00\x01\x02";
    let sealed = protect(secret).unwrap();
    assert_eq!(unprotect(&sealed).unwrap(), secret);
}

#[cfg(windows)]
#[test]
fn protected_secret_is_not_stored_in_the_clear_on_windows() {
    let secret = b"a very recognisable secret";
    let sealed = protect(secret).unwrap();
    assert!(!sealed.windows(secret.len()).any(|w| w == secret));
}

#[test]
fn private_file_is_written_and_replaced_atomically() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("key.bin");
    write_private_file(&path, b"one").unwrap();
    write_private_file(&path, b"two").unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"two");
    let leftovers: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
    assert_eq!(leftovers.len(), 1, "no temp files left behind");
}

#[cfg(unix)]
#[test]
fn private_file_is_readable_only_by_its_owner() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("key.bin");
    write_private_file(&path, b"secret").unwrap();
    let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}
