use crate::{capture_and_return_default, capture_and_return_err};
use std::path::{Path, PathBuf};
use ttf_parser::Face;
use windows::Win32::UI::WindowsAndMessaging::{SMTO_BLOCK, SendMessageTimeoutW};
use windows::{
    Win32::{
        Foundation::{LPARAM, WPARAM},
        Graphics::Gdi::{AddFontResourceW, RemoveFontResourceW},
        UI::WindowsAndMessaging::{HWND_BROADCAST, WM_FONTCHANGE},
    },
    core::{HSTRING, PCWSTR},
};
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
            let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
            let system_fonts = Path::new(&windir).join("Fonts");
            return Some(system_fonts.join(font_rel_path));
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
    let win_dir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
    let fonts_dir = Path::new(&win_dir).join("Fonts");

    let font_file_name = Path::new(font_path).file_name();
    if font_file_name.is_none() {
        capture_and_return_err!(anyhow::anyhow!("Invalid font file path: {}", font_path));
    }
    let font_file_name = font_file_name.unwrap().to_str().unwrap();
    let target_path = fonts_dir.join(font_file_name);

    let mut ref_times = 0;
    unsafe {
        while RemoveFontResourceW(PCWSTR(
            HSTRING::from(target_path.to_string_lossy().as_ref()).as_ptr(),
        ))
        .as_bool()
        {
            ref_times += 1;
            if ref_times > 30 {
                capture_and_return_default!(
                    anyhow::anyhow!(
                        "Failed to remove existing font resource after multiple attempts."
                    ),
                    Ok(())
                );
            }
        }

        let _ = SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_FONTCHANGE,
            WPARAM::default(),
            LPARAM::default(),
            SMTO_BLOCK,
            1000,
            Some(std::ptr::null_mut()),
        );
    }

    let copy_res = std::fs::copy(font_path, &target_path);
    if copy_res.is_err() {
        capture_and_return_err!(anyhow::anyhow!(
            "Failed to copy font file: {:?}",
            copy_res.err()
        ));
    }

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let fonts_key = hklm.open_subkey_with_flags(
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Fonts",
        winreg::enums::KEY_SET_VALUE,
    );
    if fonts_key.is_err() {
        capture_and_return_err!(anyhow::anyhow!(
            "Failed to open registry key: {:?}",
            fonts_key.err()
        ));
    }
    let fonts_key = fonts_key?;

    let set_result = fonts_key.set_value(font_name, &font_file_name);
    if set_result.is_err() {
        capture_and_return_err!(anyhow::anyhow!(
            "Failed to set font registry value: {:?}",
            set_result.err()
        ));
    }

    unsafe {
        loop {
            ref_times -= 1;
            AddFontResourceW(PCWSTR(
                HSTRING::from(target_path.to_string_lossy().as_ref()).as_ptr(),
            ));
            if ref_times <= 0 {
                break;
            }
        }

        let _ = SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_FONTCHANGE,
            WPARAM::default(),
            LPARAM::default(),
            SMTO_BLOCK,
            1000,
            Some(std::ptr::null_mut()),
        );
    }

    Ok(())
}
