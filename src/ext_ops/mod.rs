//! Operations with external dependencies.

type ProcessId = u32;
/// Check whether SteamCMD or RustDedicated processes are already running.
pub fn is_process_running(name_seekable: &str) -> Option<ProcessId> {
    let name_seekable: &std::path::Path = std::path::Path::new(&name_seekable);
    let name_seekable: &std::ffi::OsStr = match name_seekable.file_name() {
        Some(n) => n,
        None => return None,
    };
    let name_seekable: String = match name_seekable.to_str() {
        Some(n) => n.to_owned(),
        None => return None,
    };

    let proc_dir: &str = "/proc/";
    let dir: std::fs::ReadDir = match std::fs::read_dir(proc_dir) {
        Ok(n) => n,
        Err(_) => unreachable!("{proc_dir} should always exist"),
    };

    for entry in dir {
        let entry: std::fs::DirEntry = match entry {
            Ok(n) => n,
            Err(_) => continue,
        };
        let path: std::path::PathBuf = entry.path();
        if !path.is_dir() {
            continue;
        }

        let filename: &std::ffi::OsStr = match path.file_name() {
            Some(n) => n,
            None => continue,
        };

        let filename: &str = match filename.to_str() {
            Some(n) => n,
            None => continue,
        };

        let pid: u32 = match filename.parse::<u32>() {
            Ok(n) => n,
            Err(_) => continue,
        };

        let path: std::path::PathBuf = path.join("comm");

        let proc_name: String = match std::fs::read_to_string(&path) {
            Ok(n) => n.trim().to_owned(),
            Err(_) => continue,
        };

        if proc_name == name_seekable {
            return Some(pid);
        }
    }
    return None;
}

/// Check if RustDedicated is installed.
pub fn is_game_installed() -> bool {
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
