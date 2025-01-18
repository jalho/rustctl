//! Operations with external dependencies.

/// Check whether SteamCMD or RustDedicated processes are already running.
pub fn assure_not_running() -> Result<(), ()> {
    todo!();
}

/// Check if RustDedicated is installed.
pub fn is_game_installed() -> bool {
    todo!();
}

/// Do a fresh install of RustDedicated.
pub fn install_game<E: crate::proc::Exec>(
    steamcmd: &E,
) -> Result<crate::proc::Dependency, crate::error::ErrExec> {
    todo!();
}

/// Update an existing installation of RustDedicated.
pub fn update_game<E: crate::proc::Exec>(
    steamcmd: &E,
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
    todo!();
}

pub fn run_game<E: crate::proc::Exec>(
    rustdedicated: &E,
    tx_stdout: std::sync::mpsc::Sender<String>,
) -> Result<(), crate::error::ErrExec> {
    todo!();
}
