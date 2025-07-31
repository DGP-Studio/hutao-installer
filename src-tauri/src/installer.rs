use crate::{
    REAL_CURRENT_DIR, REQUEST_CLIENT, capture_and_return_err_message_string,
    cli::arg::UpdateArgs,
    fs::{create_http_stream, create_target_file, progressed_copy},
    utils::{
        Version,
        cert::{find_certificate, install_certificate},
        dir::get_desktop,
        font::{get_font_path, get_font_version, install_font_permanently},
        hash::run_sha256_file_hash_async,
        package_manager::{add_package, need_migration, remove_package, try_get_hutao_version},
        process::{self, is_process_running, is_process_running_by_pid, wait_for_pid},
        windows_version::get_windows_version,
    },
};
use serde::Serialize;
use std::{io::Read, path::Path, time::Instant};
use tauri::{AppHandle, Emitter, Runtime, State, WebviewWindow};
use tokio::io::AsyncWriteExt;
use winreg::{RegKey, enums::HKEY_LOCAL_MACHINE};
use winsafe::{
    CoCreateInstance, IPersistFile, IShellLink,
    co::{CLSCTX, CLSID, SW},
    prelude::{ole_IPersistFile, ole_IUnknown, shell_IShellLink},
};

#[cfg(feature = "offline")]
const OFFLINE_PACKAGE_PAYLOAD: &[u8] = include_bytes!("../Snap.Hutao.msix");

#[cfg(not(feature = "offline"))]
const OFFLINE_PACKAGE_PAYLOAD: &[u8] = &[];

const EMBEDDED_SEGOE_FLUENT_ICON_BINARY: &[u8] = include_bytes!("../SegoeIcons.ttf");
const EMBEDDED_SEGOE_FLUENT_ICON_NAME: &str = "Segoe Fluent Icons (TrueType)";
const EMBEDDED_SEGOE_FLUENT_ICON_FILENAME: &str = "SegoeIcons.ttf";
const EMBEDDED_SEGOE_FLUENT_ICON_VERSION: Version = Version::new(1, 39, 0, 0);

#[derive(Serialize, Debug, Clone)]
pub struct Config {
    pub version: String,
    pub is_update: bool,
    pub need_migration: bool,
    pub skip_self_update: bool,
    pub is_offline_mode: bool,
    pub embedded_version: Option<String>,
    pub curr_version: Option<String>,
    pub token: Option<String>,
}

#[tauri::command]
pub async fn error_dialog(title: String, message: String, window: WebviewWindow) {
    rfd::MessageDialog::new()
        .set_title(&title)
        .set_description(&message)
        .set_level(rfd::MessageLevel::Error)
        .set_parent(&window)
        .show();
}

#[tauri::command]
pub async fn confirm_dialog(title: String, message: String, window: WebviewWindow) -> bool {
    let ret = rfd::MessageDialog::new()
        .set_title(&title)
        .set_description(&message)
        .set_level(rfd::MessageLevel::Info)
        .set_parent(&window)
        .set_buttons(rfd::MessageButtons::YesNo)
        .show();

    matches!(ret, rfd::MessageDialogResult::Yes)
}

#[tauri::command]
pub async fn message_dialog(title: String, message: String, window: WebviewWindow) {
    rfd::MessageDialog::new()
        .set_title(&title)
        .set_description(&message)
        .set_level(rfd::MessageLevel::Info)
        .set_parent(&window)
        .show();
}

#[tauri::command]
pub async fn two_btn_custom_dialog(
    title: String,
    message: String,
    ok: String,
    cancel: String,
    window: WebviewWindow,
) -> bool {
    let ret = rfd::MessageDialog::new()
        .set_title(&title)
        .set_description(&message)
        .set_level(rfd::MessageLevel::Info)
        .set_parent(&window)
        .set_buttons(rfd::MessageButtons::OkCancelCustom(
            ok.clone(),
            cancel.clone(),
        ))
        .show();

    ret == rfd::MessageDialogResult::Custom(ok)
}

