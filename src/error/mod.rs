//! Error cases of the program.

/// A required dependency is missing.
#[derive(Debug)]
pub struct ErrDependencyMissing {
    pub executable: &'static str,
}
impl std::error::Error for ErrDependencyMissing {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        return None;
    }
}
impl std::fmt::Display for ErrDependencyMissing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "missing dependency '{}'", &self.executable);
    }
}

/// Executing a dependency command failed.
#[derive(Debug)]
pub struct ErrExec {
    /// Executable and its argument vector concatenated.
    pub command: String,
    pub stderr: Option<String>,
    pub status: Option<i32>,
}
impl std::error::Error for ErrExec {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        return None;
    }
}
impl std::fmt::Display for ErrExec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status: &str = match self.status {
            Some(status) => &format!("with status {status}"),
            None => "without status",
        };
        let stderr: &str = match self.stderr {
            Some(ref stderr) => &format!("\n{stderr}"),
            None => "",
        };
        return write!(
            f,
            "command failed {}: {}{}",
            &status, &self.command, &stderr
        );
    }
}

/// Installing the game server with SteamCMD failed.
#[derive(Debug)]
pub enum ErrInstallGame {
    /// SteamCMD failed.
    ErrSteamCmd(ErrExec),
    /// Precondition not met: Missing permissions to the specified installation dir.
    ErrMissingPermissions(String),
}
impl std::error::Error for ErrInstallGame {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ErrInstallGame::ErrSteamCmd(err_exec) => {
                return Some(err_exec);
            }
            ErrInstallGame::ErrMissingPermissions(_) => {
                return None;
            }
        }
    }
}
impl std::fmt::Display for ErrInstallGame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrInstallGame::ErrSteamCmd(err_exec) => {
                return err_exec.fmt(f);
            }
            ErrInstallGame::ErrMissingPermissions(path) => {
                return write!(f, "missing permissions to {path}");
            }
        }
    }
}
