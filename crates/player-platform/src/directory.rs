//! Open a directory in the desktop file manager. Call from a background service.
use std::{io, path::Path};

pub fn open(path: &Path) -> io::Result<()> {
    if !std::fs::metadata(path)?.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "目标不是文件夹",
        ));
    }
    open_native(&std::path::absolute(path)?)
}

#[cfg(target_os = "windows")]
fn open_native(path: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::{
        Foundation::RPC_E_CHANGED_MODE,
        System::Com::{
            COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoUninitialize,
        },
        UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
    };
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    if wide.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "目录路径含 NUL",
        ));
    }
    wide.push(0);
    // SAFETY: This initializes only the calling worker's COM apartment. A
    // successful call (including S_FALSE) is paired with CoUninitialize below;
    // an existing apartment of another type is left untouched.
    let com_result = unsafe {
        CoInitializeEx(
            std::ptr::null(),
            (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32,
        )
    };
    if com_result < 0 && com_result != RPC_E_CHANGED_MODE {
        return Err(io::Error::other(format!(
            "无法初始化文件夹打开服务（HRESULT {com_result:#x}）"
        )));
    }
    let initialized = com_result >= 0;
    // SAFETY: Both strings are NUL-terminated and live for the whole call. This
    // opens an existing directory without a parent HWND or command parameters.
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            windows_sys::core::w!("open"),
            wide.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    } as isize;
    if initialized {
        // SAFETY: Matches the successful initialization on this same thread.
        unsafe { CoUninitialize() };
    }
    if result > 32 {
        Ok(())
    } else {
        Err(io::Error::new(
            if result == 5 {
                io::ErrorKind::PermissionDenied
            } else {
                io::ErrorKind::Other
            },
            format!("系统无法打开文件夹（ShellExecute {result}）"),
        ))
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn open_native(path: &Path) -> io::Result<()> {
    let launcher = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let status = std::process::Command::new(launcher)
        .arg(path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "{launcher} 无法打开文件夹（{status}）"
        )))
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn open_native(_path: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "当前平台未提供文件夹打开方式",
    ))
}

#[cfg(test)]
mod tests {
    #[test]
    fn reject_missing_paths_and_files_without_launching() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "yyplayer-open-folder-{}-{nonce}",
            std::process::id()
        ));
        assert_eq!(
            super::open(&path).unwrap_err().kind(),
            std::io::ErrorKind::NotFound
        );
        std::fs::write(&path, b"not a directory").unwrap();
        assert_eq!(
            super::open(&path).unwrap_err().kind(),
            std::io::ErrorKind::InvalidInput
        );
        std::fs::remove_file(path).unwrap();
    }
}
