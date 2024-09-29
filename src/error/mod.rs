//! Main error module.

/// Non recoverable errors that the _main_ may exit with.
pub enum FatalError {
    ArgError(crate::args::ArgError),
    HttpError(crate::http::HttpError),
}
impl std::fmt::Debug for FatalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ArgError(arg0) => f.debug_tuple("ArgError").field(arg0).finish(),
            Self::HttpError(arg0) => f.debug_tuple("HttpError").field(arg0).finish(),
        }
    }
}
impl From<crate::args::ArgError> for FatalError {
    fn from(err: crate::args::ArgError) -> Self {
        return Self::ArgError(err);
    }
}
impl From<crate::http::HttpError> for FatalError {
    fn from(err: crate::http::HttpError) -> Self {
        return Self::HttpError(err);
    }
}
