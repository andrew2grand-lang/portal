//! NSPasteboard (the general pasteboard). Safe to use off the main thread.

use objc2::rc::autoreleasepool;
use objc2_app_kit::{
    NSBitmapImageFileType, NSBitmapImageRep, NSPasteboard, NSPasteboardTypeHTML, NSPasteboardTypePNG,
    NSPasteboardTypeRTF, NSPasteboardTypeString, NSPasteboardTypeTIFF,
};
use objc2_foundation::{NSData, NSDictionary, NSString, NSURL};

use super::{Contents, MAC_OURS, MAC_PRIVATE_TYPES, MAC_SOURCE_TYPE, Read};

const FILE_URL: &str = "public.file-url";

pub fn change_count() -> u64 {
    autoreleasepool(|_| NSPasteboard::generalPasteboard().changeCount() as u64)
}

pub fn read() -> Read {
    // The types are checked before the data is read: if the pasteboard
    // changed in between (a password manager copying, say), what was read
    // might belong to an item whose private mark wasn't seen. Read again.
    for _ in 0..3 {
        let before = NSPasteboard::generalPasteboard().changeCount();
        let read = read_once();
        if NSPasteboard::generalPasteboard().changeCount() == before {
            return read;
        }
    }
    Read::Empty
}

/// The plain text, whatever private marks the item carries (Put on Shelf only).
pub fn read_marked_text() -> Option<String> {
    // SAFETY: the AppKit type constant is always a valid NSString.
    autoreleasepool(|_| {
        unsafe { NSPasteboard::generalPasteboard().stringForType(NSPasteboardTypeString) }.map(|s| s.to_string())
    })
}

fn read_once() -> Read {
    autoreleasepool(|_| {
        let pb = NSPasteboard::generalPasteboard();
        let types: Vec<String> = pb.types().map(|a| a.iter().map(|t| t.to_string()).collect()).unwrap_or_default();
        if let Some(marker) = types.iter().find(|t| MAC_PRIVATE_TYPES.contains(&t.as_str())) {
            // The source's bundle id is metadata, never the content
            let source = pb.stringForType(&NSString::from_str(MAC_SOURCE_TYPE)).map(|s| s.to_string());
            return Read::Private { marker: marker.clone(), source };
        }
        if types.iter().any(|t| t == FILE_URL) {
            return Read::Files(file_paths(&pb));
        }
        let ours = types.iter().any(|t| t == MAC_OURS);
        let data = |ty: &NSString| pb.dataForType(ty).map(|d| d.to_vec());
        // SAFETY (all four): the AppKit type constants are always valid NSStrings.
        let contents = unsafe {
            Contents {
                text: pb.stringForType(NSPasteboardTypeString).map(|s| s.to_string()),
                rtf: data(NSPasteboardTypeRTF),
                html: data(NSPasteboardTypeHTML).map(|b| String::from_utf8_lossy(&b).into_owned()),
                png: data(NSPasteboardTypePNG).or_else(|| data(NSPasteboardTypeTIFF).and_then(|t| tiff_to_png(&t))),
            }
        };
        if contents.is_empty() { Read::Empty } else { Read::Contents { contents, ours } }
    })
}

/// Every pasteboard type, never the data; the source app if it says.
pub fn formats() -> super::Formats {
    autoreleasepool(|_| {
        let pb = NSPasteboard::generalPasteboard();
        let names = pb.types().map(|a| a.iter().map(|t| t.to_string()).collect()).unwrap_or_default();
        let source = pb.stringForType(&NSString::from_str(MAC_SOURCE_TYPE)).map(|s| s.to_string());
        super::Formats { names, source, ..Default::default() }
    })
}

pub fn write(c: &Contents, private: bool) -> std::io::Result<u64> {
    autoreleasepool(|_| {
        let pb = NSPasteboard::generalPasteboard();
        pb.clearContents();
        let mut ok = true;
        // SAFETY: as above.
        unsafe {
            if let Some(text) = &c.text {
                ok &= pb.setString_forType(&NSString::from_str(text), NSPasteboardTypeString);
            }
            if let Some(rtf) = &c.rtf {
                ok &= pb.setData_forType(Some(&NSData::with_bytes(rtf)), NSPasteboardTypeRTF);
            }
            if let Some(html) = &c.html {
                ok &= pb.setData_forType(Some(&NSData::with_bytes(html.as_bytes())), NSPasteboardTypeHTML);
            }
            if let Some(png) = &c.png {
                ok &= pb.setData_forType(Some(&NSData::with_bytes(png)), NSPasteboardTypePNG);
                // Some apps only take TIFF.
                if let Some(tiff) = png_to_tiff(png) {
                    pb.setData_forType(Some(&NSData::with_bytes(&tiff)), NSPasteboardTypeTIFF);
                }
            }
        }
        pb.setData_forType(Some(&NSData::new()), &NSString::from_str(MAC_OURS));
        if private {
            for ty in ["org.nspasteboard.ConcealedType", "org.nspasteboard.TransientType"] {
                ok &= pb.setData_forType(Some(&NSData::new()), &NSString::from_str(ty));
            }
        }
        if ok { Ok(pb.changeCount() as u64) } else { Err(std::io::Error::other("the pasteboard refused the data")) }
    })
}

/// Every item's file URL as a path (Finder puts file reference URLs, which
/// NSURL resolves).
fn file_paths(pb: &NSPasteboard) -> Vec<std::path::PathBuf> {
    let Some(items) = pb.pasteboardItems() else { return Vec::new() };
    let file_url = NSString::from_str(FILE_URL);
    items
        .iter()
        .filter_map(|item| {
            let s = item.stringForType(&file_url)?;
            let url = NSURL::URLWithString(&s)?;
            let url = url.filePathURL().unwrap_or(url);
            url.path().map(|p| std::path::PathBuf::from(p.to_string()))
        })
        .collect()
}

