//! File-system operations std doesn't offer.

use std::io;
use std::path::Path;

/// Moves `from` to `to`, failing with [`io::ErrorKind::AlreadyExists`] instead of
/// replacing anything already at `to`. Works for files and directories on the
/// same volume.
///
/// `std::fs::rename` silently replaces existing files on both macOS and
/// Windows, so received files must never be moved into place with it.
pub fn rename_noreplace(from: &Path, to: &Path) -> io::Result<()> {
    imp::rename_noreplace(from, to)
}

/// Writes `bytes` to `path` through a temporary file in the same folder, then
/// renames it over `path` (replacing it on purpose). On Unix the file is
/// created with mode 0600 before any byte is written.
pub fn write_private_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write;

    let dir = path.parent().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no parent"))?;
    let name = path.file_name().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?;
    let mut tmp_name = std::ffi::OsString::from(".");
    tmp_name.push(name);
    tmp_name.push(format!(".tmp-{}", std::process::id()));
    let tmp = dir.join(tmp_name);

    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let result = (|| {
        let mut file = options.open(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

/// Hides a folder in Explorer (Windows). Dot-names are already hidden on macOS.
pub fn hide(path: &Path) {
    #[cfg(windows)]
    imp::hide(path);
    #[cfg(not(windows))]
    let _ = path;
}

#[cfg(target_vendor = "apple")]
mod imp {
    use std::ffi::CString;
    use std::io;
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    pub fn rename_noreplace(from: &Path, to: &Path) -> io::Result<()> {
        let from = CString::new(from.as_os_str().as_bytes())?;
        let to = CString::new(to.as_os_str().as_bytes())?;
        // SAFETY: both pointers are valid NUL-terminated strings for the call.
        let rc = unsafe { libc::renamex_np(from.as_ptr(), to.as_ptr(), libc::RENAME_EXCL) };
        if rc == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
mod imp {
    use std::ffi::CString;
    use std::io;
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    pub fn rename_noreplace(from: &Path, to: &Path) -> io::Result<()> {
        let from = CString::new(from.as_os_str().as_bytes())?;
        let to = CString::new(to.as_os_str().as_bytes())?;
        // SAFETY: both pointers are valid NUL-terminated strings for the call.
        // (The flags are a c_uint on Linux and Android; the constant's type differs.)
        let rc = unsafe {
            libc::renameat2(libc::AT_FDCWD, from.as_ptr(), libc::AT_FDCWD, to.as_ptr(), libc::RENAME_NOREPLACE as _)
        };
        if rc == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
    }
}

#[cfg(windows)]
mod imp {
    use std::io;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use windows::Win32::Storage::FileSystem::{MOVE_FILE_FLAGS, MoveFileExW};
    use windows::core::PCWSTR;

    fn wide(p: &Path) -> Vec<u16> {
        p.as_os_str().encode_wide().chain(std::iter::once(0)).collect()
    }

    pub fn hide(path: &Path) {
        use windows::Win32::Storage::FileSystem::{
            FILE_ATTRIBUTE_HIDDEN, FILE_FLAGS_AND_ATTRIBUTES, GetFileAttributesW, SetFileAttributesW,
        };
        let w = wide(path);
        // SAFETY: w is NUL-terminated and outlives both calls.
        unsafe {
            let current = GetFileAttributesW(PCWSTR(w.as_ptr()));
            if current != u32::MAX {
                let _ = SetFileAttributesW(
                    PCWSTR(w.as_ptr()),
                    FILE_FLAGS_AND_ATTRIBUTES(current | FILE_ATTRIBUTE_HIDDEN.0),
                );
            }
        }
    }

    pub fn rename_noreplace(from: &Path, to: &Path) -> io::Result<()> {
        // canonicalize() returns std's own verbatim (\\?\) form, so long paths
        // work without building the prefix by hand. The target doesn't exist
        // yet, so resolve its parent and re-attach the name.
        let from = std::fs::canonicalize(from)?;
        let parent = to.parent().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "target has no parent"))?;
        let name = to.file_name().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "target has no name"))?;
        let to = std::fs::canonicalize(parent)?.join(name);
        let (wf, wt) = (wide(&from), wide(&to));
        // No MOVEFILE_REPLACE_EXISTING: an existing target fails with ERROR_ALREADY_EXISTS.
        // SAFETY: both buffers are NUL-terminated UTF-16 and outlive the call.
        unsafe { MoveFileExW(PCWSTR(wf.as_ptr()), PCWSTR(wt.as_ptr()), MOVE_FILE_FLAGS(0)) }
            .map_err(crate::windows::to_io_error)
    }
}