#[tauri::command]
pub async fn three_btn_custom_dialog(
    title: String,
    message: String,
    yes: String,
    no: String,
    cancel: String,
    window: WebviewWindow,
) -> (bool, bool) {
    let ret = rfd::MessageDialog::new()
        .set_title(&title)
        .set_description(&message)
        .set_level(rfd::MessageLevel::Info)
        .set_parent(&window)
        .set_buttons(rfd::MessageButtons::YesNoCancelCustom(
            yes.clone(),
            no.clone(),
            cancel.clone(),
        ))
        .show();

    (
        ret == rfd::MessageDialogResult::Custom(yes),
        ret == rfd::MessageDialogResult::Custom(no),
    )
}

#[tauri::command]
pub async fn need_self_update<R: Runtime>(app: AppHandle<R>) -> Result<bool, String> {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("installer".to_string()),
        message: Some("Checking self update".to_string()),
        level: sentry::Level::Info,
        ..Default::default()
    });
    let exe_path = std::env::current_exe().unwrap();
    let outdated = exe_path.with_extension("old");
    let outdated_exists = tokio::fs::try_exists(&outdated).await.unwrap();
    let _ = tokio::fs::remove_file(&outdated).await;

    if outdated_exists {
        return Ok(false);
    }

    let curr_ver = app.package_info().version.clone();
    let curr_ver = Version::new(curr_ver.major, curr_ver.minor, curr_ver.patch, 0);
    let url = "https://api.snapgenshin.com/patch/hutao-deployment";
    let resp = REQUEST_CLIENT.get(url).send().await;
    if resp.is_err() {
        return Err(format!("Failed to check self update: {:?}", resp.err()));
    }
    let resp = resp.unwrap();
    let json: Result<crate::api::GenericPatchResp, reqwest::Error> = resp.json().await;
    if json.is_err() {
        return Err(format!(
            "Failed to parse self update response: {:?}",
            json.err()
        ));
    }
    let json = json.unwrap();
    if json.retcode != 0 {
        return Err(format!("Failed to check self update: {:?}", json.message));
    }
    let data = json.data.unwrap();
    // remove last 2 chars
    let latest_ver = data.version;
    let latest_ver = Version::from_string(&latest_ver);
    if latest_ver.is_err() {
        return Err(format!(
            "Failed to parse latest version: {:?}",
            latest_ver.err()
        ));
    }
    let latest_ver = latest_ver.unwrap();
    Ok(curr_ver < latest_ver)
}

#[tauri::command]
pub async fn self_update<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("installer".to_string()),
        message: Some("Self-updating".to_string()),
        level: sentry::Level::Info,
        ..Default::default()
    });
    let exe_path = std::env::current_exe().unwrap();
    let outdated = exe_path.with_extension("old");
    let _ = tokio::fs::remove_file(&outdated).await;

    let res = tokio::fs::rename(&exe_path, &outdated).await;
    if let Err(e) = res {
        if e.kind() != std::io::ErrorKind::NotFound {
            capture_and_return_err_message_string!(format!("Failed to rename executable: {:?}", e));
        }
    }

    let res = REQUEST_CLIENT
        .get("https://api.qhy04.com/hutaocdn/deployment")
        .send()
        .await;
    if res.is_err() {
        return Err(format!("Failed to download new installer: {:?}", res.err()));
    }
    let res = res.unwrap();
    let new_installer_blob = res.bytes().await;
    if new_installer_blob.is_err() {
        return Err(format!(
            "Failed to get new installer content: {:?}",
            new_installer_blob.err()
        ));
    }
    let new_installer_blob = new_installer_blob.unwrap();
    let write_res = tokio::fs::write(&exe_path, new_installer_blob).await;
    if write_res.is_err() {
        capture_and_return_err_message_string!(format!(
            "Failed to write new installer: {:?}",
            write_res.err()
        ));
    }
    process::run(
        true,
        &exe_path,
        REAL_CURRENT_DIR.clone().into(),
        None::<&str>,
    );
    app.exit(0);

    Ok(())
}

