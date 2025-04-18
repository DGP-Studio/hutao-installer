pub mod cert;
pub mod device;
pub mod dir;
pub mod hash;
pub mod package_manager;
pub mod process;
pub mod uac;
pub mod windows_version;

pub trait SentryCapturable {
    fn is_err_and_capture(&self) -> bool;
}

impl<T, E: std::error::Error> SentryCapturable for Result<T, E> {
    fn is_err_and_capture(&self) -> bool {
        if let Err(e) = self {
            sentry::capture_error(e);
            return true;
        }
        false
    }
}
