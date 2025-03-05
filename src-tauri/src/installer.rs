use crate::{
    cli::arg::UpdateArgs,
    fs::{create_http_stream, create_target_file, progressed_copy},
    utils::{
        cert::{find_certificate, install_certificate},
        dir::get_desktop,
        hash::run_sha256_file_hash_async,
        package_manager::{add_package, try_get_hutao_version},
        uac::run_elevated,
    },
    REQUEST_CLIENT,
};
use serde::Serialize;
use std::{path::Path, time::Instant};
use tauri::{AppHandle, Emitter, State, WebviewWindow};
use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};
use winsafe::{
    co::{CLSCTX, CLSID, SW},
    prelude::{ole_IPersistFile, ole_IUnknown, shell_IShellLink},
    CoCreateInstance, IPersistFile, IShellLink,
};

#[derive(Serialize, Debug, Clone)]
pub struct Config {
    pub is_update: bool,
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
pub async fn message_dialog(title: String, message: String, window: WebviewWindow) {
    rfd::MessageDialog::new()
        .set_title(&title)
        .set_description(&message)
        .set_level(rfd::MessageLevel::Info)
        .set_parent(&window)
        .show();
}

#[tauri::command]
pub async fn open_tos() -> Result<(), String> {
    let url = "https://hut.ao/zh/statements/tos.html";
    if webbrowser::open(url).is_ok() {
        Ok(())
    } else {
        Err("Failed to open the link".to_string())
    }
}

#[tauri::command]
pub async fn get_config(args: State<'_, Option<UpdateArgs>>) -> Result<Config, String> {
    let exists = try_get_hutao_version().await.unwrap();

    let update_args = args.inner().clone();
    if !update_args.is_none() {
        let update_args = update_args.unwrap();
        return Ok(Config {
            is_update: true,
            curr_version: exists,
            token: update_args.token,
        });
    }

    Ok(Config {
        is_update: exists.is_some(),
        curr_version: exists,
        token: None,
    })
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
        return Ok((-1.0) as f64);
    }
    Ok(5.0 / ((elapsed as f64) / (1000 as f64)))
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

    dbg!(len);
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
    window: tauri::WebviewWindow,
) -> Result<(), String> {
    let temp_dir = std::env::temp_dir();
    let installer_path = temp_dir.as_path().join("Snap.Hutao.msix");
    let (mut stream, len) = create_http_stream(&mirror_url, 0, 0)
        .await
        .map_err(|e| format!("Failed to download msix: {:?}", e))?;
    let mut target = create_target_file(installer_path.as_os_str().to_str().unwrap())
        .await
        .map_err(|e| format!("Failed to create msix: {:?}", e))?;
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
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let path = r#"SOFTWARE\Wow6432Node\Microsoft\VisualStudio\14.0\VC\Runtimes\x64"#.to_string();
    let key = hklm.open_subkey(&path);
    match key {
        Ok(key) => {
            if let Ok(installed) = key.get_value::<u32, _>("Installed") {
                return Ok(installed == 1);
            }
        }
        Err(_) => {}
    }
    Ok(false)
}

#[tauri::command]
pub async fn install_vcrt(id: String, window: tauri::WebviewWindow) -> Result<(), String> {
    let url = "https://aka.ms/vs/17/release/vc_redist.x64.exe";
    let temp_dir = std::env::temp_dir();
    let installer_path = temp_dir.as_path().join("vc_redist.x64.exe");
    let (mut stream, len) = create_http_stream(&url, 0, 0)
        .await
        .map_err(|e| format!("Failed to download vcrt installer: {:?}", e))?;
    let mut target = create_target_file(installer_path.as_os_str().to_str().unwrap())
        .await
        .map_err(|e| format!("Failed to create vcrt installer: {:?}", e))?;
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
    if let Err(e) = cmd {
        return Err(format!("Failed to run vcrt installer: {:?}", e));
    }
    let mut cmd = cmd.unwrap();
    let status = cmd.wait().await;
    if let Err(e) = status {
        return Err(format!("Failed to wait for vcrt installer: {:?}", e));
    }
    let status = status.unwrap();
    if !status.success() {
        return Err(format!("Failed to install vcrt: {:?}", status));
    }
    let _ = tokio::fs::remove_file(installer_path).await;
    Ok(())
}

#[tauri::command]
pub async fn check_globalsign_r45(window: WebviewWindow) -> Result<(), String> {
    let find_res = find_certificate("BE, GlobalSign nv-sa, GlobalSign Code Signing Root R45").await;
    if find_res.is_err() {
        return Err(format!("Failed to find certificate: {:?}", find_res.err()));
    }

    let found = find_res.unwrap();
    if found {
        return Ok(());
    }

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
pub async fn install_package(
    sha256: String,
    id: String,
    window: tauri::WebviewWindow,
) -> Result<(), String> {
    let temp_dir = std::env::temp_dir();
    let installer_path = temp_dir.as_path().join("Snap.Hutao.msix");
    let hash = run_sha256_file_hash_async(installer_path.to_str().unwrap()).await;
    if hash.is_err() {
        return Err(format!("Failed to hash installer: {:?}", hash.err()));
    }

    let hash = hash.unwrap();
    if hash != sha256 {
        return Err("Installer hash mismatch".to_string());
    }

    let install_res = add_package(
        installer_path.as_os_str().to_str().unwrap().to_string(),
        move |opr| {
            let _ = window.emit(&id, opr);
        },
    )
    .await;
    if install_res.is_err() {
        return Err(format!(
            "Failed to install package: {:?}",
            install_res.err()
        ));
    }
    Ok(())
}

#[tauri::command]
pub async fn create_desktop_lnk() -> Result<(), String> {
    let target = r#"shell:AppsFolder\60568DGPStudio.SnapHutao_wbnnev551gwxy!App"#.to_string();
    let desktop = get_desktop().unwrap();
    let lnk = format!(r#"{}\Snap Hutao.lnk"#, desktop);

    let desktop_path = Path::new(&desktop);

    tokio::fs::create_dir_all(desktop_path)
        .await
        .map_err(|e| format!("Failed to create lnk dir: {:?}", e))?;

    let sl = CoCreateInstance::<IShellLink>(&CLSID::ShellLink, None, CLSCTX::INPROC_SERVER)
        .map_err(|e| format!("Failed to create shell link: {:?}", e))?;

    let _ = sl
        .SetPath(&target)
        .map_err(|e| format!("Failed to set shell link path: {:?}", e))?;
    let _ = sl
        .SetShowCmd(SW::SHOWNORMAL)
        .map_err(|e| format!("Failed to set shell link show cmd: {:?}", e))?;

    let pf = sl
        .QueryInterface::<IPersistFile>()
        .map_err(|e| format!("Failed to query persist file: {:?}", e))?;

    let _ = pf
        .Save(Some(&lnk), false)
        .map_err(|e| format!("Failed to save lnk: {:?}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn clear_temp_dir() -> Result<(), String> {
    let temp_dir = std::env::temp_dir();
    let _ = tokio::fs::remove_dir_all(temp_dir).await;
    Ok(())
}

#[tauri::command]
pub async fn launch_and_exit(app: AppHandle) {
    let target = r#"shell:AppsFolder\60568DGPStudio.SnapHutao_wbnnev551gwxy!App"#.to_string();
    let _ = run_elevated(target, "".to_string()).map_err(|e| format!("Failed to launch: {:?}", e));
    app.exit(0);
}
