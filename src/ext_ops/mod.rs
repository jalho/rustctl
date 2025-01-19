//! Operations with external dependencies.

/// Name (not absolute path) of the Rust game server executable (installed with
/// SteamCMD).
static EXECUTABLE_NAME_RUSTDEDICATED: &'static str = "RustDedicated";

pub struct InstallOpts {
    pub steam_app_id: u32,
}

pub fn parse_buildid_from_manifest(manifest_path: &std::path::Path) -> Option<u32> {
    if let Ok(content) = std::fs::read_to_string(manifest_path) {
        for line in content.lines() {
            let trimmed: &str = line.trim();
            if trimmed.starts_with("\"buildid\"") {
                if let Some(_) = trimmed.find('\"') {
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(buildid) = parts[1].trim_matches('"').parse::<u32>() {
                            return Some(buildid);
                        }
                    }
                }
            }
        }
    }
    return None;
}

/// Do a fresh install of RustDedicated.
pub fn install_game<E: crate::proc::Exec>(
    steamcmd: &E,
    installation_dir: &std::path::Path,
    install_opts: &InstallOpts,
) -> Result<crate::proc::Dependency, crate::error::ErrExec> {
    let steam_app_id: u32 = install_opts.steam_app_id;
    steamcmd.exec_terminating(vec![
        /*
         * Note: It seems force_install_dir doesn't really _force_ anything:
         * If no write permissions, the stuff seems to just be dumped into
         * current user's home dir instead...
         */
        "+force_install_dir",
        &installation_dir.to_string_lossy(),
        "+login",
        "anonymous",
        "+app_update",
        &steam_app_id.to_string(),
        "validate",
        "+quit",
    ])?;

    let mut appmanifest_file_path: std::path::PathBuf = std::path::PathBuf::new();
    appmanifest_file_path.push(&installation_dir);
    appmanifest_file_path.push("steamapps");
    let appmanifest_file_name: String = format!("appmanifest_{steam_app_id}.acf");
    appmanifest_file_path.push(&appmanifest_file_name);

    let appmanifest_file_path: &std::path::Path = std::path::Path::new(&appmanifest_file_path);
    if !appmanifest_file_path.is_file() {
        todo!("define error case: no appmanifest found for steam app");
    }

    let build_id: u32 = match crate::ext_ops::parse_buildid_from_manifest(&appmanifest_file_path) {
        Some(n) => n,
        None => todo!("define error case: could not parse steam build id from app manifest"),
    };

    let mut executable_path: std::path::PathBuf = installation_dir.into();
    executable_path.push(&EXECUTABLE_NAME_RUSTDEDICATED);
    if !executable_path.is_file() {
        todo!("define error case: no RustDedicated was installed??");
    }

    let executable: String = executable_path.to_string_lossy().into_owned();
    let rustdedicated: crate::proc::Dependency = crate::proc::Dependency {
        executable,
        work_dir: installation_dir.into(),
        role_displayed: String::from("game server"),
        version: crate::proc::DependencyVersion::SteamAppBuildId(build_id),
    };
    return Ok(rustdedicated);
}

/// Update an existing installation of RustDedicated.
pub fn update_game<E: crate::proc::Exec>(
    steamcmd: &E,
    current: &crate::proc::Dependency,
    install_opts: &InstallOpts,
) -> Result<Option<crate::proc::Dependency>, crate::error::ErrExec> {
    let current_installed_build_id: u32 = match current.version {
        crate::proc::DependencyVersion::Unknown => {
            unreachable!("a dependency of kind Steam app should always have a determined build ID")
        }
        crate::proc::DependencyVersion::SteamAppBuildId(n) => n,
    };
    steamcmd.exec_terminating(vec!["+app_info_update", "1", "+quit"])?;
    let (stdout, _) = steamcmd.exec_terminating(vec![
        "+app_info_print",
        &install_opts.steam_app_id.to_string(),
        "+quit",
    ])?;
    if let Some((build_id_a, build_id_b)) = parse_buildids(stdout) {
        if build_id_a != build_id_b {
            todo!("define handled error case for: conflicting build ids from remote -- which one to pick???");
        }
        if build_id_a == current_installed_build_id {
            return Ok(None);
        }
        todo!("updates are known to be available -- go install!");
    } else {
        /*
         tested to work as user that owns /home/rust/, but not as other user
         --> implies the check requires rwx or something to the exec dir
        */
        todo!("define handled error case for: not able to get latest build ids from remote (seems to require write perms in exec dir or something??)");
    }
}

/// Run game server and pass its standard output to a given channel.
pub fn run_game<E: crate::proc::Exec>(
    _rustdedicated: &E,
    _tx_stdout: std::sync::mpsc::Sender<String>,
) -> Result<(), crate::error::ErrExec> {
    todo!("run_game");
}

fn parse_buildids(content: String) -> Option<(u32, u32)> {
    let mut public_buildid: Option<u32> = None;
    let mut release_buildid: Option<u32> = None;

    let mut lines = content.lines().map(|line| line.trim());

    while let Some(line) = lines.next() {
        if line.starts_with("\"public\"") {
            while let Some(inner_line) = lines.next() {
                if inner_line.starts_with("\"buildid\"") {
                    public_buildid = inner_line
                        .split_whitespace()
                        .nth(1)
                        .and_then(|s| s.trim_matches('"').parse::<u32>().ok());
                    break;
                }
            }
        } else if line.starts_with("\"release\"") {
            while let Some(inner_line) = lines.next() {
                if inner_line.starts_with("\"buildid\"") {
                    release_buildid = inner_line
                        .split_whitespace()
                        .nth(1)
                        .and_then(|s| s.trim_matches('"').parse::<u32>().ok());
                    break;
                }
            }
        }

        if public_buildid.is_some() && release_buildid.is_some() {
            break;
        }
    }

    match (public_buildid, release_buildid) {
        (Some(public), Some(release)) => Some((public, release)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_buildids() {
        let manifest_content = r#"
        "branches"
        {
            "public"
            {
                "buildid"       "123"
                "timeupdated"   "1737126374"
            }
            "release"
            {
                "buildid"       "456"
                "timeupdated"   "1737123128"
            }
        }
        "#;
        let result = parse_buildids(manifest_content.to_string());
        assert_eq!(result, Some((123, 456)));
    }
}
