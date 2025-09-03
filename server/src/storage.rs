/// Shared client for getting config args for the game server from e.g. a
/// database.
#[derive(Clone)]
pub struct ConfigurationClient(std::sync::Arc<tokio::sync::Mutex<Configuration>>);

impl ConfigurationClient {
    pub fn init() -> Self {
        Self(std::sync::Arc::new(tokio::sync::Mutex::new(Configuration::default())))
    }

    pub async fn get_config(&self) -> Configuration {
        let config: Configuration;

        {
            let lock = self.0.lock().await;
            config = lock.clone();
        }

        config
    }
}

/// Parameters for spawning a game server process.
#[derive(Clone)]
pub struct Configuration {
    /// Executable: game server installer.
    ///
    /// For example:
    /// ```
    /// "/usr/bin/steamcmd"
    /// ```
    pub installer_exe: &'static str,

    /// Directory: game server install location.
    ///
    /// For example:
    /// ```
    /// "/home/rust/"
    /// ```
    pub game_server_root: &'static str,

    /// Executable: the game server.
    ///
    /// For example:
    /// ```
    /// "/home/rust/RustDedicated"
    /// ```
    pub game_server_exe: &'static str,

    /// File: some Steam thing associated with the game server.
    ///
    /// For example:
    /// ```
    /// "/home/rust/steamapps/appmanifest_258550.acf"
    /// ```
    pub game_manifest: &'static str,

    /// Directory: location of `steamclient.so`, which the game server requires.
    ///
    /// For example:
    /// ```
    /// "/home/rust/"
    /// ```
    pub game_server_libs: &'static str,

    pub game_instance_id: &'static str,

    pub game_world_size: u16,
    pub game_world_seed: u32,

    pub rcon_port: u16,
    pub rcon_password: String,

    /// URL from where _Carbon Modding Framework_ shall be downloaded from.
    ///
    /// For example:
    /// ```
    /// "https://github.com/CarbonCommunity/Carbon/releases/download/production_build/Carbon.Linux.Minimal.tar.gz"
    /// ```
    pub carbon_download_url: String,

    /// Startup script that shall be generated at runtime. The script is the
    /// entry point for the game server.
    ///
    /// For example:
    /// ```
    /// "/home/rust/rustctl-run-with-carbon.sh"
    /// ```
    pub game_server_startup_script: String,
}

impl Configuration {
    pub fn get_installer_args(&self) -> Vec<String> {
        vec![
            "+login".into(),
            "anonymous".into(),
            /*
             * WONTFIX: "force_install_dir" doesn't really "force" anything:
             *          Instead, SteamCMD seems to just create a new directory
             *          tree in "~/.local/share/Steam/" if it cannot access
             *          the given "force_install_dir".
             *
             *          Behavior observed in `apt` packaged version:
             *          - Package: steamcmd:i386
             *          - Version: 0~20180105-5 (latest as of July 2025)
             *          - Section: non-free/games
             *          - Maintainer: Debian Games Team
             */
            "+force_install_dir".into(),
            self.game_server_root.to_string(),
            "+app_update".into(),
            "258550".into(),
            "validate".into(),
            "+quit".into(),
        ]
    }

    pub fn get_rcon_connection_string(&self) -> String {
        format!(
            "ws://127.0.0.1:{port}/{password}",
            port = self.rcon_port,
            password = self.rcon_password,
        )
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            installer_exe: "/usr/bin/steamcmd",

            game_server_root: "/home/rust/",
            game_server_exe: "/home/rust/RustDedicated",
            game_manifest: "/home/rust/steamapps/appmanifest_258550.acf",
            game_server_libs: "/home/rust/",

            game_instance_id: "instance0",

            /*
             * Some observed maps as of 2025-08-29, buildid 19776612, world
             * size 1000:
             *
             * - seed "1": Has some land. Useful for testing because stuff can
             *   be built which requires land.
             *
             * - seed "1234": Has no land, only water.
             */
            game_world_seed: 1,
            game_world_size: 1000, // minimum world size AFAIK

            rcon_port: 28016,
            rcon_password: uuid::Uuid::new_v4().to_string(),

            carbon_download_url: "https://github.com/CarbonCommunity/Carbon/releases/download/production_build/Carbon.Linux.Minimal.tar.gz".to_string(),
            game_server_startup_script: "/home/rust/rustctl-run-with-carbon.sh".to_string(),
        }
    }
}
