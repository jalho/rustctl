//! Configuration for the program.

/// Configuration source from the filesystem.
#[derive(serde::Deserialize)]
struct ConfigSrcFs {
    root_dir: String,
    steamcmd_download: String,
    carbon_download: String,
}

pub struct PathAbsolute {
    /// Absolute path to a file or directory.
    pub path: std::path::PathBuf,
}
impl std::fmt::Display for PathAbsolute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.path.to_string_lossy()))
    }
}
impl PathAbsolute {
    pub fn parent(&self) -> std::path::PathBuf {
        let parent_path: std::path::PathBuf = match self.path.parent() {
            Some(n) => n.to_path_buf(),
            None => {
                todo!(); // TODO: Somehow ensure this is unreachable...
            }
        };
        return parent_path;
    }
}

/// Final configuration for the program constructed from sources like the
/// command line argument vector and a filesystem source.
pub struct Config {
    pub root_dir: PathAbsolute,

    pub steamcmd_download: String,
    pub steamcmd_archive: PathAbsolute,
    pub steamcmd_executable: PathAbsolute,
    pub steamcmd_installations: PathAbsolute,
    pub steamcmd_libs: PathAbsolute,

    pub carbon_download: String,
    pub carbon_archive: PathAbsolute,
    pub carbon_executable: PathAbsolute,

    pub game_manifest: PathAbsolute,
    pub game_startup_update_cooldown: std::time::Duration,
    pub game_executable: PathAbsolute,

    /// RCON password intended more like an internal constant rather
    /// than sensitive configuration value: The plan is to not expose
    /// the RCON service publicly at all but instead implement a limited
    /// wrapper around it, and the wrapper alone should be concerned
    /// with the RCON password, thus making it just an internal
    /// constant.
    pub rcon_password: String,
}
impl Config {
    pub fn new() -> Result<Self, crate::error::FatalError> {
        let config_file_path: &'static str = "/etc/rustctl/config.toml";
        let config_content: String = match std::fs::read_to_string(config_file_path) {
            Ok(n) => n,
            Err(err) => {
                return Err(crate::error::FatalError::new(
                    format!(
                    "cannot initialize config from filesystem: cannot read '{config_file_path}'"
                ),
                    Some(Box::new(err)),
                ))
            }
        };
        let config: ConfigSrcFs = match toml::from_str(&config_content) {
            Ok(n) => n,
            Err(err) => {
                return Err(crate::error::FatalError::new(
                    format!(
                    "cannot initialize config from filesystem: invalid content in '{config_file_path}'"
                ),
                    Some(Box::new(err)),
                ))
            }
        };
        let root_dir: std::path::PathBuf =
            match <std::path::PathBuf as std::str::FromStr>::from_str(&config.root_dir) {
                Ok(n) => n,
                Err(_) => {
                    // not sure what the from_str -> Result<> is for...
                    unreachable!();
                }
            };

        let mut steamcmd_archive: std::path::PathBuf = root_dir.clone();
        steamcmd_archive.push("steamcmd.tgz");

        let mut steamcmd_executable: std::path::PathBuf = root_dir.clone();
        steamcmd_executable.push("steamcmd.sh");

        let mut steamcmd_installations: std::path::PathBuf = root_dir.clone();
        steamcmd_installations.push("installations");

        let mut steamcmd_libs: std::path::PathBuf = root_dir.clone();
        steamcmd_libs.push("linux64");

        let mut carbon_archive: std::path::PathBuf = steamcmd_installations.clone();
        carbon_archive.push("carbon.tgz");

        let mut carbon_executable: std::path::PathBuf = steamcmd_installations.clone();
        carbon_executable.push("carbon/tools/environment.sh");

        let mut carbon_config: std::path::PathBuf = steamcmd_installations.clone();
        carbon_config.push("carbon/config.json");

        let mut game_manifest: std::path::PathBuf = steamcmd_installations.clone();
        game_manifest.push("steamapps/appmanifest_258550.acf");

        let mut game_executable: std::path::PathBuf = steamcmd_installations.clone();
        game_executable.push("RustDedicated");

        return Ok(Self {
            root_dir: PathAbsolute { path: root_dir },

            steamcmd_download: config.steamcmd_download,
            steamcmd_archive: PathAbsolute {
                path: steamcmd_archive,
            },
            steamcmd_executable: PathAbsolute {
                path: steamcmd_executable,
            },
            steamcmd_installations: PathAbsolute {
                path: steamcmd_installations,
            },
            steamcmd_libs: PathAbsolute {
                path: steamcmd_libs,
            },

            carbon_download: config.carbon_download,
            carbon_archive: PathAbsolute {
                path: carbon_archive,
            },
            carbon_executable: PathAbsolute {
                path: carbon_executable,
            },

            game_manifest: PathAbsolute {
                path: game_manifest,
            },
            game_startup_update_cooldown: std::time::Duration::from_secs(60 * 60),
            game_executable: PathAbsolute {
                path: game_executable,
            },

            rcon_password: String::from("Your_Rcon_Password"),
        });
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
