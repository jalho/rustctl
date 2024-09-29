//! Configuration for the program.

use std::os::unix::fs::PermissionsExt;

/// Errors regarding bad args given to the program.
pub enum ArgError {
    /// Unexpected amount of arguments failure.
    ArgvLen,
    /// Unknown argument failure.
    ArgUnknown,
    /// Filesystem read failure.
    ConfigReadFs(std::io::ErrorKind),
    /// TOML format parsing, missing or bad value failures.
    ConfigInvalid(String),
}
impl From<std::io::Error> for ArgError {
    fn from(err: std::io::Error) -> Self {
        return Self::ConfigReadFs(err.kind());
    }
}
impl From<toml::de::Error> for ArgError {
    fn from(err: toml::de::Error) -> Self {
        return Self::ConfigInvalid(String::from(err.message()));
    }
}
impl std::fmt::Debug for ArgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ArgvLen => write!(f, "ArgvLen"),
            Self::ArgUnknown => write!(f, "ArgUnknown"),
            Self::ConfigReadFs(arg0) => f.debug_tuple("ConfigReadFs").field(arg0).finish(),
            Self::ConfigInvalid(arg0) => f.debug_tuple("ConfigInvalid").field(arg0).finish(),
        }
    }
}

/// Configuration for the program.
#[derive(serde::Deserialize)]
pub struct Config {
    /// Where _SteamCMD_ shall be downloaded from over the internet.
    pub download_url_steamcmd: String,

    /// Where the program shall install _SteamCMD_, _RustDedicated_, _Carbon_ etc.
    pub rustctl_root_dir: std::path::PathBuf,
}
impl Config {
    /// Where the configuration of this program will be stored at by default.
    pub fn default_fs_path() -> std::path::PathBuf {
        return "/etc/rustctl/config.toml".into();
    }

    /// Get configuration from filesystem.
    pub fn get_from_fs(config_file_path: std::path::PathBuf) -> Result<Self, ArgError> {
        let content_raw: String = std::fs::read_to_string(&config_file_path)?;
        let config_parsed: Config = toml::from_str(&content_raw)?;

        if !config_parsed.rustctl_root_dir.is_dir() {
            return Err(ArgError::ConfigInvalid(format!(
                "not a directory: '{}'",
                config_parsed.rustctl_root_dir.to_string_lossy()
            )));
        }

        let meta: std::fs::Metadata = std::fs::metadata(&config_parsed.rustctl_root_dir)?;

        use std::os::unix::fs::MetadataExt;
        let owner_uid: u32 = meta.uid();

        /* Really needed to only check that the current user owns the dir, but
        it is easier to just check against uid 1000 which suffices for my
        intended use case (single user Debian system). */
        let required_owner_uid: u32 = 1000;
        if owner_uid != required_owner_uid {
            return Err(ArgError::ConfigInvalid(format!(
                "not owned by user {}: '{}'",
                required_owner_uid,
                &config_parsed.rustctl_root_dir.to_string_lossy()
            )));
        }

        let permissions: u32 = meta.permissions().mode();
        let can_read = permissions & 0o400 != 0;
        let can_write = permissions & 0o200 != 0;
        let can_execute = permissions & 0o100 != 0;
        if !can_read || !can_write || !can_execute {
            return Err(ArgError::ConfigInvalid(format!(
                "rwx permissions required: '{}'",
                config_parsed.rustctl_root_dir.to_string_lossy()
            )));
        }

        return Ok(config_parsed);
    }
}

/// The commands of the program.
pub enum Command {
    Config,
    GameStart,
    HealthStart,
    Help,
    Version,
    WebStart,
}
impl Command {
    /// Determine the command based on the program's arguments.
    pub fn get(argv: Vec<String>) -> Result<Self, ArgError> {
        if argv.len() < 2 {
            return Err(ArgError::ArgvLen);
        } else if argv[1] == "config" {
            return Ok(Self::Config);
        } else if argv[1] == "game" {
            return Ok(Self::GameStart);
        } else if argv[1] == "health" {
            return Ok(Self::HealthStart);
        } else if argv[1] == "--help" {
            return Ok(Self::Help);
        } else if argv[1] == "web" {
            return Ok(Self::WebStart);
        } else if argv[1] == "--version" {
            return Ok(Self::Version);
        } else {
            return Err(ArgError::ArgUnknown);
        }
    }
}
