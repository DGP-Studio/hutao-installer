use crate::utils::hash::run_md5_hash;
use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};

pub fn get_device_id() -> Result<String, String> {
    let username = whoami::username();
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let path = r#"SOFTWARE\Microsoft\Cryptography"#;
    let key = hklm.open_subkey(path);
    if key.is_err() {
        return Err(format!("Failed to open registry key: {:?}", key.err()));
    }

    let key = key.unwrap();
    let mac_guid = key.get_value::<String, _>("MachineGuid");
    let raw_device_id = format!("{}{}", username, mac_guid.unwrap());
    Ok(run_md5_hash(raw_device_id.as_str()).to_ascii_uppercase())
}
