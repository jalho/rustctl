use crate::{
    core::JoinWith,
    system::{check_process_running, find_single_file, FindSingleFileError, FoundFile},
};

mod state;

pub enum Game {
    /// Not installed
    Ni(state::NotInstalled),

    /// Installed, not running, not updated
    INrNu(state::InstalledNotRunningNotUpdated),

    /// Installed, not running, updated
    INrU(state::InstalledNotRunningUpdated),

    /// Running, not healthy
    RNh(state::RunningNotHealthy),

    /// Running, healthy
    Rh(state::RunningHealthy),
}

impl Game {
    /// Determine initial state.
    pub fn check() -> Result<Self, std::process::ExitCode> {
        let game_executable = std::path::Path::new("RustDedicated");
        let game_executable: FoundFile = match find_single_file(
            game_executable,
            Some(std::path::Path::new("/mnt/c")), // on WSL, skip C:\ because it's so damn slow to traverse
        ) {
            Ok(found_file) => found_file,
            Err(FindSingleFileError::FileNotFound { .. }) => {
                return Ok(Self::Ni(state::NotInstalled {}))
            }
            Err(FindSingleFileError::ManyFilesFound {
                paths_absolute_found,
            }) => {
                log::error!(
                    "Cannot start game: Ambiguous installation: Found in {} places: {}",
                    paths_absolute_found.len(),
                    paths_absolute_found.join_with(", ")
                );
                return Err(std::process::ExitCode::FAILURE);
            }
        };

        let pid: u32 = match check_process_running(&game_executable.filename) {
            Ok(Some(pid)) => pid,
            Ok(None) => return Ok(Self::INrNu(state::InstalledNotRunningNotUpdated {})),
            Err(err) => {
                log::error!("Cannot start game: Cannot check whether there is a game server process already running: {err}");
                return Err(std::process::ExitCode::FAILURE);
            }
        };

        /* Let's say _already running_ is an illegal initial state. We want this
        program to spawn the game server so we can do all kinds of tricks as its
        parent. */
        log::error!(
            "Cannot start game: There is already a game server process running: process ID {pid}"
        );
        return Err(std::process::ExitCode::FAILURE);
    }

    pub fn start(self) -> Result<crate::game::state::RunningHealthy, std::process::ExitCode> {
        match self {
            Game::Ni(s) => s
                .install_latest_version_from_remote()?
                .spawn_game_server_process()?
                .healthcheck_timeout(),

            Game::INrNu(s) => s
                .update_existing_installation_from_remote()?
                .spawn_game_server_process()?
                .healthcheck_timeout(),

            Game::INrU(s) => s.spawn_game_server_process()?.healthcheck_timeout(),

            Game::RNh(s) => s.healthcheck_timeout(),

            Game::Rh(s) => Ok(s),
        }
    }
}
