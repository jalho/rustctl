//! Dumpster for miscellaneous stuff yet to be better categorized.

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
) -> Result<
    (
        libc::pid_t,
        std::thread::JoinHandle<()>,
        std::thread::JoinHandle<()>,
    ),
    crate::error::FatalError,
> {
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
            &config.rcon_port.to_string(),
            "+rcon.web",
            "1",
            "+rcon.password",
            &config.rcon_password,
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

    #[allow(deprecated)]
    let mut child: std::process::Child = match std::os::unix::process::CommandExt::before_exec(
        &mut std::process::Command::new(CMD_STRACE),
        || {
            /* We need a to set a dedicated PID & PGID in order to be able to control
            termination of both the child 'strace' and (grand)child 'RustDedicated' (the
            game server). */
            let pid_for_game = match get_free_pid() {
                Ok(n) => n,
                Err(err) => {
                    log::error!("cannot launch game: cannot determine a free PID");
                    return Err(err);
                }
            };
            unsafe { libc::setpgid(pid_for_game, pid_for_game) };
            return Ok(());
        },
    )
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
                    log::error!("Cannot read game server STDOUT: {:#?}", err);
                    continue;
                }
            };
            match tx_stdout.send(line) {
                Err(err) => {
                    log::error!("Cannot send game server STDOUT: {:#?}", err);
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
                    log::error!("Cannot read game server STDERR: {:#?}", err);
                    continue;
                }
            };
            match tx_stderr.send(line) {
                Err(err) => {
                    log::error!("Cannot send game server STDERR: {:#?}", err);
                    return;
                }
                _ => {}
            }
        }
    });

    let child_pgid: libc::pid_t = unsafe { libc::getpgid(child.id() as libc::pid_t) };

    return Ok((child_pgid, th_stdout, th_stderr));
}

fn get_free_pid() -> Result<i32, std::io::Error> {
    let mut insomniac: std::process::Child =
        std::process::Command::new("sleep").arg("0").spawn()?;
    let pid = insomniac.id() as i32;
    let _ = insomniac.wait();
    return Ok(pid);
}

pub enum GameServerState {
    Playable,
}

/// Handle game server's emitted log lines (STDOUT) and the wrapping strace's
/// filesystem detected events (STDERR).
pub fn handle_game_server_fs_events(
    config: &crate::args::Config,
    rx_stdout: std::sync::mpsc::Receiver<String>,
    rx_stderr: std::sync::mpsc::Receiver<String>,
    tx_game_server_state: std::sync::mpsc::Sender<GameServerState>,
) -> (std::thread::JoinHandle<()>, std::thread::JoinHandle<()>) {
    let log_level: crate::args::LogLevel = config.log_level.clone();
    let th_stdout = std::thread::spawn(move || loop {
        let msg: String = match rx_stdout.recv() {
            Ok(n) => n,
            Err(err) => {
                log::error!("Cannot receive game server STDOUT: {:#?}", err);
                return;
            }
        };
        match log_level {
            crate::args::LogLevel::normal => {}
            crate::args::LogLevel::all => {
                log::info!("{msg}");
            }
        }
        if msg == "Server startup complete" {
            match tx_game_server_state.send(GameServerState::Playable) {
                Err(err) => {
                    log::error!("Cannot send game server state across threads: {:#?}", err);
                    return;
                }
                _ => {}
            }
        }
    });

    let log_level: crate::args::LogLevel = config.log_level.clone();
    let cwd: String = config.root_dir.to_string();
    let th_stderr = std::thread::spawn(move || {
        let cwd: &str = &cwd;
        loop {
            let msg: String = match rx_stderr.recv() {
                Ok(n) => n,
                Err(err) => {
                    log::error!("Cannot receive game server STDERR: {:#?}", err);
                    return;
                }
            };
            match log_level {
                crate::args::LogLevel::normal => {
                    if let Some(strace_output) = parse_syscall_and_string_args(&msg) {
                        // TODO: Make the `is_fs_edit` logic more robust: Only make
                        //       log of really changed files, perhaps only in some
                        //       select paths, and ignore some likely uninteresting
                        //       spam? An example of a somewhat sensible filter
                        //       for "normal" log level as of commit `e1b5913` and
                        //       latest deps as of 2024-11-16 (specific to my machine):
                        //       $ grep -vE "faccessat2|/sys/kernel/|/home/jka/\.steam/|carbon.*\.log|inotify_add_watch|/home/jka/.config|/home/rust/installations/carbon/managed/.*\.dll|/tmp/|statx|/dev/|/home/rust/installations/RustDedicated_Data/Managed/.*\.dll|/home/rust/installations/carbon/temp/"
                        if is_fs_edit(&strace_output) && in_scope(cwd, &strace_output) {
                            log::info!(
                                "{} {}",
                                strace_output.syscall_name,
                                strace_output.argv_strings.join(" ")
                            );
                        }
                    }
                }
                crate::args::LogLevel::all => {
                    log::debug!("{msg}\n{:#?}", parse_syscall_and_string_args(&msg));
                }
            }
        }
    });
    return (th_stdout, th_stderr);
}

