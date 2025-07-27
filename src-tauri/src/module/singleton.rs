use tauri::{
    AppHandle, Manager, RunEvent, Runtime,
    plugin::{self, TauriPlugin},
};
use windows::{
    Win32::{
        Foundation::{
            CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HINSTANCE, HWND, LPARAM,
            LRESULT, WPARAM,
        },
        Graphics::Gdi::HBRUSH,
        System::{
            DataExchange::COPYDATASTRUCT,
            Threading::{AttachThreadInput, CreateMutexW, GetCurrentThreadId, ReleaseMutex},
        },
        UI::{
            Input::KeyboardAndMouse::SetFocus,
            WindowsAndMessaging::{
                CREATESTRUCTW, CreateWindowExW, DefWindowProcW, DestroyWindow, FindWindowW,
                GWL_STYLE, GWLP_USERDATA, GetWindowLongPtrW, GetWindowThreadProcessId, HCURSOR,
                HICON, IsIconic, IsWindowVisible, RegisterClassExW, SW_RESTORE, SW_SHOW,
                SendMessageW, SetForegroundWindow, SetWindowLongPtrW, ShowWindow, WM_COPYDATA,
                WM_CREATE, WM_DESTROY, WNDCLASS_STYLES, WNDCLASSEXW, WS_EX_LAYERED,
                WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_OVERLAPPED, WS_POPUP,
                WS_VISIBLE,
            },
        },
    },
    core::PCWSTR,
};

const WMCOPYDATA_SINGLE_INSTANCE_DATA: usize = 1542;
const SINGLETON_ID: &str = "HUTAO-INSTALLER-SINGLETON";

pub struct UserData<R: Runtime> {
    pub app: Option<AppHandle<R>>,
    pub hwnd: *mut Option<HWND>,
}

impl<R: Runtime> UserData<R> {
    unsafe fn from_hwnd_raw(hwnd: HWND) -> *mut Self {
        GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Self
    }

    unsafe fn from_hwnd<'a>(hwnd: HWND) -> &'a mut Self {
        &mut *Self::from_hwnd_raw(hwnd)
    }
}

#[derive(Default)]
pub struct SingletonState {
    mutex: Option<isize>,
    hwnd: Option<isize>,
}

pub fn init<R: Runtime>(userdata: UserData<R>) -> (bool, SingletonState) {
    let clz_name = encode_wide(format!("{SINGLETON_ID}-sic"));
    let wnd_name = encode_wide(format!("{SINGLETON_ID}-siw"));
    let mtx_name = encode_wide(format!("{SINGLETON_ID}-sim"));

    unsafe {
        let hmutex = CreateMutexW(None, true, PCWSTR(mtx_name.as_ptr()));

        if GetLastError() == ERROR_ALREADY_EXISTS {
            let hwnd = FindWindowW(PCWSTR(clz_name.as_ptr()), PCWSTR(wnd_name.as_ptr()));
            if let Ok(hwnd) = hwnd {
                if !hwnd.is_invalid() {
                    sentry::add_breadcrumb(sentry::Breadcrumb {
                        category: Some("singleton".into()),
                        message: Some("Another instance is running".into()),
                        level: sentry::Level::Warning,
                        ..Default::default()
                    });

                    let data = COPYDATASTRUCT {
                        dwData: WMCOPYDATA_SINGLE_INSTANCE_DATA,
                        cbData: 0,
                        lpData: std::ptr::null_mut(),
                    };

                    SendMessageW(
                        hwnd,
                        WM_COPYDATA,
                        WPARAM(0).into(),
                        LPARAM(&data as *const _ as _).into(),
                    );
                    return (false, SingletonState::default());
                }
            }

            sentry::add_breadcrumb(sentry::Breadcrumb {
                category: Some("singleton".into()),
                message: Some("Mutex found, but window not found".into()),
                level: sentry::Level::Warning,
                ..Default::default()
            });
            (true, SingletonState::default())
        } else {
            sentry::add_breadcrumb(sentry::Breadcrumb {
                category: Some("singleton".into()),
                message: Some("Mutex created".into()),
                level: sentry::Level::Info,
                ..Default::default()
            });
            let userdata = Box::into_raw(Box::new(userdata));
            let hwnd = create_event_target_window(&clz_name, &wnd_name, userdata);
            (
                true,
                SingletonState {
                    mutex: Some(hmutex.unwrap().0 as isize),
                    hwnd: Some(hwnd.0 as isize),
                },
            )
        }
    }
}

pub fn init_as_plugin<R: Runtime>() -> TauriPlugin<R> {
    plugin::Builder::new("singleton")
        .setup(|app, _api| {
            let (res, state) = init(UserData {
                app: Some(app.clone()),
                hwnd: std::ptr::null_mut(),
            });
            if !res {
                sentry::add_breadcrumb(sentry::Breadcrumb {
                    category: Some("singleton_as_plugin".into()),
                    message: Some("Another instance is running".into()),
                    level: sentry::Level::Warning,
                    ..Default::default()
                });
                app.cleanup_before_exit();
                std::process::exit(0);
            } else {
                sentry::add_breadcrumb(sentry::Breadcrumb {
                    category: Some("singleton_as_plugin".into()),
                    message: Some("Singleton instance created".into()),
                    level: sentry::Level::Info,
                    ..Default::default()
                });
                app.manage(state);
            }

            Ok(())
        })
        .on_event(|app, event| {
            if let RunEvent::Exit = event {
                sentry::add_breadcrumb(sentry::Breadcrumb {
                    category: Some("singleton_as_plugin".into()),
                    message: Some("Singleton instance destroy invoked".into()),
                    level: sentry::Level::Info,
                    ..Default::default()
                });
                destroy_plugin(app);
            }
        })
        .build()
}