#[tauri::command]
pub async fn open_browser(url: String) -> Result<(), String> {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("installer".to_string()),
        message: Some(format!("Opening browser: {url}")),
        level: sentry::Level::Info,
        ..Default::default()
    });
    if webbrowser::open(&url).is_ok() {
        Ok(())
    } else {
        Err(format!("Failed to open browser: {url:?}"))
    }
}

#[tauri::command]
pub async fn get_config<R: Runtime>(
    args: State<'_, Option<UpdateArgs>>,
    app: AppHandle<R>,
) -> Result<Config, String> {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("installer".to_string()),
        message: Some("Getting config".to_string()),
        level: sentry::Level::Info,
        ..Default::default()
    });

    let curr_ver = app.package_info().version.clone();
    let curr_ver = Version::new(curr_ver.major, curr_ver.minor, curr_ver.patch, 0);
    let offline = env!("BUILD_MODE") == "offline";
    let embedded_version = if offline {
        Some(
            Version::from_string(env!("EMBEDDED_VERSION"))
                .unwrap()
                .to_string(),
        )
    } else {
        None
    };

    let need_migration = need_migration();
    let exists = try_get_hutao_version();

    let update_args = args.inner().clone();
    if let Some(update_args) = update_args {
        return Ok(Config {
            version: curr_ver.to_string(),
            is_update: true,
            need_migration,
            skip_self_update: true,
            is_offline_mode: false,
            embedded_version,
            curr_version: exists,
            token: update_args.token,
        });
    }

    Ok(Config {
        version: curr_ver.to_string(),
        is_update: exists.is_some(),
        need_migration,
        skip_self_update: offline,
        is_offline_mode: offline,
        embedded_version,
        curr_version: exists,
        token: None,
    })
}

#[tauri::command]
pub async fn get_changelog(lang: String, from: String) -> Result<String, String> {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("installer".to_string()),
        message: Some(format!("Getting {lang} changelog: {from}")),
        level: sentry::Level::Info,
        ..Default::default()
    });
    let url = format!("https://api.qhy04.com/hutaocdn/changelog?lang={lang}&from={from}");
    let res = REQUEST_CLIENT.get(&url).send().await;
    if res.is_err() {
        return Err(format!("Failed to send http request: {:?}", res.err()));
    }

    let res = res.unwrap();
    let ctnt = res.text().await;
    if ctnt.is_err() {
        return Err(format!("Failed to get response content: {:?}", ctnt.err()));
    }

    Ok(ctnt.unwrap())
}

#[tauri::command]
pub async fn speedtest_5mb(url: String) -> Result<f64, String> {
    let start = Instant::now();
    let res = REQUEST_CLIENT
        .get(&url)
        .header("Range", "bytes=0-5242875")
        .send()
        .await;
    let elapsed = start.elapsed().as_millis();
    if res.is_err() {
        return Ok(-1.0);
    }
    Ok(5.0 / ((elapsed as f64) / (1000f64)))
}

#[tauri::command]
pub async fn check_temp_package_valid(sha256: String) -> Result<bool, String> {
    let temp_dir = std::env::temp_dir();
    let installer_path = temp_dir.as_path().join("Snap.Hutao.msix");
    let exists = tokio::fs::try_exists(installer_path.clone()).await.unwrap();
    if !exists {
        return Ok(false);
    }

    let hash = run_sha256_file_hash_async(installer_path.to_str().unwrap()).await;
    if hash.is_err() {
        capture_and_return_err_message_string!(format!(
            "Failed to hash installer: {:?}",
            hash.err()
        ));
    }

    let hash = hash.unwrap();
    Ok(hash == sha256)
}

