use crate::{
    module::singleton::{self, SingletonState, UserData},
    utils::process::{is_process_running, wait_for_pid},
    REQUEST_CLIENT,
};
use std::ptr::null_mut;
use tauri::Wry;
use windows::{
    core::{HRESULT, PCWSTR},
    Win32::{
        Foundation::{HWND, LPARAM, S_OK, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Controls::{
                TaskDialogIndirect, TASKDIALOGCONFIG, TASKDIALOGCONFIG_0, TASKDIALOG_NOTIFICATIONS,
                TDCBF_CANCEL_BUTTON, TDE_CONTENT, TDF_SHOW_MARQUEE_PROGRESS_BAR,
                TDF_USE_HICON_MAIN, TDM_ENABLE_BUTTON, TDM_SET_PROGRESS_BAR_MARQUEE,
                TDM_UPDATE_ELEMENT_TEXT, TDN_CREATED, TDN_DESTROYED,
            },
            WindowsAndMessaging::{
                LoadIconW, SendMessageW, SetProcessDPIAware, HICON, IDCANCEL, IDI_APPLICATION,
                WM_CLOSE,
            },
        },
    },
};

pub struct TaskDialogState {
    pub hwnd: *mut Option<HWND>,
    pub state: *const SingletonState,
}
unsafe impl Send for TaskDialogState {}
unsafe impl Sync for TaskDialogState {}

