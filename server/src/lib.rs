pub mod constants;

pub struct GameParameters {
    pub game_world_size: u16,
    pub game_world_seed: u32,

    pub rcon_port: u16,
    pub rcon_password: String,
    pub game_owner_steamid: String,

    pub carbon_download_url: String,

    pub game_name: String,
    pub game_description: String,
    pub game_url_home: String,
    pub game_url_header: String,
    pub game_url_logo: String,
}

impl GameParameters {
    pub fn get_installer_args(&self) -> Vec<&'static str> {
        vec![
            "+login",
            "anonymous",
            "+force_install_dir",
            constants::paths::ROOT_DIR,
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