#[tauri::command]
pub async fn head_package(mirror_url: String) -> Result<u64, String> {
    let res = REQUEST_CLIENT.head(&mirror_url).send().await;
    if res.is_err() {
        return Err(format!("Failed to send http request: {:?}", res.err()));
    }

    let res = res.unwrap();
    let headers = res.headers();
    let len = headers.get("content-length");
    if len.is_none() {
        return Err("Failed to get content length".to_string());
    }

    let len = len.unwrap().to_str();
    if len.is_err() {
        return Err(format!("Failed to parse content length: {:?}", len.err()));
    }

    let len = len.unwrap();

    let len = len.parse::<u64>();
    if len.is_err() {
        return Err(format!("Failed to parse content length: {:?}", len.err()));
    }

    Ok(len.unwrap())
}

#[tauri::command]
pub async fn extract_package() -> Result<(), String> {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("installer".to_string()),
        message: Some("Extracting package".to_string()),
        level: sentry::Level::Info,
        ..Default::default()
    });
    let temp_dir = std::env::temp_dir();
    let installer_path = temp_dir.as_path().join("Snap.Hutao.msix");

    let decompressed_data = decompress(OFFLINE_PACKAGE_PAYLOAD);
    if decompressed_data.is_err() {
        capture_and_return_err_message_string!(format!(
            "Failed to decompress offline package: {:?}",
            decompressed_data.err()
        ));
    }
    let decompressed_data = decompressed_data?;

    let file = tokio::fs::File::create(installer_path).await;
    if file.is_err() {
        capture_and_return_err_message_string!(format!(
            "Failed to create installer: {:?}",
            file.err()
        ));
    }

    let mut file = file.unwrap();
    let write_res = file.write_all(&decompressed_data).await;
    if write_res.is_err() {
        capture_and_return_err_message_string!(format!(
            "Failed to write installer: {:?}",
            write_res.err()
        ));
    }
    Ok(())
}

#[tauri::command]
pub async fn download_package(
    mirror_url: String,
    id: String,
    window: WebviewWindow,
) -> Result<(), String> {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("installer".to_string()),
        message: Some(format!("Downloading package from {mirror_url}")),
        level: sentry::Level::Info,
        ..Default::default()
    });
    let temp_dir = std::env::temp_dir();
    let installer_path = temp_dir.as_path().join("Snap.Hutao.msix");
    let stream_res = create_http_stream(&mirror_url, 0, 0).await;
    if stream_res.is_err() {
        return Err(format!("Failed to download msix: {:?}", stream_res.err()));
    }
    let (mut stream, len) = stream_res.unwrap();
    let target_file_create_res =
        create_target_file(installer_path.as_os_str().to_str().unwrap()).await;
    if target_file_create_res.is_err() {
        capture_and_return_err_message_string!(format!(
            "Failed to create target msix file: {:?}",
            target_file_create_res.err()
        ));
    }
    let mut target = target_file_create_res.unwrap();
    let progress_noti = move |downloaded: usize| {
        let _ = window.emit(&id, serde_json::json!((downloaded, len)));
    };
    let res = progressed_copy(&mut stream, &mut target, progress_noti).await;
    if res.is_err() {
        return Err(format!("Failed to download msix: {:?}", res.err()));
    }
    // close streams
    drop(stream);
    drop(target);
    Ok(())
}

#[tauri::command]
pub async fn check_vcrt() -> Result<bool, String> {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("installer".to_string()),
        message: Some("Checking vcrt".to_string()),
        level: sentry::Level::Info,
        ..Default::default()
    });
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let x64_path = r#"SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64"#.to_string();
    let x86_path =
        r#"SOFTWARE\WOW6432Node\Microsoft\VisualStudio\14.0\VC\Runtimes\x64"#.to_string();
    let x64_key = hklm.open_subkey(&x64_path);
    let x86_key = hklm.open_subkey(&x86_path);

    if let Ok(key) = x64_key {
        if let Ok(x64_installed) = key.get_value::<u32, _>("Installed") {
            if let Ok(key) = x86_key {
                if let Ok(x86_installed) = key.get_value::<u32, _>("Installed") {
                    return Ok(x64_installed == 1 && x86_installed == 1);
                }
            }
        }
    }

    Ok(false)
}

