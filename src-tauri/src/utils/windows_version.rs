use crate::{capture_and_return_default, utils::Version};
use std::str::FromStr;
use winreg::{RegKey, enums::HKEY_LOCAL_MACHINE};

//noinspection SpellCheckingInspection
#[repr(C)]
#[allow(clippy::upper_case_acronyms)]
struct OSVERSIONINFOW {
    dw_os_version_info_size: u32,
    dw_major_version: u32,
    dw_minor_version: u32,
    dw_build_number: u32,
    dw_platform_id: u32,
    sz_csd_version: [u16; 128],
}

extern "system" {
    #[allow(non_snake_case)]
    fn RtlGetVersion(lpVersionInformation: *mut OSVERSIONINFOW) -> i32;
}

pub fn get_windows_version() -> Version {
    let mut version_info = OSVERSIONINFOW {
        dw_os_version_info_size: size_of::<OSVERSIONINFOW>() as u32,
        dw_major_version: 0,
        dw_minor_version: 0,
        dw_build_number: 0,
        dw_platform_id: 0,
        sz_csd_version: [0; 128],
    };

    let api_result = unsafe { RtlGetVersion(&mut version_info) };

    if api_result == 0 && version_info.dw_major_version < 10 {
        return Version::new(
            version_info.dw_major_version as u64,
            version_info.dw_minor_version as u64,
            version_info.dw_build_number as u64,
            0,
        );
    }

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
            anyhow::anyhow!("Failed to get major version: {:?}", major.err()),
            Version::new(0, 0, 0, 0)
        );
    }

    let minor = key.get_value::<u32, _>("CurrentMinorVersionNumber");
    if minor.is_err() {
        capture_and_return_default!(
            anyhow::anyhow!("Failed to get minor version: {:?}", minor.err()),
            Version::new(0, 0, 0, 0)
        );
    }

    let build = key.get_value::<String, _>("CurrentBuildNumber");
    if build.is_err() {
        capture_and_return_default!(
            anyhow::anyhow!("Failed to get build version: {:?}", build.err()),
            Version::new(0, 0, 0, 0)
        );
    }

    let revision = key.get_value::<u32, _>("UBR");
    if revision.is_err() {
        capture_and_return_default!(
            anyhow::anyhow!("Failed to get revision version: {:?}", revision.err()),
            Version::new(0, 0, 0, 0)
        );
    }

    let major = major.unwrap().into();
    let minor = minor.unwrap().into();
    let build = u64::from_str(build.unwrap().as_str());
    if build.is_err() {
        capture_and_return_default!(
            anyhow::anyhow!("Failed to parse build version: {:?}", build.err()),
            Version::new(0, 0, 0, 0)
        );
    }
    let build = build.unwrap();
    let revision = revision.unwrap().into();
    Version::new(major, minor, build, revision)
}
