//! SPAKE2-P256-SHA256-HKDF-HMAC against RFC 9382 (Mission 04 §2).
//! `fixtures/rfc9382-vectors.json` is Appendix B of the RFC, as printed
//! there (w, x and y given; AAD empty).

use portal_core::spake2::{Context, Error, Role, SHARE_LEN, Start, scalar_from_be, scalar_from_wide};

fn hex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}

fn b32(s: &str) -> [u8; 32] {
    hex(s).try_into().unwrap()
}

struct Vector {
    ctx: Context,
    w: String,
    x: String,
    y: String,
    pa: String,
    pb: String,
    tt: String,
    ka: String,
    kc_a: String,
    kc_b: String,
    ke: String,
    a_conf: String,
    b_conf: String,
}

fn vectors() -> Vec<Vector> {
    let text = include_str!("fixtures/rfc9382-vectors.json");
    let list: Vec<serde_json::Value> = serde_json::from_str(text).unwrap();
    let get = |v: &serde_json::Value, k: &str| v[k].as_str().unwrap_or_else(|| panic!("{k}")).to_owned();
    list.iter()
        .map(|v| Vector {
            ctx: Context { a: get(v, "A").into_bytes(), b: get(v, "B").into_bytes(), aad: vec![] },
            w: get(v, "w"),
            x: get(v, "x"),
            y: get(v, "y"),
            pa: get(v, "pa"),
            pb: get(v, "pb"),
            tt: get(v, "tt"),
            ka: get(v, "ka"),
            kc_a: get(v, "kca"),
            kc_b: get(v, "kcb"),
            ke: get(v, "ke"),
            a_conf: get(v, "a_conf"),
            b_conf: get(v, "b_conf"),
        })
        .collect()
}

#[test]
fn matches_every_rfc_9382_test_vector() {
    let all = vectors();
    assert_eq!(all.len(), 4);
    for v in all {
        let w = scalar_from_be(&b32(&v.w));
        let a = Start::with_secret(Role::A, w, scalar_from_be(&b32(&v.x)));
        let b = Start::with_secret(Role::B, w, scalar_from_be(&b32(&v.y)));
        assert_eq!(a.share().to_vec(), hex(&v.pa), "pA");
        assert_eq!(b.share().to_vec(), hex(&v.pb), "pB");

        let ka = a.finish(&v.ctx, &b.share()).unwrap();
        let kb = b.finish(&v.ctx, &a.share()).unwrap();
        let (tt, ka_, kc_a, kc_b) = ka.internals();
        assert_eq!(tt, hex(&v.tt).as_slice(), "TT");
        assert_eq!(ka_.to_vec(), hex(&v.ka), "Ka");
        assert_eq!(kc_a.to_vec(), hex(&v.kc_a), "KcA");
        assert_eq!(kc_b.to_vec(), hex(&v.kc_b), "KcB");
        assert_eq!(ka.shared_secret().to_vec(), hex(&v.ke), "Ke");
        assert_eq!(kb.shared_secret(), ka.shared_secret());
        assert_eq!(ka.confirmation().to_vec(), hex(&v.a_conf), "A conf");
        assert_eq!(kb.confirmation().to_vec(), hex(&v.b_conf), "B conf");
        assert!(kb.verify(&ka.confirmation()), "B accepts A's confirmation");
        assert!(ka.verify(&kb.confirmation()), "A accepts B's confirmation");
    }
}

#[test]
fn a_confirmation_echoed_back_is_rejected() {
    let w = scalar_from_wide(&[7; 48]);
    let ctx = Context { a: b"a".to_vec(), b: b"b".to_vec(), aad: b"session".to_vec() };
    let (a, b) = (Start::new(Role::A, w), Start::new(Role::B, w));
    let (ka, kb) = (a.finish(&ctx, &b.share()).unwrap(), b.finish(&ctx, &a.share()).unwrap());
    assert!(!ka.verify(&ka.confirmation()), "A's own confirmation, reflected to A");
    assert!(!kb.verify(&kb.confirmation()), "B's own confirmation, reflected to B");
    assert!(kb.verify(&ka.confirmation()) && ka.verify(&kb.confirmation()));
}

#[test]
fn a_different_password_fails_on_both_sides() {
    let ctx = Context { a: b"a".to_vec(), b: b"b".to_vec(), aad: vec![] };
    let a = Start::new(Role::A, scalar_from_wide(&[1; 48]));
    let b = Start::new(Role::B, scalar_from_wide(&[2; 48]));
    let (ka, kb) = (a.finish(&ctx, &b.share()).unwrap(), b.finish(&ctx, &a.share()).unwrap());
    assert!(!kb.verify(&ka.confirmation()));
    assert!(!ka.verify(&kb.confirmation()));
}

#[test]
fn different_identities_or_data_fail_on_both_sides() {
    let w = scalar_from_wide(&[3; 48]);
    let here = Context { a: b"client fp".to_vec(), b: b"server fp".to_vec(), aad: b"exporter 1".to_vec() };
    for there in [
        Context { a: b"other fp".to_vec(), ..here.clone() },
        Context { b: b"other fp".to_vec(), ..here.clone() },
        Context { aad: b"exporter 2".to_vec(), ..here.clone() },
    ] {
        let (a, b) = (Start::new(Role::A, w), Start::new(Role::B, w));
        let (ka, kb) = (a.finish(&here, &b.share()).unwrap(), b.finish(&there, &a.share()).unwrap());
        assert!(!kb.verify(&ka.confirmation()));
        assert!(!ka.verify(&kb.confirmation()));
    }
}

#[test]
fn shares_that_arent_points_on_the_curve_are_refused() {
    let a = Start::new(Role::A, scalar_from_wide(&[4; 48]));
    let ctx = Context::default();
    let good = Start::new(Role::B, scalar_from_wide(&[4; 48])).share();
    let mut off_curve = good;
    off_curve[64] ^= 1;
    let mut compressed = vec![0x02];
    compressed.extend_from_slice(&good[1..33]);
    for bad in [&off_curve[..], &compressed[..], &[0u8][..], &[0u8; SHARE_LEN][..], &good[..64]] {
        assert_eq!(a.finish(&ctx, bad).err(), Some(Error::BadShare), "{bad:02x?}");
    }
}

#[test]
fn wide_hashes_reduce_mod_the_group_order() {
    let ramp: [u8; 48] = std::array::from_fn(|i| i as u8 + 1);
    // Computed with Python's integers: int.from_bytes(v, 'big') % n
    use p256::elliptic_curve::PrimeField;
    assert_eq!(
        scalar_from_wide(&ramp).to_repr().to_vec(),
        hex("185bb819b6065810af9bb667d1070711af70bf5a9ee0802ebfa3354ec1926898")
    );
    assert_eq!(
        scalar_from_wide(&[0xff; 48]).to_repr().to_vec(),
        hex("431905529c0166ce652e96b7ccca0a99679b73e19ad16947f01cf013fc632550")
    );
}