#[tauri::command]
pub async fn install_vcrt(id: String, window: WebviewWindow) -> Result<(), String> {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("installer".to_string()),
        message: Some("Installing vcrt".to_string()),
        level: sentry::Level::Info,
        ..Default::default()
    });

    const VCRT_INSTALLER_NAME: &str = "vc_redist.x64.exe";

    let temp_dir = std::env::temp_dir();
    let installer_path = temp_dir.as_path().join(VCRT_INSTALLER_NAME);

    let installer_running_status =
        is_process_running(VCRT_INSTALLER_NAME.to_string(), None).unwrap_or_default();
    if !installer_running_status.0 {
        sentry::add_breadcrumb(sentry::Breadcrumb {
            category: Some("installer".to_string()),
            message: Some("Downloading vcrt".to_string()),
            level: sentry::Level::Info,
            ..Default::default()
        });
        let url = "https://aka.ms/vs/17/release/vc_redist.x64.exe";
        let stream_res = create_http_stream(url, 0, 0).await;
        if stream_res.is_err() {
            return Err(format!(
                "Failed to download vcrt installer: {:?}",
                stream_res.err()
            ));
        }
        let (mut stream, len) = stream_res.unwrap();
        let target_file_create_res =
            create_target_file(installer_path.as_os_str().to_str().unwrap()).await;
        if target_file_create_res.is_err() {
            capture_and_return_err_message_string!(format!(
                "Failed to create target vcrt installer file: {:?}",
                target_file_create_res.err()
            ));
        }
        let mut target = target_file_create_res.unwrap();
        let progress_noti = move |downloaded: usize| {
            let _ = window.emit(&id, serde_json::json!((downloaded, len)));
        };
        progressed_copy(&mut stream, &mut target, progress_noti)
            .await
            .unwrap();
        // close streams
        drop(stream);
        drop(target);
    }

    let id = if installer_running_status.0 {
        sentry::add_breadcrumb(sentry::Breadcrumb {
            category: Some("installer".to_string()),
            message: Some("VCRT installer running, wait for it".to_string()),
            level: sentry::Level::Info,
            ..Default::default()
        });
        installer_running_status.1.unwrap()
    } else {
        sentry::add_breadcrumb(sentry::Breadcrumb {
            category: Some("installer".to_string()),
            message: Some("Spawning vcrt installer".to_string()),
            level: sentry::Level::Info,
            ..Default::default()
        });
        let cmd = tokio::process::Command::new(&installer_path)
            .arg("/install")
            .arg("/quiet")
            .arg("/norestart")
            .spawn();
        if cmd.is_err() {
            capture_and_return_err_message_string!(format!(
                "Failed to spawn vcrt installer: {:?}",
                cmd.err()
            ));
        }
        cmd.unwrap().id().unwrap()
    };

    let status = wait_for_pid(id);
    if status.is_err() {
        capture_and_return_err_message_string!(format!(
            "Failed to wait for vcrt installer: {:?}",
            status.err()
        ));
    }
    let status = status.unwrap();
    let code = status.code().unwrap();
    if !status.success() && code != 1638 && code != 3010 {
        capture_and_return_err_message_string!(format!("VCRT installer failed: {:?}", status));
    }
    let _ = tokio::fs::remove_file(installer_path).await;
    Ok(())
}

