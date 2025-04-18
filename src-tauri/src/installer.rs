use crate::{
    cli::arg::UpdateArgs,
    fs::{create_http_stream, create_target_file, progressed_copy},
    utils::{
        cert::{find_certificate, install_certificate},
        dir::get_desktop,
        hash::run_sha256_file_hash_async,
        package_manager::{add_package, try_get_hutao_version},
        process::is_process_running,
        uac::run_elevated,
        SentryCapturable,
    },
    REQUEST_CLIENT,
};
use serde::Serialize;
use std::io::Error;
use std::{path::Path, time::Instant};
use tauri::{AppHandle, Emitter, Runtime, State, WebviewWindow};
use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};
use winsafe::{
    co::{CLSCTX, CLSID, SW},
    prelude::{ole_IPersistFile, ole_IUnknown, shell_IShellLink},
    CoCreateInstance, IPersistFile, IShellLink,
};

#[derive(Serialize, Debug, Clone)]
pub struct Config {
    pub is_update: bool,
    pub is_auto_update: bool,
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
pub async fn need_self_update<R: Runtime>(app: AppHandle<R>) -> Result<bool, String> {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("installer".to_string()),
        message: Some("Checking self update".to_string()),
        level: sentry::Level::Info,
        ..Default::default()
    });
    let exe_path = std::env::current_exe().unwrap();
    let outdated = exe_path.with_extension("old");
    let _ = tokio::fs::remove_file(&outdated).await;

    let curr_ver = app.package_info().version.clone();
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
    let latest_ver = data.version[..data.version.len() - 2].to_string();
    let latest_ver = semver::Version::parse(&latest_ver);
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
            sentry::capture_error(&e);
            return Err(format!("Failed to rename executable: {:?}", e));
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
    if write_res.is_err_and_capture() {
        return Err(format!(
            "Failed to write new installer: {:?}",
            write_res.err()
        ));
    }
    let _ = run_elevated(&exe_path, "");
    app.exit(0);

    Ok(())
}

#[tauri::command]
pub async fn open_browser(url: String) -> Result<(), String> {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("installer".to_string()),
        message: Some(format!("Opening browser: {}", url)),
        level: sentry::Level::Info,
        ..Default::default()
    });
    if webbrowser::open(&url).is_ok() {
        Ok(())
    } else {
        Err("Failed to open the link".to_string())
    }
}

#[tauri::command]
pub async fn get_config(args: State<'_, Option<UpdateArgs>>) -> Result<Config, String> {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("installer".to_string()),
        message: Some("Getting config".to_string()),
        level: sentry::Level::Info,
        ..Default::default()
    });
    let exists = try_get_hutao_version().unwrap();

    let update_args = args.inner().clone();
    if update_args.is_some() {
        let update_args = update_args.unwrap();
        return Ok(Config {
            is_update: true,
            is_auto_update: true,
            curr_version: exists,
            token: update_args.token,
        });
    }

    Ok(Config {
        is_update: exists.is_some(),
        is_auto_update: false,
        curr_version: exists,
        token: None,
    })
}

