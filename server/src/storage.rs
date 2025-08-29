/// Shared client for getting config args for the game server from e.g. a
/// database.
#[derive(Clone)]
pub struct GameServerConfigurationShared(std::sync::Arc<tokio::sync::Mutex<GameServerConfiguration>>);

impl GameServerConfigurationShared {
    pub fn init() -> Self {
        Self(std::sync::Arc::new(tokio::sync::Mutex::new(
            GameServerConfiguration::default(),
        )))
    }

    pub async fn get_config(&self) -> GameServerConfiguration {
        let config: GameServerConfiguration;

        {
            let lock = self.0.lock().await;
            config = lock.clone();
        }

        config
    }
}

/// Parameters for spawning a game server process.
#[derive(Clone)]
pub struct GameServerConfiguration {
    /// Executable: game server installer.
    pub installer_exe: &'static str,

    /// Directory: game server install location.
    pub game_server_root: &'static str,

    /// Executable: the game server.
    pub game_server_exe: &'static str,

    /// File: some Steam thing associated with the game server.
    pub game_manifest: &'static str,

    /// Directory: location of `steamclient.so`, which the game server requires.
    pub game_server_libs: &'static str,

    game_instance_id: &'static str,

    game_world_size: u16,
    game_world_seed: u32,

    rcon_port: u16,
    rcon_password: String,
}

impl GameServerConfiguration {
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

    pub fn get_game_args(&self) -> Vec<String> {
        vec![
            "-batchmode".into(),
            "+server.identity".into(),
            self.game_instance_id.to_owned(),
            "+rcon.port".into(),
            self.rcon_port.to_string(),
            "+rcon.web".into(),
            "1".into(),
            "+rcon.password".into(),
            self.rcon_password.clone(),
            "+server.worldsize".into(),
            self.game_world_size.to_string(),
            "+server.seed".into(),
            self.game_world_seed.to_string(),
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

impl Default for GameServerConfiguration {
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
        }
    }
}
