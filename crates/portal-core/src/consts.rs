//! Protocol and product constants.
//!
//! The protocol ids (ALPN `portal/1`, the pairing exporter label, the UDP
//! announce magic, the TLS server name) are deliberately independent of the
//! product name: renaming the app must never stop two versions from finding
//! each other and saying which one to update.

/// The product name, from `productName` in `src-tauri/tauri.conf.json` (the
/// one place it's set; see `build.rs`).
pub const APP_NAME: &str = env!("SEAMDROP_APP_NAME");

/// Bumped on any incompatible wire change. Sent in every hello.
/// 2: SeamDrop (resumable transfers, shared clipboard).
/// 3: Mission 04 (pairing by typed code over SPAKE2; clipboard consent per pair).
/// 4: Mission 05 (liveness, hold and kept, the queue's hand-over, Shelf sync,
///    device pictures).
pub const PROTOCOL_VERSION: u32 = 4;

/// DNS-SD service type (also listed in macOS `NSBonjourServices`).
pub const SERVICE_TYPE: &str = "_seamdrop._tcp.local.";

/// Default TCP listen port and UDP broadcast port. Falls back to a random
/// free port when taken; the real port is always advertised.
pub const DEFAULT_PORT: u16 = 47920;

/// TLS application protocol id.
pub const ALPN: &[u8] = b"portal/1";

/// Prefix of the unique mDNS host name: `seamdrop-<device id>.local.`
pub const HOST_PREFIX: &str = "seamdrop-";

/// Hidden staging folder inside the receive folder (same volume, so the
/// final move is a rename).
pub const INCOMING_DIR: &str = ".seamdrop-incoming";

/// The staging folder of builds before the rename (never resumable: removed).
pub const LEGACY_INCOMING_DIR: &str = ".portal-incoming";

/// App identifier: names the app-data, log and settings folders.
pub const BUNDLE_ID: &str = "com.seamdrop.desktop";

/// The identifier before the rename: where settings, paired devices and keys
/// are migrated from (the folder itself is left in place).
pub const LEGACY_BUNDLE_ID: &str = "com.project1000.portal";
