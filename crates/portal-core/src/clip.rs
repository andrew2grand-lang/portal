//! The shared clipboard (Mission 03 §4): what can travel, the limits, how it
//! travels (in parts on the control channel of the paired, encrypted link),
//! and what's said about it. The log and Details only ever say the kind and
//! size ("text, 1.2 KB"); a notice shows a short first line.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;

pub use portal_sys::clipboard::{Contents, Read};

use crate::proto::Msg;

/// Images up to this size (as PNG) go on the clipboard; bigger ones are sent
/// as a file instead.
pub const IMAGE_LIMIT: usize = 25 * 1024 * 1024;
/// Text, RTF and HTML together, the same way.
pub const TEXT_LIMIT: usize = 8 * 1024 * 1024;
/// Raw bytes per part (base64 makes it 1 MiB on the wire).
pub const PART: usize = 768 * 1024;
/// Parts an incoming clipboard may have (both limits, plus a little).
pub const MAX_PARTS: u32 = ((IMAGE_LIMIT + TEXT_LIMIT) / PART + 2) as u32;
/// Received items kept for "Copy again" (in memory only).
/// Recently received: the last 5 (memory only, cleared on quit; Mission 05 §3.1).
pub const HISTORY: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TooBig {
    Image(u64),
    Text(u64),
}

pub fn text_bytes(c: &Contents) -> usize {
    c.text.as_ref().map_or(0, String::len) + c.rtf.as_ref().map_or(0, Vec::len) + c.html.as_ref().map_or(0, String::len)
}

/// Within the limits for a clipboard (else: send it as a file).
pub fn check(c: &Contents) -> Result<(), TooBig> {
    if let Some(png) = c.png.as_ref().filter(|p| p.len() > IMAGE_LIMIT) {
        return Err(TooBig::Image(png.len() as u64));
    }
    let t = text_bytes(c);
    if t > TEXT_LIMIT { Err(TooBig::Text(t as u64)) } else { Ok(()) }
}

/// "12 bytes", "1.3 KB", "4.1 MB"
pub fn size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["bytes", "KB", "MB", "GB"];
    let mut v = bytes as f64;
    let mut u = 0;
    while v >= 1000.0 && u < UNITS.len() - 1 {
        v /= 1000.0;
        u += 1;
    }
    if u == 0 { format!("{bytes} bytes") } else { format!("{v:.1} {}", UNITS[u]) }
}

/// "text", "rich text" or "image": what it is, for people.
pub fn kind(c: &Contents) -> &'static str {
    if c.png.is_some() {
        "image"
    } else if c.rtf.is_some() || c.html.is_some() {
        "rich text"
    } else {
        "text"
    }
}

/// For the log and Details: "text, 1.2 KB". Never the contents.
pub fn summary(c: &Contents) -> String {
    let bytes = if c.png.is_some() { c.png.as_ref().map_or(0, Vec::len) } else { text_bytes(c) };
    format!("{}, {}", kind(c), size(bytes as u64))
}

/// For the notice: “meeting notes…” (first line, about 30 characters), or
/// "An image, 1200 × 800".
pub fn preview(c: &Contents) -> String {
    if let Some(line) = c.text.as_deref().and_then(|t| t.lines().map(str::trim).find(|l| !l.is_empty())) {
        const MAX: usize = 30;
        let short: String = line.chars().take(MAX).collect();
        let more = line.chars().count() > MAX;
        return format!("“{}{}”", short.trim_end(), if more { "…" } else { "" });
    }
    match c.png.as_deref().and_then(portal_sys::clipboard::convert::png_size) {
        Some((w, h)) => format!("An image, {w} × {h}"),
        None if c.png.is_some() => "An image".into(),
        None => "Rich text".into(),
    }
}

const TEXT: u8 = b'T';
const RTF: u8 = b'R';
const HTML: u8 = b'H';
const PNG: u8 = b'P';

