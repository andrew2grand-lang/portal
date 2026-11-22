//! Device pictures (Mission 05 §2.3): a photo, or a colour and an icon, so
//! devices are easy to tell apart. Shared with paired devices over the
//! encrypted link (protocol 4 `picture`).
//!
//! A picture from another device is checked **before** it's decoded: PNG
//! only (never SVG), its signature, its dimensions from the header (at most
//! 256 × 256) and its size (at most 64 KB); then decoded once, fully, with
//! the `png` crate (it must be a whole, readable image). Anything else falls
//! back to the default picture. Photos chosen here are cropped to a circle,
//! scaled down and re-encoded as PNG by the window (which also drops EXIF
//! and location data: a fresh encode carries none), then checked the same way.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use serde::{Deserialize, Serialize};

pub const MAX_SIDE: u32 = 256;
pub const MAX_BYTES: usize = 64 * 1024;
const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Icon {
    Laptop,
    Desktop,
    Phone,
    Tablet,
}

/// A device's picture.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Picture {
    /// A photo: a PNG, base64 on the wire.
    Photo { png: String },
    /// A colour (0xRRGGBB) and an icon.
    Mark { color: u32, icon: Icon },
}

/// Why a picture wasn't used.
#[derive(Debug, PartialEq, Eq)]
pub enum Bad {
    NotBase64,
    TooBig,
    NotPng,
    TooLarge { width: u32, height: u32 },
    Unreadable,
    Color,
}

/// Checks a PNG: signature, header dimensions and size first, then a full
/// decode. Returns (width, height).
pub fn check_png(bytes: &[u8]) -> Result<(u32, u32), Bad> {
    if bytes.len() > MAX_BYTES {
        return Err(Bad::TooBig);
    }
    // Signature, then the IHDR chunk, which must come first: length 13,
    // "IHDR", width, height (big-endian).
    if bytes.len() < 33 || bytes[..8] != SIGNATURE || &bytes[12..16] != b"IHDR" {
        return Err(Bad::NotPng);
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().expect("4 bytes"));
    let height = u32::from_be_bytes(bytes[20..24].try_into().expect("4 bytes"));
    if width == 0 || height == 0 || width > MAX_SIDE || height > MAX_SIDE {
        return Err(Bad::TooLarge { width, height });
    }
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().map_err(|_| Bad::Unreadable)?;
    let size = reader.output_buffer_size().ok_or(Bad::Unreadable)?;
    // Bounded by the header check above (256 × 256 × 8 bytes at most).
    let mut buf = vec![0u8; size];
    reader.next_frame(&mut buf).map_err(|_| Bad::Unreadable)?;
    Ok((width, height))
}

/// Checks a picture from another device (or before sending ours).
pub fn check(p: &Picture) -> Result<(), Bad> {
    match p {
        Picture::Photo { png } => {
            let bytes = B64.decode(png).map_err(|_| Bad::NotBase64)?;
            check_png(&bytes).map(|_| ())
        }
        Picture::Mark { color, .. } if *color <= 0xFF_FF_FF => Ok(()),
        Picture::Mark { .. } => Err(Bad::Color),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_of(w: u32, h: u32) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut out, w, h);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut wr = enc.write_header().unwrap();
            wr.write_image_data(&vec![200u8; (w * h * 4) as usize]).unwrap();
        }
        out
    }

    #[test]
    fn a_small_png_is_fine() {
        assert_eq!(check_png(&png_of(64, 64)), Ok((64, 64)));
        let p = Picture::Photo { png: B64.encode(png_of(256, 256)) };
        assert_eq!(check(&p), Ok(()));
    }

    #[test]
    fn too_large_too_big_not_png_and_broken_are_refused_before_use() {
        assert_eq!(check_png(&png_of(257, 10)), Err(Bad::TooLarge { width: 257, height: 10 }));
        // A header claiming a huge image is refused from the header alone.
        let mut lying = png_of(8, 8);
        lying[16..20].copy_from_slice(&100_000u32.to_be_bytes());
        assert_eq!(check_png(&lying), Err(Bad::TooLarge { width: 100_000, height: 8 }));
        assert_eq!(check_png(&vec![0u8; MAX_BYTES + 1]), Err(Bad::TooBig));
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg"><script>alert(1)</script></svg>"#;
        assert_eq!(check_png(svg), Err(Bad::NotPng));
        let mut cut = png_of(32, 32);
        cut.truncate(cut.len() - 20);
        assert_eq!(check_png(&cut), Err(Bad::Unreadable));
        assert_eq!(check(&Picture::Photo { png: "not base64!".into() }), Err(Bad::NotBase64));
        assert_eq!(check(&Picture::Mark { color: 0x1_00_00_00, icon: Icon::Laptop }), Err(Bad::Color));
    }
}
