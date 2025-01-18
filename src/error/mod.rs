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
        return write!(f, "");
    }
}
