#[cfg(windows)]
use anyhow::{bail, Result};

#[cfg(windows)]
pub fn run_elevated(exe: &std::path::Path) -> Result<()> {
    run_elevated_with_args(exe, "")
}

#[cfg(windows)]
pub fn run_elevated_with_args(exe: &std::path::Path, args: &str) -> Result<()> {
    use std::{ffi::OsStr, os::windows::ffi::OsStrExt, ptr};
    use windows_sys::Win32::UI::{
        Shell::ShellExecuteW,
        WindowsAndMessaging::SW_SHOWNORMAL,
    };

    fn wide(value: &OsStr) -> Vec<u16> {
        value.encode_wide().chain(std::iter::once(0)).collect()
    }

    let verb = wide(OsStr::new("runas"));
    let file = wide(exe.as_os_str());
    let parameters = wide(OsStr::new(args));

    let result = unsafe {
        ShellExecuteW(
            ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            parameters.as_ptr(),
            ptr::null(),
            SW_SHOWNORMAL,
        )
    };

    let result_code = result as isize;

    if result_code <= 32 {
        bail!("Failed to start elevated process. ShellExecuteW returned {result_code}");
    }

    Ok(())
}

#[cfg(not(windows))]
pub fn run_elevated(_exe: &std::path::Path) -> anyhow::Result<()> {
    anyhow::bail!("Launching setup.exe with UAC elevation is only supported on Windows.");
}

#[cfg(not(windows))]
pub fn run_elevated_with_args(
    _exe: &std::path::Path,
    _args: &str,
) -> anyhow::Result<()> {
    anyhow::bail!("Launching elevated processes is only supported on Windows.");
}