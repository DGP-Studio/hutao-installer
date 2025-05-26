use crate::{capture_and_return_err, capture_and_return_err_message_string};
use std::{ffi::OsStr, os::windows::process::ExitStatusExt, process::ExitStatus};
use windows::{
    core::{w, HSTRING, PCWSTR, PWSTR},
    Win32::{
        Foundation::{CloseHandle, WAIT_EVENT, WAIT_OBJECT_0},
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
                TH32CS_SNAPPROCESS,
            },
            Threading::{
                GetExitCodeProcess, OpenProcess, QueryFullProcessImageNameW, INFINITE,
                PROCESS_NAME_FORMAT, PROCESS_QUERY_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION,
                PROCESS_SYNCHRONIZE,
            },
        },
        UI::{
            Shell::{
                ShellExecuteExW, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
            },
            WindowsAndMessaging::{
                DispatchMessageW, MsgWaitForMultipleObjects, PeekMessageW, TranslateMessage,
                PM_REMOVE, QS_ALLINPUT,
            },
        },
    },
};

pub fn run<S: AsRef<OsStr>, T: AsRef<OsStr>>(elevated: bool, program_path: S, args: Option<T>) {
    let file = PCWSTR(HSTRING::from(program_path.as_ref()).as_ptr());
    let par = if args.is_some() {
        PCWSTR(HSTRING::from(args.unwrap().as_ref()).as_ptr())
    } else {
        PCWSTR::null()
    };

    let mut sei = SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOASYNC | SEE_MASK_NOCLOSEPROCESS,
        lpVerb: if elevated { w!("runas") } else { w!("open") },
        lpFile: file,
        lpParameters: par,
        nShow: 1,
        ..Default::default()
    };
    unsafe {
        let _ = ShellExecuteExW(&mut sei);
        let process = sei.hProcess;
        let _ = CloseHandle(process);
    }
}

pub fn is_process_running(
    proc_name: String,
    specific_path_part: Option<String>,
) -> Result<(bool, Option<u32>), String> {
    let mut found = false;
    let mut pid: Option<u32> = None;
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot.is_err() {
            capture_and_return_err_message_string!(format!(
                "Failed to create snapshot: {:?}",
                snapshot.err()
            ));
        }
        let snapshot = snapshot.unwrap();
        if snapshot.is_invalid() {
            capture_and_return_err_message_string!(format!(
                "Failed to create snapshot: {:?}",
                windows::core::Error::from_win32()
            ));
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let current_name = String::from_utf16_lossy(&entry.szExeFile)
                    .trim_end_matches('\0')
                    .to_lowercase();
                if current_name == proc_name.to_lowercase() {
                    if specific_path_part.is_none() {
                        found = true;
                        pid = Some(entry.th32ProcessID);
                        break;
                    }

                    let specific_path_part = specific_path_part.as_ref().unwrap();

                    if let Some(path) = get_process_path(entry.th32ProcessID) {
                        if path.contains(specific_path_part) {
                            found = true;
                            pid = Some(entry.th32ProcessID);
                            break;
                        }
                    }
                }

                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);
    }
    Ok((found, pid))
}

pub fn is_process_running_by_pid(pid: u32) -> bool {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot.is_err() {
            return false;
        }
        let snapshot = snapshot.unwrap();
        if snapshot.is_invalid() {
            return false;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                if entry.th32ProcessID == pid {
                    let _ = CloseHandle(snapshot);
                    return true;
                }

                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);
    }
    false
}

pub fn get_process_path(pid: u32) -> Option<String> {
    // QueryFullProcessImageName
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) };
    if handle.is_err() {
        return None;
    }
    let handle = handle.unwrap();
    let mut buffer = [0u16; 1024];
    let mut size = buffer.len() as u32;
    let ret = unsafe {
        QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_FORMAT(0),
            PWSTR(buffer.as_mut_ptr()),
            &mut size,
        )
    };
    let _ = unsafe { CloseHandle(handle) };
    if ret.is_err() {
        return None;
    }
    let path = String::from_utf16_lossy(&buffer[..size as usize]);
    Some(path)
}

pub fn wait_for_pid(pid: u32) -> Result<ExitStatus, anyhow::Error> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_SYNCHRONIZE, false, pid);
        if handle.is_err() {
            capture_and_return_err!(anyhow::anyhow!(
                "Failed to open process with pid {}: {:?}",
                pid,
                handle.err()
            ))
        }

        let handle = handle?;
        if handle.is_invalid() {
            capture_and_return_err!(anyhow::anyhow!(
                "Invalid handle for pid {}: {:?}",
                pid,
                windows::core::Error::from_win32()
            ));
        }

        let mut exit_code = 0;
        let mut wait_result;
        loop {
            wait_result = MsgWaitForMultipleObjects(Some(&[handle]), false, INFINITE, QS_ALLINPUT);
            match wait_result {
                WAIT_OBJECT_0 => break,
                WAIT_EVENT(1u32) => {
                    let msg = std::ptr::null_mut();
                    while PeekMessageW(msg, None, 0, 0, PM_REMOVE).into() {
                        let _ = TranslateMessage(msg);
                        DispatchMessageW(msg);
                    }
                }
                _ => {
                    capture_and_return_err!(anyhow::anyhow!(
                        "WaitForSingleObject failed for pid {}",
                        pid
                    ));
                }
            }
        }

        let result = GetExitCodeProcess(handle, &mut exit_code);

        let _ = CloseHandle(handle);
        if result.is_err() {
            capture_and_return_err!(anyhow::anyhow!(
                "Failed to get exit code for pid {}: {:?}",
                pid,
                result.err()
            ));
        }

        Ok(ExitStatus::from_raw(exit_code))
    }
}