pub async fn install_webview2(command: String) {
    unsafe {
        let _ = SetProcessDPIAware();
    }

    let mut dialog_hwnd: Option<HWND> = None;

    let (res, singleton_state) = singleton::init(UserData::<Wry> {
        app: None,
        hwnd: &mut dialog_hwnd as *mut Option<HWND>,
    });
    if !res {
        std::process::exit(0);
    }

    let state = TaskDialogState {
        hwnd: &mut dialog_hwnd as *mut Option<HWND>,
        state: &singleton_state as *const SingletonState,
    };

    let title = "安装 WebView2 运行时";
    let heading = "当前系统缺少 WebView2 运行时，正在安装...";
    let content = "正在下载安装程序...";
    let title_utf16_nul = title
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<u16>>();
    let heading_utf16_nul = heading
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<u16>>();
    let content_utf16_nul = content
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<u16>>();

    unsafe extern "system" fn callback(
        hwnd: HWND,
        msg: TASKDIALOG_NOTIFICATIONS,
        _w_param: WPARAM,
        _l_param: LPARAM,
        lp_ref_data: isize,
    ) -> HRESULT {
        let state = lp_ref_data as *const TaskDialogState;
        let conf = (*state).hwnd;
        let singleton_state = (*state).state;
        match msg {
            TDN_CREATED => {
                (*conf).replace(hwnd);
                SendMessageW(
                    hwnd,
                    TDM_SET_PROGRESS_BAR_MARQUEE.0 as u32,
                    Some(WPARAM(1)),
                    Some(LPARAM(1)),
                );
            }
            TDN_DESTROYED => {
                if (*conf).is_some() {
                    (*conf).take();
                    exit_and_release_mutex(1, &*singleton_state);
                }
            }
            _ => {}
        };
        S_OK
    }

    tokio::task::spawn_blocking(move || {
        // get HICON of the current process
        let hmodule = unsafe { GetModuleHandleW(PCWSTR(null_mut())).unwrap() };
        let hicon = unsafe { LoadIconW(Some(hmodule.into()), IDI_APPLICATION) };

        let config: TASKDIALOGCONFIG = TASKDIALOGCONFIG {
            cbSize: u32::try_from(size_of::<TASKDIALOGCONFIG>()).unwrap(),
            hInstance: unsafe { GetModuleHandleW(PCWSTR(std::ptr::null())).unwrap().into() },
            pszWindowTitle: PCWSTR(title_utf16_nul.as_ptr()),
            pszMainInstruction: PCWSTR(heading_utf16_nul.as_ptr()),
            pszContent: PCWSTR(content_utf16_nul.as_ptr()),
            dwFlags: TDF_SHOW_MARQUEE_PROGRESS_BAR | TDF_USE_HICON_MAIN,
            pfCallback: Some(callback),
            lpCallbackData: &state as *const TaskDialogState as isize,
            dwCommonButtons: TDCBF_CANCEL_BUTTON,
            Anonymous1: TASKDIALOGCONFIG_0 {
                hMainIcon: if let Ok(hicon) = hicon {
                    hicon
                } else {
                    HICON(null_mut())
                },
            },
            ..TASKDIALOGCONFIG::default()
        };
        let _ = unsafe { TaskDialogIndirect(&config, None, None, None) };
    });

    while dialog_hwnd.is_none() {
        std::hint::spin_loop();
    }

    let temp_dir = std::env::temp_dir();
    let installer_path = temp_dir.as_path().join("MicrosoftEdgeWebview2Setup.exe");
    let webview_installer_running_info =
        is_process_running("MicrosoftEdgeWebview2Setup.exe".to_string(), None).unwrap_or_default();
    if !webview_installer_running_info.0 {
        // use reqwest to download the installer
        let res = REQUEST_CLIENT
            .get("https://go.microsoft.com/fwlink/p/?LinkId=2124703")
            .send()
            .await;
        if let Err(e) = res {
            let hwnd = dialog_hwnd.take();
            unsafe {
                SendMessageW(hwnd.unwrap(), WM_CLOSE, Some(WPARAM(0)), Some(LPARAM(0)));
            }
            error_dialog(format!("WebView2 运行时下载失败: {}", e));
            exit_and_release_mutex(0, &singleton_state);
            return;
        }
        let res = res.unwrap();

        let wv2_installer_blob = res.bytes().await;
        if let Err(e) = wv2_installer_blob {
            let hwnd = dialog_hwnd.take();
            unsafe {
                SendMessageW(hwnd.unwrap(), WM_CLOSE, Some(WPARAM(0)), Some(LPARAM(0)));
            }
            error_dialog(format!("WebView2 运行时下载失败: {}", e));
            exit_and_release_mutex(0, &singleton_state);
            return;
        }
        let wv2_installer_blob = wv2_installer_blob.unwrap();

        let write_res = tokio::fs::write(&installer_path, wv2_installer_blob).await;
        if let Err(e) = write_res {
            let hwnd = dialog_hwnd.take();
            unsafe {
                SendMessageW(hwnd.unwrap(), WM_CLOSE, Some(WPARAM(0)), Some(LPARAM(0)));
            }
            error_dialog(format!("WebView2 运行时安装程序写入失败: {}", e));
            exit_and_release_mutex(0, &singleton_state);
            return;
        }
    }

    // change content of the dialog
    let content = "正在安装 WebView2 运行时...";
    let content_utf16_nul = content
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<u16>>();
    unsafe {
        SendMessageW(
            *dialog_hwnd.as_ref().unwrap(),
            TDM_UPDATE_ELEMENT_TEXT.0 as u32,
            Some(WPARAM(TDE_CONTENT.0.try_into().unwrap())),
            Some(LPARAM(content_utf16_nul.as_ptr() as isize)),
        );
        SendMessageW(
            *dialog_hwnd.as_ref().unwrap(),
            TDM_ENABLE_BUTTON.0 as u32,
            Some(WPARAM(IDCANCEL.0 as usize)),
            Some(LPARAM(0)),
        );
    }

    let id = if webview_installer_running_info.0 {
        webview_installer_running_info.1.unwrap()
    } else {
        tokio::process::Command::new(installer_path.clone())
            .arg("/install")
            .spawn()
            .unwrap()
            .id()
            .unwrap()
    };

    let status = wait_for_pid(id);
    if let Err(e) = status {
        let hwnd = dialog_hwnd.take();
        unsafe {
            SendMessageW(hwnd.unwrap(), WM_CLOSE, Some(WPARAM(0)), Some(LPARAM(0)));
        }
        error_dialog(format!("WebView2 运行时安装失败: {}", e));
        exit_and_release_mutex(0, &singleton_state);
        return;
    }
    let status = status.unwrap();

    let _ = tokio::fs::remove_file(installer_path).await;
    if status.success() {
        // close the dialog
        let hwnd = dialog_hwnd.take();
        unsafe {
            SendMessageW(hwnd.unwrap(), WM_CLOSE, Some(WPARAM(0)), Some(LPARAM(0)));
        }
        let _ = tokio::process::Command::new(std::env::current_exe().unwrap())
            .arg(command.clone())
            .spawn();
        exit_and_release_mutex(0, &singleton_state);
    } else {
        let hwnd = dialog_hwnd.take();
        unsafe {
            SendMessageW(hwnd.unwrap(), WM_CLOSE, Some(WPARAM(0)), Some(LPARAM(0)));
        }
        error_dialog("WebView2 运行时安装失败".to_string());
        exit_and_release_mutex(0, &singleton_state);
    }
}

fn error_dialog(description: String) {
    rfd::MessageDialog::new()
        .set_title("出错了")
        .set_description(description)
        .set_level(rfd::MessageLevel::Error)
        .show();
}

fn exit_and_release_mutex(code: i32, state: &SingletonState) {
    singleton::destroy(state);
    std::process::exit(code);
}
