//! Dumpster for miscellaneous stuff yet to be better categorized.

use std::path::PathBuf;

use log::info;

const CMD_STRACE: &str = "strace";

/// Initialize a global logging utility.
pub fn init_logger() -> Result<log4rs::Handle, crate::error::FatalError> {
    let stdout = log4rs::append::console::ConsoleAppender::builder()
        .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
            "[{d(%Y-%m-%dT%H:%M:%S%.3f)}] {h([{l}])} - {m}{n}",
        )))
        .build();
    let logger_config: log4rs::Config = match log4rs::Config::builder()
        .appender(log4rs::config::Appender::builder().build("stdout", Box::new(stdout)))
        .build(
            log4rs::config::Root::builder()
                .appender("stdout")
                .build(log::LevelFilter::Debug),
        ) {
        Ok(n) => n,
        Err(err) => {
            return Err(crate::error::FatalError::new(
                format!("bad logger config"),
                Some(Box::new(err)),
            ));
        }
    };
    let logger: log4rs::Handle = match log4rs::init_config(logger_config) {
        Ok(n) => n,
        Err(err) => {
            return Err(crate::error::FatalError::new(
                format!("bad instantiation of logger"),
                Some(Box::new(err)),
            ));
        }
    };
    return Ok(logger);
}

/// Install or update an existing installation of the game server.
pub fn install_update_game_server(
    rustctl_root_dir: &std::path::PathBuf,
    steamcmd_executable_filename: &std::path::PathBuf,
    steamcmd_installations_dir_name: &std::path::PathBuf,
    game_server_executable_filename: &std::path::PathBuf,
) -> Result<(), crate::error::FatalError> {
    let mut steamcmd_executable_absolute: std::path::PathBuf = rustctl_root_dir.clone();
    steamcmd_executable_absolute.push(steamcmd_executable_filename);

    // TODO: Accept &std::path::PathBuf in run_with_strace?
    let steamcmd_executable_absolute: &str = &steamcmd_executable_absolute.to_string_lossy();

    /* Game server installation location must be different than where the installer is for some reason... */
    let mut game_server_install_dir: std::path::PathBuf = rustctl_root_dir.clone();
    game_server_install_dir.push(steamcmd_installations_dir_name);
    if !game_server_install_dir.is_dir() {
        match std::fs::create_dir(&game_server_install_dir) {
            Ok(_) => {}
            Err(err) => {
                return Err(crate::error::FatalError::new(format!("cannot install or update game server: cannot create installation directory '{}'", game_server_install_dir.to_string_lossy()), Some(Box::new(err))));
            }
        }
    }

    info!(
        "Installing or updating game server with SteamCMD to '{}'",
        game_server_install_dir.to_string_lossy()
    );
    let mut paths_touched: Vec<String> = match run_with_strace(
        steamcmd_executable_absolute,
        vec![
            "+force_install_dir",
            &game_server_install_dir.to_string_lossy(),
            "+login",
            "anonymous",
            "+app_update",
            "258550",
            "validate",
            "+quit",
        ],
        rustctl_root_dir,
    ) {
        Err(StraceFilesError::DecodeUtf8(err)) => {
            return Err(crate::error::FatalError::new(format!("cannot install or update game server: cannot decode output of '{CMD_STRACE}' with '{steamcmd_executable_absolute}' as UTF-8"), Some(Box::new(err))))
        },
        Err(StraceFilesError::ExitStatus) => {
            return Err(crate::error::FatalError::new(format!("cannot install or update game server: '{CMD_STRACE}' with '{steamcmd_executable_absolute}' exited with unsuccessful status"), None))
        },
        Err(StraceFilesError::IO(err)) => {
            return Err(crate::error::FatalError::new(format!("cannot install or update game server: cannot execute '{CMD_STRACE}' with '{steamcmd_executable_absolute}'"), Some(Box::new(err))))
        },
        Ok(n) => n,
    };
    paths_touched.sort();

    /* TODO: Fix fs strace with SteamCMD... The installation DOES yield
    `./installations/RustDedicated` (The main thing we want!) yet the way we use
    strace doesn't pick it up. However, it does detect `./installations/steamapps/downloading/258550/RustDedicated`
    among other things! No idea what OS mechanism yields the `./installations/RustDedicated`...
    (Already tried adding "rename" to the watched syscalls -- Did not help!) */
    log::info!(
        "Installed or updated {} game server files with SteamCMD: {}",
        paths_touched.len(),
        paths_touched.join(", ")
    );

    let mut game_server_executable_absolute: std::path::PathBuf = game_server_install_dir.clone();
    game_server_executable_absolute.push(game_server_executable_filename);
    if !game_server_executable_absolute.is_file() {
        return Err(crate::error::FatalError::new(
            format!(
                "unexpected game server installation: did not contain file '{}' ('{}')",
                game_server_executable_filename.to_string_lossy(),
                game_server_executable_absolute.to_string_lossy(),
            ),
            None,
        ));
    }

    return Ok(());
}

