//! Configuration for the program.

/// Configuration for the program.
#[derive(serde::Deserialize)]
pub struct Config {
    /// Where the program shall install _SteamCMD_, _RustDedicated_, _Carbon_ etc.
    pub rustctl_root_dir: std::path::PathBuf,

    /// Where _SteamCMD_ shall be downloaded from over the internet.
    pub steamcmd_download_url: String,
    /// Name of a .tgz file in which the downloaded _SteamCMD_ distribution shall be saved.
    pub steamcmd_target_file_name_tgz: std::path::PathBuf,
    /// Name of the _SteamCMD_ executable expected to be extracted from the distributed .tgz file.
    pub steamcmd_executable_name: std::path::PathBuf,
    /// Name of directory within `rustctl_root_dir` in which SteamCMD shall install the game server.
    /// For whatever reason this must be different from the directory in which the installer itself
    /// (SteamCMD) is installed.
    pub steamcmd_installations_dir_name: std::path::PathBuf,

}
impl Config {
    /// Where the configuration of this program will be stored at by default.
    pub fn default_fs_path() -> std::path::PathBuf {
        return "/etc/rustctl/config.toml".into();
    }

    /// Get configuration from filesystem.
    pub fn get_from_fs(
        config_file_path: std::path::PathBuf,
    ) -> Result<Self, crate::error::FatalError> {
        let content_raw: String = match std::fs::read_to_string(&config_file_path) {
            Ok(n) => n,
            Err(err) => {
                return Err(crate::error::FatalError::new(
                    format!(
                        "cannot read config from filesystem: '{}'",
                        config_file_path.to_string_lossy()
                    ),
                    Some(Box::new(err)),
                ));
            }
        };
        let config_parsed: Config = match toml::from_str(&content_raw) {
            Ok(n) => n,
            Err(err) => {
                return Err(crate::error::FatalError::new(
                    format!(
                        "cannot parse config from TOML: '{}'",
                        config_file_path.to_string_lossy()
                    ),
                    Some(Box::new(err)),
                ));
            }
        };

        if !config_parsed.rustctl_root_dir.is_dir() {
            return Err(crate::error::FatalError::new(
                format!(
                    "bad program root directory: not a directory: '{}'",
                    config_parsed.rustctl_root_dir.to_string_lossy()
                ),
                None,
            ));
        }

        let meta: std::fs::Metadata = match std::fs::metadata(&config_parsed.rustctl_root_dir) {
            Ok(n) => n,
            Err(err) => {
                return Err(crate::error::FatalError::new(
                    format!(
                        "bad program root directory: cannot read metadata: '{}'",
                        config_parsed.rustctl_root_dir.to_string_lossy()
                    ),
                    Some(Box::new(err)),
                ));
            }
        };

        use std::os::unix::fs::MetadataExt;
        let owner_uid: u32 = meta.uid();

        /* Really needed to only check that the current user owns the dir, but
        it is easier to just check against uid 1000 which suffices for my
        intended use case (single user Debian system). */
        let required_owner_uid: u32 = 1000;
        if owner_uid != required_owner_uid {
            return Err(crate::error::FatalError::new(
                format!(
                    "bad program root directory: not owned by user {}: '{}'",
                    required_owner_uid,
                    config_parsed.rustctl_root_dir.to_string_lossy()
                ),
                None,
            ));
        }

        use std::os::unix::fs::PermissionsExt;
        let permissions: u32 = meta.permissions().mode();
        let can_read = permissions & 0o400 != 0;
        let can_write = permissions & 0o200 != 0;
        let can_execute = permissions & 0o100 != 0;
        if !can_read || !can_write || !can_execute {
            return Err(crate::error::FatalError::new(
                format!(
                    "bad program root directory: missing rwx permissions: '{}'",
                    config_parsed.rustctl_root_dir.to_string_lossy()
                ),
                None,
            ));
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
    pub fn get(argv: Vec<String>) -> Result<Self, crate::error::FatalError> {
        let arg_count_min: usize = 2;
        if argv.len() < arg_count_min {
            return Err(crate::error::FatalError::new(
                format!("expected at least {} arguments", arg_count_min),
                None,
            ));
        }

        let arg1: &String = &argv[1];
        if arg1 == "config" {
            return Ok(Self::Config);
        } else if arg1 == "game" {
            return Ok(Self::GameStart);
        } else if arg1 == "health" {
            return Ok(Self::HealthStart);
        } else if arg1 == "--help" {
            return Ok(Self::Help);
        } else if arg1 == "web" {
            return Ok(Self::WebStart);
        } else if arg1 == "--version" {
            return Ok(Self::Version);
        } else {
            return Err(crate::error::FatalError::new(
                format!("unknown argument: '{}'", arg1),
                None,
            ));
        }
    }
}
