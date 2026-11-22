//! Conversions between what travels (PNG, plain HTML) and what each OS's
//! clipboard holds (Windows: device-independent bitmaps and the "HTML
//! Format" with its byte-offset header). Pure, so they're tested anywhere.

use std::io::Cursor;

/// Width and height from a PNG's header.
pub fn png_size(png: &[u8]) -> Option<(u32, u32)> {
    if png.len() < 24 || &png[..8] != b"\x89PNG\r\n\x1a\n" || &png[12..16] != b"IHDR" {
        return None;
    }
    let be = |i: usize| u32::from_be_bytes([png[i], png[i + 1], png[i + 2], png[i + 3]]);
    Some((be(16), be(20)))
}

/// A PNG decoded to 8-bit RGBA pixels, row by row from the top.
pub fn rgba_of(png_bytes: &[u8]) -> Option<Vec<u8>> {
    let mut decoder = png::Decoder::new(Cursor::new(png_bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    buf.truncate(info.buffer_size());
    let px = (info.width as usize) * (info.height as usize);
    Some(match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf.as_chunks::<3>().0.iter().flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => {
            buf.as_chunks::<2>().0.iter().flat_map(|c| [c[0], c[0], c[0], c[1]]).collect()
        }
        png::ColorType::Grayscale => buf.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return None,
    })
    .filter(|v| v.len() == px * 4)
}

/// RGBA pixels (top row first) as a PNG.
pub fn png_from_rgba(w: u32, h: u32, rgba: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.set_compression(png::Compression::Fast);
    enc.write_header().ok()?.write_image_data(rgba).ok()?;
    Some(out)
}

fn le_u32(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?))
}

/// A Windows device-independent bitmap (CF_DIB / CF_DIBV5: header, optional
/// masks, pixels) as a PNG. 24- and 32-bit, uncompressed or bit fields;
/// anything else (palettes, RLE, JPEG inside) is refused rather than guessed.
pub fn dib_to_png(dib: &[u8]) -> Option<Vec<u8>> {
    let header = le_u32(dib, 0)? as usize;
    if header < 40 {
        return None;
    }
    let w = le_u32(dib, 4)? as i32;
    let h = le_u32(dib, 8)? as i32;
    let bits = u16::from_le_bytes(dib.get(14..16)?.try_into().ok()?);
    let compression = le_u32(dib, 16)?;
    if w <= 0 || h == 0 || !(bits == 24 || bits == 32) || !(compression == 0 || compression == 3) {
        return None;
    }
    if compression == 3 && bits != 32 {
        return None;
    }
    let (width, height) = (w as usize, h.unsigned_abs() as usize);
    if width.checked_mul(height)? > 100_000_000 {
        return None;
    }
    // Bit-field masks follow a plain 40-byte header; V4/V5 headers carry them.
    let masks = if compression == 3 && header == 40 { 12 } else { 0 };
    let start = header + masks;
    let stride = (width * bits as usize / 8).div_ceil(4) * 4;
    let pixels = dib.get(start..start.checked_add(stride.checked_mul(height)?)?)?;
    let top_down = h < 0;
    let mut rgba = Vec::with_capacity(width * height * 4);
    let mut any_alpha = false;
    for y in 0..height {
        let row = if top_down { y } else { height - 1 - y };
        let line = &pixels[row * stride..row * stride + stride];
        for x in 0..width {
            let p = &line[x * bits as usize / 8..];
            let a = if bits == 32 { p[3] } else { 255 };
            any_alpha |= bits == 32 && a != 0;
            rgba.extend_from_slice(&[p[2], p[1], p[0], a]);
        }
    }
    // 32-bit bitmaps often leave the fourth byte at 0 meaning "unused".
    if bits == 32 && !any_alpha {
        rgba.as_chunks_mut::<4>().0.iter_mut().for_each(|p| p[3] = 255);
    }
    png_from_rgba(width as u32, height as u32, &rgba)
}

