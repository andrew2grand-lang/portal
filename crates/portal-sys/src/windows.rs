//! Windows-only helpers.

use std::io;

use windows::Win32::Foundation::{HLOCAL, LocalFree};
use windows::Win32::Security::Cryptography::{
    CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
};
use windows::core::PCWSTR;

/// Maps a `windows::core::Error` to an `io::Error`, keeping Win32 codes so
/// `kind()` works (e.g. ERROR_ALREADY_EXISTS -> AlreadyExists).
pub(crate) fn to_io_error(e: windows::core::Error) -> io::Error {
    let hr = e.code().0 as u32;
    if hr & 0xFFFF_0000 == 0x8007_0000 {
        io::Error::from_raw_os_error((hr & 0xFFFF) as i32)
    } else {
        io::Error::other(e)
    }
}

pub(crate) fn dpapi(input: &[u8], seal: bool) -> io::Result<Vec<u8>> {
    let len =
        u32::try_from(input.len()).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "secret too large"))?;
    let data_in = CRYPT_INTEGER_BLOB { cbData: len, pbData: input.as_ptr() as *mut u8 };
    let mut data_out = CRYPT_INTEGER_BLOB::default();
    // SAFETY: data_in points at `input` for the duration of the call; data_out
    // is filled by the OS and released with LocalFree below.
    unsafe {
        if seal {
            CryptProtectData(&data_in, PCWSTR::null(), None, None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut data_out)
        } else {
            CryptUnprotectData(&data_in, None, None, None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut data_out)
        }
        .map_err(to_io_error)?;
        let out = std::slice::from_raw_parts(data_out.pbData, data_out.cbData as usize).to_vec();
        let _ = LocalFree(Some(HLOCAL(data_out.pbData.cast())));
        Ok(out)
    }
}

// ---- Connection doctor probes ---------------------------------------------

use std::path::Path;

use windows::Win32::NetworkManagement::WindowsFirewall::{
    INetFwPolicy2, INetFwRule, NET_FW_ACTION_ALLOW, NET_FW_ACTION_BLOCK, NET_FW_RULE_DIR_IN, NetFwPolicy2,
};
use windows::Win32::Networking::NetworkListManager::{
    INetworkListManager, NLM_ENUM_NETWORK_CONNECTED, NLM_NETWORK_CATEGORY_DOMAIN_AUTHENTICATED,
    NLM_NETWORK_CATEGORY_PRIVATE, NetworkListManager,
};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize, IDispatch,
};
use windows::Win32::System::Ole::IEnumVARIANT;
use windows::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD, RegGetValueW};
use windows::Win32::System::Variant::{VARIANT, VT_DISPATCH, VariantClear};
use windows::core::{Interface, w};

/// COM for the current thread, for as long as this lives.
struct Com(bool);

impl Com {
    fn init() -> Com {
        // S_OK and S_FALSE need balancing; RPC_E_CHANGED_MODE means the
        // thread already has COM in another mode, which is fine to use.
        let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        Com(hr.is_ok())
    }
}