/// Failures with running a command with strace, watching touched filesystem.
enum StraceFilesError {
    IO(std::io::Error),
    ExitStatus,
    DecodeUtf8(std::string::FromUtf8Error),
}
impl From<std::io::Error> for StraceFilesError {
    fn from(err: std::io::Error) -> Self {
        return Self::IO(err);
    }
}
impl From<std::string::FromUtf8Error> for StraceFilesError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        return Self::DecodeUtf8(err);
    }
}

/// Run a given command with strace, watching touched filesystem.
fn run_with_strace(
    cmd: &str,
    argv: Vec<&str>,
    cwd: &std::path::PathBuf,
) -> Result<Vec<String>, StraceFilesError> {
    let strace_argv = vec![vec!["-ff", "-e", "trace=file", cmd], argv].concat();
    let out: std::process::Output = std::process::Command::new(CMD_STRACE)
        .current_dir(cwd)
        .args(strace_argv)
        .output()?;
    if !out.status.success() {
        return Err(StraceFilesError::ExitStatus);
    }
    let stderr: String = String::from_utf8(out.stderr)?;
    let paths: std::collections::HashSet<String> = extract_modified_paths(&stderr);
    return Ok(paths.into_iter().collect());
}

/// Install _SteamCMD_ (game server installer).
pub fn install_steamcmd(
    url: &String,
    rustctl_root_dir: &std::path::PathBuf,
    steamcmd_tgz_filename: &std::path::PathBuf,
    steamcmd_executable_filename: &std::path::PathBuf,
) -> Result<(), crate::error::FatalError> {
    let mut steamcmd_tgz_absolute: PathBuf = rustctl_root_dir.clone();
    steamcmd_tgz_absolute.push(steamcmd_tgz_filename);

    if steamcmd_tgz_absolute.is_file() {
        log::debug!(
            "SteamCMD distribution '{}' has been downloaded earlier -- Not downloading again",
            steamcmd_tgz_absolute.to_string_lossy()
        );
    } else {
        let response: reqwest::blocking::Response = match reqwest::blocking::get(url) {
            Ok(n) => n,
            Err(err) => {
                return Err(crate::error::FatalError::new(
                    format!(
                        "cannot install SteamCMD: cannot fetch distribution from '{}'",
                        url
                    ),
                    Some(Box::new(err)),
                ));
            }
        };
        let mut file: std::fs::File = match std::fs::File::create(&steamcmd_tgz_absolute) {
            Ok(n) => n,
            Err(err) => {
                return Err(crate::error::FatalError::new(
                    format!(
                        "cannot install SteamCMD: cannot create file '{}'",
                        steamcmd_tgz_absolute.to_string_lossy()
                    ),
                    Some(Box::new(err)),
                ));
            }
        };
        let mut reader = std::io::BufReader::new(response);
        // stream to disk
        match std::io::copy(&mut reader, &mut file) {
            Err(err) => {
                return Err(crate::error::FatalError::new(
                    format!(
                        "cannot install SteamCMD: cannot write response from '{}' to '{}'",
                        url,
                        steamcmd_tgz_absolute.to_string_lossy()
                    ),
                    Some(Box::new(err)),
                ));
            }
            _ => {}
        }
        log::info!("Downloaded SteamCMD from {}", url);
    }

    let mut steamcmd_executable_absolute: std::path::PathBuf = rustctl_root_dir.clone();
    steamcmd_executable_absolute.push(steamcmd_executable_filename);

    if steamcmd_executable_absolute.is_file() {
        log::debug!(
            "SteamCMD executable '{}' has been extracted earlier -- Not extracting again",
            steamcmd_executable_absolute.to_string_lossy()
        );
    } else {
        let cmd_tar: &str = "tar";
        let mut paths_touched: Vec<String> = match run_with_strace(
            cmd_tar,
            vec!["-xzf", &steamcmd_tgz_filename.to_string_lossy()],
            rustctl_root_dir,
        ) {
            Ok(n) => n,
            Err(StraceFilesError::DecodeUtf8(err)) => {
                return Err(crate::error::FatalError::new(
                    format!(
                        "cannot install SteamCMD: cannot decode output of '{CMD_STRACE}' with '{cmd_tar}' as UTF-8",
                    ),
                    Some(Box::new(err)),
                ))
            }
            Err(StraceFilesError::ExitStatus) => {
                return Err(crate::error::FatalError::new(format!("cannot install SteamCMD: '{CMD_STRACE}' with '{cmd_tar}' exited with unsuccessful status"), None))
            }
            Err(StraceFilesError::IO(err)) => {
                return Err(crate::error::FatalError::new(
                    format!(
                        "cannot install SteamCMD: cannot execute '{CMD_STRACE}' with '{cmd_tar}'",
                    ),
                    Some(Box::new(err)),
                ))
            }
        };
        paths_touched.sort();

        log::info!(
            "Extracted {} files from SteamCMD distribution '{}': {}",
            paths_touched.len(),
            steamcmd_tgz_absolute.to_string_lossy(),
            paths_touched.join(", ")
        );
    }

    if !steamcmd_executable_absolute.is_file() {
        return Err(crate::error::FatalError::new(
            format!(
                "unexpected distribution of SteamCMD: did not contain file '{}' ('{}')",
                steamcmd_executable_filename.to_string_lossy(),
                steamcmd_executable_absolute.to_string_lossy(),
            ),
            None,
        ));
    }

    return Ok(());
}