fn tiff_to_png(tiff: &[u8]) -> Option<Vec<u8>> {
    let rep = NSBitmapImageRep::imageRepWithData(&NSData::with_bytes(tiff))?;
    // SAFETY: an empty properties dictionary is valid for PNG.
    let png = unsafe { rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new()) }?;
    Some(png.to_vec())
}

fn png_to_tiff(png: &[u8]) -> Option<Vec<u8>> {
    let rep = NSBitmapImageRep::imageRepWithData(&NSData::with_bytes(png))?;
    Some(rep.TIFFRepresentation()?.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tiff_image_comes_out_as_the_same_png_pixels() {
        let rgba: Vec<u8> = (0..4 * 3 * 4).map(|i| (i * 7) as u8).collect();
        let png = super::super::convert::png_from_rgba(
            4,
            3,
            &rgba.iter().enumerate().map(|(i, v)| if i % 4 == 3 { 255 } else { *v }).collect::<Vec<_>>(),
        )
        .unwrap();
        let back = tiff_to_png(&png_to_tiff(&png).unwrap()).unwrap();
        assert_eq!(super::super::convert::png_size(&back), Some((4, 3)));
        assert_eq!(super::super::convert::rgba_of(&back), super::super::convert::rgba_of(&png));
    }

    /// Uses the real pasteboard: `cargo test -p portal-sys -- --ignored real_pasteboard`.
    /// Puts back the text that was there.
    #[test]
    #[ignore = "uses the real clipboard"]
    fn real_pasteboard() {
        let saved = autoreleasepool(|_| unsafe {
            NSPasteboard::generalPasteboard().stringForType(NSPasteboardTypeString).map(|s| s.to_string())
        });

        // What we write comes back, marked as ours.
        let png = super::super::convert::png_from_rgba(1, 1, &[1, 2, 3, 255]).unwrap();
        let c = Contents {
            text: Some("meeting notes".into()),
            rtf: Some(b"{\\rtf1 meeting notes}".to_vec()),
            html: Some("<b>meeting</b> notes".into()),
            png: Some(png),
        };
        let count = super::super::write(&c).unwrap();
        assert_eq!(change_count(), count);
        match super::super::read() {
            Read::Contents { contents, ours } => {
                assert!(ours);
                assert_eq!(contents.text.as_deref(), Some("meeting notes"));
                assert_eq!(contents.html.as_deref(), Some("<b>meeting</b> notes"));
                assert!(contents.rtf.is_some() && contents.png.is_some());
            }
            other => panic!("{other:?}"),
        }

        // Review 04 (privacy): text sent as private is marked as password
        // managers mark theirs, so clipboard managers (and SeamDrop) leave it
        super::super::write_private(&Contents { text: Some("hunter2".into()), ..Default::default() }).unwrap();
        let names = super::super::formats().names;
        for ty in ["org.nspasteboard.ConcealedType", "org.nspasteboard.TransientType"] {
            assert!(names.iter().any(|n| n == ty), "{ty} in {names:?}");
        }
        assert!(matches!(super::super::read(), Read::Private { .. }));

        // A password manager's copy is never read.
        autoreleasepool(|_| unsafe {
            let pb = NSPasteboard::generalPasteboard();
            pb.clearContents();
            pb.setString_forType(&NSString::from_str("hunter2"), NSPasteboardTypeString);
            pb.setData_forType(Some(&NSData::new()), &NSString::from_str("org.nspasteboard.ConcealedType"));
            pb.setString_forType(
                &NSString::from_str("com.agilebits.onepassword7"),
                &NSString::from_str(MAC_SOURCE_TYPE),
            );
        });
        assert_eq!(
            super::super::read(),
            Read::Private {
                marker: "org.nspasteboard.ConcealedType".into(),
                source: Some("com.agilebits.onepassword7".into())
            }
        );

        // Files copied in Finder come back as their paths
        let dir = std::env::temp_dir().join("seamdrop pasteboard test");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("Résumé Q3.pdf");
        std::fs::write(&file, b"x").unwrap();
        autoreleasepool(|_| {
            let pb = NSPasteboard::generalPasteboard();
            pb.clearContents();
            let url = NSURL::fileURLWithPath(&NSString::from_str(&file.display().to_string()));
            pb.setString_forType(&url.absoluteString().unwrap(), &NSString::from_str(FILE_URL));
        });
        // (The path may come back decomposed, as APFS keeps names: the same file.)
        let Read::Files(paths) = super::super::read() else { panic!("files") };
        use std::os::unix::fs::MetadataExt;
        assert_eq!(paths.len(), 1);
        assert_eq!(std::fs::metadata(&paths[0]).unwrap().ino(), std::fs::metadata(&file).unwrap().ino());
        let _ = std::fs::remove_dir_all(&dir);

        // Someone else's copy isn't ours.
        autoreleasepool(|_| unsafe {
            let pb = NSPasteboard::generalPasteboard();
            pb.clearContents();
            pb.setString_forType(&NSString::from_str("from another app"), NSPasteboardTypeString);
        });
        assert!(matches!(super::super::read(), Read::Contents { ours: false, .. }));

        autoreleasepool(|_| unsafe {
            let pb = NSPasteboard::generalPasteboard();
            pb.clearContents();
            if let Some(text) = saved {
                pb.setString_forType(&NSString::from_str(&text), NSPasteboardTypeString);
            }
        });
    }
}
