//! Errors of the program.

pub mod fatal {
    //! Fatal, i.e. non-recoverable errors of the program that force it to
    //! terminate.

    use crate::core::JoinWith;

    #[derive(Debug)]
    pub enum Error {
        MalformedSteamAppInfo {
            source_display: String,
            content_utf8: String,
        },
        MissingExpectedWorkingDirectory(std::path::PathBuf),
        AmbiguousExistingInstallation(Vec<std::path::PathBuf>),
        CannotCheckUpdates(CCU),
        FailedInstallAttempt(FIA),
        GameStartError {
            system_error: std::io::Error,
            executable_path_absolute: std::path::PathBuf,
            exec_dir_path_absolute: std::path::PathBuf,
        },
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::MissingExpectedWorkingDirectory(_path_buf) => None,
                Error::AmbiguousExistingInstallation(_vec) => None,
                Error::CannotCheckUpdates(CCU::AmbiguousLocalCache { .. }) => None,
                Error::CannotCheckUpdates(CCU::CannotWipeLocalCache { .. }) => None,
                Error::CannotCheckUpdates(CCU::CannotFetchRemoteInfo(_meta)) => None,
                Error::FailedInstallAttempt(FIA::CannotInstall(_meta)) => None,
                Error::FailedInstallAttempt(FIA::InvalidInstallation(
                    II::MissingRequiredFile { .. },
                )) => None,
                Error::FailedInstallAttempt(FIA::InvalidInstallation(
                    II::AmbiguousRequiredFile { .. },
                )) => None,
                Error::GameStartError {
                    system_error,
                    executable_path_absolute: _,
                    exec_dir_path_absolute: _,
                } => Some(system_error),
                Error::MalformedSteamAppInfo { .. } => None,
            }
        }
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::MissingExpectedWorkingDirectory(path_buf) => write!(
                    f,
                    "missing expected working directory: {}",
                    path_buf.to_string_lossy()
                ),
                Error::CannotCheckUpdates(CCU::AmbiguousLocalCache {
                    cache_filename_seeked,
                    cache_paths_absolute_found,
                }) => write!(
                    f,
                    "cannot check updates: ambigous local cache: found in {} places: {}",
                    cache_paths_absolute_found.len(),
                    cache_paths_absolute_found.join_with(", ")
                ),
                Error::CannotCheckUpdates(CCU::CannotWipeLocalCache {
                    cache_path_absolute_found,
                    system_error: _,
                }) => write!(
                    f,
                    "cannot check updates: cannot wipe local cache {}",
                    cache_path_absolute_found.to_string_lossy(),
                ),
                Error::CannotCheckUpdates(CCU::CannotFetchRemoteInfo(steamcmd_error_meta)) => {
                    write!(
                        f,
                        "cannot check updates: SteamCMD failed: {steamcmd_error_meta} "
                    )
                }
                Error::FailedInstallAttempt(FIA::CannotInstall(steamcmd_error_meta)) => {
                    return write!(
                        f,
                        "cannot install game: SteamCMD failed: {steamcmd_error_meta}"
                    );
                }
                Error::FailedInstallAttempt(FIA::InvalidInstallation(
                    II::MissingRequiredFile { filename_seeked },
                )) => {
                    return write!(
                        f,
                        "installation failed: missing required file: {}",
                        filename_seeked.to_string_lossy()
                    );
                }
                Error::FailedInstallAttempt(FIA::InvalidInstallation(
                    II::AmbiguousRequiredFile {
                        paths_absolute_found,
                    },
                )) => {
                    return write!(
                        f,
                        "installation failed: ambiguous required file: found in {} places: {}",
                        paths_absolute_found.len(),
                        paths_absolute_found.join_with(", ")
                    );
                }
                Error::GameStartError {
                    system_error: _,
                    executable_path_absolute,
                    exec_dir_path_absolute,
                } => {
                    return write!(
                        f,
                        "cannot start game: attempted executable {} in {}",
                        executable_path_absolute.to_string_lossy(),
                        exec_dir_path_absolute.to_string_lossy()
                    );
                }
                Error::AmbiguousExistingInstallation(existing_installations) => write!(
                    f,
                    "ambiguous existing installation: found in {} places: {}",
                    existing_installations.len(),
                    existing_installations.join_with(", ")
                ),
                Error::MalformedSteamAppInfo {
                    source_display,
                    content_utf8,
                } => {
                    write!(
                        f,
                        "malformed app info from {source_display}:\n{content_utf8}"
                    )
                }
            }
        }
    }

    /// CannotCheckUpdates
    #[derive(Debug)]
    pub enum CCU {
        AmbiguousLocalCache {
            cache_filename_seeked: std::path::PathBuf,
            cache_paths_absolute_found: Vec<std::path::PathBuf>,
        },
        CannotWipeLocalCache {
            cache_path_absolute_found: std::path::PathBuf,
            system_error: std::io::Error,
        },
        CannotFetchRemoteInfo(SteamCMDErrorMeta),
    }

    /// FailedInstallAttempt
    #[derive(Debug)]
    pub enum FIA {
        CannotInstall(SteamCMDErrorMeta),
        InvalidInstallation(II),
    }

    /// InvalidInstallation
    #[derive(Debug)]
    pub enum II {
        MissingRequiredFile {
            filename_seeked: std::path::PathBuf,
        },
        AmbiguousRequiredFile {
            paths_absolute_found: Vec<std::path::PathBuf>,
        },
    }

    #[derive(Debug)]
    pub struct SteamCMDErrorMeta {
        pub steamcmd_command_argv: crate::core::SteamCMDArgv,
        pub steamcmd_exit_status: Option<std::process::ExitStatus>,
        pub steamcmd_stdout: Vec<u8>,
        pub steamcmd_stderr: Vec<u8>,
    }

    impl std::fmt::Display for SteamCMDErrorMeta {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let status: &str = match self.steamcmd_exit_status {
                Some(n) => &format!("with {n}"),
                None => "without status",
            };

            let operation: &str = match self.steamcmd_command_argv {
                crate::core::SteamCMDArgv::InstallGame(ref vec) => {
                    &format!("install game: argv: [{}]", vec.join(" "))
                }
                crate::core::SteamCMDArgv::FetchGameInfo(ref vec) => {
                    &format!("fetch game info: argv: [{}]", vec.join(" "))
                }
            };

            return write!(
                f,
                "{status}: {operation}: {} bytes in STDOUT, {} in STDERR",
                self.steamcmd_stdout.len(),
                self.steamcmd_stderr.len(),
            );
        }
    }
}

mod recoverable {
    //! Recoverable errors that should draw attention (via logging) but that
    //! don't force the program to terminate.
}