impl Drop for Com {
    fn drop(&mut self) {
        if self.0 {
            unsafe { CoUninitialize() };
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetCategory {
    Public,
    Private,
    Domain,
}

/// One connected network as Windows classifies it.
#[derive(Clone, Debug)]
pub struct NetProfile {
    /// "Home Wi-Fi", "Unidentified network"
    pub name: String,
    pub category: NetCategory,
    /// Adapter GUIDs, `{XXXXXXXX-...}` upper case (netdev's `name` on Windows).
    pub adapters: Vec<String>,
}

pub fn network_profiles() -> io::Result<Vec<NetProfile>> {
    let _com = Com::init();
    let mut out = Vec::new();
    // SAFETY: plain COM calls on interfaces we own; all out-params are checked.
    unsafe {
        let nlm: INetworkListManager = CoCreateInstance(&NetworkListManager, None, CLSCTX_ALL).map_err(to_io_error)?;
        let networks = nlm.GetNetworks(NLM_ENUM_NETWORK_CONNECTED).map_err(to_io_error)?;
        loop {
            let mut slot = [None];
            let mut fetched = 0u32;
            if networks.Next(&mut slot, Some(&mut fetched)).is_err() || fetched == 0 {
                break;
            }
            let Some(net) = slot[0].take() else { break };
            let name = net.GetName().map(|b| b.to_string()).unwrap_or_default();
            let category = match net.GetCategory().map_err(to_io_error)? {
                c if c == NLM_NETWORK_CATEGORY_PRIVATE => NetCategory::Private,
                c if c == NLM_NETWORK_CATEGORY_DOMAIN_AUTHENTICATED => NetCategory::Domain,
                _ => NetCategory::Public,
            };
            let mut adapters = Vec::new();
            if let Ok(conns) = net.GetNetworkConnections() {
                loop {
                    let mut c = [None];
                    let mut got = 0u32;
                    if conns.Next(&mut c, Some(&mut got)).is_err() || got == 0 {
                        break;
                    }
                    if let Some(conn) = c[0].take()
                        && let Ok(guid) = conn.GetAdapterId()
                    {
                        adapters.push(format!("{{{guid:?}}}").to_uppercase());
                    }
                }
            }
            out.push(NetProfile { name, category, adapters });
        }
    }
    Ok(out)
}

/// Windows Firewall inbound rules that name this program.
#[derive(Clone, Debug, Default)]
pub struct FirewallRules {
    /// Union of NET_FW_PROFILE2_* bits (1 Domain, 2 Private, 4 Public) of
    /// enabled inbound Allow rules.
    pub allow_profiles: i32,
    /// Names of enabled inbound Block rules (these beat any Allow rule).
    pub block: Vec<String>,
}

pub fn firewall_rules_for(exe: &Path) -> io::Result<FirewallRules> {
    let _com = Com::init();
    let want = exe.to_string_lossy().to_lowercase();
    let mut out = FirewallRules::default();
    // SAFETY: COM enumeration; each VARIANT is cleared after use.
    unsafe {
        let policy: INetFwPolicy2 = CoCreateInstance(&NetFwPolicy2, None, CLSCTX_ALL).map_err(to_io_error)?;
        let rules = policy.Rules().map_err(to_io_error)?;
        let en: IEnumVARIANT = rules._NewEnum().map_err(to_io_error)?.cast().map_err(to_io_error)?;
        loop {
            let mut var = [VARIANT::default()];
            let mut fetched = 0u32;
            let hr = en.Next(&mut var, &mut fetched);
            if hr.is_err() || fetched == 0 {
                break;
            }
            let rule: Option<INetFwRule> = if var[0].Anonymous.Anonymous.vt == VT_DISPATCH {
                let disp: Option<IDispatch> = (*var[0].Anonymous.Anonymous.Anonymous.pdispVal).clone();
                disp.and_then(|d| d.cast().ok())
            } else {
                None
            };
            let _ = VariantClear(&mut var[0]);
            let Some(rule) = rule else { continue };
            let app = rule.ApplicationName().map(|b| b.to_string().to_lowercase()).unwrap_or_default();
            if app != want {
                continue;
            }
            let enabled = rule.Enabled().map(|b| b.as_bool()).unwrap_or(false);
            let inbound = rule.Direction().map(|d| d == NET_FW_RULE_DIR_IN).unwrap_or(false);
            if !enabled || !inbound {
                continue;
            }
            match rule.Action() {
                Ok(a) if a == NET_FW_ACTION_ALLOW => out.allow_profiles |= rule.Profiles().unwrap_or(0),
                Ok(a) if a == NET_FW_ACTION_BLOCK => {
                    out.block.push(rule.Name().map(|b| b.to_string()).unwrap_or_else(|_| "(unnamed rule)".into()))
                }
                _ => {}
            }
        }
    }
    Ok(out)
}

/// Windows Security → Ransomware protection → Controlled folder access.
/// `None` if it can't be read.
pub fn controlled_folder_access() -> Option<bool> {
    let mut value = 0u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    // SAFETY: value/size describe a valid u32 buffer.
    let result = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            w!("SOFTWARE\\Microsoft\\Windows Defender\\Windows Defender Exploit Guard\\Controlled Folder Access"),
            w!("EnableControlledFolderAccess"),
            RRF_RT_REG_DWORD,
            None,
            Some((&mut value as *mut u32).cast()),
            Some(&mut size),
        )
    };
    result.is_ok().then_some(value == 1)
}

/// How "Fix it" for the firewall went.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FirewallFix {
    Fixed,
    /// The administrator question was answered No.
    Declined,
    Failed(String),
}

/// Removes inbound Block rules for `exe` and puts SeamDrop's Allow rules back,
/// in an elevated PowerShell (Windows asks for an administrator). Blocks
/// until it's done.
pub fn fix_firewall(exe: &Path, name: &str) -> FirewallFix {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let inner = crate::firewall::fix_script(&exe.to_string_lossy(), name);
    let launcher = crate::firewall::elevated_launcher(&inner);
    let out = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand"])
        .arg(crate::firewall::encoded(&launcher))
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    match out.map(|o| o.status.code()) {
        Ok(Some(0)) => FirewallFix::Fixed,
        Ok(Some(1223)) => FirewallFix::Declined,
        Ok(code) => FirewallFix::Failed(format!("the repair stopped (code {})", code.unwrap_or(-1))),
        Err(e) => FirewallFix::Failed(format!("PowerShell didn't start ({e})")),
    }
}

#[cfg(test)]
mod firewall_tests {
    use super::*;

    fn run(script: &str) -> Option<i32> {
        std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand"])
            .arg(crate::firewall::encoded(script))
            .status()
            .ok()
            .and_then(|s| s.code())
    }

    /// Runs the repair (not elevated: CI runs as an administrator) on a made
    /// up program path: a Block rule goes, both Allow rules arrive.
    #[test]
    #[ignore = "changes Windows Firewall rules; needs an administrator"]
    fn real_firewall_fix() {
        let exe = Path::new(r"C:\SeamDrop Test\it's seamdrop.exe");
        let quoted = exe.to_string_lossy().replace('\'', "''");
        let block = format!(
            "New-NetFirewallRule -DisplayName 'seamdrop test block' -Direction Inbound -Action Block -Program '{quoted}' | Out-Null"
        );
        assert_eq!(run(&block), Some(0), "made a Block rule");
        let before = firewall_rules_for(exe).unwrap();
        assert_eq!(before.block.len(), 1, "{before:?}");

        assert_eq!(run(&crate::firewall::fix_script(&exe.to_string_lossy(), "SeamDrop Test")), Some(0));
        let after = firewall_rules_for(exe).unwrap();
        let cleanup = format!(
            "Get-NetFirewallApplicationFilter -Program '{quoted}' | Get-NetFirewallRule | Remove-NetFirewallRule"
        );
        run(&cleanup);
        assert!(after.block.is_empty(), "{after:?}");
        assert_eq!(after.allow_profiles & 7, 7, "allowed on every profile: {after:?}");
    }
}
