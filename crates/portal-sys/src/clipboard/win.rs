//! The Win32 clipboard. Reading opens it with no owner; writing needs an
//! owner window (EmptyClipboard with none makes SetClipboardData fail), so a
//! message-only window is made for each write and destroyed after.

use std::time::Duration;

use windows::Win32::Foundation::{CloseHandle, GlobalFree, HANDLE, HGLOBAL, HWND};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData, GetClipboardFormatNameW, GetClipboardOwner,
    GetClipboardSequenceNumber, IsClipboardFormatAvailable, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
};
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock};
use windows::Win32::System::Ole::{CF_DIB, CF_DIBV5, CF_HDROP, CF_UNICODETEXT};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, HWND_MESSAGE, WINDOW_EX_STYLE, WINDOW_STYLE,
};
use windows::core::{HSTRING, PWSTR, w};

use super::convert;
use super::{Contents, Read, WINDOWS_OPT_OUT_FORMATS, WINDOWS_OURS, WINDOWS_PRIVATE_FORMATS};

fn format(name: &str) -> u32 {
    // SAFETY: a valid, NUL-terminated wide string.
    unsafe { RegisterClipboardFormatW(&HSTRING::from(name)) }
}

pub fn change_count() -> u64 {
    // SAFETY: no arguments, no state.
    u64::from(unsafe { GetClipboardSequenceNumber() })
}

/// Holds the clipboard open; closes it when dropped.
struct Open;

impl Open {
    /// Another app may hold it for a moment: try for about half a second.
    fn new(owner: Option<HWND>) -> std::io::Result<Open> {
        for _ in 0..25 {
            // SAFETY: `owner` is null or a window this thread created.
            if unsafe { OpenClipboard(owner) }.is_ok() {
                return Ok(Open);
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        Err(std::io::Error::other("another app is holding the clipboard"))
    }
}

impl Drop for Open {
    fn drop(&mut self) {
        // SAFETY: we opened it.
        let _ = unsafe { CloseClipboard() };
    }
}

fn has(fmt: u32) -> bool {
    // SAFETY: the clipboard is open.
    unsafe { IsClipboardFormatAvailable(fmt) }.is_ok()
}

/// The bytes of one format (the clipboard must be open).
fn bytes(fmt: u32) -> Option<Vec<u8>> {
    // SAFETY: the clipboard is open; the handle is the clipboard's, locked
    // only while copied and never freed by us.
    unsafe {
        let handle = GetClipboardData(fmt).ok()?;
        let global = HGLOBAL(handle.0);
        let ptr = GlobalLock(global) as *const u8;
        if ptr.is_null() {
            return None;
        }
        let out = std::slice::from_raw_parts(ptr, GlobalSize(global)).to_vec();
        let _ = GlobalUnlock(global);
        Some(out)
    }
}

/// Text up to its terminating NUL.
fn cut_at_nul(mut b: Vec<u8>) -> Vec<u8> {
    if let Some(i) = b.iter().position(|&c| c == 0) {
        b.truncate(i);
    }
    b
}

/// The .exe name of the app that put what's on the clipboard, if Windows
/// says (its window may be gone). Never the content.
fn owner_app() -> Option<String> {
    // SAFETY: plain Win32 queries; the process handle is closed before returning.
    unsafe {
        let hwnd = GetClipboardOwner().ok()?;
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let named = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len);
        let _ = CloseHandle(process);
        named.ok()?;
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        path.rsplit('\\').next().filter(|n| !n.is_empty()).map(str::to_owned)
    }
}

/// Every format's name (standard ones by their CF_ name), never the data;
/// the two "Can…" DWORDs; the owner app.
pub fn formats() -> super::Formats {
    let Ok(_open) = Open::new(None) else { return super::Formats::default() };
    let mut out = super::Formats { source: owner_app(), ..Default::default() };
    let mut fmt = 0u32;
    loop {
        // SAFETY: the clipboard is open; 0 starts the enumeration.
        fmt = unsafe { EnumClipboardFormats(fmt) };
        if fmt == 0 {
            break;
        }
        let mut buf = [0u16; 256];
        // SAFETY: buf is valid for its length.
        let len = unsafe { GetClipboardFormatNameW(fmt, &mut buf) };
        let name = if len > 0 { String::from_utf16_lossy(&buf[..len as usize]) } else { standard_name(fmt) };
        let dword = || bytes(fmt).filter(|b| b.len() >= 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
        match name.as_str() {
            "CanIncludeInClipboardHistory" => out.can_include_in_history = dword(),
            "CanUploadToCloudClipboard" => out.can_upload_to_cloud = dword(),
            _ => {}
        }
        out.names.push(name);
    }
    out
}

fn standard_name(fmt: u32) -> String {
    match fmt {
        1 => "CF_TEXT".into(),
        2 => "CF_BITMAP".into(),
        7 => "CF_OEMTEXT".into(),
        8 => "CF_DIB".into(),
        13 => "CF_UNICODETEXT".into(),
        15 => "CF_HDROP".into(),
        16 => "CF_LOCALE".into(),
        17 => "CF_DIBV5".into(),
        n => format!("format {n}"),
    }
}

/// The plain text, whatever private formats the item carries (Put on Shelf only).
pub fn read_marked_text() -> Option<String> {
    let b = {
        let _open = Open::new(None).ok()?;
        bytes(u32::from(CF_UNICODETEXT.0))?
    };
    let units: Vec<u16> = b.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c)).take_while(|&u| u != 0).collect();
    Some(String::from_utf16_lossy(&units))
}