/// One byte string: for each representation, a tag, an 8-byte length, the bytes.
pub fn encode(c: &Contents) -> Vec<u8> {
    let mut out = Vec::new();
    let mut put = |tag: u8, bytes: &[u8]| {
        out.push(tag);
        out.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
        out.extend_from_slice(bytes);
    };
    if let Some(t) = &c.text {
        put(TEXT, t.as_bytes());
    }
    if let Some(r) = &c.rtf {
        put(RTF, r);
    }
    if let Some(h) = &c.html {
        put(HTML, h.as_bytes());
    }
    if let Some(p) = &c.png {
        put(PNG, p);
    }
    out
}

pub fn decode(mut b: &[u8]) -> Result<Contents, String> {
    let mut c = Contents::default();
    while !b.is_empty() {
        let (&tag, rest) = b.split_first().ok_or("empty")?;
        let len: [u8; 8] = rest.get(..8).ok_or("cut short")?.try_into().map_err(|_| "cut short")?;
        let len = usize::try_from(u64::from_be_bytes(len)).map_err(|_| "too long")?;
        let bytes = rest.get(8..8usize.checked_add(len).ok_or("too long")?).ok_or("cut short")?;
        let text = || String::from_utf8(bytes.to_vec()).map_err(|_| "not UTF-8".to_owned());
        match tag {
            TEXT => c.text = Some(text()?),
            RTF => c.rtf = Some(bytes.to_vec()),
            HTML => c.html = Some(text()?),
            PNG => c.png = Some(bytes.to_vec()),
            other => return Err(format!("unknown part {other}")),
        }
        b = &rest[8 + len..];
    }
    Ok(c)
}

/// The messages that carry `c` (id: chosen by the sender, unique per link).
pub fn parts(id: u64, c: &Contents, auto: bool) -> Vec<Msg> {
    let bytes = encode(c);
    let chunks: Vec<&[u8]> = if bytes.is_empty() { vec![&[][..]] } else { bytes.chunks(PART).collect() };
    let of = chunks.len() as u32;
    chunks
        .into_iter()
        .enumerate()
        .map(|(n, chunk)| Msg::Clip { id, n: n as u32, of, auto, data: B64.encode(chunk), private: false })
        .collect()
}

/// Puts an incoming clipboard back together, part by part, in order.
#[derive(Debug)]
pub struct Assembly {
    pub id: u64,
    pub auto: bool,
    of: u32,
    next: u32,
    bytes: Vec<u8>,
}

impl Assembly {
    pub fn new(id: u64, of: u32, auto: bool) -> Result<Self, String> {
        if of == 0 || of > MAX_PARTS {
            return Err(format!("{of} parts"));
        }
        Ok(Self { id, auto, of, next: 0, bytes: Vec::new() })
    }

    /// Adds part `n`. Returns the contents once the last part is in.
    pub fn push(&mut self, id: u64, n: u32, data: &str) -> Result<Option<Contents>, String> {
        if id != self.id || n != self.next {
            return Err("part out of order".into());
        }
        let chunk = B64.decode(data).map_err(|_| "bad part")?;
        if chunk.len() > PART {
            return Err("part too big".into());
        }
        self.bytes.extend_from_slice(&chunk);
        self.next += 1;
        if self.next < self.of {
            return Ok(None);
        }
        let c = decode(&self.bytes)?;
        // The sender keeps to the limits; so does what we accept.
        if c.png.as_ref().is_some_and(|p| p.len() > IMAGE_LIMIT) || text_bytes(&c) > TEXT_LIMIT {
            return Err("over the limit".into());
        }
        Ok(Some(c))
    }
}

/// The clipboard the node reads and writes: the system's, or one in memory
/// (tests, which must never touch the real one).
pub trait ClipboardIo: Send + Sync + std::fmt::Debug {
    fn change_count(&self) -> u64;
    fn read(&self) -> Read;
    /// Replaces it with `c`, marked as ours; returns the change count after.
    fn write(&self, c: &Contents) -> std::io::Result<u64>;
    /// The same, marked private as password managers mark theirs.
    fn write_private(&self, c: &Contents) -> std::io::Result<u64>;
    /// The text even if it's marked private, with the change count it was
    /// read at: only for Put on Shelf, on the person's click (Mission 05 §3.1).
    fn read_marked_text(&self) -> Option<(String, u64)>;
}