/// Determine whether a given strace operation concerns something in scope of
/// managing the game server, i.e. is at least not outside of the configured
/// root dir where every command is expected to take place. Not exhaustive check
/// but trims away all kinds of debug and temp paths like `/sys/kernel/`, `/tmp/`
/// etc.
fn in_scope(root_dir: &str, operation: &StraceLine) -> bool {
    for str_arg in &operation.argv_strings {
        // cba with cross platform -- this is a very Linux specific implementation anyway
        if str_arg.starts_with(&root_dir) || !str_arg.starts_with("/") {
            return true;
        }
    }
    return false;
}

/// Determine whether a given parsed output line of strace represents an
/// operation that caused changes in the filesystem, such as files written or
/// removed etc.
fn is_fs_edit(operation: &StraceLine) -> bool {
    // read-only operations
    if operation.syscall_name == "openat" && operation.constants.contains(&String::from("O_RDONLY"))
    {
        return false;
    }

    // filesystem checks
    if operation.syscall_name == "newfstatat"
        || operation.syscall_name == "stat"
        || operation.syscall_name == "statx"
        || operation.syscall_name == "lstat"
        || operation.syscall_name == "readlink"
        || operation.syscall_name == "access"
        || operation.syscall_name == "faccessat2"
    {
        return false;
    }

    // other
    if operation.syscall_name == "getcwd"
        || operation.syscall_name == "chdir"
        || operation.syscall_name == "inotify_add_watch"
    {
        return false;
    }

    return true;
}

