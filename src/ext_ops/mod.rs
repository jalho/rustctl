//! Operations with external dependencies.

/// _Absolute path_ to the directory in which _RustDedicated_ executable is
/// expected to be installed by _SteamCMD_.
static PATH_ABS_RDS_INSTALLATION: &'static str = "/home/rust/";

/// Name (not absolute path) of the Rust game server executable (installed with
/// SteamCMD).
static EXECUTABLE_NAME_RUSTDEDICATED: &'static str = "RustDedicated";

/// The closest thing to a _version_ that Steam apps have as far as I know. I
/// assume this is an incrementing non-negative, non-zero integer.
type SteamAppBuildId = u32;

/// Check if RustDedicated is installed.
pub fn is_game_installed() -> Option<SteamAppBuildId> {
    todo!("is_game_installed");
}

/// Do a fresh install of RustDedicated.
pub fn install_game<E: crate::proc::Exec>(
    steamcmd: &E,
) -> Result<crate::proc::Dependency, crate::error::ErrExec> {
    todo!("install_game");
}

/// Update an existing installation of RustDedicated.
pub fn update_game<E: crate::proc::Exec>(
    steamcmd: &E,
    current_version: SteamAppBuildId,
) -> Result<crate::proc::Dependency, crate::error::ErrExec> {
    /*
     *  ```
     *  $ steamcmd app_info_update 1
     *  $ steamcmd app_info_print 258550
     *  ```
     *  Then extract the build number and compare it
     *  against the value in the app manifest: `steamapps/
     *  appmanifest_258550.acf` under the server install tree
     */
    todo!("update_game");
}

/// Run game server and pass its standard output to a given channel.
pub fn run_game<E: crate::proc::Exec>(
    rustdedicated: &E,
    tx_stdout: std::sync::mpsc::Sender<String>,
) -> Result<(), crate::error::ErrExec> {
    todo!("run_game");
}
