//! Small, safe wrappers over the OS APIs SeamDrop needs.
//!
//! Everything here is plain Rust or FFI with no C build step, so the Windows
//! code can be type-checked from a Mac with
//! `cargo check -p portal-sys --target x86_64-pc-windows-msvc`.

pub mod clipboard;
pub mod firewall;
pub mod fs;
pub mod host;
pub mod power;
pub mod secret;

#[cfg(windows)]
pub mod windows;
