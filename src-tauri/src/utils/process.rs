use std::{io::Error, os::windows::process::ExitStatusExt, process::ExitStatus};
use windows::{
    core::PWSTR,
    Win32::{
        Foundation::{CloseHandle, GetLastError, WAIT_EVENT, WAIT_OBJECT_0},
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
        UI::WindowsAndMessaging::{
            DispatchMessageW, MsgWaitForMultipleObjects, PeekMessageW, TranslateMessage, PM_REMOVE,
            QS_ALLINPUT,
        },
    },
};

pub fn is_process_running(
    proc_name: String,
    specific_path_part: Option<String>,
) -> Result<(bool, Option<u32>), String> {
    let mut found = false;
    let mut pid: Option<u32> = None;
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if let Err(e) = snapshot {
            return Err(format!("Failed to create snapshot: {:?}", e));
        }
        let snapshot = snapshot.unwrap();
        if snapshot.is_invalid() {
            return Err("Failed to create snapshot: invalid handle".to_string());
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

pub fn wait_for_pid(pid: u32) -> Result<ExitStatus, Error> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_SYNCHRONIZE, false, pid);
        if handle.is_err() {
            return Err(Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!(
                    "Failed to open process with pid {}, {:?}",
                    pid,
                    GetLastError()
                ),
            ));
        }

        let handle = handle?;
        if handle.is_invalid() {
            return Err(Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid handle for pid {}", pid),
            ));
        }

        let mut exit_code = 0;
        let mut wait_result =
            MsgWaitForMultipleObjects(Some(&[handle]), false, INFINITE, QS_ALLINPUT);
        while wait_result != WAIT_OBJECT_0 {
            if wait_result == WAIT_EVENT(1) {
                let msg = std::ptr::null_mut();
                while PeekMessageW(msg, None, 0, 0, PM_REMOVE).into() {
                    let _ = TranslateMessage(msg);
                    DispatchMessageW(msg);
                }
                wait_result =
                    MsgWaitForMultipleObjects(Some(&[handle]), false, INFINITE, QS_ALLINPUT);
            } else {
                return Err(Error::new(
                    std::io::ErrorKind::TimedOut,
                    format!("WaitForSingleObject failed for pid {}", pid),
                ));
            }
        }

        let result = GetExitCodeProcess(handle, &mut exit_code);

        let _ = CloseHandle(handle);
        if result.is_err() {
            return Err(Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to get exit code for pid {}", pid),
            ));
        }

        Ok(ExitStatus::from_raw(exit_code))
    }
}
