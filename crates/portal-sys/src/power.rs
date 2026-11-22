//! Keeping the computer awake while files move.
//!
//! - macOS: a user-initiated `NSProcessInfo` activity. No idle sleep, and no
//!   App Nap throttling while SeamDrop works from the menu bar.
//! - Windows: a `PowerRequestSystemRequired` power request. Unlike
//!   `SetThreadExecutionState` it belongs to a handle, not to a thread.
//!
//! The display can still sleep, and closing a laptop's lid still sleeps it.

/// Held while a transfer runs; dropping it lets the computer sleep again.
pub struct KeepAwake {
    _token: imp::Token,
}

impl KeepAwake {
    /// `reason` shows in `pmset -g assertions` / `powercfg /requests`.
    /// None if the OS said no (the transfer runs anyway).
    pub fn new(reason: &str) -> Option<KeepAwake> {
        imp::begin(reason).map(|token| KeepAwake { _token: token })
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use objc2::rc::Retained;
    use objc2::runtime::{NSObjectProtocol, ProtocolObject};
    use objc2_foundation::{NSActivityOptions, NSProcessInfo, NSString};

    pub struct Token(Retained<ProtocolObject<dyn NSObjectProtocol>>);

    // SAFETY: the token is an opaque object only ever handed back to
    // -[NSProcessInfo endActivity:], which is thread-safe.
    unsafe impl Send for Token {}

    pub fn begin(reason: &str) -> Option<Token> {
        let info = NSProcessInfo::processInfo();
        Some(Token(info.beginActivityWithOptions_reason(NSActivityOptions::UserInitiated, &NSString::from_str(reason))))
    }

    impl Drop for Token {
        fn drop(&mut self) {
            // SAFETY: the object came from beginActivityWithOptions:reason:.
            unsafe { NSProcessInfo::processInfo().endActivity(&self.0) }
        }
    }
}

#[cfg(windows)]
mod imp {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::Power::{
        PowerClearRequest, PowerCreateRequest, PowerRequestSystemRequired, PowerSetRequest,
    };
    use windows::Win32::System::Threading::{POWER_REQUEST_CONTEXT_SIMPLE_STRING, REASON_CONTEXT, REASON_CONTEXT_0};
    use windows::core::PWSTR;

    pub struct Token(HANDLE);

    // SAFETY: a power request handle can be cleared and closed from any thread.
    unsafe impl Send for Token {}

    pub fn begin(reason: &str) -> Option<Token> {
        let mut text: Vec<u16> = reason.encode_utf16().chain(Some(0)).collect();
        let context = REASON_CONTEXT {
            Version: 0, // POWER_REQUEST_CONTEXT_VERSION
            Flags: POWER_REQUEST_CONTEXT_SIMPLE_STRING,
            Reason: REASON_CONTEXT_0 { SimpleReasonString: PWSTR(text.as_mut_ptr()) },
        };
        // SAFETY: `context` and the string it points to outlive the call,
        // which copies the reason.
        let handle = unsafe { PowerCreateRequest(&context) }.ok()?;
        // SAFETY: `handle` is the request just created.
        if unsafe { PowerSetRequest(handle, PowerRequestSystemRequired) }.is_err() {
            // SAFETY: closing the handle we own.
            unsafe {
                let _ = CloseHandle(handle);
            }
            return None;
        }
        Some(Token(handle))
    }

    impl Drop for Token {
        fn drop(&mut self) {
            // SAFETY: the request was set in `begin`; we own the handle.
            unsafe {
                let _ = PowerClearRequest(self.0, PowerRequestSystemRequired);
                let _ = CloseHandle(self.0);
            }
        }
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
mod imp {
    pub struct Token;

    pub fn begin(_reason: &str) -> Option<Token> {
        Some(Token)
    }
}
