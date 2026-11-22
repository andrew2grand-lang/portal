//! Repairing SeamDrop's Windows Firewall rules (the "Fix it" button).
//!
//! Pressing Cancel on Windows' "allow this app?" question leaves an inbound
//! Block rule for the program, and Block always wins over the installer's
//! Allow rules. The fix removes every inbound rule for the program and adds
//! the installer's two Allow rules back (TCP for files, UDP for finding the
//! other computer), on all network profiles, as the installer does.
//!
//! The script is built here, on every OS, so its quoting can be tested from a
//! Mac; it only runs on Windows (`windows::fix_firewall`), elevated.

/// The PowerShell that does the repair. Exits 0 when the rules are in place.
pub fn fix_script(exe: &str, name: &str) -> String {
    let q = |s: &str| format!("'{}'", s.replace('\'', "''"));
    format!(
        "$ErrorActionPreference = 'Stop'\n\
         $exe = {exe}\n\
         $name = {name}\n\
         try {{\n\
         \x20 Get-NetFirewallApplicationFilter -Program $exe -ErrorAction SilentlyContinue |\n\
         \x20   Get-NetFirewallRule | Where-Object {{ $_.Direction -eq 'Inbound' }} | Remove-NetFirewallRule\n\
         \x20 New-NetFirewallRule -DisplayName $name -Description ($name + ': receive files from your other computer') \
         -Direction Inbound -Action Allow -Program $exe -Profile Any -Protocol TCP | Out-Null\n\
         \x20 New-NetFirewallRule -DisplayName $name -Description ($name + ': find your other computer (mDNS and broadcast)') \
         -Direction Inbound -Action Allow -Program $exe -Profile Any -Protocol UDP | Out-Null\n\
         \x20 exit 0\n\
         }} catch {{ exit 2 }}\n",
        exe = q(exe),
        name = q(name),
    )
}

/// Runs `inner` (a script) in an elevated PowerShell and waits: Windows asks
/// for an administrator first. Exit 1223 means that question was declined.
pub fn elevated_launcher(inner: &str) -> String {
    format!(
        "try {{\n\
         \x20 $p = Start-Process -FilePath powershell.exe -Verb RunAs -Wait -PassThru -WindowStyle Hidden \
         -ArgumentList '-NoProfile','-NonInteractive','-ExecutionPolicy','Bypass','-EncodedCommand','{}'\n\
         \x20 exit $p.ExitCode\n\
         }} catch {{ exit 1223 }}\n",
        encoded(inner)
    )
}

/// PowerShell's -EncodedCommand form: base64 of the UTF-16LE text, so no
/// quoting survives a command line.
pub fn encoded(script: &str) -> String {
    let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    base64(&bytes)
}

fn base64(bytes: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ABC[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_the_standard() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        // "hi" as UTF-16LE, what PowerShell expects
        assert_eq!(encoded("hi"), "aABpAA==");
    }

    #[test]
    fn a_path_with_quotes_and_spaces_stays_one_string() {
        let script = fix_script(r"C:\Program Files\Sam's Apps\seamdrop.exe", "SeamDrop");
        assert!(script.contains(r"$exe = 'C:\Program Files\Sam''s Apps\seamdrop.exe'"), "{script}");
        assert!(script.contains("$name = 'SeamDrop'"));
    }

    #[test]
    fn it_removes_inbound_rules_then_adds_both_allow_rules_back() {
        let script = fix_script(r"C:\SeamDrop\seamdrop.exe", "SeamDrop");
        let remove = script.find("Remove-NetFirewallRule").expect("removes");
        let tcp = script.find("-Protocol TCP").expect("TCP rule");
        let udp = script.find("-Protocol UDP").expect("UDP rule");
        assert!(remove < tcp && tcp < udp);
        assert_eq!(script.matches("-Action Allow").count(), 2);
        assert!(!script.contains("-Action Block"));
        assert!(script.contains("-Profile Any"));
    }

    #[test]
    fn the_launcher_asks_for_an_administrator_and_passes_the_exit_code_on() {
        let launcher = elevated_launcher("exit 0");
        assert!(launcher.contains("-Verb RunAs -Wait -PassThru"));
        assert!(launcher.contains(&encoded("exit 0")));
        assert!(launcher.contains("exit $p.ExitCode"));
        assert!(launcher.contains("exit 1223"), "declined is told apart");
    }
}