/// Install or update an existing installation of the game server.
pub fn install_update_game_server(
    config: &crate::args::Config,
) -> Result<(), crate::error::FatalError> {
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
                if &manifest_age < &config.game_startup_update_cooldown {
                    log::info!("Game server seems to have been updated recently: App manifest '{}' was last modified {} seconds ago, cooldown being {} seconds -- Not updating again!",
                          &config.game_manifest, manifest_age.as_secs(), &config.game_startup_update_cooldown.as_secs());
                    return Ok(());
                } else {
                    log::debug!("Game server app manifest '{}' was last modified {} seconds ago -- Update cooldown is {} seconds",
                           &config.game_manifest, manifest_age.as_secs(), &config.game_startup_update_cooldown.as_secs());
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

    log::info!(
        "Installing or updating game server with SteamCMD to '{}'",
        &config.steamcmd_installations
    );
    let steamcmd_executable: String = config.steamcmd_executable.to_string();
    let steamcmd_installations_dir: String = config.steamcmd_installations.to_string();
    let mut cmd_steamcmd = crate::proc::Command::strace(
        &config.root_dir.path,
        vec![
            &steamcmd_executable,
            "+force_install_dir",
            &steamcmd_installations_dir,
            "+login",
            "anonymous",
            "+app_update",
            "258550",
            "validate",
            "+quit",
        ],
    );
    let paths_touched: Vec<(String, u64)> = cmd_steamcmd.run_to_end()?;
    let paths_touched_subset = paths_touched.iter().take(10);
    log::info!(
        "Installed or updated {} game server files with SteamCMD: Biggest {}: {}",
        paths_touched.len(),
        paths_touched_subset.len(),
        paths_touched_subset
            .into_iter()
            .cloned()
            .map(|(path, size)| format!("{}: {}", human_readable_size(size), path))
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

    let steamcmd_extractable: String = config.steamcmd_archive.to_string();
    let mut cmd_tar = crate::proc::Command::strace(
        &config.steamcmd_archive.parent(),
        vec!["tar", "-xzf", &steamcmd_extractable],
    );
    let paths_touched: Vec<(String, u64)> = cmd_tar.run_to_end()?;
    let paths_touched_subset = paths_touched.iter().take(10);
    log::info!(
        "Extracted {} files from SteamCMD distribution '{}': Biggest {}: {}",
        paths_touched.len(),
        &config.steamcmd_archive,
        paths_touched_subset.len(),
        paths_touched_subset
            .into_iter()
            .cloned()
            .map(|(path, size)| format!("{}: {}", human_readable_size(size), path))
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

    let carbon_extractable: String = config.carbon_archive.to_string();
    let mut cmd_tar = crate::proc::Command::strace(
        &config.carbon_archive.parent(),
        vec!["tar", "-xzf", &carbon_extractable],
    );
    let paths_touched: Vec<(String, u64)> = cmd_tar.run_to_end()?;
    let paths_touched_subset = paths_touched.iter().take(10);
    log::info!(
        "Extracted {} files from Carbon distribution '{}': Biggest {}: {}",
        paths_touched.len(),
        &config.carbon_archive,
        paths_touched_subset.len(),
        paths_touched_subset
            .into_iter()
            .cloned()
            .map(|(path, size)| format!("{}: {}", human_readable_size(size), path))
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

    return Ok(());
}

fn ws_rcon_command(
    mut rcon_websocket: tungstenite::WebSocket<
        tungstenite::stream::MaybeTlsStream<std::net::TcpStream>,
    >,
    rcon_command: &str,
) -> Result<(), crate::error::FatalError> {
    match rcon_websocket.send(tungstenite::Message::Text(
        format!(
            "{{ \"Identifier\": 42, \"Message\": \"{}\" }}",
            rcon_command
        )
        .into(),
    )) {
        Err(err) => {
            return Err(crate::error::FatalError::new(
                format!("cannot send RCON command over WebSocket"),
                Some(Box::new(err)),
            ));
        }
        _ => {}
    }
    log::debug!(
        "Sent RCON command over WebSocket: '{}' -- Waiting for response...",
        rcon_command
    );
    loop {
        let msg = match rcon_websocket.read() {
            Ok(n) => n,
            Err(err) => {
                return Err(crate::error::FatalError::new(
                    format!("cannot read RCON response over WebSocket"),
                    Some(Box::new(err)),
                ));
            }
        };
        log::debug!("Got RCON message: {:#?}", msg);
        if let Ok(text) = msg.into_text() {
            if text.contains("\"Identifier\": 42") {
                break;
            }
        }
    }

    return Ok(());
}

pub fn get_rcon_websocket(
    rx_game_server_state: std::sync::mpsc::Receiver<GameServerState>,
    config: &crate::args::Config,
) -> Result<
    tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>,
    crate::error::FatalError,
> {
    match rx_game_server_state.recv_timeout(config.game_startup_timeout) {
        Ok(GameServerState::Playable) => {
            // The expected case: Game server eventually becomes playable after startup.
        }
        Err(err) => {
            return Err(crate::error::FatalError::new(
                format!(
                    "server startup completion not detected within {} minutes",
                    config.game_startup_timeout.as_secs() / 60
                ),
                Some(Box::new(err)),
            ));
        }
    };
    let (websocket, _) = match tungstenite::connect(format!(
        "ws://127.0.0.1:{}/{}",
        &config.rcon_port.to_string(),
        &config.rcon_password
    )) {
        Ok((websocket, http_response)) => (websocket, http_response),
        Err(err) => {
            return Err(crate::error::FatalError::new(
                format!("cannot connect WebSocket for RCON"),
                Some(Box::new(err)),
            ));
        }
    };
    return Ok(websocket);
}

pub fn configure_carbon(
    rcon_websocket: tungstenite::WebSocket<
        tungstenite::stream::MaybeTlsStream<std::net::TcpStream>,
    >,
) -> Result<(), crate::error::FatalError> {
    /*
      WebSocket RCON:
      `c.gocommunity`
      docs: https://docs.carbonmod.gg/docs/core/commands#c.gocommunity
      [Accessed 2024-10-27]
    */
    ws_rcon_command(rcon_websocket, "c.gocommunity")?;

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
pub fn extract_modified_paths(
    strace_output_raw: &str,
    cwd: &std::path::PathBuf,
) -> std::collections::HashSet<String> {
    let mut modified_paths: std::collections::HashSet<String> = std::collections::HashSet::new();
    for line in strace_output_raw.lines() {
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
            if let Some(strace_output) = parse_syscall_and_string_args(line) {
                if let Some(last_string_arg) = strace_output.argv_strings.last() {
                    let last_string_arg: String = last_string_arg.to_owned();
                    let file_path: &std::path::Path = std::path::Path::new(&last_string_arg);
                    let file_path_absolute: String;
                    if file_path.is_absolute() {
                        file_path_absolute = last_string_arg;
                    } else {
                        file_path_absolute = cwd.join(file_path).to_string_lossy().to_string();
                    }
                    modified_paths.insert(file_path_absolute);
                }
            }
        }
    }
    return modified_paths;
}

pub fn get_sizes(paths: std::collections::HashSet<String>) -> Vec<(String, u64)> {
    let mut paths_with_sizes: Vec<(String, u64)> = vec![];
    for modified_path in &paths {
        if let Ok(metadata) = std::fs::metadata(modified_path) {
            paths_with_sizes.push((modified_path.to_string(), metadata.len()));
        }
    }
    paths_with_sizes.sort_by(|a, b| b.1.cmp(&a.1));
    return paths_with_sizes;
}

#[derive(PartialEq, std::fmt::Debug)]
struct StraceLine {
    syscall_name: String,
    argv_strings: Vec<String>,
    constants: Vec<String>,
}

/// Parse name of the syscall and all of its passed string arguments from a
/// single line of strace output.
fn parse_syscall_and_string_args(strace_output_line: &str) -> Option<StraceLine> {
    let syscall_re: regex::Regex = regex::Regex::new(r"(\w+)\(").ok()?;
    let syscall_name: String = syscall_re
        .captures(strace_output_line)?
        .get(1)?
        .as_str()
        .to_string();

    let quoted_re: regex::Regex = regex::Regex::new(r#""(.*?)""#).ok()?;
    let strings: Vec<String> = quoted_re
        .captures_iter(strace_output_line)
        .filter_map(|n| n.get(1).map(|m| m.as_str().to_string()))
        .collect::<Vec<_>>();

    let constants_re: regex::Regex = regex::Regex::new(r#"[A-Z_]{2,}"#).ok()?;
    let constants: Vec<String> = constants_re
        .captures_iter(strace_output_line)
        .filter_map(|n| n.get(0).map(|m| m.as_str().to_string()))
        .collect::<Vec<_>>();

    return Some(StraceLine {
        syscall_name,
        argv_strings: strings,
        constants,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_syscall_and_string_args() {
        /*
            Case syscall + 1 string arg
        */
        let actual: Option<StraceLine> = parse_syscall_and_string_args(
            r#"[pid 18596] faccessat2(AT_FDCWD, "temp.txt", W_OK, AT_EACCESS) = 0"#,
        );
        let expected: Option<StraceLine> = Some(StraceLine {
            syscall_name: String::from("faccessat2"),
            argv_strings: vec![String::from("temp.txt")],
            constants: vec![
                String::from("AT_FDCWD"),
                String::from("W_OK"),
                String::from("AT_EACCESS"),
            ],
        });
        assert_eq!(
            actual, expected,
            "syscall name and one string arg parsed from fork line"
        );

        /*
            Case syscall + 2 string args
        */
        let actual: Option<StraceLine> = parse_syscall_and_string_args(
            r#"[pid 25024] renameat2(AT_FDCWD, "temp.txt", AT_FDCWD, "temp2.txt", RENAME_NOREPLACE) = 0"#,
        );
        let expected: Option<StraceLine> = Some(StraceLine {
            syscall_name: String::from("renameat2"),
            argv_strings: vec![String::from("temp.txt"), String::from("temp2.txt")],
            constants: vec![
                String::from("AT_FDCWD"),
                String::from("AT_FDCWD"),
                String::from("RENAME_NOREPLACE"),
            ],
        });

        assert_eq!(
            actual, expected,
            "syscall name and two string args parsed from fork line"
        );

        /*
            Case not-fork
        */
        let actual: Option<StraceLine> = parse_syscall_and_string_args(
            r#"openat(AT_FDCWD, "temp.txt", O_WRONLY|O_CREAT|O_TRUNC, 0666) = 3"#,
        );
        let expected: Option<StraceLine> = Some(StraceLine {
            syscall_name: String::from("openat"),
            argv_strings: vec![String::from("temp.txt")],
            constants: vec![
                String::from("AT_FDCWD"),
                String::from("O_WRONLY"),
                String::from("O_CREAT"),
                String::from("O_TRUNC"),
            ],
        });
        assert_eq!(
            actual, expected,
            "syscall name and one string arg parsed from not-fork line"
        );

        // TODO: Add test case for UPPER_CASE_FILENAME.txt: Should not be considered a "constant"
    }

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
