//! Pixel and markup conversions the clipboard needs, tested on any OS.

use super::convert::*;

/// A PNG of `w`×`h` RGBA pixels.
fn png_of(w: u32, h: u32, rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().unwrap().write_image_data(rgba).unwrap();
    out
}

fn pixels(png_bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let (w, h) = png_size(png_bytes).unwrap();
    (w, h, rgba_of(png_bytes).unwrap())
}

#[test]
fn a_png_survives_the_trip_through_a_windows_bitmap() {
    // 2×2, with see-through pixels.
    let rgba = [255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 0, 10, 20, 30, 255];
    let dib = png_to_dib(&png_of(2, 2, &rgba)).unwrap();
    assert_eq!(&dib[..4], &40u32.to_le_bytes(), "a BITMAPINFOHEADER");
    assert_eq!(pixels(&dib_to_png(&dib).unwrap()), (2, 2, rgba.to_vec()));
}

#[test]
fn a_24_bit_bottom_up_bitmap_with_padded_rows_reads_right() {
    // 3×2, BGR, rows padded to 4 bytes, bottom row first.
    let mut dib = header(40, 3, 2, 24, 0);
    dib.extend_from_slice(&[0, 0, 255, 0, 255, 0, 255, 0, 0, 0, 0, 0]); // bottom: red green blue + pad
    dib.extend_from_slice(&[255, 255, 255, 0, 0, 0, 128, 128, 128, 0, 0, 0]); // top: white black grey + pad
    let (w, h, px) = pixels(&dib_to_png(&dib).unwrap());
    assert_eq!((w, h), (3, 2));
    #[rustfmt::skip]
    let want = [255, 255, 255, 255, 0, 0, 0, 255, 128, 128, 128, 255,
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255];
    assert_eq!(px, want);
}

#[test]
fn a_32_bit_bitmap_without_alpha_is_opaque_and_top_down_works() {
    // 1×2 top-down (negative height), alpha bytes all zero ("unused").
    let mut dib = header(40, 1, -2, 32, 0);
    dib.extend_from_slice(&[1, 2, 3, 0, 4, 5, 6, 0]);
    let (_, _, px) = pixels(&dib_to_png(&dib).unwrap());
    assert_eq!(px, [3, 2, 1, 255, 6, 5, 4, 255]);
}

#[test]
fn a_bitfields_bitmap_and_a_v5_header_are_read() {
    let mut dib = header(40, 1, 1, 32, 3);
    for mask in [0x00ff_0000u32, 0x0000_ff00, 0x0000_00ff] {
        dib.extend_from_slice(&mask.to_le_bytes());
    }
    dib.extend_from_slice(&[9, 8, 7, 255]);
    assert_eq!(pixels(&dib_to_png(&dib).unwrap()).2, [7, 8, 9, 255]);

    let mut v5 = header(124, 1, 1, 32, 3);
    v5.extend_from_slice(&[9, 8, 7, 255]);
    assert_eq!(pixels(&dib_to_png(&v5).unwrap()).2, [7, 8, 9, 255]);
}

#[test]
fn bitmaps_it_cannot_read_are_refused_not_guessed() {
    assert!(dib_to_png(&header(40, 2, 2, 8, 0)).is_none(), "paletted");
    assert!(dib_to_png(&header(40, 2, 2, 24, 1)).is_none(), "compressed");
    assert!(dib_to_png(&header(40, 4000, 4000, 32, 0)).is_none(), "pixel data missing");
    assert!(dib_to_png(&[1, 2, 3]).is_none());
}

/// A DIB header of `size` bytes (40 or 124) with masks for a V5 header.
fn header(size: u32, w: i32, h: i32, bits: u16, compression: u32) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&size.to_le_bytes());
    v.extend_from_slice(&w.to_le_bytes());
    v.extend_from_slice(&h.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&bits.to_le_bytes());
    v.extend_from_slice(&compression.to_le_bytes());
    v.resize(40, 0);
    if size == 124 {
        for mask in [0x00ff_0000u32, 0x0000_ff00, 0x0000_00ff, 0xff00_0000] {
            v.extend_from_slice(&mask.to_le_bytes());
        }
        v.resize(124, 0);
    }
    v
}

