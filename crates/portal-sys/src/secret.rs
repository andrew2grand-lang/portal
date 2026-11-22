//! Protects small secrets (the device's private key) at rest.
//!
//! Windows: DPAPI (`CryptProtectData`, current-user scope), so the key file is
//! useless on another account or machine. macOS: the bytes are stored as-is in
//! a 0600 file (see `fs::write_private_file`); the Keychain waits until SeamDrop
//! has a stable Developer ID signature.

use std::io;

#[cfg(not(windows))]
pub fn protect(secret: &[u8]) -> io::Result<Vec<u8>> {
    Ok(secret.to_vec())
}

#[cfg(not(windows))]
pub fn unprotect(sealed: &[u8]) -> io::Result<Vec<u8>> {
    Ok(sealed.to_vec())
}

#[cfg(windows)]
pub fn protect(secret: &[u8]) -> io::Result<Vec<u8>> {
    crate::windows::dpapi(secret, true)
}

#[cfg(windows)]
pub fn unprotect(sealed: &[u8]) -> io::Result<Vec<u8>> {
    crate::windows::dpapi(sealed, false)
}
