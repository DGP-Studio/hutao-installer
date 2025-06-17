// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod api;
pub mod cli;
pub mod fs;
pub mod installer;
pub mod module;
pub mod utils;

use crate::utils::Version;
use clap::Parser;
use cli::arg::{Command, UpdateArgs};
use module::singleton;
use reqwest::header::{HeaderMap, HeaderValue};
use sentry::protocol::Context;
use std::collections::BTreeMap;
use tauri::{WindowEvent, window::Color};
use tauri_utils::{WindowEffect, config::WindowEffectsConfig};
use utils::{device::get_device_id, windows_version::get_windows_version};

lazy_static::lazy_static! {
    pub static ref REQUEST_CLIENT: reqwest::Client = reqwest::Client::builder()
        .default_headers(hutao_trace_headers())
        .user_agent(ua_string())
        .gzip(true)
        .read_timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();
}

fn ua_string() -> String {
    let pkg_name = format!("HutaoInstaller-{}", env!("BUILD_MODE"));
    let winver = get_windows_version();
    let cpu_cores = num_cpus::get();
    let wv2ver = tauri::webview_version();
    let wv2ver = if let Ok(ver) = wv2ver {
        ver
    } else {
        "Unknown".to_string()
    };
    format!(
        "{}/{} Webview2/{} Windows/{}.{}.{} Threads/{}",
        pkg_name,
        env!("CARGO_PKG_VERSION"),
        wv2ver,
        winver.major,
        winver.minor,
        winver.build,
        cpu_cores
    )
}

fn hutao_trace_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    let hutao_device_id = get_device_id();
    if hutao_device_id.is_err() {
        return headers;
    }
    headers.insert(
        "x-hutao-device-id",
        HeaderValue::from_str(hutao_device_id.unwrap().as_str()).unwrap(),
    );

    headers
}

fn main() {
    use windows::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};
    let _ = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) };

    unsafe {
        std::env::set_var("RUST_BACKTRACE", "1");
        std::env::set_var("RUST_LIB_BACKTRACE", "1");
    }

    let _guard = sentry::init((
        "https://59ff148bff0f509baf01516d1f075d11@sentry.snapgenshin.com/10",
        sentry::ClientOptions {
            release: sentry::release_name!(),
            debug: cfg!(debug_assertions),
            auto_session_tracking: true,
            max_breadcrumbs: 1000,
            sample_rate: 1.0,
            traces_sample_rate: 1.0,
            ..Default::default()
        },
    ));

    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("app".into()),
        message: Some("HutaoInstaller started".into()),
        level: sentry::Level::Info,
        ..Default::default()
    });

    let cli = cli::Cli::parse();
    let command = cli.command();
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(configure_sentry_scope(cli.command_as_str()));

    let wv2ver = tauri::webview_version();
    if wv2ver.is_err() {
        sentry::add_breadcrumb(sentry::Breadcrumb {
            category: Some("webview2".into()),
            message: Some("WebView2 not installed".into()),
            level: sentry::Level::Warning,
            ..Default::default()
        });
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(module::wv2::install_webview2(cli.command_as_str()));
        return;
    }

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
    let win_ver = get_windows_version();
    let win10_22h2_ver = Version::new(10, 0, 19045, 5371);
    let win11_ver = Version::new(10, 0, 22000, 0);

    if win_ver < win10_22h2_ver {
        rfd::MessageDialog::new()
            .set_title("错误")
            .set_description(format!(
                "不支持的操作系统版本\n需要 10.0.19045.5371 及更新版本，当前系统版本: {}",
                win_ver
            ))
            .set_level(rfd::MessageLevel::Error)
            .show();
        return;
    }
    // use 22000 as the build number of Windows 11
    let is_win11 = win_ver >= win11_ver;
    let is_win11_ = is_win11;

    // set cwd to temp dir
    let temp_dir = std::env::temp_dir();
    let res = std::env::set_current_dir(&temp_dir);
    if res.is_err() {
        rfd::MessageDialog::new()
            .set_title("错误")
            .set_description("无法访问临时文件夹")
            .set_level(rfd::MessageLevel::Error)
            .show();
        capture_and_return!(anyhow::anyhow!(
            "Failed to set current dir to temp dir: {:?}, {:?}",
            temp_dir,
            res.err()
        ));
    }
    tauri::Builder::default()
        .plugin(singleton::init_as_plugin())
        .invoke_handler(tauri::generate_handler![
            // things which can be run directly
            api::generic_is_oversea,
            api::generic_get_patch,
            api::homa_request_verify_code,
            api::homa_register,
            api::homa_login,
            api::homa_use_redeem_code,
            api::homa_fetch_cdn,
            api::homa_fetch_userinfo,
            installer::error_dialog,
            installer::confirm_dialog,
            installer::message_dialog,
            installer::need_self_update,
            installer::self_update,
            installer::get_config,
            installer::get_changelog,
            installer::open_browser,
            installer::speedtest_5mb,
            installer::check_temp_package_valid,
            installer::head_package,
            installer::extract_package,
            installer::download_package,
            installer::check_vcrt,
            installer::install_vcrt,
            installer::check_globalsign_r45,
            installer::is_hutao_running,
            installer::kill_process,
            installer::remove_outdated_package,
            installer::install_package,
            installer::create_desktop_lnk,
            installer::exit,
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
                .inner_size(700.0, 400.0)
            .additional_browser_args("--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --autoplay-policy=no-user-gesture-required")
            .center();
            if !cfg!(debug_assertions) {
                main_window = main_window.data_directory(temp_dir_for_data).visible(false);
            }
            let main_window = main_window.build().unwrap();
            sentry::add_breadcrumb(
                sentry::Breadcrumb {
                    category: Some("app".into()),
                    message: Some("Main window created".into()),
                    level: sentry::Level::Info,
                    ..Default::default()
                },
            );
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

async fn configure_sentry_scope(command: String) {
    let ip_address = api::generic_get_ip_info().await.unwrap_or_default().ip;

    sentry::configure_scope(|scope| {
        scope.set_context(
            "app",
            Context::Other(BTreeMap::from([
                ("Name".to_string(), "HutaoInstaller".into()),
                ("Version".to_string(), env!("CARGO_PKG_VERSION").into()),
                ("Command".to_string(), command.into()),
                ("Mode".to_string(), env!("BUILD_MODE").into()),
            ])),
        );

        let wv2ver_ = tauri::webview_version();
        scope.set_context(
            "WebView2",
            Context::Other(BTreeMap::from([
                ("Supported".to_string(), wv2ver_.is_ok().into()),
                ("Version".to_string(), wv2ver_.unwrap_or_default().into()),
            ])),
        );

        scope.set_user(
            sentry::User {
                id: Some(get_device_id().unwrap_or_default()),
                ip_address: Some(ip_address.parse().unwrap()),
                ..Default::default()
            }
            .into(),
        );

        let windows_version = get_windows_version();
        scope.set_context(
            "os",
            Context::Other(BTreeMap::from([
                ("name".to_string(), "Windows".into()),
                ("version".to_string(), windows_version.to_string().into()),
            ])),
        );
    });
}
