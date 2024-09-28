//! Configuration for the program.

/// Configuration for the program.
pub struct Config {
    /// Where _SteamCMD_ shall be downloaded from over the internet.
    pub download_url_steamcmd: String,
}

impl Config {
    /// Where the configuration of this program will be stored at by default.
    pub fn default_fs_path() -> std::path::PathBuf {
        return "/etc/rustctl/config.toml".into();
    }

    /// Get configuration from filesystem.
    pub fn get_from_fs(config_file_path: std::path::PathBuf) -> Result<Self, std::io::Error> {
        let content: String = std::fs::read_to_string(&config_file_path)?;

        println!("Read {} chars from {:?}", content.len(), config_file_path);
        // TODO: Parse TOML!

        return Ok(Self {
            download_url_steamcmd: "http://127.0.0.1:8080/steamcmd.tgz".into(),
        });
    }
}
