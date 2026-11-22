pub(crate) fn to_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(DIGITS[(b >> 4) as usize] as char);
        s.push(DIGITS[(b & 0xf) as usize] as char);
    }
    s
}

pub(crate) fn from_hex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            let byte = s.get(i..i + 2)?;
            if !byte.bytes().all(|c| c.is_ascii_hexdigit()) {
                return None;
            }
            u8::from_str_radix(byte, 16).ok()
        })
        .collect()
}

pub(crate) fn random_bytes<const N: usize>() -> [u8; N] {
    use ring::rand::{SecureRandom, SystemRandom};
    let mut out = [0u8; N];
    SystemRandom::new().fill(&mut out).expect("OS random number generator failed");
    out
}

pub(crate) fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// "Desk PC", "Desk PC and MacBook Air", "Desk PC, MacBook Air and Studio"
pub fn and_list(list: &[String]) -> String {
    match list {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}
