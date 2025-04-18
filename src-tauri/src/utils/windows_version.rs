use crate::utils::{SentryCapturable, Version};
use std::str::FromStr;
use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};

pub fn get_windows_version() -> Version {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let path = r#"SOFTWARE\Microsoft\Windows NT\CurrentVersion"#;
    let key = hklm.open_subkey(path);
    if key.is_err_and_capture("Failed to open registry key") {
        return Version::new(0, 0, 0, 0);
    }

    let key = key.unwrap();
    let major = key.get_value::<u32, _>("CurrentMajorVersionNumber");
    let minor = key.get_value::<u32, _>("CurrentMinorVersionNumber");
    let build = key.get_value::<String, _>("CurrentBuildNumber");
    let revision = key.get_value::<u32, _>("UBR");
    if major.is_err() || minor.is_err() || build.is_err() || revision.is_err() {
        return Version::new(0, 0, 0, 0);
    }

    let major = major.unwrap().into();
    let minor = minor.unwrap().into();
    let build = u64::from_str(build.unwrap().as_str()).unwrap();
    let revision = revision.unwrap().into();
    Version::new(major, minor, build, revision)
}