#[tauri::command]
pub async fn check_globalsign_r45(window: WebviewWindow) -> Result<(), String> {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("installer".to_string()),
        message: Some("Checking globalsign r45 certificate".to_string()),
        level: sentry::Level::Info,
        ..Default::default()
    });
    let find_res = find_certificate("BE, GlobalSign nv-sa, GlobalSign Code Signing Root R45").await;
    if find_res.is_err() {
        return Err(format!("Failed to find certificate: {:?}", find_res.err()));
    }

    let found = find_res.unwrap();
    if found {
        return Ok(());
    }

    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("installer".to_string()),
        message: Some("Installing globalsign r45 certificate".to_string()),
        level: sentry::Level::Info,
        ..Default::default()
    });
    let url = "https://secure.globalsign.com/cacert/codesigningrootr45.crt";
    let res = REQUEST_CLIENT.get(url).send().await;
    if res.is_err() {
        return Err(format!("Failed to send http request: {:?}", res.err()));
    }

    let res = res.unwrap();
    let cert_ctnt = res.bytes().await;
    if cert_ctnt.is_err() {
        return Err(format!(
            "Failed to get certificate content: {:?}",
            cert_ctnt.err()
        ));
    }

    let cert_ctnt = cert_ctnt.unwrap();
    let install_res = install_certificate(cert_ctnt, window).await;
    if install_res.is_err() {
        return Err(format!(
            "Failed to install certificate: {:?}",
            install_res.err()
        ));
    }

    Ok(())
}

#[tauri::command]
pub async fn check_segoe_fluent_icons_font() -> Result<bool, String> {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("installer".to_string()),
        message: Some("Checking Segoe Fluent Icons font".to_string()),
        level: sentry::Level::Info,
        ..Default::default()
    });

    let win_ver = get_windows_version();
    if win_ver.build >= 22000 {
        // Windows 11 and later have Segoe Fluent Icons font pre-installed
        return Ok(true);
    }

    let font_name = "Segoe Fluent Icons";
    let font_path = get_font_path(font_name);
    if font_path.is_none() {
        return Ok(false);
    }

    let font_path = font_path.unwrap();
    let font_exists = tokio::fs::try_exists(&font_path).await.unwrap_or(false);
    if !font_exists {
        return Ok(false);
    }

    let font_raw_version = get_font_version(&font_path);
    if font_raw_version.is_none() {
        return Ok(false);
    }

    let font_raw_version = font_raw_version.unwrap();
    let font_raw_version = font_raw_version[8..].to_string();
    let font_version = Version::from_string(&font_raw_version);
    if font_version.is_err() {
        return Ok(false);
    }

    let font_version = font_version.unwrap();
    Ok(font_version >= EMBEDDED_SEGOE_FLUENT_ICON_VERSION)
}

#[tauri::command]
pub async fn install_segoe_fluent_icons_font() -> Result<(), String> {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("installer".to_string()),
        message: Some("Installing Segoe Fluent Icons font".to_string()),
        level: sentry::Level::Info,
        ..Default::default()
    });

    let temp_dir = std::env::temp_dir();
    let font_file = temp_dir.join(EMBEDDED_SEGOE_FLUENT_ICON_FILENAME);

    let decompressed_data = decompress(EMBEDDED_SEGOE_FLUENT_ICON_BINARY);
    if decompressed_data.is_err() {
        capture_and_return_err_message_string!(format!(
            "Failed to decompress embedded font: {:?}",
            decompressed_data.err()
        ));
    }
    let decompressed_data = decompressed_data?;

    let file = tokio::fs::File::create(&font_file).await;
    if file.is_err() {
        capture_and_return_err_message_string!(format!(
            "Failed to create font file: {:?}",
            file.err()
        ));
    }

    let mut file = file.unwrap();
    let write_res = file.write_all(&decompressed_data).await;
    if write_res.is_err() {
        capture_and_return_err_message_string!(format!(
            "Failed to write font file: {:?}",
            write_res.err()
        ));
    }

    let install_res =
        install_font_permanently(font_file.to_str().unwrap(), EMBEDDED_SEGOE_FLUENT_ICON_NAME);
    if install_res.is_err() {
        capture_and_return_err_message_string!(format!(
            "Failed to install font: {:?}",
            install_res.err()
        ));
    }

    let _ = tokio::fs::remove_file(font_file).await;
    Ok(())
}

