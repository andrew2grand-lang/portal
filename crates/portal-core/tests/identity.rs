use portal_core::identity::Identity;

#[test]
fn first_run_creates_an_identity_and_later_runs_load_the_same_one() {
    let dir = tempfile::tempdir().unwrap();

    let first = Identity::load_or_create(dir.path()).unwrap();
    let again = Identity::load_or_create(dir.path()).unwrap();

    assert_eq!(first.device_id.as_str().len(), 32);
    assert!(first.device_id.as_str().chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    assert_eq!(first.device_id, again.device_id);
    assert_eq!(first.fingerprint, again.fingerprint);
    assert_eq!(first.cert_der, again.cert_der);
}

#[test]
fn two_installs_get_different_ids_and_keys() {
    let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let a = Identity::load_or_create(a.path()).unwrap();
    let b = Identity::load_or_create(b.path()).unwrap();
    assert_ne!(a.device_id, b.device_id);
    assert_ne!(a.fingerprint, b.fingerprint);
}

#[test]
fn fingerprint_is_sha256_of_the_certificate() {
    let dir = tempfile::tempdir().unwrap();
    let id = Identity::load_or_create(dir.path()).unwrap();
    let expected = ring::digest::digest(&ring::digest::SHA256, &id.cert_der);
    assert_eq!(id.fingerprint.as_bytes(), expected.as_ref());
    assert_eq!(id.fingerprint.to_hex().len(), 64);
}

#[test]
fn an_unreadable_key_gets_a_new_key_but_keeps_the_device_id() {
    let dir = tempfile::tempdir().unwrap();
    let first = Identity::load_or_create(dir.path()).unwrap();
    std::fs::write(dir.path().join("key.bin"), b"garbage").unwrap();

    let second = Identity::load_or_create(dir.path()).unwrap();

    assert_eq!(first.device_id, second.device_id);
    assert_ne!(first.fingerprint, second.fingerprint);
}
