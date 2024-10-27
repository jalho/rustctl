//! Dumpster for miscellaneous stuff yet to be better categorized.

use log::{debug, error, info};

const CMD_STRACE: &str = "strace";
const ENV_LD_LIBRARY_PATH: &str = "LD_LIBRARY_PATH";

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

pub fn start_game(
    tx_stdout: std::sync::mpsc::Sender<String>,
    tx_stderr: std::sync::mpsc::Sender<String>,
    config: &crate::args::Config,
) -> Result<(std::thread::JoinHandle<()>, std::thread::JoinHandle<()>), crate::error::FatalError> {
    let startup_with_argv: String = format!(
        "source {} && {} {}",
        &config.carbon_executable,
        &config.game_executable,
        vec![
            "-batchmode",
            "+server.identity",
            "instance0",
            "+server.port",
            "28015",
            "+rcon.port",
            "28016",
            "+rcon.web",
            "1",
            "+rcon.password",
            "Your_Rcon_Password",
            "+server.worldsize",
            "1000",
            "+server.seed",
            "1234",
            "+server.maxplayers",
            "10",
            "+server.hostname",
            "0.0.0.0",
        ]
        .join(" ")
    );
    let argv = vec!["-ff", "-e", "trace=file", "bash", "-c", &startup_with_argv];

    let libs_paths_prev: String = match std::env::var(ENV_LD_LIBRARY_PATH) {
        Ok(n) => n,
        Err(_) => String::from(""),
    };

    let mut child: std::process::Child = match std::process::Command::new(CMD_STRACE)
        .current_dir(&config.steamcmd_installations.path)
        .args(argv)
        .env(
            ENV_LD_LIBRARY_PATH,
            format!("{libs_paths_prev}:{}", &config.steamcmd_libs),
        )
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(n) => n,
        Err(err) => {
            return Err(crate::error::FatalError::new(
                format!("cannot launch game with {CMD_STRACE}: cannot spawn"),
                Some(Box::new(err)),
            ));
        }
    };
    let stdout: std::process::ChildStdout = match child.stdout.take() {
        Some(n) => n,
        None => {
            return Err(crate::error::FatalError::new(
                format!("cannot launch game with {CMD_STRACE}: cannot get handle for STDOUT"),
                None,
            ));
        }
    };
    let stderr: std::process::ChildStderr = match child.stderr.take() {
        Some(n) => n,
        None => {
            return Err(crate::error::FatalError::new(
                format!("cannot launch game with {CMD_STRACE}: cannot get handle for STDERR"),
                None,
            ));
        }
    };

    let th_stdout = std::thread::spawn(move || {
        let reader = std::io::BufReader::new(stdout);
        use std::io::BufRead;
        for line in reader.lines() {
            let line = match line {
                Ok(n) => n,
                Err(err) => {
                    error!("Cannot read game server STDOUT: {:#?}", err);
                    continue;
                }
            };
            match tx_stdout.send(line) {
                Err(err) => {
                    error!("Cannot send game server STDOUT: {:#?}", err);
                    return;
                }
                _ => {}
            }
        }
    });
    let th_stderr = std::thread::spawn(move || {
        let reader = std::io::BufReader::new(stderr);
        use std::io::BufRead;
        for line in reader.lines() {
            let line = match line {
                Ok(n) => n,
                Err(err) => {
                    error!("Cannot read game server STDERR: {:#?}", err);
                    continue;
                }
            };
            match tx_stderr.send(line) {
                Err(err) => {
                    error!("Cannot send game server STDERR: {:#?}", err);
                    return;
                }
                _ => {}
            }
        }
    });

    return Ok((th_stdout, th_stderr));
}

