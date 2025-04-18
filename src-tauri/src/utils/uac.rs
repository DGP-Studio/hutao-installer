use std::{ffi::OsStr, mem::size_of};
use windows::{
    core::{w, HSTRING, PCWSTR},
    Win32::{
        Foundation::{GetLastError, HANDLE},
        UI::Shell::{
            ShellExecuteExW, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
        },
    },
};

pub struct SendableHandle(pub HANDLE);
unsafe impl Send for SendableHandle {}
unsafe impl Sync for SendableHandle {}

pub fn run_elevated<S: AsRef<OsStr>, T: AsRef<OsStr>>(
    program_path: S,
    args: T,
) -> std::io::Result<SendableHandle> {
    let file = HSTRING::from(program_path.as_ref());
    let par = HSTRING::from(args.as_ref());

    let mut sei = SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOASYNC | SEE_MASK_NOCLOSEPROCESS,
        lpVerb: w!("runas"),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: PCWSTR(par.as_ptr()),
        nShow: 1,
        ..Default::default()
    };
    unsafe {
        ShellExecuteExW(&mut sei)?;
        let process = { sei.hProcess };
        if process.is_invalid() {
            sentry::capture_message(
                format!("Failed to run elevated: {:?}", GetLastError()).as_str(),
                sentry::Level::Error,
            );
            return Err(std::io::Error::last_os_error());
        };
        Ok(SendableHandle(process))
    }
}
