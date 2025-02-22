use std::path::Path;

use state::{
    InstalledNotRunningNotUpdated, InstalledNotRunningUpdated, NotInstalled, RunningHealthy,
    RunningNotHealthy,
};

use crate::{
    core::JoinWith,
    system::{check_process_running, find_single_file, FindSingleFileError, FoundFile},
};

mod state;

pub enum Game<'res> {
    /// Not installed
    Ni(NotInstalled<'res>),

    /// Installed, not running, not updated
    INrNu(InstalledNotRunningNotUpdated<'res>),

    /// Installed, not running, updated
    INrU(InstalledNotRunningUpdated<'res>),

    /// Running, not healthy
    RNh(RunningNotHealthy<'res>),

    /// Running, healthy
    Rh(RunningHealthy<'res>),
}

impl<'res> Game<'res> {
    /// Determine initial state.
    pub fn check(expected: &'res Resources) -> Result<Self, std::process::ExitCode> {
        let game_executable: FoundFile = match find_single_file(
            &expected.game_executable,
            Some(std::path::Path::new("/mnt/c")), // on WSL, skip C:\ because it's so damn slow to traverse
        ) {
            Ok(found_file) => {
                log::info!("Found {found_file}");
                found_file
            }

            Err(FindSingleFileError::FileNotFound { .. }) => {
                log::info!(
                    "Game server is not yet installed: Searched for {}",
                    &expected.game_executable.to_string_lossy()
                );
                return Ok(Self::Ni(NotInstalled::new(&expected)));
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

        match check_process_running(&game_executable.filename) {
            Ok(Some(pid)) => {
                /* Let's say _already running_ is an illegal initial state.
                We want this program to spawn the game server so we can do all
                kinds of tricks as its parent. */
                log::error!("Cannot start game: There is already a game server process running: process ID {pid}");
                return Err(std::process::ExitCode::FAILURE);
            }
            Ok(None) => {
                log::info!("No game server process is running yet");
                return Ok(Self::INrNu(InstalledNotRunningNotUpdated::new(&expected)));
            }
            Err(err) => {
                log::error!("Cannot start game: Cannot check whether there is a game server process already running: {err}");
                return Err(std::process::ExitCode::FAILURE);
            }
        }
    }

    pub fn start(self) -> Result<RunningHealthy<'res>, std::process::ExitCode> {
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

/// Some expected resources related to the game server.
pub struct Resources {
    /// Absolute path to the root directory where the game server executable is
    /// installed at.
    root_abs: std::path::PathBuf,

    /// Filename, _not the absolute path_, of the game server executable.
    pub game_executable: std::path::PathBuf,

    /// Path to the Steam app manifest file, _relative to the game server
    /// executable_.
    manifest_path: std::path::PathBuf,

    /// Steam app ID of the game server.
    app_id: u32,

    /// Name, _not full path_, of the Steam cache file that seems to interfere
    /// with querying app info from remote. It is unclear whether SteamCMD can
    /// be used without having to delete the cache file. The exact location of
    /// the file seems to vary depending on the Linux distribution. Differing
    /// behavior seen at least on Debian 12, Ubuntu 24 and Arch. Common
    /// nominator seems to be that it's called `appinfo.vdf` (_Valve Data File_,
    /// maybe?), and it's located _somewhere_ under the current user's home.
    cache_filename: std::path::PathBuf,
}

impl Resources {
    pub fn new() -> Self {
        let app_id: u32 = 258550;
        let cache_filename = Path::new("appinfo.vdf").to_path_buf();
        let game_executable = Path::new("RustDedicated").to_path_buf();
        let manifest_path = Path::new(&format!("steamapps/appmanifest_{app_id}.acf")).to_path_buf();
        let root_abs = Path::new("/home/rust").to_path_buf();

        return Self {
            app_id,
            cache_filename,
            game_executable,
            manifest_path,
            root_abs,
        };
    }
}
