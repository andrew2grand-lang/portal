//! SPAKE2 (RFC 9382), ciphersuite SPAKE2-P256-SHA256-HKDF-HMAC: the
//! password-authenticated key exchange behind pairing by typed code
//! (Mission 04 §2, `docs/PROTOCOL.md`).
//!
//! Written from the RFC and checked against its test vectors (Appendix B;
//! `tests/spake2.rs`). P-256 arithmetic comes from RustCrypto's `p256`; the
//! RustCrypto `spake2` crate is not used: it follows magic-wormhole's
//! variant rather than RFC 9382, and has never been independently audited.
//!
//! Roles are fixed: A is the TLS client, B the TLS server. There is no
//! symmetric mode. Points travel as uncompressed SEC1 (65 bytes).

use p256::elliptic_curve::ops::Reduce;
use p256::elliptic_curve::sec1::{FromSec1Point, ToSec1Point};
use p256::elliptic_curve::{Field, PrimeField};
use p256::{AffinePoint, FieldBytes, ProjectivePoint, PublicKey, Scalar};
use ring::{hkdf, hmac};

/// Length of a share on the wire: an uncompressed SEC1 point.
pub const SHARE_LEN: usize = 65;
/// Length of a key confirmation (HMAC-SHA256).
pub const CONFIRM_LEN: usize = 32;

/// RFC 9382 Table 1, P-256: M and N (compressed SEC1).
const M_HEX: &str = "02886e2f97ace46e55ba9dd7242579f2993b64e16ef3dcab95afd497333d8fa12f";
const N_HEX: &str = "03d8bbd6c639c62937b04d997f38c3770719c629d7014d49a24b4f98baa1292b49";

/// Which side of the exchange this is. A uses M and sends cA; B uses N and
/// sends cB.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// The TLS client
    A,
    /// The TLS server
    B,
}

/// What both sides must agree on besides the password: the identities of A
/// and B, and additional data bound into the confirmation keys.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Context {
    pub a: Vec<u8>,
    pub b: Vec<u8>,
    pub aad: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("the other side's share isn't a valid point")]
    BadShare,
    #[error("the shared point is the identity")]
    Identity,
}

/// One side, after picking its secret: holds x (or y) and its share.
pub struct Start {
    role: Role,
    w: Scalar,
    secret: Scalar,
    share: [u8; SHARE_LEN],
}

/// The keys of a finished exchange: Ke (the shared secret, unused by
/// pairing beyond confirmation) and both confirmation MACs' keys.
pub struct Keys {
    role: Role,
    ke: [u8; 16],
    ka: [u8; 16],
    kc_a: [u8; 16],
    kc_b: [u8; 16],
    tt: Vec<u8>,
}

impl Start {
    /// Picks the secret uniformly from [1, p) with the system's random source.
    pub fn new(role: Role, w: Scalar) -> Self {
        Self::with_secret(role, w, random_scalar())
    }

    /// For the RFC's test vectors: a given secret.
    pub fn with_secret(role: Role, w: Scalar, secret: Scalar) -> Self {
        let x = ProjectivePoint::GENERATOR * secret;
        let blind = match role {
            Role::A => m() * w,
            Role::B => n() * w,
        };
        let share = encode(&(blind + x));
        Self { role, w, secret, share }
    }

    /// pA (as A) or pB (as B).
    pub fn share(&self) -> [u8; SHARE_LEN] {
        self.share
    }

    /// Takes the other side's share and derives the keys.
    pub fn finish(&self, ctx: &Context, theirs: &[u8]) -> Result<Keys, Error> {
        let other = decode(theirs)?;
        // A: K = h*x*(pB - w*N); B: K = h*y*(pA - w*M). P-256's cofactor h is 1.
        let unblind = match self.role {
            Role::A => n() * self.w,
            Role::B => m() * self.w,
        };
        let k = (other - unblind) * self.secret;
        if bool::from(k.to_affine().is_identity()) {
            return Err(Error::Identity);
        }
        let (pa, pb): (&[u8], &[u8]) = match self.role {
            Role::A => (&self.share, theirs),
            Role::B => (theirs, &self.share),
        };
        let mut tt = Vec::with_capacity(512);
        for part in [&ctx.a[..], &ctx.b[..], pa, pb, &encode(&k)[..], &self.w.to_repr()[..]] {
            tt.extend_from_slice(&(part.len() as u64).to_le_bytes());
            tt.extend_from_slice(part);
        }
        // Ke || Ka = Hash(TT); KcA || KcB = HKDF(Ka, nil, "ConfirmationKeys" || AAD)
        let hash = ring::digest::digest(&ring::digest::SHA256, &tt);
        let (mut ke, mut ka) = ([0u8; 16], [0u8; 16]);
        ke.copy_from_slice(&hash.as_ref()[..16]);
        ka.copy_from_slice(&hash.as_ref()[16..]);
        let mut kc = [0u8; 32];
        hkdf::Salt::new(hkdf::HKDF_SHA256, &[])
            .extract(&ka)
            .expand(&[b"ConfirmationKeys", &ctx.aad], Len(32))
            .and_then(|okm| okm.fill(&mut kc))
            .expect("32 bytes is a valid HKDF-SHA256 output length");
        let (mut kc_a, mut kc_b) = ([0u8; 16], [0u8; 16]);
        kc_a.copy_from_slice(&kc[..16]);
        kc_b.copy_from_slice(&kc[16..]);
        Ok(Keys { role: self.role, ke, ka, kc_a, kc_b, tt })
    }
}