#[tauri::command]
pub async fn get_changelog(lang: String, from: String) -> Result<String, String> {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("installer".to_string()),
        message: Some(format!("Getting {} changelog: {}", lang, from)),
        level: sentry::Level::Info,
        ..Default::default()
    });
    let url = format!(
        "https://api.qhy04.com/hutaocdn/changelog?lang={}&from={}",
        lang, from
    );
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
    let exists = tokio::fs::metadata(installer_path.clone()).await;
    if exists.is_err() {
        return Ok(false);
    }

    let hash = run_sha256_file_hash_async(installer_path.to_str().unwrap()).await;
    if hash.is_err() {
        return Err(format!("Failed to hash installer: {:?}", hash.err()));
    }

    let hash = hash?;
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
pub async fn download_package(
    mirror_url: String,
    id: String,
    window: WebviewWindow,
) -> Result<(), String> {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("installer".to_string()),
        message: Some(format!("Downloading package from {}", mirror_url)),
        level: sentry::Level::Info,
        ..Default::default()
    });
    let temp_dir = std::env::temp_dir();
    let installer_path = temp_dir.as_path().join("Snap.Hutao.msix");
    let stream_res = create_http_stream(&mirror_url, 0, 0).await;
    if stream_res.is_err() {
        return Err(format!("Failed to download msix: {:?}", stream_res.err()));
    }
    let (mut stream, len) = stream_res?;
    let target_file_create_res =
        create_target_file(installer_path.as_os_str().to_str().unwrap()).await;
    if target_file_create_res.is_err() {
        return Err(format!(
            "Failed to create msix: {:?}",
            target_file_create_res.err()
        ));
    }
    let mut target = target_file_create_res?;
    let progress_noti = move |downloaded: usize| {
        let _ = window.emit(&id, serde_json::json!((downloaded, len)));
    };
    progressed_copy(&mut stream, &mut target, progress_noti).await?;
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
    let path = r#"SOFTWARE\Wow6432Node\Microsoft\VisualStudio\14.0\VC\Runtimes\x64"#.to_string();
    let key = hklm.open_subkey(&path);
    if let Ok(key) = key {
        if let Ok(installed) = key.get_value::<u32, _>("Installed") {
            return Ok(installed == 1);
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
    let url = "https://aka.ms/vs/17/release/vc_redist.x64.exe";
    let temp_dir = std::env::temp_dir();
    let installer_path = temp_dir.as_path().join("vc_redist.x64.exe");
    let stream_res = create_http_stream(url, 0, 0).await;
    if stream_res.is_err() {
        return Err(format!(
            "Failed to download vcrt installer: {:?}",
            stream_res.err()
        ));
    }
    let (mut stream, len) = stream_res?;
    let target_file_create_res =
        create_target_file(installer_path.as_os_str().to_str().unwrap()).await;
    if target_file_create_res.is_err() {
        return Err(format!(
            "Failed to create vcrt installer: {:?}",
            target_file_create_res.err()
        ));
    }
    let mut target = target_file_create_res?;
    let progress_noti = move |downloaded: usize| {
        let _ = window.emit(&id, serde_json::json!((downloaded, len)));
    };
    progressed_copy(&mut stream, &mut target, progress_noti).await?;
    // close streams
    drop(stream);
    drop(target);

    let cmd = tokio::process::Command::new(&installer_path)
        .arg("/install")
        .arg("/quiet")
        .arg("/norestart")
        .spawn();
    if cmd.is_err_and_capture() {
        return Err(format!("Failed to spawn vcrt installer: {:?}", cmd.err()));
    }
    let mut cmd = cmd.unwrap();
    let status = cmd.wait().await;
    if status.is_err_and_capture() {
        return Err(format!(
            "Failed to wait for vcrt installer: {:?}",
            status.err()
        ));
    }
    let status = status.unwrap();
    if !status.success() {
        if status.code().unwrap() != 3010 {
            sentry::capture_error(&Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to install vcrt: {:?}", status),
            ));
            return Err(format!("Failed to install vcrt: {:?}", status));
        }
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

    let found = find_res?;
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
        message: Some(format!("Killing process {}", pid)),
        level: sentry::Level::Info,
        ..Default::default()
    });
    let handle = unsafe {
        windows::Win32::System::Threading::OpenProcess(
            windows::Win32::System::Threading::PROCESS_TERMINATE,
            false,
            pid,
        )
    };
    if handle.is_err_and_capture() {
        return Err(format!("Failed to open process: {:?}", handle.err()));
    }
    let handle = handle.unwrap();
    let ret = unsafe { windows::Win32::System::Threading::TerminateProcess(handle, 1) };
    if ret.is_err_and_capture() {
        return Err(format!("Failed to terminate process: {:?}", ret.err()));
    }
    Ok(())
}

#[tauri::command]
pub async fn install_package(
    sha256: String,
    id: String,
    window: WebviewWindow,
) -> Result<(), String> {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("installer".to_string()),
        message: Some("Installing package".to_string()),
        level: sentry::Level::Info,
        ..Default::default()
    });
    let temp_dir = std::env::temp_dir();
    let installer_path = temp_dir.as_path().join("Snap.Hutao.msix");
    let hash = run_sha256_file_hash_async(installer_path.to_str().unwrap()).await;
    if hash.is_err() {
        return Err(format!("Failed to hash installer: {:?}", hash.err()));
    }

    let hash = hash?;
    if hash != sha256 {
        return Err("Installer hash mismatch".to_string());
    }

    let install_res = add_package(
        installer_path.as_os_str().to_str().unwrap().to_string(),
        move |opr| {
            let _ = window.emit(&id, opr);
        },
    );
    if install_res.is_err() {
        return Err(format!(
            "Failed to install package: {:?}",
            install_res.err()
        ));
    }

    let _ = tokio::fs::remove_file(installer_path).await;
    Ok(())
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
    let desktop = get_desktop()?;
    let lnk = format!(r#"{}\Snap Hutao.lnk"#, desktop);

    let desktop_path = Path::new(&desktop);

    let create_dir_res = tokio::fs::create_dir_all(desktop_path).await;
    if create_dir_res.is_err_and_capture() {
        return Err(format!(
            "Failed to create lnk dir: {:?}",
            create_dir_res.err()
        ));
    }

    let sl = CoCreateInstance::<IShellLink>(&CLSID::ShellLink, None, CLSCTX::INPROC_SERVER);
    if sl.is_err_and_capture() {
        return Err(format!("Failed to create shell link: {:?}", sl.err()));
    }
    let sl = sl.unwrap();

    let set_path_res = sl.SetPath(&target);
    if set_path_res.is_err_and_capture() {
        return Err(format!(
            "Failed to set shell link path: {:?}",
            set_path_res.err()
        ));
    }

    let set_show_cmd_res = sl.SetShowCmd(SW::SHOWNORMAL);
    if set_show_cmd_res.is_err_and_capture() {
        return Err(format!(
            "Failed to set shell link show cmd: {:?}",
            set_show_cmd_res.err()
        ));
    }

    let pf = sl.QueryInterface::<IPersistFile>();
    if pf.is_err_and_capture() {
        return Err(format!("Failed to query persist file: {:?}", pf.err()));
    }
    let pf = pf.unwrap();

    let save_res = pf.Save(Some(&lnk), false);
    if save_res.is_err_and_capture() {
        return Err(format!("Failed to save lnk: {:?}", save_res.err()));
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
    let _ = run_elevated(target, "");
    app.exit(0);
}
