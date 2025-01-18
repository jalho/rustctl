//! Operations with external dependencies.

/// _Absolute path_ to the directory in which _RustDedicated_ executable is
/// expected to be installed by _SteamCMD_.
static PATH_ABS_RDS_INSTALLATION: &'static str = "/home/rust/";

/// Name (not absolute path) of the Rust game server executable (installed with
/// SteamCMD).
static EXECUTABLE_NAME_RUSTDEDICATED: &'static str = "RustDedicated";

/// Steam app ID of the Rust game server (RustDedicated).
static STEAM_APP_ID_RUSTDEDICATED: u32 = 258550;

#[derive(serde::Deserialize)]
struct SteamAppManifest {
    buildid: SteamAppBuildId,
}

/// The closest thing to a _version_ that Steam apps have as far as I know. I
/// assume this is an incrementing non-negative, non-zero integer.
type SteamAppBuildId = u32;

/// Check if RustDedicated is installed.
pub fn is_game_installed() -> Option<SteamAppBuildId> {
    let executable_path: &std::path::Path =
        &std::path::Path::new(PATH_ABS_RDS_INSTALLATION).join(EXECUTABLE_NAME_RUSTDEDICATED);

    if !executable_path.is_file() {
        return None;
    }

    if let Ok(metadata) = executable_path.metadata() {
        if std::os::unix::fs::PermissionsExt::mode(&metadata.permissions()) & 0o111 == 0 {
            return None;
        }
    } else {
        return None;
    }

    let appmanifest_file_name: String = format!("appmanifest_{STEAM_APP_ID_RUSTDEDICATED}.acf");
    let manifest_path: &std::path::Path = std::path::Path::new(&appmanifest_file_name);

    if !manifest_path.is_file() {
        return None;
    }

    if let Ok(file) = std::fs::File::open(manifest_path) {
        if let Ok(manifest) = serde_json::from_reader::<_, SteamAppManifest>(file) {
            return Some(manifest.buildid);
        }
    }
    return None;
}

/// Do a fresh install of RustDedicated.
pub fn install_game<E: crate::proc::Exec>(
    steamcmd: &E,
) -> Result<crate::proc::Dependency, crate::error::ErrExec> {
    steamcmd.exec_terminating(
        Some(std::path::Path::new(&PATH_ABS_RDS_INSTALLATION)),
        vec![
            "+force_install_dir",
            &PATH_ABS_RDS_INSTALLATION,
            "+login",
            "anonymous",
            "+app_update",
            "258550",
            "validate",
            "+quit",
        ],
    )?;
    todo!("verify installation success");
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
