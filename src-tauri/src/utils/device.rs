use crate::{capture_and_return_err, utils::hash::run_md5_hash};
use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};

pub fn get_device_id() -> Result<String, anyhow::Error> {
    let username = whoami::username();
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let path = r#"SOFTWARE\Microsoft\Cryptography"#;
    let key = hklm.open_subkey(path);
    if key.is_err() {
        capture_and_return_err!(anyhow::anyhow!(
            "Failed to open registry key: {:?}",
            key.err()
        ));
    }

    let key = key?;
    let mac_guid = key.get_value::<String, _>("MachineGuid");
    let raw_device_id = format!("{}{}", username, mac_guid?);

    let res = run_md5_hash(raw_device_id.as_str());
    if res.is_err() {
        capture_and_return_err!(anyhow::anyhow!("Failed to calculate md5: {:?}", res.err()));
    }
    let res = res?;
    Ok(res.to_ascii_uppercase())
}