impl Keys {
    /// This side's key confirmation: cA = MAC(KcA, TT) as A, cB = MAC(KcB, TT) as B.
    pub fn confirmation(&self) -> [u8; CONFIRM_LEN] {
        let key = match self.role {
            Role::A => &self.kc_a,
            Role::B => &self.kc_b,
        };
        let tag = hmac::sign(&hmac::Key::new(hmac::HMAC_SHA256, key), &self.tt);
        let mut out = [0u8; CONFIRM_LEN];
        out.copy_from_slice(tag.as_ref());
        out
    }

    /// Checks the other side's confirmation, in constant time. Different keys
    /// per role, so this side's own confirmation echoed back never passes.
    pub fn verify(&self, theirs: &[u8]) -> bool {
        let key = match self.role {
            Role::A => &self.kc_b,
            Role::B => &self.kc_a,
        };
        hmac::verify(&hmac::Key::new(hmac::HMAC_SHA256, key), &self.tt, theirs).is_ok()
    }

    /// The shared secret Ke (pairing doesn't need it beyond confirmation).
    pub fn shared_secret(&self) -> [u8; 16] {
        self.ke
    }

    /// For the RFC's test vectors.
    pub fn internals(&self) -> (&[u8], [u8; 16], [u8; 16], [u8; 16]) {
        (&self.tt, self.ka, self.kc_a, self.kc_b)
    }
}

/// A scalar from 48 bytes (a hash 128 bits longer than p, so the reduction
/// mod p has negligible bias, as RFC 9382 §3.2 suggests).
pub fn scalar_from_wide(bytes: &[u8; 48]) -> Scalar {
    // value = hi * 2^192 + lo, each half 192 bits (below p, so exact)
    let half = |b: &[u8]| {
        let mut fb = FieldBytes::default();
        fb[8..].copy_from_slice(b);
        Scalar::from_repr(fb).expect("a 192-bit value is below p")
    };
    let mut two_192 = FieldBytes::default();
    two_192[7] = 1;
    let two_192 = Scalar::from_repr(two_192).expect("2^192 is below p");
    half(&bytes[..24]) * two_192 + half(&bytes[24..])
}

/// A scalar from its 32-byte big-endian form, reduced mod p (test vectors).
pub fn scalar_from_be(bytes: &[u8; 32]) -> Scalar {
    let mut fb = FieldBytes::default();
    fb.copy_from_slice(bytes);
    <Scalar as Reduce<FieldBytes>>::reduce(&fb)
}

fn random_scalar() -> Scalar {
    loop {
        let bytes = crate::util::random_bytes::<32>();
        let mut fb = FieldBytes::default();
        fb.copy_from_slice(&bytes);
        // Rejection sampling: uniform in [1, p)
        if let Some(s) = Option::<Scalar>::from(Scalar::from_repr(fb))
            && !bool::from(s.is_zero())
        {
            return s;
        }
    }
}

fn m() -> ProjectivePoint {
    fixed(M_HEX)
}

fn n() -> ProjectivePoint {
    fixed(N_HEX)
}

fn fixed(hex: &str) -> ProjectivePoint {
    let bytes = crate::util::from_hex(hex).expect("RFC 9382 constant");
    PublicKey::from_sec1_bytes(&bytes).expect("RFC 9382 constant is on the curve").to_projective()
}

fn encode(p: &ProjectivePoint) -> [u8; SHARE_LEN] {
    let point = p.to_affine().to_sec1_point(false);
    let mut out = [0u8; SHARE_LEN];
    out.copy_from_slice(point.as_bytes());
    out
}

/// Only uncompressed points on the curve, never the identity.
fn decode(bytes: &[u8]) -> Result<ProjectivePoint, Error> {
    if bytes.len() != SHARE_LEN || bytes[0] != 0x04 {
        return Err(Error::BadShare);
    }
    let sec1 = p256::Sec1Point::from_bytes(bytes).map_err(|_| Error::BadShare)?;
    let point: Option<AffinePoint> = AffinePoint::from_sec1_point(&sec1).into();
    match point {
        Some(p) if !bool::from(p.is_identity()) => Ok(p.into()),
        _ => Err(Error::BadShare),
    }
}

/// HKDF output length for ring.
struct Len(usize);

impl hkdf::KeyType for Len {
    fn len(&self) -> usize {
        self.0
    }
}
