pub mod cert;
pub mod device;
pub mod dir;
pub mod hash;
pub mod package_manager;
pub mod process;
pub mod uac;
pub mod windows_version;

pub trait SentryCapturable {
    fn is_err_and_capture(&self, message: &str) -> bool;
}

impl<T, E: std::fmt::Debug> SentryCapturable for Result<T, E> {
    fn is_err_and_capture(&self, message: &str) -> bool {
        if let Err(e) = self {
            sentry::capture_message(
                format!("{}: {:?}", message, e).as_str(),
                sentry::Level::Error,
            );
            return true;
        }
        false
    }
}
