//! Pairing by typed code (Mission 04 §2): the code, the password it becomes,
//! what the exchange is bound to, and the limits.

use std::time::{Duration, Instant};

use portal_core::model::Fingerprint;
use portal_core::pairing::{FailCap, LOCK_AFTER, Limiter, context, new_code, normalize_code, password};
use portal_core::spake2::{Role, Start};

#[test]
fn codes_are_six_random_digits_with_leading_zeros_kept() {
    let mut seen = std::collections::HashSet::new();
    let mut leading_zero = false;
    for _ in 0..3000 {
        let code = new_code();
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|c| c.is_ascii_digit()), "{code}");
        leading_zero |= code.starts_with('0');
        seen.insert(code);
    }
    assert!(seen.len() > 2980, "codes should almost never repeat: {} unique", seen.len());
    assert!(leading_zero);
}

#[test]
fn a_typed_code_may_have_spaces_but_must_be_six_digits() {
    assert_eq!(normalize_code("048 213").as_deref(), Some("048213"));
    assert_eq!(normalize_code(" 048-213 ").as_deref(), Some("048213"));
    assert_eq!(normalize_code("048213").as_deref(), Some("048213"));
    for bad in ["", "04821", "0482130", "04821a", "048 21"] {
        assert_eq!(normalize_code(bad), None, "{bad:?}");
    }
}

/// Both sides of one exchange, with codes `a_code` and `b_code`, in the
/// given contexts; whether each side accepts the other's confirmation.
fn exchange(
    a_code: &str,
    b_code: &str,
    a_ctx: &portal_core::spake2::Context,
    b_ctx: &portal_core::spake2::Context,
) -> (bool, bool) {
    let a = Start::new(Role::A, password(a_code));
    let b = Start::new(Role::B, password(b_code));
    let ka = a.finish(a_ctx, &b.share()).unwrap();
    let kb = b.finish(b_ctx, &a.share()).unwrap();
    (ka.verify(&kb.confirmation()), kb.verify(&ka.confirmation()))
}

#[test]
fn the_same_code_pairs_and_any_other_code_does_not() {
    let fp = |b| Fingerprint::from_bytes([b; 32]);
    let ctx = context(&[1; 32], fp(2), fp(3));
    assert_eq!(exchange("048213", "048213", &ctx, &ctx), (true, true));
    assert_eq!(exchange("048213", "048214", &ctx, &ctx), (false, false));
    assert_eq!(exchange("148213", "048213", &ctx, &ctx), (false, false));
}

#[test]
fn the_exchange_is_bound_to_both_certificates_and_the_tls_session() {
    let fp = |b| Fingerprint::from_bytes([b; 32]);
    let here = context(&[1; 32], fp(2), fp(3));
    // A man in the middle has two TLS sessions: another exporter, and his
    // own certificate on each side.
    for there in [context(&[9; 32], fp(2), fp(3)), context(&[1; 32], fp(9), fp(3)), context(&[1; 32], fp(2), fp(9))] {
        assert_eq!(exchange("048213", "048213", &here, &there), (false, false));
    }
    // The roles are fixed: swapping client and server is another exchange
    assert_eq!(exchange("048213", "048213", &here, &context(&[1; 32], fp(3), fp(2))), (false, false));
}

#[test]
fn wrong_codes_from_anyone_lock_pairing_until_it_is_allowed_again() {
    let mut cap = FailCap::default();
    for n in 1..LOCK_AFTER {
        cap.failed();
        assert!(!cap.locked(), "{n} wrong codes");
    }
    cap.failed();
    assert!(cap.locked(), "{LOCK_AFTER} wrong codes, whoever typed them");
    cap.failed();
    assert!(cap.locked());
    cap.allow_again();
    assert!(!cap.locked());
    cap.failed();
    assert!(!cap.locked(), "counting starts again from Allow pairing again");
}

#[test]
fn pairing_attempts_are_rate_limited_per_device_and_address() {
    let mut limiter = Limiter::default();
    let t0 = Instant::now();
    for _ in 0..3 {
        assert!(limiter.allow("device-a", t0));
    }
    assert!(!limiter.allow("device-a", t0), "a fourth attempt within 10 minutes is refused");
    assert!(limiter.allow("device-b", t0), "other devices are unaffected");
    assert!(limiter.allow("device-a", t0 + Duration::from_secs(601)), "the window slides");
}

#[test]
fn after_a_cancel_the_same_device_cannot_prompt_for_a_minute() {
    let mut limiter = Limiter::default();
    let t0 = Instant::now();
    limiter.cool_down("device-a", t0);
    assert!(!limiter.allow("device-a", t0 + Duration::from_secs(30)));
    assert!(limiter.allow("device-a", t0 + Duration::from_secs(61)));
}

#[test]
fn stale_limiter_entries_are_forgotten() {
    let mut limiter = Limiter::default();
    let t0 = Instant::now();
    for i in 0..1000 {
        limiter.allow(&format!("key-{i}"), t0);
        limiter.cool_down(&format!("key-{i}"), t0);
    }
    limiter.allow("fresh", t0 + Duration::from_secs(700));
    assert!(limiter.tracked() <= 2, "{} keys still tracked", limiter.tracked());
}