#[tauri::command]
pub async fn check_win32_long_path_support() -> Result<(), String> {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("installer".to_string()),
        message: Some("Checking Win32 long path support".to_string()),
        level: sentry::Level::Info,
        ..Default::default()
    });
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hklm.open_subkey(r#"SYSTEM\CurrentControlSet\Control\FileSystem"#);
    if key.is_err() {
        return Err(format!("Failed to open registry key: {:?}", key.err()));
    }
    let key = key.unwrap();
    let value = key.get_value::<u32, _>("LongPathsEnabled").unwrap_or(0);
    if value == 0 {
        sentry::add_breadcrumb(sentry::Breadcrumb {
            category: Some("installer".to_string()),
            message: Some("Enabling long path support".to_string()),
            level: sentry::Level::Info,
            ..Default::default()
        });

        let key = hklm.open_subkey_with_flags(
            r#"SYSTEM\CurrentControlSet\Control\FileSystem"#,
            winreg::enums::KEY_SET_VALUE,
        );
        if key.is_err() {
            return Err(format!(
                "Failed to open registry key for writing: {:?}",
                key.err()
            ));
        }
        let key = key.unwrap();
        let set_value_res = key.set_value("LongPathsEnabled", &1u32);
        if set_value_res.is_err() {
            return Err(format!(
                "Failed to set registry value: {:?}",
                set_value_res.err()
            ));
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn is_hutao_running() -> Result<(bool, Option<u32>), String> {
    is_process_running(
        "Snap.Hutao.exe".to_string(),
        "60568DGPStudio.SnapHutao".to_string().into(),
    )
}

#[tauri::command]
pub async fn kill_process(pid: u32) -> Result<(), String> {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("installer".to_string()),
        message: Some(format!("Killing process {pid}")),
        level: sentry::Level::Info,
        ..Default::default()
    });

    if !is_process_running_by_pid(pid) {
        return Ok(());
    }

    let handle = unsafe {
        windows::Win32::System::Threading::OpenProcess(
            windows::Win32::System::Threading::PROCESS_TERMINATE,
            false,
            pid,
        )
    };
    if handle.is_err() {
        capture_and_return_err_message_string!(format!(
            "Failed to open process: {:?}",
            handle.err()
        ));
    }
    let handle = handle.unwrap();

    if !is_process_running_by_pid(pid) {
        return Ok(());
    }

    let ret = unsafe { windows::Win32::System::Threading::TerminateProcess(handle, 1) };
    if ret.is_err() {
        capture_and_return_err_message_string!(format!(
            "Failed to terminate process: {:?}",
            ret.err()
        ));
    }
    Ok(())
}

#[tauri::command]
pub async fn remove_outdated_package() -> Result<(), String> {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("installer".to_string()),
        message: Some("Removing outdated package".to_string()),
        level: sentry::Level::Info,
        ..Default::default()
    });
    let res = remove_package("60568DGPStudio.SnapHutao_ebfp3nyc27j86".to_string());
    if res.is_err() {
        capture_and_return_err_message_string!(format!(
            "Failed to remove package: {:?}",
            res.err()
        ));
    }

    Ok(())
}

