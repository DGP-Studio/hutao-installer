pub mod cert;
pub mod device;
pub mod dir;
pub mod hash;
pub mod package_manager;
pub mod process;
pub mod windows_version;

#[macro_export]
macro_rules! capture_and_return {
    ($err:expr) => {{
        let err = $err;
        sentry_anyhow::capture_anyhow(&err);
        return;
    }};
}

#[macro_export]
macro_rules! capture_and_return_default {
    ($err:expr, $default:expr) => {{
        let err = $err;
        sentry_anyhow::capture_anyhow(&err);
        return $default;
    }};
}

#[macro_export]
macro_rules! capture_and_return_err {
    ($err:expr) => {{
        let err = $err;
        sentry_anyhow::capture_anyhow(&err);
        return Err(err);
    }};
}

#[macro_export]
macro_rules! capture_and_return_err_message_string {
    ($err_message:expr) => {{
        let msg = $err_message;
        sentry_anyhow::capture_anyhow(&anyhow::anyhow!(msg.clone()));
        return Err(msg);
    }};
}

pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub build: u64,
    pub revision: u64,
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.major == other.major
            && self.minor == other.minor
            && self.build == other.build
            && self.revision == other.revision
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for Version {}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.major != other.major {
            return self.major.cmp(&other.major);
        }
        if self.minor != other.minor {
            return self.minor.cmp(&other.minor);
        }
        if self.build != other.build {
            return self.build.cmp(&other.build);
        }
        self.revision.cmp(&other.revision)
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.revision == 0 {
            return write!(f, "{}.{}.{}", self.major, self.minor, self.build);
        }

        write!(
            f,
            "{}.{}.{}.{}",
            self.major, self.minor, self.build, self.revision
        )
    }
}

impl std::fmt::Debug for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Version {{ major: {}, minor: {}, build: {}, revision: {} }}",
            self.major, self.minor, self.build, self.revision
        )
    }
}

impl Clone for Version {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for Version {}

impl Version {
    pub fn new(major: u64, minor: u64, build: u64, revision: u64) -> Self {
        Self {
            major,
            minor,
            build,
            revision,
        }
    }

    pub fn from_string(version: &str) -> Result<Self, anyhow::Error> {
        let parts: Vec<&str> = version.split('.').collect();
        // allow 1 to 4 parts, defaulting missing parts to 0
        if parts.len() > 4 {
            return Err(anyhow::anyhow!("Version string has too many parts"));
        }

        let major = parts.first();
        let major = match major {
            Some(&major) => major.parse::<u64>(),
            None => Ok(0),
        };
        let major = match major {
            Ok(major) => major,
            Err(e) => return Err(anyhow::anyhow!("Failed to parse major version: {:?}", e)),
        };

        let minor = parts.get(1);
        let minor = match minor {
            Some(&minor) => minor.parse::<u64>(),
            None => Ok(0),
        };
        let minor = match minor {
            Ok(minor) => minor,
            Err(e) => return Err(anyhow::anyhow!("Failed to parse minor version: {:?}", e)),
        };

        let build = parts.get(2);
        let build = match build {
            Some(&build) => build.parse::<u64>(),
            None => Ok(0),
        };
        let build = match build {
            Ok(build) => build,
            Err(e) => return Err(anyhow::anyhow!("Failed to parse build version: {:?}", e)),
        };

        let revision = parts.get(3);
        let revision = match revision {
            Some(&revision) => revision.parse::<u64>(),
            None => Ok(0),
        };
        let revision = match revision {
            Ok(revision) => revision,
            Err(e) => return Err(anyhow::anyhow!("Failed to parse revision version: {:?}", e)),
        };

        Ok(Self {
            major,
            minor,
            build,
            revision,
        })
    }
}