pub fn destroy(state: &SingletonState) {
    if let Some(hmutex) = state.mutex {
        unsafe {
            let _ = ReleaseMutex(HANDLE(hmutex as _));
            let _ = CloseHandle(HANDLE(hmutex as _));
        }
    }
    if let Some(hwnd) = state.hwnd {
        unsafe {
            let _ = DestroyWindow(HWND(hwnd as _));
        }
    }
    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("singleton".into()),
        message: Some("Singleton instance destroyed".into()),
        level: sentry::Level::Info,
        ..Default::default()
    });
}

pub fn destroy_plugin<R: Runtime, M: Manager<R>>(manager: &M) {
    if let Some(state) = manager.try_state::<SingletonState>() {
        destroy(state.inner())
    }
}

unsafe extern "system" fn singleton_window_proc<R: Runtime>(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            let create_struct = &*(lparam.0 as *const CREATESTRUCTW);
            let userdata = create_struct.lpCreateParams as *const UserData<R>;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, userdata as _);
            sentry::add_breadcrumb(sentry::Breadcrumb {
                category: Some("singleton".into()),
                message: Some("Singleton window created".into()),
                level: sentry::Level::Info,
                ..Default::default()
            });
            LRESULT(0)
        }

        WM_COPYDATA => {
            let cds_ptr = lparam.0 as *const COPYDATASTRUCT;
            if (*cds_ptr).dwData == WMCOPYDATA_SINGLE_INSTANCE_DATA {
                let userdata = UserData::<R>::from_hwnd(hwnd);
                if let Some(app) = &userdata.app {
                    let window = app.get_webview_window("main");
                    if let Some(window) = window {
                        match window.hwnd() {
                            Ok(hwnd) => {
                                switch_to(HWND(hwnd.0 as _));
                                sentry::add_breadcrumb(sentry::Breadcrumb {
                                    category: Some("singleton".into()),
                                    message: Some("Switch to webview window".into()),
                                    level: sentry::Level::Info,
                                    ..Default::default()
                                });
                            }
                            Err(e) => {
                                sentry::add_breadcrumb(sentry::Breadcrumb {
                                    category: Some("singleton".into()),
                                    message: Some(format!("Failed to get window handle: {}", e)),
                                    level: sentry::Level::Error,
                                    ..Default::default()
                                });
                                let _ = window.set_focus();
                            }
                        }
                    }
                } else if let Some(hwnd) = *userdata.hwnd {
                    switch_to(HWND(hwnd.0 as _));
                    sentry::add_breadcrumb(sentry::Breadcrumb {
                        category: Some("singleton".into()),
                        message: Some("Switch to wv2 install window".into()),
                        level: sentry::Level::Info,
                        ..Default::default()
                    });
                }
            }
            LRESULT(1)
        }

        WM_DESTROY => {
            let userdata = UserData::<R>::from_hwnd_raw(hwnd);
            drop(Box::from_raw(userdata));
            LRESULT(0)
        }

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn create_event_target_window<R: Runtime>(
    class_name: &[u16],
    window_name: &[u16],
    userdata: *const UserData<R>,
) -> HWND {
    unsafe {
        let class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: WNDCLASS_STYLES(0),
            lpfnWndProc: Some(singleton_window_proc::<R>),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: HINSTANCE(std::ptr::null_mut()),
            hIcon: HICON(std::ptr::null_mut()),
            hCursor: HCURSOR(std::ptr::null_mut()),
            hbrBackground: HBRUSH(std::ptr::null_mut()),
            lpszMenuName: PCWSTR(std::ptr::null()),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            hIconSm: HICON(std::ptr::null_mut()),
        };

        RegisterClassExW(&class);

        let hwnd = CreateWindowExW(
            WS_EX_NOACTIVATE | WS_EX_TRANSPARENT | WS_EX_LAYERED | WS_EX_TOOLWINDOW,
            PCWSTR(class_name.as_ptr()),
            PCWSTR(window_name.as_ptr()),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            None,
            None,
            None,
            Some(userdata as _),
        );
        let hwnd = hwnd.unwrap();
        SetWindowLongPtrW(hwnd, GWL_STYLE, (WS_VISIBLE | WS_POPUP).0 as isize);
        hwnd
    }
}

fn encode_wide(string: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    std::os::windows::prelude::OsStrExt::encode_wide(string.as_ref())
        .chain(std::iter::once(0))
        .collect()
}

unsafe fn switch_to(hwnd: HWND) {
    let curr_thread_id = GetCurrentThreadId();
    let target_thread_id = GetWindowThreadProcessId(hwnd, None);

    let _ = AttachThreadInput(curr_thread_id, target_thread_id, true);

    let is_window_visible: bool = IsWindowVisible(hwnd).into();
    if !is_window_visible {
        let _ = ShowWindow(hwnd, SW_SHOW);
    }

    if IsIconic(hwnd).into() {
        let _ = ShowWindow(hwnd, SW_RESTORE);
    }
    let _ = SetForegroundWindow(hwnd);
    let _ = SetFocus(Some(hwnd));

    let _ = AttachThreadInput(curr_thread_id, target_thread_id, false);
}