#[derive(Debug, Default)]
pub struct SystemClipboard;

impl ClipboardIo for SystemClipboard {
    fn change_count(&self) -> u64 {
        portal_sys::clipboard::change_count()
    }
    fn read(&self) -> Read {
        portal_sys::clipboard::read()
    }
    fn write(&self, c: &Contents) -> std::io::Result<u64> {
        portal_sys::clipboard::write(c)
    }
    fn write_private(&self, c: &Contents) -> std::io::Result<u64> {
        portal_sys::clipboard::write_private(c)
    }
    fn read_marked_text(&self) -> Option<(String, u64)> {
        portal_sys::clipboard::read_marked_text()
    }
}

/// A clipboard in memory, with the ways other apps fill it.
#[derive(Debug)]
pub struct MemoryClipboard(
    std::sync::Mutex<(u64, Read)>,
    std::sync::atomic::AtomicU32,
    /// The text behind the last private copy (`copy_private_text`), and its count.
    std::sync::Mutex<Option<(String, u64)>>,
);

impl Default for MemoryClipboard {
    fn default() -> Self {
        Self(std::sync::Mutex::new((0, Read::Empty)), std::sync::atomic::AtomicU32::new(0), std::sync::Mutex::new(None))
    }
}

impl MemoryClipboard {
    fn set(&self, r: Read) -> u64 {
        let mut g = self.0.lock().unwrap_or_else(|e| e.into_inner());
        g.0 += 1;
        g.1 = r;
        g.0
    }
    /// Another app copied `c`.
    pub fn copy(&self, c: Contents) {
        self.set(Read::Contents { contents: c, ours: false });
    }
    /// A password manager copied something (macOS's concealed type).
    pub fn copy_private(&self) {
        self.copy_private_from("org.nspasteboard.ConcealedType", None);
    }
    /// Something marked private by `marker`, copied in `source` (if known).
    pub fn copy_private_from(&self, marker: &str, source: Option<&str>) {
        self.set(Read::Private { marker: marker.into(), source: source.map(Into::into) });
    }
    /// A password manager copied `text` (marked private).
    pub fn copy_private_text(&self, text: &str) {
        let n = self.set(Read::Private { marker: "org.nspasteboard.ConcealedType".into(), source: None });
        *self.2.lock().unwrap_or_else(|e| e.into_inner()) = Some((text.into(), n));
    }
    /// Files were copied in Finder or Explorer.
    pub fn copy_files(&self, paths: Vec<std::path::PathBuf>) {
        self.set(Read::Files(paths));
    }
    pub fn current(&self) -> Read {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).1.clone()
    }
    /// Another app holds the clipboard for the next `reads` reads (a browser's
    /// copy button, say): until then a read finds it busy, as on Windows.
    pub fn busy_for(&self, reads: u32) {
        self.1.store(reads, std::sync::atomic::Ordering::SeqCst);
    }
}

impl ClipboardIo for MemoryClipboard {
    fn change_count(&self) -> u64 {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).0
    }
    fn read(&self) -> Read {
        use std::sync::atomic::Ordering::SeqCst;
        let mut n = self.1.load(SeqCst);
        while n > 0 {
            match self.1.compare_exchange(n, n - 1, SeqCst, SeqCst) {
                Ok(_) => return Read::Busy,
                Err(now) => n = now,
            }
        }
        self.current()
    }
    fn write(&self, c: &Contents) -> std::io::Result<u64> {
        Ok(self.set(Read::Contents { contents: c.clone(), ours: true }))
    }
    /// Reads back as private, as the real clipboards do
    fn write_private(&self, c: &Contents) -> std::io::Result<u64> {
        let n = self.set(Read::Private { marker: "org.nspasteboard.ConcealedType".into(), source: None });
        *self.2.lock().unwrap_or_else(|e| e.into_inner()) = c.text.clone().map(|t| (t, n));
        Ok(n)
    }
    fn read_marked_text(&self) -> Option<(String, u64)> {
        let count = self.change_count();
        self.2.lock().unwrap_or_else(|e| e.into_inner()).clone().filter(|(_, n)| *n == count)
    }
}

