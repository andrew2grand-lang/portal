//! This install's long-lived identity: device id, key pair and self-signed
//! certificate. Peers pin the certificate's fingerprint after pairing.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::model::{DeviceId, Fingerprint};
use crate::util::{from_hex, to_hex};
use crate::{Error, Result};

pub(crate) const IDENTITY_FILE: &str = "identity.json";
pub(crate) const KEY_FILE: &str = "key.bin";

#[derive(Clone)]
pub struct Identity {
    pub device_id: DeviceId,
    pub cert_der: Vec<u8>,
    pub fingerprint: Fingerprint,
    key_der: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
struct Stored {
    device_id: DeviceId,
    cert: String,
}

impl Identity {
    /// Loads the identity from `dir`, creating it on first run. If the key
    /// can't be read back (corrupt file, DPAPI blob from another account), a
    /// new key and certificate are made but the device id is kept, so peers
    /// see "this device's key changed" and pair again.
    pub fn load_or_create(dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(dir)?;
        let stored: Option<Stored> =
            std::fs::read(dir.join(IDENTITY_FILE)).ok().and_then(|b| serde_json::from_slice(&b).ok());

        if let Some(stored) = &stored {
            if let Some(identity) = Self::load(dir, stored) {
                return Ok(identity);
            }
            tracing::warn!("identity key unreadable; generating a new key for the same device id");
        }
        let device_id = stored.map(|s| s.device_id).unwrap_or_else(DeviceId::random);
        Self::create(dir, device_id)
    }

    fn load(dir: &Path, stored: &Stored) -> Option<Self> {
        let cert_der = from_hex(&stored.cert)?;
        let sealed = std::fs::read(dir.join(KEY_FILE)).ok()?;
        let key_der = portal_sys::secret::unprotect(&sealed).ok()?;
        rcgen::KeyPair::try_from(key_der.as_slice()).ok()?;
        Some(Self {
            device_id: stored.device_id.clone(),
            fingerprint: Fingerprint::of_cert(&cert_der),
            cert_der,
            key_der,
        })
    }

    fn create(dir: &Path, device_id: DeviceId) -> Result<Self> {
        let host = format!("{}{}.local", crate::consts::HOST_PREFIX, device_id);
        let certified = rcgen::generate_simple_self_signed(vec![host]).map_err(|e| Error::Cert(e.to_string()))?;
        let cert_der = certified.cert.der().to_vec();
        let key_der = certified.signing_key.serialize_der();

        portal_sys::fs::write_private_file(&dir.join(KEY_FILE), &portal_sys::secret::protect(&key_der)?)?;
        let stored = Stored { device_id: device_id.clone(), cert: to_hex(&cert_der) };
        portal_sys::fs::write_private_file(&dir.join(IDENTITY_FILE), &serde_json::to_vec_pretty(&stored)?)?;

        Ok(Self { device_id, fingerprint: Fingerprint::of_cert(&cert_der), cert_der, key_der })
    }

    /// PKCS#8 DER of the private key.
    pub fn key_der(&self) -> &[u8] {
        &self.key_der
    }
}

impl std::fmt::Debug for Identity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Identity")
            .field("device_id", &self.device_id)
            .field("fingerprint", &self.fingerprint)
            .finish_non_exhaustive()
    }
}
