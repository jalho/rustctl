//! Dumpster for miscellaneous stuff yet to be better categorized.

use std::path::PathBuf;

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
        let cmd_strace: &str = "strace";
        let cmd_tar: &str = "tar";
        let out: std::process::Output = match std::process::Command::new(cmd_strace)
            .current_dir(rustctl_root_dir)
            .args([
                "-e",
                "trace=file",
                cmd_tar,
                "-xzf",
                &steamcmd_tgz_filename.to_string_lossy(),
            ])
            .output()
        {
            Ok(n) => n,
            Err(err) => {
                return Err(crate::error::FatalError::new(
                    format!(
                        "cannot install SteamCMD: cannot execute '{}' with '{}'",
                        cmd_tar, cmd_strace
                    ),
                    Some(Box::new(err)),
                ));
            }
        };
        if !out.status.success() {
            return Err(crate::error::FatalError::new(
                format!(
                    "cannot install SteamCMD: '{}' with '{}' exited unsuccessful status",
                    cmd_tar, cmd_strace
                ),
                None,
            ));
        }
        let stderr = match String::from_utf8(out.stderr) {
            Ok(n) => n,
            Err(err) => {
                return Err(crate::error::FatalError::new(
                    format!("cannot check SteamCMD installation: cannot collect STDERR of '{}' as UTF-8", cmd_strace),
                    Some(Box::new(err)),
                ));
            }
        };
        let paths: std::collections::HashSet<String> = extract_modified_paths(&stderr);
        let paths: Vec<&str> = paths.iter().map(|s| s.as_str()).collect::<Vec<&str>>();
        log::info!(
            "Extracted {} files from SteamCMD distribution '{}': {}",
            paths.len(),
            steamcmd_tgz_absolute.to_string_lossy(),
            paths.join(", ")
        );
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
