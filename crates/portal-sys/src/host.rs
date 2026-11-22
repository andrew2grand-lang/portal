//! The computer's user-facing name ("Sam's MacBook Pro", "DESK-PC") and
//! which version of its OS it runs (for the technical log).

/// Returns the name the user gave this computer, if the OS has one.
pub fn computer_name() -> Option<String> {
    imp::computer_name().map(|n| n.trim().to_owned()).filter(|n| !n.is_empty())
}

/// "macOS 15.1 (24B83)", "Windows 11 23H2 (build 22631)"; the bare OS name
/// when the version can't be read.
pub fn os_version() -> String {
    imp::os_version().unwrap_or_else(|| std::env::consts::OS.to_owned())
}

#[cfg(target_os = "macos")]
mod imp {
    pub fn computer_name() -> Option<String> {
        let out = std::process::Command::new("/usr/sbin/scutil").args(["--get", "ComputerName"]).output().ok()?;
        out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
    }

    pub fn os_version() -> Option<String> {
        let ask = |flag: &str| {
            let out = std::process::Command::new("/usr/bin/sw_vers").arg(flag).output().ok()?;
            out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
        };
        Some(format!("macOS {} ({})", ask("-productVersion")?, ask("-buildVersion")?))
    }
}

#[cfg(windows)]
mod imp {
    use windows::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW};
    use windows::Win32::System::SystemInformation::{ComputerNameDnsHostname, GetComputerNameExW};
    use windows::core::{PCWSTR, PWSTR, w};

    /// A string value under HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion
    fn current_version(name: PCWSTR) -> Option<String> {
        let mut buf = [0u16; 128];
        let mut size = std::mem::size_of_val(&buf) as u32;
        // SAFETY: buf/size describe a valid buffer of `size` bytes.
        unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                w!("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion"),
                name,
                RRF_RT_REG_SZ,
                None,
                Some(buf.as_mut_ptr().cast()),
                Some(&mut size),
            )
        }
        .ok()
        .ok()?;
        let len = (size as usize / 2).saturating_sub(1).min(buf.len());
        Some(String::from_utf16_lossy(&buf[..len]))
    }

    pub fn os_version() -> Option<String> {
        let build = current_version(w!("CurrentBuildNumber"))?;
        // Windows 11 still says "Windows 10" in ProductName; the build tells.
        let name = if build.parse::<u32>().is_ok_and(|b| b >= 22000) { "Windows 11" } else { "Windows 10" };
        Some(match current_version(w!("DisplayVersion")) {
            Some(v) => format!("{name} {v} (build {build})"),
            None => format!("{name} (build {build})"),
        })
    }

    pub fn computer_name() -> Option<String> {
        let mut buf = [0u16; 256];
        let mut len = buf.len() as u32;
        // SAFETY: buf is valid for `len` UTF-16 units; the OS writes at most that many.
        unsafe { GetComputerNameExW(ComputerNameDnsHostname, Some(PWSTR(buf.as_mut_ptr())), &mut len) }.ok()?;
        Some(String::from_utf16_lossy(&buf[..len as usize]))
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
mod imp {
    pub fn computer_name() -> Option<String> {
        std::fs::read_to_string("/etc/hostname").ok()
    }

    pub fn os_version() -> Option<String> {
        let text = std::fs::read_to_string("/etc/os-release").ok()?;
        let line = text.lines().find(|l| l.starts_with("PRETTY_NAME="))?;
        Some(line.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_owned())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn says_which_os_version_this_is() {
        let v = super::os_version();
        eprintln!("{v}");
        #[cfg(target_os = "macos")]
        assert!(v.starts_with("macOS 1") || v.starts_with("macOS 2"), "{v}");
        #[cfg(windows)]
        assert!(v.starts_with("Windows 1") && v.contains("build "), "{v}");
        assert!(!v.is_empty());
    }
}
