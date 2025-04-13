use std::str::FromStr;
use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};

pub fn get_windows_version() -> (u32, u32, u32, u32) {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let path = r#"SOFTWARE\Microsoft\Windows NT\CurrentVersion"#;
    let key = hklm.open_subkey(&path);
    if key.is_err() {
        return (0, 0, 0, 0);
    }

    let key = key.unwrap();
    let major = key.get_value::<u32, _>("CurrentMajorVersionNumber");
    let minor = key.get_value::<u32, _>("CurrentMinorVersionNumber");
    let build = key.get_value::<String, _>("CurrentBuildNumber");
    let revision = key.get_value::<u32, _>("UBR");
    if major.is_err() || minor.is_err() || build.is_err() || revision.is_err() {
        return (0, 0, 0, 0);
    }

    let major = major.unwrap();
    let minor = minor.unwrap();
    let build = u32::from_str(build.unwrap().as_str()).unwrap();
    let revision = revision.unwrap();
    (major, minor, build, revision)
}