#[tauri::command]
pub async fn install_package(
    sha256: String,
    id: String,
    offline_mode: bool,
    window: WebviewWindow,
) -> Result<bool, String> {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("installer".to_string()),
        message: Some("Installing package".to_string()),
        level: sentry::Level::Info,
        ..Default::default()
    });
    let temp_dir = std::env::temp_dir();
    let installer_path = temp_dir.as_path().join("Snap.Hutao.msix");
    if !offline_mode {
        let hash = run_sha256_file_hash_async(installer_path.to_str().unwrap()).await;
        if hash.is_err() {
            capture_and_return_err_message_string!(format!(
                "Failed to hash installer: {:?}",
                hash.err()
            ));
        }

        let hash = hash.unwrap();
        if hash != sha256 {
            return Err("Installer hash mismatch".to_string());
        }
    }

    let install_res = add_package(
        installer_path.as_os_str().to_str().unwrap().to_string(),
        move |opr| {
            let _ = window.emit(&id, opr);
        },
    );
    if install_res.is_err() {
        capture_and_return_err_message_string!(format!(
            "Failed to add package: {:?}",
            install_res.err()
        ));
    }

    if install_res.unwrap() {
        let _ = tokio::fs::remove_file(installer_path).await;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
pub async fn create_desktop_lnk() -> Result<(), String> {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("installer".to_string()),
        message: Some("Creating desktop lnk".to_string()),
        level: sentry::Level::Info,
        ..Default::default()
    });
    let target = r#"shell:AppsFolder\60568DGPStudio.SnapHutao_wbnnev551gwxy!App"#.to_string();
    let desktop = get_desktop().unwrap();
    let lnk = format!(r#"{desktop}\Snap Hutao.lnk"#);

    let desktop_path = Path::new(&desktop);

    let create_dir_res = tokio::fs::create_dir_all(desktop_path).await;
    if create_dir_res.is_err() {
        capture_and_return_err_message_string!(format!(
            "Failed to create lnk dir: {:?}",
            create_dir_res.err()
        ));
    }

    let sl = CoCreateInstance::<IShellLink>(
        &CLSID::ShellLink,
        None::<&IShellLink>,
        CLSCTX::INPROC_SERVER,
    );
    if sl.is_err() {
        capture_and_return_err_message_string!(format!(
            "Failed to create shell link: {:?}",
            sl.err()
        ));
    }
    let sl = sl.unwrap();

    let set_path_res = sl.SetPath(&target);
    if set_path_res.is_err() {
        capture_and_return_err_message_string!(format!(
            "Failed to set shell link path: {:?}",
            set_path_res.err()
        ));
    }

    let set_show_cmd_res = sl.SetShowCmd(SW::SHOWNORMAL);
    if set_show_cmd_res.is_err() {
        capture_and_return_err_message_string!(format!(
            "Failed to set shell link show cmd: {:?}",
            set_show_cmd_res.err()
        ));
    }

    let pf = sl.QueryInterface::<IPersistFile>();
    if pf.is_err() {
        capture_and_return_err_message_string!(format!(
            "Failed to query shell link persist file: {:?}",
            pf.err()
        ));
    }
    let pf = pf.unwrap();

    let save_res = pf.Save(Some(&lnk), false);
    if save_res.is_err() {
        capture_and_return_err_message_string!(format!(
            "Failed to save shell link: {:?}",
            save_res.err()
        ));
    }

    Ok(())
}

#[tauri::command]
pub async fn exit(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub async fn launch_and_exit(app: AppHandle) {
    let target = r#"shell:AppsFolder\60568DGPStudio.SnapHutao_wbnnev551gwxy!App"#.to_string();
    process::run(true, target, REAL_CURRENT_DIR.clone().into(), None::<&str>);
    app.exit(0);
}

fn decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut decompressed_data = Vec::new();
    #[cfg(debug_assertions)]
    {
        decompressed_data.extend_from_slice(data);
        if decompressed_data.is_empty() {
            return Err("Offline package payload is empty".to_string());
        }
    }
    #[cfg(not(debug_assertions))]
    {
        let mut decoder = flate2::read::GzDecoder::new(data);
        let decompress_res = decoder.read_to_end(&mut decompressed_data);
        if decompress_res.is_err() {
            return Err(format!(
                "Failed to decompress data: {:?}",
                decompress_res.err()
            ));
        }
    }
    Ok(decompressed_data)
}
