//! Main error module.

/// Non recoverable errors that the _main_ may exit with.
pub enum FatalError {
    ArgError(crate::args::ArgError),
    InstallError(crate::misc::InstallError),
}
impl std::fmt::Debug for FatalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ArgError(arg0) => f.debug_tuple("ArgError").field(arg0).finish(),
            Self::InstallError(arg0) => f.debug_tuple("InstallError").field(arg0).finish(),
        }
    }
}
impl From<crate::args::ArgError> for FatalError {
    fn from(err: crate::args::ArgError) -> Self {
        return Self::ArgError(err);
    }
}
impl From<crate::misc::InstallError> for FatalError {
    fn from(err: crate::misc::InstallError) -> Self {
        return Self::InstallError(err);
    }
}