pub fn handle_game_fs_events(
    rx_stdout: std::sync::mpsc::Receiver<String>,
    rx_stderr: std::sync::mpsc::Receiver<String>,
    config: &crate::args::Config,
) -> (std::thread::JoinHandle<()>, std::thread::JoinHandle<()>) {
    let th_stdout = std::thread::spawn(move || loop {
        let msg: String = match rx_stdout.recv() {
            Ok(n) => n,
            Err(err) => {
                error!("Cannot receive game server STDOUT: {:#?}", err);
                return;
            }
        };
        info!("{msg}");
    });

    let game_server_cwd: std::path::PathBuf = config.steamcmd_installations.path.clone();
    let th_stderr = std::thread::spawn(move || loop {
        let msg: String = match rx_stderr.recv() {
            Ok(n) => n,
            Err(err) => {
                error!("Cannot receive game server STDERR: {:#?}", err);
                return;
            }
        };
        let paths_touched: std::collections::HashSet<String> =
            extract_modified_paths(&msg, &game_server_cwd);
        if paths_touched.len() > 0 {
            if let Some(game_server_file_touched) = paths_touched.into_iter().next() {
                // the game server attempts to do a bunch of openat(AT_FDCWD, "/sys/kernel/**/trace_marker", O_WRONLY)
                if game_server_file_touched == "/sys/kernel/tracing/trace_marker"
                    || game_server_file_touched == "/sys/kernel/debug/tracing/trace_marker"
                {
                    continue;
                }
                debug!("{msg}");
            }
        }
    });
    return (th_stdout, th_stderr);
}

