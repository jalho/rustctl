//! Error cases of the program.

/// A precondition is not met: Critical dependency is missing or something.
#[derive(Debug)]
pub enum ErrPrecondition {
    MissingDependency(String),
    MissingPermission(String),
}
impl std::error::Error for ErrPrecondition {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        return None;
    }
}
impl std::fmt::Display for ErrPrecondition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrPrecondition::MissingDependency(dependency) => {
                return write!(f, "precondition not met: dependency missing: {dependency}");
            }
            ErrPrecondition::MissingPermission(permission) => {
                return write!(
                    f,
                    "precondition not met: insufficient permissions: {permission}"
                );
            }
        }
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