/// Extract filesystem paths from `strace` output that were modified (created, written to etc.)
fn extract_modified_paths(strace_output: &str) -> std::collections::HashSet<String> {
    let mut modified_paths: std::collections::HashSet<String> = std::collections::HashSet::new();
    for line in strace_output.lines() {
        let mut_mode: bool = line.contains("O_WRONLY")
            || line.contains("O_RDWR")
            || line.contains("O_CREAT")
            || line.contains("O_TRUNC");

        if (line.contains("open") && mut_mode)
            || (line.contains("openat") && mut_mode)
            || line.contains("chmod")
            || line.contains("pwrite")
            || line.contains("rename")
            || line.contains("unlink")
            || line.contains("write")
        {
            if let Some(path) = extract_quoted_substring(line) {
                modified_paths.insert(path);
            }
        }
    }
    return modified_paths;
}

fn extract_quoted_substring(input: &str) -> Option<String> {
    let mut last_quoted_substring: Option<String> = None;
    let mut in_quotes: bool = false;
    let mut start: usize = 0;

    for (i, c) in input.char_indices() {
        match c {
            '\"' => {
                if in_quotes {
                    // if closing a quote, capture the substring
                    if start < i {
                        last_quoted_substring = Some(input[start..i].to_string());
                    }
                    in_quotes = false; // close the quote
                } else {
                    // if opening a quote, mark the start
                    in_quotes = true;
                    start = i + 1; // move start past the quote
                }
            }
            _ => {}
        }
    }

    return last_quoted_substring;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_modified_paths() {
        let strace_output = r#"
        [pid 30158] openat(AT_FDCWD, "steamcmd.sh", O_WRONLY|O_CREAT|O_EXCL|O_NOCTTY|O_NONBLOCK|O_CLOEXEC, 0764) = 4
        [pid 30159] openat(AT_FDCWD, "steamcmd.tgz", O_RDONLY) = 3
        [pid 30167] chmod("/tmp/dumps", 0777)   = 0
        [pid 30208] access("/home/rust/installations/RustDedicated_Data", F_OK) = -1 ENOENT (No such file or directory)
        [pid 30208] rename("/home/rust/installations/steamapps/downloading/258550/RustDedicated", "/home/rust/installations/RustDedicated") = 0
        [pid 30209] unlink("/home/rust/installations/steamapps/downloading/state_258550_258552.patch") = 0
        "#;
        let modified_paths = extract_modified_paths(strace_output);

        // renamed (from)
        assert_eq!(modified_paths.contains("/home/rust/installations/steamapps/downloading/258550/RustDedicated"), false);
        // renamed (to)
        assert!(modified_paths.contains("/home/rust/installations/RustDedicated"));

        // modified
        assert!(modified_paths.contains("steamcmd.sh"));
        assert!(modified_paths.contains("/tmp/dumps"));

        // removed
        assert!(modified_paths.contains("/home/rust/installations/steamapps/downloading/state_258550_258552.patch"));

        // only accessed
        assert_eq!(
            modified_paths.contains("/home/rust/installations/RustDedicated_Data"),
            false
        );

        // opened in non modifying mode
        assert_eq!(modified_paths.contains("steamcmd.tgz"), false);
    }
}