/// Install or update an existing installation of the game server.
pub fn install_update_game_server(
    config: &crate::args::Config,
) -> Result<(), crate::error::FatalError> {
    let steamcmd_executable_absolute = &config.steamcmd_executable;

    /* Game server installation location must be different than where the installer is for some reason... */
    if !&config.steamcmd_installations.path.is_dir() {
        match std::fs::create_dir(&config.steamcmd_installations.path) {
            Ok(_) => {}
            Err(err) => {
                return Err(crate::error::FatalError::new(
                    format!("cannot install or update game server: cannot create installation directory '{}'",
                        config.steamcmd_installations),
                    Some(Box::new(err))),
                );
            }
        }
    }

    // only update & validate against remote if not checked recently
    let manifest_modified: Option<std::time::SystemTime> = match &config
        .game_manifest
        .path
        .metadata()
    {
        Ok(n) => {
            match n.modified() {
                Ok(n) => Some(n),
                Err(err) => {
                    return Err(crate::error::FatalError::new(
                        format!("cannot determine last modification time of game server app manifest '{}'", &config.game_manifest),
                        Some(Box::new(err)),
                    ));
                }
            }
        }
        _ => {
            // case fresh install
            None
        }
    };
    if let Some(n) = manifest_modified {
        let now: std::time::SystemTime = std::time::SystemTime::now();
        match now.duration_since(n) {
            Ok(manifest_age) => {
                let cooldown = std::time::Duration::from_secs(60 * 15);
                if manifest_age < cooldown {
                    info!("Game server seems to have been updated recently: App manifest '{}' was last modified {} seconds ago, cooldown being {} seconds -- Not updating again!",
                          &config.game_manifest, manifest_age.as_secs(), cooldown.as_secs());
                    return Ok(());
                } else {
                    debug!("Game server app manifest '{}' was last modified {} seconds ago -- Update cooldown is {} seconds",
                           &config.game_manifest, manifest_age.as_secs(), cooldown.as_secs());
                }
            }
            Err(err) => {
                return Err(crate::error::FatalError::new(
                    format!("cannot determine time since last modification of game server app manifest '{}'", &config.game_manifest),
                    Some(Box::new(err)),
                ));
            }
        }
    }

    info!(
        "Installing or updating game server with SteamCMD to '{}'",
        &config.steamcmd_installations
    );
    let paths_touched: Vec<(String, u64)> = match run_with_strace(
        &format!("{}", steamcmd_executable_absolute),
        vec![
            "+force_install_dir",
            &format!("{}", config.steamcmd_installations),
            "+login",
            "anonymous",
            "+app_update",
            "258550",
            "validate",
            "+quit",
        ],
        &config.root_dir.path,
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
    let paths_touched_subset = paths_touched.iter().take(10);

    log::info!(
        "Installed or updated {} game server files with SteamCMD: Biggest {}: {}",
        paths_touched.len(),
        paths_touched_subset.len(),
        paths_touched_subset
            .into_iter()
            .cloned()
            .map(|(path, size)| format!("{} bytes: {}", human_readable_size(size), path))
            .collect::<Vec<String>>()
            .join(", ")
    );

    if !&config.game_executable.path.is_file() {
        return Err(crate::error::FatalError::new(
            format!(
                "unexpected game server installation: did not contain file '{}'",
                &config.game_executable,
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
) -> Result<Vec<(String, u64)>, StraceFilesError> {
    let strace_argv = vec![vec!["-ff", "-e", "trace=file", cmd], argv].concat();
    let out: std::process::Output = std::process::Command::new(CMD_STRACE)
        .current_dir(cwd)
        .args(strace_argv)
        .output()?;
    if !out.status.success() {
        return Err(StraceFilesError::ExitStatus);
    }
    let stderr: String = String::from_utf8(out.stderr)?;

    let paths: std::collections::HashSet<String> = extract_modified_paths(&stderr, &cwd);
    let paths: Vec<(String, u64)> = get_sizes(paths);

    return Ok(paths);
}

/// Install _SteamCMD_ (game server installer).
pub fn install_steamcmd(config: &crate::args::Config) -> Result<(), crate::error::FatalError> {
    if config.steamcmd_archive.path.is_file() {
        log::debug!(
            "SteamCMD distribution '{}' has been downloaded earlier -- Not downloading again",
            &config.steamcmd_archive
        );
    } else {
        let response: reqwest::blocking::Response =
            match reqwest::blocking::get(&config.steamcmd_download) {
                Ok(n) => n,
                Err(err) => {
                    return Err(crate::error::FatalError::new(
                        format!(
                            "cannot install SteamCMD: cannot fetch distribution from '{}'",
                            &config.steamcmd_download
                        ),
                        Some(Box::new(err)),
                    ));
                }
            };
        let mut file: std::fs::File = match std::fs::File::create(&config.steamcmd_archive.path) {
            Ok(n) => n,
            Err(err) => {
                return Err(crate::error::FatalError::new(
                    format!(
                        "cannot install SteamCMD: cannot create file '{}'",
                        &config.steamcmd_archive
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
                        &config.steamcmd_download, &config.steamcmd_archive
                    ),
                    Some(Box::new(err)),
                ));
            }
            _ => {}
        }
        log::info!("Downloaded SteamCMD from {}", &config.steamcmd_download);
    }

    let cmd_tar: &str = "tar";
    let paths_touched: Vec<(String, u64)> = match run_with_strace(
        cmd_tar,
        vec!["-xzf", &format!("{}", &config.steamcmd_archive)],
        &config.steamcmd_archive.parent(),
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

    let paths_touched_subset = paths_touched.iter().take(10);

    log::info!(
        "Extracted {} files from SteamCMD distribution '{}': Biggest {}: {}",
        paths_touched.len(),
        &config.steamcmd_archive,
        paths_touched_subset.len(),
        paths_touched_subset
            .into_iter()
            .cloned()
            .map(|(path, size)| format!("{} bytes: {}", human_readable_size(size), path))
            .collect::<Vec<String>>()
            .join(", ")
    );

    if !&config.steamcmd_executable.path.is_file() {
        return Err(crate::error::FatalError::new(
            format!(
                "unexpected distribution of SteamCMD: did not contain file '{}'",
                &config.steamcmd_executable,
            ),
            None,
        ));
    }

    return Ok(());
}

/// Install _Carbon_ (game modding framework).
pub fn install_carbon(config: &crate::args::Config) -> Result<(), crate::error::FatalError> {
    if config.carbon_archive.path.is_file() {
        log::debug!(
            "Carbon distribution '{}' has been downloaded earlier -- Not downloading again",
            &config.carbon_archive
        );
    } else {
        let response: reqwest::blocking::Response =
            match reqwest::blocking::get(&config.carbon_download) {
                Ok(n) => n,
                Err(err) => {
                    return Err(crate::error::FatalError::new(
                        format!(
                            "cannot install Carbon: cannot fetch distribution from '{}'",
                            config.carbon_download
                        ),
                        Some(Box::new(err)),
                    ));
                }
            };
        let mut file: std::fs::File = match std::fs::File::create(&config.carbon_archive.path) {
            Ok(n) => n,
            Err(err) => {
                return Err(crate::error::FatalError::new(
                    format!(
                        "cannot install Carbon: cannot create file '{}'",
                        &config.carbon_archive
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
                        "cannot install Carbon: cannot write response from '{}' to '{}'",
                        &config.carbon_download, &config.carbon_archive
                    ),
                    Some(Box::new(err)),
                ));
            }
            _ => {}
        }
        log::info!("Downloaded Carbon from {}", &config.carbon_download);
    }

    let cmd_tar: &str = "tar";
    let paths_touched: Vec<(String, u64)> = match run_with_strace(
        cmd_tar,
        vec!["-xzf", &format!("{}", &config.carbon_archive)],
        &config.carbon_archive.parent(),
    ) {
        Ok(n) => n,
        Err(StraceFilesError::DecodeUtf8(err)) => {
            return Err(crate::error::FatalError::new(
                format!(
                    "cannot install Carbon: cannot decode output of '{CMD_STRACE}' with '{cmd_tar}' as UTF-8",
                ),
                Some(Box::new(err)),
            ))
        }
        Err(StraceFilesError::ExitStatus) => {
            return Err(crate::error::FatalError::new(format!("cannot install Carbon: '{CMD_STRACE}' with '{cmd_tar}' exited with unsuccessful status"), None))
        }
        Err(StraceFilesError::IO(err)) => {
            return Err(crate::error::FatalError::new(
                format!(
                    "cannot install Carbon: cannot execute '{CMD_STRACE}' with '{cmd_tar}'",
                ),
                Some(Box::new(err)),
            ))
        }
    };

    let paths_touched_subset = paths_touched.iter().take(10);

    log::info!(
        "Extracted {} files from Carbon distribution '{}': Biggest {}: {}",
        paths_touched.len(),
        &config.carbon_archive,
        paths_touched_subset.len(),
        paths_touched_subset
            .into_iter()
            .cloned()
            .map(|(path, size)| format!("{} bytes: {}", human_readable_size(size), path))
            .collect::<Vec<String>>()
            .join(", ")
    );

    if !config.carbon_executable.path.is_file() {
        return Err(crate::error::FatalError::new(
            format!(
                "unexpected distribution of Carbon: did not contain file '{}'",
                &config.carbon_executable,
            ),
            None,
        ));
    }

    /*
      TODO: Fix Carbon config manipulation: It seems the config file doesn't
      exist in the downloaded archive and cannot be created with just
      `IsModded: false` because it get overwritten at startup. Maybe the
      value can be changed via some API after startup?

      Consider this command that can be issued via WebSocket RCON:
      `c.gocommunity`
      docs: https://docs.carbonmod.gg/docs/core/commands#c.gocommunity
      [Accessed 2024-10-27]
    */
    if !&config.carbon_config.path.is_file() {
        return Err(crate::error::FatalError::new(
            format!(
                "unexpected distribution of Carbon: did not contain file '{}'",
                &config.carbon_config,
            ),
            None,
        ));
    }
    configure_carbon(&config.carbon_config.path)?;
    info!(
        "Configured Carbon: Set 'IsModded' to false in '{}'",
        &config.carbon_config
    );

    return Ok(());
}

fn configure_carbon(
    config_path_absolute: &std::path::PathBuf,
) -> Result<(), crate::error::FatalError> {
    let json_content: String = match std::fs::read_to_string(&config_path_absolute) {
        Ok(n) => n,
        Err(err) => {
            return Err(crate::error::FatalError::new(
                format!(
                    "cannot configure Carbon: cannot read config file '{}'",
                    config_path_absolute.to_string_lossy()
                ),
                Some(Box::new(err)),
            ));
        }
    };
    let mut json_data: serde_json::Value = match serde_json::from_str(&json_content) {
        Ok(n) => n,
        Err(err) => {
            return Err(crate::error::FatalError::new(
                format!(
                    "cannot configure Carbon: cannot deserialize JSON config file '{}'",
                    config_path_absolute.to_string_lossy()
                ),
                Some(Box::new(err)),
            ));
        }
    };
    if let Some(ismod_value) = json_data.get_mut("IsModded") {
        if ismod_value == true {
            *ismod_value = serde_json::json!(false);
        }
    }
    let new_json_content: String = match serde_json::to_string_pretty(&json_data) {
        Ok(n) => n,
        Err(_) => {
            // we just deserialized succesfully so surely we can serialize
            unreachable!()
        }
    };
    let mut file = match std::fs::File::create(&config_path_absolute) {
        Ok(n) => n,
        Err(err) => {
            return Err(crate::error::FatalError::new(
                format!(
                    "cannot configure Carbon: cannot open config file in write mode: '{}'",
                    config_path_absolute.to_string_lossy()
                ),
                Some(Box::new(err)),
            ));
        }
    };
    match std::io::Write::write_all(&mut file, new_json_content.as_bytes()) {
        Err(err) => {
            return Err(crate::error::FatalError::new(
                format!(
                    "cannot configure Carbon: cannot write config file '{}'",
                    config_path_absolute.to_string_lossy()
                ),
                Some(Box::new(err)),
            ));
        }
        _ => {}
    }
    return Ok(());
}

fn human_readable_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size: f64 = bytes as f64;
    let mut unit: usize = 0;

    while size >= 1000.0 && unit < UNITS.len() - 1 {
        size /= 1000.0;
        unit += 1;
    }

    return format!("{:.2} {}", size, UNITS[unit]);
}

/// Extract filesystem paths from `strace` output that were modified (created, written to etc.)
fn extract_modified_paths(
    strace_output: &str,
    cwd: &std::path::PathBuf,
) -> std::collections::HashSet<String> {
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
                let file_path = std::path::Path::new(&path);
                let file_path_absolute: String;
                if file_path.is_absolute() {
                    file_path_absolute = path;
                } else {
                    file_path_absolute = cwd.join(file_path).to_string_lossy().to_string();
                }
                modified_paths.insert(file_path_absolute);
            }
        }
    }
    return modified_paths;
}

fn get_sizes(paths: std::collections::HashSet<String>) -> Vec<(String, u64)> {
    let mut paths_with_sizes: Vec<(String, u64)> = vec![];
    for modified_path in &paths {
        if let Ok(metadata) = std::fs::metadata(modified_path) {
            paths_with_sizes.push((modified_path.to_string(), metadata.len()));
        }
    }
    paths_with_sizes.sort_by(|a, b| b.1.cmp(&a.1));
    return paths_with_sizes;
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
        let cwd = std::path::Path::new("/home/rust");
        let strace_output = r#"
        [pid 30158] openat(AT_FDCWD, "steamcmd.sh", O_WRONLY|O_CREAT|O_EXCL|O_NOCTTY|O_NONBLOCK|O_CLOEXEC, 0764) = 4
        [pid 30159] openat(AT_FDCWD, "steamcmd.tgz", O_RDONLY) = 3
        [pid 30159] openat(AT_FDCWD, "/some/absolute/path.txt", O_RDONLY) = 3
        [pid 30167] chmod("/tmp/dumps", 0777)   = 0
        [pid 30208] access("/home/rust/installations/RustDedicated_Data", F_OK) = -1 ENOENT (No such file or directory)
        [pid 30208] rename("/home/rust/installations/steamapps/downloading/258550/RustDedicated", "/home/rust/installations/RustDedicated") = 0
        [pid 30209] unlink("/home/rust/installations/steamapps/downloading/state_258550_258552.patch") = 0
        "#;
        let modified_paths = extract_modified_paths(strace_output, &cwd.to_path_buf());

        // renamed (from)
        assert_eq!(
            modified_paths
                .contains("/home/rust/installations/steamapps/downloading/258550/RustDedicated"),
            false
        );
        // renamed (to)
        assert!(modified_paths.contains("/home/rust/installations/RustDedicated"));

        // modified
        assert!(modified_paths.contains("/home/rust/steamcmd.sh"));
        assert!(modified_paths.contains("/tmp/dumps"));

        // removed
        assert!(modified_paths
            .contains("/home/rust/installations/steamapps/downloading/state_258550_258552.patch"));

        // only accessed
        assert_eq!(
            modified_paths.contains("/home/rust/installations/RustDedicated_Data"),
            false
        );

        // opened in non modifying mode
        assert_eq!(modified_paths.contains("steamcmd.tgz"), false);
        assert_eq!(modified_paths.contains("/home/rust/steamcmd.tgz"), false);
        assert_eq!(modified_paths.contains("/some/absolute/path.txt"), false);
    }
}
