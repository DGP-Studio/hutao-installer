use crate::capture_and_return_default;
use crate::utils::Version;
use std::str::FromStr;
use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};

pub fn get_windows_version() -> Version {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let path = r#"SOFTWARE\Microsoft\Windows NT\CurrentVersion"#;
    let key = hklm.open_subkey(path);
    if key.is_err() {
        capture_and_return_default!(
            anyhow::anyhow!("Failed to open registry key: {:?}", key.err()),
            Version::new(0, 0, 0, 0)
        );
    }

    let key = key.unwrap();
    let major = key.get_value::<u32, _>("CurrentMajorVersionNumber");
    if major.is_err() {
        capture_and_return_default!(
            anyhow::anyhow!("Failed to get major version"),
            Version::new(0, 0, 0, 0)
        );
    }

    let minor = key.get_value::<u32, _>("CurrentMinorVersionNumber");
    if minor.is_err() {
        capture_and_return_default!(
            anyhow::anyhow!("Failed to get minor version"),
            Version::new(0, 0, 0, 0)
        );
    }

    let build = key.get_value::<String, _>("CurrentBuildNumber");
    if build.is_err() {
        capture_and_return_default!(
            anyhow::anyhow!("Failed to get build version"),
            Version::new(0, 0, 0, 0)
        );
    }

    let revision = key.get_value::<u32, _>("UBR");
    if revision.is_err() {
        capture_and_return_default!(
            anyhow::anyhow!("Failed to get revision version"),
            Version::new(0, 0, 0, 0)
        );
    }

    let major = major.unwrap().into();
    let minor = minor.unwrap().into();
    let build = u64::from_str(build.unwrap().as_str());
    if build.is_err() {
        capture_and_return_default!(
            anyhow::anyhow!("Failed to parse build version"),
            Version::new(0, 0, 0, 0)
        );
    }
    let build = build.unwrap();
    let revision = revision.unwrap().into();
    Version::new(major, minor, build, revision)
}
