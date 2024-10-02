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
    let paths_touched: Vec<String> = match run_with_strace(
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

    /* TODO: Fix strace with SteamCMD... Gotta dive some levels of indirection? Sample log:
    ```
    [2024-10-02T21:56:54.929] [INFO] - Installing or updating game server with SteamCMD to '/home/rust/installations'
    [2024-10-02T22:01:03.724] [INFO] - Installed or updated 1 game server files with SteamCMD: /dev/tty
    ```
    Expecting more like dozens or hundreds of touched paths, `./installations/RustDedicated` among them... */
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
    let strace_argv = vec![vec!["-e", "trace=file", cmd], argv].concat();
    let out: std::process::Output = std::process::Command::new(CMD_STRACE)
        .current_dir(cwd)
        .args(strace_argv)
        .output()?;
    if !out.status.success() {
        return Err(StraceFilesError::ExitStatus);
    }
    let stderr = String::from_utf8(out.stderr)?;
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
        let paths_touched: Vec<String> = match run_with_strace(
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
        if line.contains("openat") || line.contains("open") {
            if line.contains("O_WRONLY")
                || line.contains("O_RDWR")
                || line.contains("O_CREAT")
                || line.contains("O_TRUNC")
            {
                if let Some(path) = extract_path_from_syscall(line) {
                    modified_paths.insert(path);
                }
            }
        } else if line.contains("write")
            || line.contains("pwrite")
            || line.contains("unlink")
            || line.contains("chmod")
            || line.contains("utimensat")
        {
            if let Some(path) = extract_path_from_syscall(line) {
                modified_paths.insert(path);
            }
        }
    }
    return modified_paths;
}

/// Extract file path from an `strace` output syscall line.
fn extract_path_from_syscall(line: &str) -> Option<String> {
    // find the first occurrence of a string that looks like a file path (i.e. a quoted string)
    if let Some(start) = line.find('\"') {
        if let Some(end) = line[start + 1..].find('\"') {
            return Some(line[start + 1..start + 1 + end].to_string());
        }
    }
    return None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_modified_paths() {
        let strace_output = r#"
        openat(AT_FDCWD, "/etc/passwd", O_RDONLY|O_CLOEXEC) = 3
        openat(AT_FDCWD, "dummy-steamcmd.txt", O_WRONLY|O_CREAT|O_EXCL|O_NOCTTY|O_NONBLOCK|O_CLOEXEC, 0644) = 4
        write(4, "some data", 9) = 9
        utimensat(4, NULL, [UTIME_OMIT, {tv_sec=1727534497, tv_nsec=0}], 0) = 0
        openat(AT_FDCWD, "/etc/group", O_RDONLY|O_CLOEXEC) = 3
        unlink("old-file.txt") = 0
        "#;
        let modified_paths = extract_modified_paths(strace_output);
        assert!(modified_paths.contains("/etc/passwd") == false); // not modified
        assert!(modified_paths.contains("dummy-steamcmd.txt")); // created/modified
        assert!(modified_paths.contains("old-file.txt")); // deleted
    }
}
