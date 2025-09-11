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
    pub game_world_size: u16,
    pub game_world_seed: u32,

    pub rcon_port: u16,
    pub rcon_password: String,
    pub game_owner_steamid: String,

    /// URL from where _Carbon Modding Framework_ shall be downloaded from.
    ///
    /// For example:
    /// ```
    /// "https://github.com/CarbonCommunity/Carbon/releases/download/production_build/Carbon.Linux.Minimal.tar.gz"
    /// ```
    pub carbon_download_url: String,

    pub game_name: String,
    pub game_description: String,
    pub game_url_home: String,
    pub game_url_header: String,
    pub game_url_logo: String,
}

impl Configuration {
    pub fn get_installer_args(&self) -> Vec<&'static str> {
        vec![
            "+login",
            "anonymous",
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
            "+force_install_dir",
            rustctl_backend::constants::paths::ROOT_DIR,
            "+app_update",
            "258550",
            "validate",
            "+quit",
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
            game_owner_steamid: "76561198135242017".to_string(),

            carbon_download_url: "https://github.com/CarbonCommunity/Carbon/releases/download/production_build/Carbon.Linux.Minimal.tar.gz".to_string(),

            game_name: "rustctl".to_string(),
            game_description: "rustctl managed server".to_string(),
            game_url_home: "https://github.com/jalho/rustctl".to_string(),
            game_url_header: "https://upload.wikimedia.org/wikipedia/commons/thumb/c/c1/Vexillum_aboense.jpg/1280px-Vexillum_aboense.jpg".to_string(),
            game_url_logo: "https://upload.wikimedia.org/wikipedia/commons/thumb/b/bc/Flag_of_Finland.svg/60px-Flag_of_Finland.svg.png".to_string(),
        }
    }
}
