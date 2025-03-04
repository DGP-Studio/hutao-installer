use windows::
    Win32::{
        Foundation::HANDLE,
        UI::Shell::{
            FOLDERID_Desktop, SHGetKnownFolderPath, KF_FLAG_DEFAULT,
        },
    }
;

pub fn get_desktop() -> Result<String, String> {
    let pwstr = unsafe {
        SHGetKnownFolderPath(&FOLDERID_Desktop, KF_FLAG_DEFAULT, HANDLE::default())
            .map(|pwstr| {
                pwstr
                    .to_string()
                    .map_err(|e| format!("Failed to convert pwstr: {:?}", e))
            })
            .map_err(|e| format!("Failed to get known folder path: {:?}", e))??
    };
    Ok(pwstr)
}
