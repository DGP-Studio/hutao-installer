// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod api;
pub mod cli;
pub mod installer;
pub mod module;
pub mod utils;

use clap::Parser;
use cli::arg::{Command, UpdateArgs};
use std::time::Duration;
use tauri::{window::Color, WindowEvent};
use tauri_utils::{config::WindowEffectsConfig, WindowEffect};
use utils::uac::{check_elevated, run_elevated};

lazy_static::lazy_static! {
    pub static ref REQUEST_CLIENT: reqwest::Client = reqwest::Client::builder()
        .user_agent(ua_string())
        .gzip(true)
        .zstd(true)
        .read_timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();
}

fn ua_string() -> String {
    let winver = nt_version::get();
    let cpu_cores = num_cpus::get();
    let wv2ver = tauri::webview_version();
    let wv2ver = if let Ok(ver) = wv2ver {
        ver
    } else {
        "Unknown".to_string()
    };
    format!(
        "HutaoInstaller/{} Webview2/{} Windows/{}.{}.{} Threads/{}",
        env!("CARGO_PKG_VERSION"),
        wv2ver,
        winver.0,
        winver.1,
        winver.2 & 0xffff,
        cpu_cores
    )
}

fn main() {
    use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
    let _ = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) };
    let cli = cli::Cli::parse();
    let command = cli.command();
    let wv2ver = tauri::webview_version();
    if wv2ver.is_err() {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(module::wv2::install_webview2());
        return;
    }

    // TODO: bypass elevation
    // let elevated = check_elevated().unwrap_or(false);
    // if !elevated {
    //     let _ = run_elevated(std::env::current_exe().unwrap(), cli.command_as_str());
    //     return;
    // }

    match command {
        Command::Install => {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(tauri_main(None));
        }
        Command::Update(args) => {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(tauri_main(Some(args)));
        }
    }
}

async fn tauri_main(args: Option<UpdateArgs>) {
    tauri::async_runtime::set(tokio::runtime::Handle::current());
    let (major, minor, build) = nt_version::get();
    let build = (build & 0xffff) as u16;
    let is_lower_than_win10_22h2 = major < 10 && build < 19045;
    let is_lower_than_win11_22h2 = major < 10 && build > 22000 && build < 22621;
    if is_lower_than_win10_22h2 || is_lower_than_win11_22h2 {
        rfd::MessageDialog::new()
            .set_title("错误")
            .set_description("不支持的操作系统版本")
            .show();
        return;
    }
    // use 22000 as the build number of Windows 11
    let is_win11 = major == 10 && minor == 0 && build >= 22000;
    let is_win11_ = is_win11;

    // set cwd to temp dir
    let temp_dir = std::env::temp_dir();
    let res = std::env::set_current_dir(&temp_dir);
    if res.is_err() {
        rfd::MessageDialog::new()
            .set_title("错误")
            .set_description("无法访问临时文件夹")
            .show();
        return;
    }
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            // things which can be run directly
            api::generic_get_patch,
            api::homa_login,
            api::homa_fetch_cdn,
            api::homa_fetch_userinfo,
            installer::error_dialog,
            installer::message_dialog,
            installer::get_config,
            installer::open_tos,
            installer::speedtest_1mb,
            installer::create_desktop_lnk,
            installer::launch_and_exit
        ])
        .manage(args)
        .setup(move |app| {
            let temp_dir_for_data = temp_dir.join("HutaoInstaller");
            let mut main_window = tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title(" ")
            .resizable(false)
            .maximizable(false)
            .transparent(true)
            .inner_size(520.0, 300.0)
            .center();
            if !cfg!(debug_assertions) {
                main_window = main_window.data_directory(temp_dir_for_data).visible(false);
            }
            let main_window = main_window.build().unwrap();
            #[cfg(debug_assertions)]
            {
                let window = tauri::Manager::get_webview_window(app, "main");
                if let Some(window) = window {
                    window.open_devtools();
                }
            }
            if is_win11 {
                let _ = main_window.set_effects(Some(WindowEffectsConfig {
                    effects: vec![WindowEffect::Mica],
                    ..Default::default()
                }));
            } else {
                // if mica is not available, just use solid background.
                let _ = match dark_light::detect()? {
                    dark_light::Mode::Dark => {
                        main_window.set_background_color(Some(Color(0, 0, 0, 255)))
                    }
                    _ => main_window.set_background_color(Some(Color(255, 255, 255, 255))),
                };
            }
            Ok(())
        })
        .on_window_event(move |window, event| {
            if let WindowEvent::ThemeChanged(theme) = event {
                if !is_win11_ {
                    match theme {
                        tauri::Theme::Dark => {
                            let _ = window.set_background_color(Some(Color(0, 0, 0, 255)));
                        }
                        tauri::Theme::Light => {
                            let _ = window.set_background_color(Some(Color(255, 255, 255, 255)));
                        }
                        _ => {}
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