/// A PNG as a 32-bit CF_DIB: a 40-byte header, then bottom-up BGRA rows.
pub fn png_to_dib(png_bytes: &[u8]) -> Option<Vec<u8>> {
    let (w, h) = png_size(png_bytes)?;
    let rgba = rgba_of(png_bytes)?;
    let (width, height) = (w as usize, h as usize);
    let mut dib = Vec::with_capacity(40 + rgba.len());
    dib.extend_from_slice(&40u32.to_le_bytes());
    dib.extend_from_slice(&(w as i32).to_le_bytes());
    dib.extend_from_slice(&(h as i32).to_le_bytes());
    dib.extend_from_slice(&1u16.to_le_bytes());
    dib.extend_from_slice(&32u16.to_le_bytes());
    dib.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
    dib.extend_from_slice(&(rgba.len() as u32).to_le_bytes());
    dib.resize(40, 0);
    for y in (0..height).rev() {
        for p in rgba[y * width * 4..(y + 1) * width * 4].as_chunks::<4>().0 {
            dib.extend_from_slice(&[p[2], p[1], p[0], p[3]]);
        }
    }
    Some(dib)
}

/// Plain HTML as Windows' "HTML Format": a header of byte offsets, then the
/// document with the fragment marked.
pub fn cf_html_wrap(html: &str) -> Vec<u8> {
    const START: &str = "<html><body>\r\n<!--StartFragment-->";
    const END: &str = "<!--EndFragment-->\r\n</body></html>";
    let header = |sh: usize, eh: usize, sf: usize, ef: usize| {
        format!(
            "Version:0.9\r\nStartHTML:{sh:010}\r\nEndHTML:{eh:010}\r\nStartFragment:{sf:010}\r\nEndFragment:{ef:010}\r\n"
        )
    };
    let h = header(0, 0, 0, 0).len();
    let sf = h + START.len();
    let ef = sf + html.len();
    let eh = ef + END.len();
    format!("{}{START}{html}{END}", header(h, eh, sf, ef)).into_bytes()
}

/// The fragment of a Windows "HTML Format" clipboard item.
pub fn cf_html_fragment(data: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(data.split(|b| *b == 0).next()?).ok()?;
    let field = |name: &str| -> Option<usize> {
        let at = text.find(&format!("{name}:"))? + name.len() + 1;
        text[at..].split(['\r', '\n']).next()?.trim().parse().ok()
    };
    let (sf, ef) = (field("StartFragment")?, field("EndFragment")?);
    let frag = data.get(sf..ef)?;
    Some(String::from_utf8_lossy(frag).into_owned())
}

/// The paths in a DROPFILES block (CF_HDROP: files copied in Explorer).
/// Empty if the block is malformed.
pub fn dropfiles_paths(data: &[u8]) -> Vec<std::path::PathBuf> {
    let u32_at = |i: usize| data.get(i..i + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
    let (Some(offset), Some(wide)) = (u32_at(0), u32_at(16)) else { return Vec::new() };
    let Some(list) = data.get(offset as usize..) else { return Vec::new() };
    let mut paths = Vec::new();
    if wide != 0 {
        let units: Vec<u16> = list.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c)).collect();
        let mut rest = &units[..];
        loop {
            let Some(end) = rest.iter().position(|&u| u == 0) else { return Vec::new() };
            if end == 0 {
                return paths;
            }
            paths.push(std::path::PathBuf::from(String::from_utf16_lossy(&rest[..end])));
            rest = &rest[end + 1..];
        }
    } else {
        let mut rest = list;
        loop {
            let Some(end) = rest.iter().position(|&b| b == 0) else { return Vec::new() };
            if end == 0 {
                return paths;
            }
            paths.push(std::path::PathBuf::from(String::from_utf8_lossy(&rest[..end]).into_owned()));
            rest = &rest[end + 1..];
        }
    }
}
