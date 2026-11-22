//! Value types shared by the core and its callers.

use serde::{Deserialize, Serialize};

use crate::util::{from_hex, random_bytes, to_hex};

/// A device's permanent id: 16 random bytes as 32 lowercase hex chars.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DeviceId(String);

impl DeviceId {
    pub fn random() -> Self {
        Self(to_hex(&random_bytes::<16>()))
    }

    pub fn parse(s: &str) -> Option<Self> {
        let ok = s.len() == 32 && s.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c));
        ok.then(|| Self(s.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for DeviceId {
    type Error = String;
    fn try_from(s: String) -> Result<Self, String> {
        Self::parse(&s).ok_or_else(|| format!("invalid device id {s:?}"))
    }
}

impl From<DeviceId> for String {
    fn from(id: DeviceId) -> String {
        id.0
    }
}

impl std::fmt::Display for DeviceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::fmt::Debug for DeviceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DeviceId({})", &self.0[..8])
    }
}

/// SHA-256 of a certificate's DER bytes.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Fingerprint([u8; 32]);

impl Fingerprint {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn of_cert(der: &[u8]) -> Self {
        let digest = ring::digest::digest(&ring::digest::SHA256, der);
        let mut out = [0u8; 32];
        out.copy_from_slice(digest.as_ref());
        Self(out)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        to_hex(&self.0)
    }

    /// Constant-time comparison (no early exit on the first differing byte).
    pub fn ct_eq(&self, other: &Fingerprint) -> bool {
        use subtle::ConstantTimeEq;
        self.0.ct_eq(&other.0).into()
    }
}

impl TryFrom<String> for Fingerprint {
    type Error = String;
    fn try_from(s: String) -> Result<Self, String> {
        let bytes = from_hex(&s).filter(|b| b.len() == 32).ok_or("invalid fingerprint")?;
        let mut out = [0u8; 32];
        out.copy_from_slice(&bytes);
        Ok(Self(out))
    }
}

impl From<Fingerprint> for String {
    fn from(f: Fingerprint) -> String {
        f.to_hex()
    }
}

impl std::fmt::Debug for Fingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Fingerprint({}…)", &self.to_hex()[..12])
    }
}

/// What this computer does with files from one paired device
/// ("Files from <device>" in Settings → Devices). Mission 02 §7.1.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Incoming {
    /// Ask every time (the default).
    #[default]
    Ask,
    /// Always accept.
    Always,
    /// Never: refuse, and tell the sender why.
    Never,
}

/// Settings → Appearance (Mission 04 §1.7).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Appearance {
    /// Follows the system (the default)
    #[default]
    System,
    Light,
    Dark,
}

/// Where clipboard items a paired device sends end up here (Mission 04 §4.1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClipTo {
    /// Straight to the clipboard, and on the Shelf (the default: if you
    /// pressed Send, you usually want to paste).
    #[default]
    Clipboard,
    /// The Shelf only: what's copied here stays.
    Shelf,
}

/// Send / Both / Receive: Mission 01's per-computer roles. No longer a
/// setting; kept on the wire because Mission 01 builds still read it
/// (a computer that refuses our files says Send).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Send,
    #[default]
    Both,
    Receive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Os {
    Macos,
    Windows,
    Linux,
    #[serde(other)]
    Other,
}

impl Os {
    pub fn parse(s: &str) -> Os {
        match s {
            "macos" => Os::Macos,
            "windows" => Os::Windows,
            "linux" => Os::Linux,
            _ => Os::Other,
        }
    }

    pub fn current() -> Os {
        if cfg!(target_os = "macos") {
            Os::Macos
        } else if cfg!(windows) {
            Os::Windows
        } else if cfg!(target_os = "linux") {
            Os::Linux
        } else {
            Os::Other
        }
    }
}