pub fn read() -> Read {
    // Copy what's needed while the clipboard is open, and convert after
    // closing it: other apps can't use the clipboard while it's open here.
    let (text, rtf, html, png, dib, ours) = {
        // Another app holds it past the half second: say busy, not empty, so
        // the caller tries again (Mission 05 §1.9).
        let Ok(_open) = Open::new(None) else { return Read::Busy };
        if let Some(marker) = WINDOWS_PRIVATE_FORMATS.iter().find(|f| has(format(f))) {
            return Read::Private { marker: (*marker).to_owned(), source: owner_app() };
        }
        // A DWORD 0 in either means "don't keep or sync this".
        let opted_out = WINDOWS_OPT_OUT_FORMATS.iter().find(|f| {
            let fmt = format(f);
            has(fmt) && bytes(fmt).is_some_and(|b| b.len() >= 4 && b[..4] == [0, 0, 0, 0])
        });
        if let Some(marker) = opted_out {
            return Read::Private { marker: format!("{marker} = 0"), source: owner_app() };
        }
        if has(u32::from(CF_HDROP.0)) {
            return Read::Files(bytes(u32::from(CF_HDROP.0)).map(|b| convert::dropfiles_paths(&b)).unwrap_or_default());
        }
        let png = bytes(format("PNG"));
        let dib =
            if png.is_some() { None } else { bytes(u32::from(CF_DIBV5.0)).or_else(|| bytes(u32::from(CF_DIB.0))) };
        (
            bytes(u32::from(CF_UNICODETEXT.0)),
            bytes(format("Rich Text Format")),
            bytes(format("HTML Format")),
            png,
            dib,
            has(format(WINDOWS_OURS)),
        )
    };
    let text = text.map(|b| {
        let units: Vec<u16> =
            b.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c)).take_while(|&u| u != 0).collect();
        String::from_utf16_lossy(&units)
    });
    let rtf = rtf.map(cut_at_nul).filter(|b| !b.is_empty());
    let html = html.and_then(|b| convert::cf_html_fragment(&b));
    let png = png.or_else(|| dib.and_then(|d| convert::dib_to_png(&d)));
    let contents = Contents { text, rtf, html, png };
    if contents.is_empty() { Read::Empty } else { Read::Contents { contents, ours } }
}

