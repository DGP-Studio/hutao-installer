use crate::{cli::arg::UpdateArgs, utils::{dir::get_desktop, package_manager::try_get_hutao_version, uac::run_elevated}};
use serde::Serialize;
use serde_json::Value;
use std::path::Path;
use tauri::{AppHandle, State, WebviewWindow};

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
    let ret = rfd::MessageDialog::new()
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
        })
    }

    Ok(Config {
        is_update: exists.is_some(),
        curr_version: exists,
        token: None,
    })
}

#[tauri::command]
pub async fn create_desktop_lnk() -> Result<(), String> {
    let target = r#"shell:AppsFolder\60568DGPStudio.SnapHutao_wbnnev551gwxy!App"#.to_string();
    let desktop = get_desktop().unwrap();
    let lnk = format!(r#"{}\Snap Hutao.lnk"#, desktop);

    let target_path = Path::new(&target);
    let desktop_path = Path::new(&desktop);
    let lnk_path = Path::new(&lnk);

    tokio::fs::create_dir_all(desktop_path).await.map_err(|e| format!("Failed to create lnk dir: {:?}", e))?;
    let sl = mslnk::ShellLink::new(target_path)
        .map_err(|e| format!("Failed to create shell link: {:?}", e))?;
    sl.create_lnk(lnk_path)
        .map_err(|e| format!("Failed to create lnk: {:?}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn launch_and_exit(app: AppHandle) {
    let target = r#"shell:AppsFolder\60568DGPStudio.SnapHutao_wbnnev551gwxy!App"#.to_string();
    let _ = run_elevated(target, "".to_string()).map_err(|e| format!("Failed to launch: {:?}", e));
    app.exit(0);
}