#[test]
fn html_round_trips_through_the_windows_html_format() {
    let html = "<b>Meeting</b> notes — café";
    let wrapped = cf_html_wrap(html);
    let text = String::from_utf8(wrapped.clone()).unwrap();
    assert!(text.starts_with("Version:0.9\r\nStartHTML:"));
    assert_eq!(cf_html_fragment(&wrapped).as_deref(), Some(html));
}

#[test]
fn the_fragment_is_found_in_what_browsers_put_on_the_clipboard() {
    let body = "<html><body>\r\n<!--StartFragment--><p>Hello <i>you</i></p><!--EndFragment-->\r\n</body></html>";
    let head = |sh: usize, eh: usize, sf: usize, ef: usize| {
        format!(
            "Version:0.9\r\nStartHTML:{sh:010}\r\nEndHTML:{eh:010}\r\nStartFragment:{sf:010}\r\nEndFragment:{ef:010}\r\nSourceURL:https://example.com/\r\n"
        )
    };
    let h0 = head(0, 0, 0, 0).len();
    let sf = h0 + body.find("<p>").unwrap();
    let ef = h0 + body.find("<!--EndFragment-->").unwrap();
    let data = format!("{}{body}", head(h0, h0 + body.len(), sf, ef));
    assert_eq!(cf_html_fragment(data.as_bytes()).as_deref(), Some("<p>Hello <i>you</i></p>"));
    assert_eq!(cf_html_fragment(b"not the format").as_deref(), None);
}

#[test]
fn a_png_says_its_size() {
    assert_eq!(png_size(&png_of(3, 7, &[0; 3 * 7 * 4])), Some((3, 7)));
    assert_eq!(png_size(b"\x89PNG"), None);
}

/// A DROPFILES block (CF_HDROP) as Explorer puts it on the clipboard:
/// 20-byte header, then NUL-terminated paths, then one more NUL.
fn dropfiles(paths: &[&str], wide: bool) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&20u32.to_le_bytes()); // pFiles
    out.extend_from_slice(&[0; 8]); // pt
    out.extend_from_slice(&0u32.to_le_bytes()); // fNC
    out.extend_from_slice(&u32::from(wide).to_le_bytes()); // fWide
    for p in paths {
        if wide {
            for u in p.encode_utf16().chain([0]) {
                out.extend_from_slice(&u.to_le_bytes());
            }
        } else {
            out.extend_from_slice(p.as_bytes());
            out.push(0);
        }
    }
    out.extend_from_slice(if wide { &[0, 0] } else { &[0] });
    out
}

#[test]
fn files_copied_in_explorer_are_read_as_paths() {
    let paths = [r"C:\Users\Sam\Desktop\Résumé Q3.pdf", r"D:\Photos\2026"];
    assert_eq!(dropfiles_paths(&dropfiles(&paths, true)), paths.map(std::path::PathBuf::from).to_vec());
    assert_eq!(dropfiles_paths(&dropfiles(&[r"C:\a.txt"], false)), vec![std::path::PathBuf::from(r"C:\a.txt")]);
}

#[test]
fn a_broken_file_list_reads_as_no_files() {
    assert!(dropfiles_paths(&[]).is_empty());
    assert!(dropfiles_paths(&[1, 2, 3]).is_empty());
    let mut bad = dropfiles(&[r"C:\a.txt"], true);
    bad[0] = 200; // offset past the end
    assert!(dropfiles_paths(&bad).is_empty());
    // No final NUL: the last path still counts only if it ends
    let mut cut = dropfiles(&[r"C:\a.txt"], true);
    cut.truncate(cut.len() - 4);
    assert!(dropfiles_paths(&cut).is_empty());
}
