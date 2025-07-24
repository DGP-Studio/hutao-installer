use crate::{capture_and_return_default, capture_and_return_err};
use std::path::{Path, PathBuf};
use ttf_parser::Face;
use windows::Win32::UI::WindowsAndMessaging::{HWND_BROADCAST, SendMessageW, WM_FONTCHANGE};
use winreg::{RegKey, enums::HKEY_LOCAL_MACHINE};

pub fn get_font_path(font_display_name: &str) -> Option<PathBuf> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let fonts = hklm.open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Fonts");
    if fonts.is_err() {
        capture_and_return_default!(
            anyhow::anyhow!("Failed to open registry key: {:?}", fonts.err()),
            None
        );
    }
    let fonts = fonts.unwrap();

    for (name, value) in fonts.enum_values().flatten() {
        if name.starts_with(font_display_name) {
            let font_rel_path: String = value.to_string();
            let system_fonts =
                std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into()) + "\\Fonts\\";
            return Some(PathBuf::from(system_fonts).join(font_rel_path));
        }
    }

    None
}

pub fn get_font_version(font_path: &PathBuf) -> Option<String> {
    let data = std::fs::read(font_path);
    if data.is_err() {
        capture_and_return_default!(
            anyhow::anyhow!("Failed to read font file: {:?}", data.err()),
            None
        );
    }
    let data = data.unwrap();
    let face = Face::parse(&data, 0);
    if face.is_err() {
        capture_and_return_default!(
            anyhow::anyhow!("Failed to parse font file: {:?}", face.err()),
            None
        );
    }
    let face = face.unwrap();

    for name in face.names() {
        if name.name_id == ttf_parser::name_id::VERSION {
            if let Some(version_str) = name.to_string() {
                return Some(version_str);
            }
        }
    }

    None
}

pub fn install_font_permanently(font_path: &str, font_name: &str) -> Result<(), anyhow::Error> {
    let fonts_dir = r"C:\Windows\Fonts";
    let font_file_name = Path::new(font_path).file_name();
    if font_file_name.is_none() {
        capture_and_return_err!(anyhow::anyhow!("Invalid font file path: {}", font_path));
    }
    let font_file_name = font_file_name.unwrap().to_str().unwrap();
    let target_path = Path::new(fonts_dir).join(font_file_name);

    std::fs::copy(font_path, &target_path)?;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let fonts_key = hklm.open_subkey_with_flags(
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts",
        winreg::enums::KEY_SET_VALUE,
    )?;

    let set_result = fonts_key.set_value(font_name, &font_file_name);
    if set_result.is_err() {
        capture_and_return_err!(anyhow::anyhow!(
            "Failed to set font registry value: {:?}",
            set_result.err()
        ));
    }

    unsafe {
        SendMessageW(HWND_BROADCAST, WM_FONTCHANGE, None, None);
    }

    Ok(())
}
