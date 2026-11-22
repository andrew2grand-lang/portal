//! SeamDrop's core: discovery, pairing, links, transfers and cross-OS file
//! safety. No Tauri dependency, so everything here runs headless in tests.

pub mod clip;
pub mod consts;
pub mod doctor;
pub mod error;
pub mod events;
pub mod fsafe;
pub mod identity;
pub mod migrate;
pub mod model;
pub mod net;
pub mod node;
pub mod open;
pub mod pairing;
pub mod picture;
pub mod proto;
pub mod shelf;
pub mod spake2;
pub mod store;
pub mod tls;
pub mod transfer;
mod util;

pub use error::{Error, Result};
pub use node::{Config, Core};
