//! Dumpster for miscellaneous stuff yet to be better categorized.

use log::{debug, info};

/// Initialize a global logging utility.
pub fn init_logger() -> Result<log4rs::Handle, crate::args::ArgError> {
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
            return Err(crate::args::ArgError::ConfigInvalid(format!(
                "{:?}",
                err.errors()
            )));
        }
    };
    let logger: log4rs::Handle = match log4rs::init_config(logger_config) {
        Ok(n) => n,
        // SetLoggerError is not really an arg error but whatever
        Err(err) => return Err(crate::args::ArgError::ConfigInvalid(format!("{}", err))),
    };
    return Ok(logger);
}

pub enum InstallError {
    HttpError(crate::http::HttpError),
    ExtractError(std::io::ErrorKind),
}
impl std::fmt::Debug for InstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HttpError(arg0) => f.debug_tuple("HttpError").field(arg0).finish(),
            Self::ExtractError(arg0) => f.debug_tuple("ExtractError").field(arg0).finish(),
        }
    }
}
impl From<crate::http::HttpError> for InstallError {
    fn from(err: crate::http::HttpError) -> Self {
        return Self::HttpError(err);
    }
}

/// Install _SteamCMD_ (game server installer).
pub fn install_steamcmd(
    url: &String,
    download_dir: &std::path::PathBuf,
    target_file_name: &std::path::PathBuf,
    expected_extracted_steamcmd_entrypoint: &std::path::PathBuf,
) -> Result<(), InstallError> {
    let mut path = download_dir.clone();
    path.push(target_file_name);

    if !path.is_file() {
        let mut response: std::net::TcpStream = crate::http::request(url)?;
        let streamed_size: usize = crate::http::stream_to_disk(&mut response, &path)?;
        log::info!("Downloaded SteamCMD: {} bytes from {}", streamed_size, url);
    } else {
        log::debug!(
            "SteamCMD distribution '{}' has been downloaded earlier -- Not downloading again",
            path.to_string_lossy()
        );
    }

    let cmd_strace: &str = "strace";
    if !expected_extracted_steamcmd_entrypoint.is_file() {
        let out: std::process::Output = match std::process::Command::new(cmd_strace)
            .current_dir(download_dir)
            .args([
                "-e",
                "trace=file",
                "tar",
                "-xzf",
                &target_file_name.to_string_lossy(),
            ])
            .output()
        {
            Ok(n) => n,
            Err(err) => {
                return Err(InstallError::ExtractError(err.kind()));
            }
        };
        if !out.status.success() {
            todo!(); /* TODO: Make a FatalError */
        }
        let stderr = String::from_utf8(out.stderr).unwrap(); /* TODO: Make a FatalError */
        let paths: std::collections::HashSet<String> = extract_modified_paths(&stderr);
        let paths: Vec<&str> = paths.iter().map(|s| s.as_str()).collect::<Vec<&str>>();
        info!(
            "Extracted files from SteamCMD distribution: {}",
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