/// What Send clipboard did.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipSent {
    /// A sentence to show ("Sent to Desk PC. Paste it there with ⌘V.")
    pub said: String,
    /// Files were on the clipboard: nothing was sent, and these can go as a
    /// transfer (Mission 04 §4.2).
    pub files: Vec<std::path::PathBuf>,
}

/// On screen when a copy was marked private (Mission 04 §4.3). It doesn't say
/// "we detected": the app said so.
pub const PRIVATE_SAID: &str = "The app you copied this from marked it as private, so it wasn’t sent. To send it anyway, paste it into the Shelf’s text box.";

/// Details' plain words for a private copy: who marked it, where known.
/// (`marker` goes in the line's technical part.)
pub fn private_words(source: Option<&str>) -> String {
    match source.and_then(app_name) {
        Some(name) if is_browser(name) => {
            format!("Not sent: {name} marked this copy as private, as it does in private windows (Tor Browser always does)")
        }
        Some(name) => format!("Not sent: {name} marked it as private, as password managers do"),
        None => "Not sent: the app you copied this from marked it as private (password managers do, and browsers in private windows)".into(),
    }
}

/// A friendly name for the app that copied, from its .exe name (Windows) or
/// bundle id (macOS).
fn app_name(source: &str) -> Option<&'static str> {
    let s = source.to_ascii_lowercase();
    let s = s.trim_end_matches(".exe");
    Some(match s {
        "firefox" | "org.mozilla.firefox" | "org.mozilla.firefoxdeveloperedition" => "Firefox",
        "org.torproject.torbrowser" => "Tor Browser",
        "chrome" | "com.google.chrome" => "Chrome",
        "msedge" | "com.microsoft.edgemac" => "Edge",
        "brave" | "com.brave.browser" => "Brave",
        "com.apple.safari" => "Safari",
        "keepass" => "KeePass",
        "keepassxc" | "org.keepassxc.keepassxc" => "KeePassXC",
        "bitwarden" | "com.bitwarden.desktop" => "Bitwarden",
        "1password" => "1Password",
        _ if s.starts_with("com.agilebits.onepassword") || s.starts_with("com.1password") => "1Password",
        "com.apple.passwords" | "com.apple.keychainaccess" => "Passwords",
        _ => return None,
    })
}

fn is_browser(name: &str) -> bool {
    matches!(name, "Firefox" | "Tor Browser" | "Chrome" | "Edge" | "Brave" | "Safari")
}

/// A small preview of a copied image for Recently received: at most 96 px on
/// its longest side, as a `data:` URL. None if the image can't be read.
pub fn thumbnail(png_bytes: &[u8]) -> Option<String> {
    use base64::Engine;
    let decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width as usize, info.height as usize);
    if w == 0 || h == 0 || w > 16_384 || h > 16_384 {
        return None;
    }
    let channels = match info.color_type {
        png::ColorType::Rgba => 4,
        png::ColorType::Rgb => 3,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Grayscale => 1,
        png::ColorType::Indexed => return None,
    };
    if info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    let scale = (96.0 / w.max(h) as f64).min(1.0);
    let (tw, th) = (((w as f64 * scale).round() as usize).max(1), ((h as f64 * scale).round() as usize).max(1));
    let mut out = Vec::with_capacity(tw * th * 4);
    for y in 0..th {
        for x in 0..tw {
            let (sx, sy) = ((x * w / tw).min(w - 1), (y * h / th).min(h - 1));
            let i = (sy * w + sx) * channels;
            let px = &buf[i..i + channels];
            let rgba = match channels {
                4 => [px[0], px[1], px[2], px[3]],
                3 => [px[0], px[1], px[2], 255],
                2 => [px[0], px[0], px[0], px[1]],
                _ => [px[0], px[0], px[0], 255],
            };
            out.extend_from_slice(&rgba);
        }
    }
    let mut encoded = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut encoded, tw as u32, th as u32);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut wr = enc.write_header().ok()?;
        wr.write_image_data(&out).ok()?;
    }
    Some(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(encoded)))
}