/// Puts `data` on the open clipboard as `fmt` (the clipboard owns it after).
fn put(fmt: u32, data: &[u8]) -> std::io::Result<()> {
    // SAFETY: the clipboard is open with an owner window; the memory is
    // moveable, filled while locked, and handed over on success.
    unsafe {
        let global = GlobalAlloc(GMEM_MOVEABLE, data.len().max(1)).map_err(std::io::Error::other)?;
        let ptr = GlobalLock(global) as *mut u8;
        if ptr.is_null() {
            let _ = GlobalFree(Some(global));
            return Err(std::io::Error::other("couldn't lock clipboard memory"));
        }
        std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
        let _ = GlobalUnlock(global);
        if let Err(e) = SetClipboardData(fmt, Some(HANDLE(global.0))) {
            let _ = GlobalFree(Some(global));
            return Err(std::io::Error::other(e));
        }
    }
    Ok(())
}

pub fn write(c: &Contents, private: bool) -> std::io::Result<u64> {
    // SAFETY: a message-only STATIC window, destroyed below on this thread.
    let owner = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("STATIC"),
            w!(""),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            None,
            None,
        )
    }
    .map_err(std::io::Error::other)?;
    // Converted before opening the clipboard, so it's open only briefly.
    let dib = c.png.as_deref().and_then(convert::png_to_dib);
    let result = (|| {
        let _open = Open::new(Some(owner))?;
        // SAFETY: open, with an owner.
        unsafe { EmptyClipboard() }.map_err(std::io::Error::other)?;
        if let Some(text) = &c.text {
            let wide: Vec<u8> = text.encode_utf16().chain([0]).flat_map(u16::to_le_bytes).collect();
            put(u32::from(CF_UNICODETEXT.0), &wide)?;
        }
        if let Some(rtf) = &c.rtf {
            put(format("Rich Text Format"), &[rtf.as_slice(), &[0]].concat())?;
        }
        if let Some(html) = &c.html {
            put(format("HTML Format"), &[convert::cf_html_wrap(html).as_slice(), &[0]].concat())?;
        }
        if let Some(png) = &c.png {
            put(format("PNG"), png)?;
        }
        if let Some(dib) = &dib {
            put(u32::from(CF_DIB.0), dib)?;
        }
        put(format(WINDOWS_OURS), &[1])?;
        if private {
            put(format("ExcludeClipboardContentFromMonitorProcessing"), &[0])?;
            for f in WINDOWS_OPT_OUT_FORMATS {
                put(format(f), &0u32.to_le_bytes())?;
            }
        }
        Ok(change_count())
    })();
    // SAFETY: the window this thread made above.
    let _ = unsafe { DestroyWindow(owner) };
    // Closing the clipboard (dropping `_open`) bumps the counter once more on
    // some systems: report the count as it is now.
    result.map(|_| change_count())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Writes `formats` as another app would (with its own owner window).
    fn put_as_another_app(formats: &[(u32, Vec<u8>)]) {
        let owner = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                w!(""),
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                None,
                None,
            )
        }
        .unwrap();
        {
            let _open = Open::new(Some(owner)).unwrap();
            unsafe { EmptyClipboard() }.unwrap();
            for (fmt, data) in formats {
                put(*fmt, data).unwrap();
            }
        }
        let _ = unsafe { DestroyWindow(owner) };
    }

    fn wide(s: &str) -> Vec<u8> {
        s.encode_utf16().chain([0]).flat_map(u16::to_le_bytes).collect()
    }

    /// Mission 05 §1.9: another app holding the clipboard open (a browser's
    /// copy button, for a moment) reads as busy, not empty, so automatic sync
    /// tries again instead of dropping the copy. Before the fix: `Empty`.
    #[test]
    #[ignore = "uses the real clipboard"]
    fn real_clipboard_held_by_another_app_reads_as_busy() {
        put_as_another_app(&[(13, wide("copied in a browser"))]);
        let (held, release) = (std::sync::mpsc::channel::<()>(), std::sync::mpsc::channel::<()>());
        let holder = std::thread::spawn(move || {
            let _open = Open::new(None).unwrap();
            held.0.send(()).unwrap();
            let _ = release.1.recv_timeout(Duration::from_secs(3));
        });
        held.1.recv().unwrap();
        assert_eq!(super::super::read(), Read::Busy, "held open by another thread past the half-second retry");
        release.0.send(()).unwrap();
        holder.join().unwrap();
        match super::super::read() {
            Read::Contents { contents, .. } => assert_eq!(contents.text.as_deref(), Some("copied in a browser")),
            other => panic!("once free it reads: {other:?}"),
        }
    }

    /// Uses the real clipboard: `cargo test -p portal-sys -- --ignored real_clipboard`.
    #[test]
    #[ignore = "uses the real clipboard"]
    fn real_clipboard() {
        let png = convert::png_from_rgba(2, 1, &[1, 2, 3, 255, 4, 5, 6, 255]).unwrap();
        let c = Contents {
            text: Some("meeting notes".into()),
            rtf: Some(b"{\\rtf1 meeting notes}".to_vec()),
            html: Some("<b>meeting</b> notes".into()),
            png: Some(png.clone()),
        };
        super::super::write(&c).unwrap();
        match super::super::read() {
            Read::Contents { contents, ours } => {
                assert!(ours, "marked as ours");
                assert_eq!(contents.text.as_deref(), Some("meeting notes"));
                assert_eq!(contents.html.as_deref(), Some("<b>meeting</b> notes"));
                assert_eq!(contents.rtf.as_deref(), Some(&b"{\\rtf1 meeting notes}"[..]));
                assert_eq!(convert::rgba_of(&contents.png.unwrap()), convert::rgba_of(&png));
            }
            other => panic!("{other:?}"),
        }

        let text = u32::from(CF_UNICODETEXT.0);
        put_as_another_app(&[(text, wide("from another app"))]);
        assert!(matches!(super::super::read(), Read::Contents { ours: false, .. }));

        // Review 04 (privacy): text sent as private carries the formats that
        // keep it out of clipboard history and the cloud clipboard
        super::super::write_private(&Contents { text: Some("hunter2".into()), ..Default::default() }).unwrap();
        let f = super::super::formats();
        assert!(f.names.iter().any(|n| n == "ExcludeClipboardContentFromMonitorProcessing"), "{:?}", f.names);
        assert_eq!((f.can_include_in_history, f.can_upload_to_cloud), (Some(0), Some(0)));
        assert!(matches!(super::super::read(), Read::Private { .. }));

        // What password managers set: never read.
        put_as_another_app(&[
            (text, wide("hunter2")),
            (format("ExcludeClipboardContentFromMonitorProcessing"), vec![0]),
        ]);
        let marker = |r: Read| match r {
            Read::Private { marker, .. } => marker,
            other => panic!("{other:?}"),
        };
        assert_eq!(marker(super::super::read()), "ExcludeClipboardContentFromMonitorProcessing");
        put_as_another_app(&[(text, wide("hunter2")), (format("CanUploadToCloudClipboard"), vec![0, 0, 0, 0])]);
        assert_eq!(marker(super::super::read()), "CanUploadToCloudClipboard = 0");
        put_as_another_app(&[(text, wide("hunter2")), (format("CanIncludeInClipboardHistory"), vec![0, 0, 0, 0])]);
        assert_eq!(marker(super::super::read()), "CanIncludeInClipboardHistory = 0");
        put_as_another_app(&[(text, wide("hunter2")), (format("Clipboard Viewer Ignore"), vec![0])]);
        assert_eq!(marker(super::super::read()), "Clipboard Viewer Ignore");
        // Explicitly allowed stays readable.
        put_as_another_app(&[(text, wide("fine")), (format("CanUploadToCloudClipboard"), vec![1, 0, 0, 0])]);
        assert!(matches!(super::super::read(), Read::Contents { .. }));

        // An image only as a bitmap (no PNG), as many apps put it.
        put_as_another_app(&[(u32::from(CF_DIB.0), convert::png_to_dib(&png).unwrap())]);
        match super::super::read() {
            Read::Contents { contents, .. } => {
                assert_eq!(convert::rgba_of(&contents.png.unwrap()), convert::rgba_of(&png))
            }
            other => panic!("{other:?}"),
        }
        put_as_another_app(&[]);
    }
}
