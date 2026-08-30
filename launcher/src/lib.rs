pub struct GameServerConfig {}

impl Default for GameServerConfig {
    fn default() -> Self {
        Self {  }
    }
}

pub fn launch_game_server(_config: &GameServerConfig) -> std::process::ExitCode {
    std::process::ExitCode::from(44)
}
