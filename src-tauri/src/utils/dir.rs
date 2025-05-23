use windows::Win32::UI::Shell::{FOLDERID_Desktop, SHGetKnownFolderPath, KF_FLAG_DEFAULT};

pub fn get_desktop() -> Result<String, anyhow::Error> {
    let pwstr = unsafe {
        SHGetKnownFolderPath(&FOLDERID_Desktop, KF_FLAG_DEFAULT, None)
            .map(|pwstr| {
                pwstr
                    .to_string()
                    .map_err(|e| anyhow::anyhow!("Failed to convert pwstr: {:?}", e))
            })
            .map_err(|e| anyhow::anyhow!("Failed to get known folder path: {:?}", e))??
    };
    Ok(pwstr)
}
