use serde::{Deserialize, Serialize};

use crate::REQUEST_CLIENT;

#[derive(Deserialize, Serialize, Debug)]
pub struct HomaDistributionGetAcceleratedMirrorResp {
    pub retcode: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<GenericPatchPackageMirror>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct HomaPassportLoginResp {
    pub retcode: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct HomaPassportLoginReq {
    #[serde(rename = "UserName")]
    pub username: String,
    #[serde(rename = "Password")]
    pub password: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct HomaPassportUserInfo {
    #[serde(rename = "NormalizedUserName")]
    pub normalized_username: Option<String>,
    #[serde(rename = "UserName")]
    pub username: Option<String>,
    #[serde(rename = "IsLicensedDeveloper")]
    pub is_licensed_developer: bool,
    #[serde(rename = "IsMaintainer")]
    pub is_maintainer: bool,
    #[serde(rename = "GachaLogExpireAt")]
    pub gacha_log_expire_at: String,
    #[serde(rename = "CdnExpireAt")]
    pub cdn_expire_at: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct HomaPassportUserInfoResp {
    pub retcode: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<HomaPassportUserInfo>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GenericIp {
    pub ip: String,
    pub division: String,
}

impl Default for GenericIp {
    fn default() -> Self {
        Self {
            ip: "0.0.0.0".to_string(),
            division: String::new(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GenericIpResp {
    pub retcode: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<GenericIp>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GenericPatchData {
    pub version: String,
    pub validation: String,
    pub cache_time: String,
    pub mirrors: Vec<GenericPatchPackageMirror>,
    pub urls: Vec<String>,
    pub sha256: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GenericPatchPackageMirror {
    pub url: String,
    pub mirror_name: String,
    pub mirror_type: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GenericPatchResp {
    pub retcode: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<GenericPatchData>,
}

pub async fn generic_get_ip_info() -> Result<GenericIp, anyhow::Error> {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("api".to_string()),
        message: Some("Fetching ip info".to_string()),
        level: sentry::Level::Info,
        ..Default::default()
    });
    let url = "https://api.snapgenshin.com/ip";
    let resp = REQUEST_CLIENT.get(url).send().await;
    if resp.is_err() {
        return Err(anyhow::anyhow!("Failed to send request: {:?}", resp.err()));
    }
    let resp = resp?;
    let json: Result<GenericIpResp, reqwest::Error> = resp.json().await;
    if json.is_err() {
        return Err(anyhow::anyhow!("Failed to parse json: {:?}", json.err()));
    }
    let json = json?;
    if json.retcode != 0 {
        return Err(anyhow::anyhow!("Failed to fetch ip: {:?}", json.message));
    }
    Ok(json.data.unwrap())
}

#[tauri::command]
pub async fn generic_is_oversea() -> Result<bool, String> {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("api".to_string()),
        message: Some("Checking if oversea".to_string()),
        level: sentry::Level::Info,
        ..Default::default()
    });
    let data = generic_get_ip_info().await;
    if data.is_err() {
        return Err(format!("Failed to fetch ip info: {:?}", data.err()));
    }
    let division = data.unwrap().division;
    Ok(division == "Oversea")
}

#[tauri::command]
pub async fn generic_get_patch() -> Result<GenericPatchData, String> {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("api".to_string()),
        message: Some("Fetching patch".to_string()),
        level: sentry::Level::Info,
        ..Default::default()
    });
    let url = "https://api.snapgenshin.com/patch/hutao";
    let resp = REQUEST_CLIENT.get(url).send().await;
    if resp.is_err() {
        return Err(format!("Failed to send request: {:?}", resp.err()));
    }
    let resp = resp.unwrap();
    let json: Result<GenericPatchResp, reqwest::Error> = resp.json().await;
    if json.is_err() {
        return Err(format!("Failed to parse json: {:?}", json.err()));
    }
    let json = json.unwrap();
    if json.retcode != 0 {
        return Err(format!("Failed to fetch patch: {:?}", json.message));
    }
    Ok(json.data.unwrap())
}

#[tauri::command]
pub async fn homa_login(login_req: HomaPassportLoginReq) -> Result<HomaPassportLoginResp, String> {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("api".to_string()),
        message: Some("Logging in homa".to_string()),
        level: sentry::Level::Info,
        ..Default::default()
    });
    let url = "https://homa.snapgenshin.com/Passport/Login";
    let resp = REQUEST_CLIENT.post(url).json(&login_req).send().await;
    if resp.is_err() {
        return Err(format!("Failed to send request: {:?}", resp.err()));
    }
    let resp = resp.unwrap();
    let json: Result<HomaPassportLoginResp, reqwest::Error> = resp.json().await;
    if json.is_err() {
        return Err(format!("Failed to parse json: {:?}", json.err()));
    }
    Ok(json.unwrap())
}

#[tauri::command]
pub async fn homa_fetch_userinfo(token: String) -> Result<HomaPassportUserInfo, String> {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("api".to_string()),
        message: Some("Fetching userinfo from homa".to_string()),
        level: sentry::Level::Info,
        ..Default::default()
    });
    let url = "https://homa.snapgenshin.com/Passport/UserInfo";
    let resp = REQUEST_CLIENT
        .get(url)
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    if resp.is_err() {
        return Err(format!("Failed to send request: {:?}", resp.err()));
    }
    let resp = resp.unwrap();
    let json: Result<HomaPassportUserInfoResp, reqwest::Error> = resp.json().await;
    if json.is_err() {
        return Err(format!("Failed to parse json: {:?}", json.err()));
    }
    let json = json.unwrap();
    if json.retcode != 0 {
        return Err(format!("Failed to fetch userinfo: {:?}", json.message));
    }
    Ok(json.data.unwrap())
}

#[tauri::command]
pub async fn homa_fetch_cdn(token: String, filename: String) -> Result<String, String> {
    sentry::add_breadcrumb(sentry::Breadcrumb {
        category: Some("api".to_string()),
        message: Some("Fetching cdn from homa".to_string()),
        level: sentry::Level::Info,
        ..Default::default()
    });
    let url = format!(
        "https://homa.snapgenshin.com/Distribution/GetAcceleratedMirror?Filename={}",
        filename
    );
    let resp = REQUEST_CLIENT
        .get(&url)
        .header("Authorization", &format!("Bearer {}", token))
        .send()
        .await;
    if resp.is_err() {
        return Err(format!("Failed to send request: {:?}", resp.err()));
    }
    let resp = resp.unwrap();
    let json: Result<HomaDistributionGetAcceleratedMirrorResp, reqwest::Error> = resp.json().await;
    if json.is_err() {
        return Err(format!("Failed to parse json: {:?}", json.err()));
    }
    let json = json.unwrap();
    if json.retcode != 0 {
        return Err(format!("Failed to fetch cdn: {:?}", json.message));
    }
    let data = json.data.unwrap();
    Ok(data.url)
}
