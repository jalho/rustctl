//! Main error module.

/// Non recoverable errors that the _main_ may exit with.
#[derive(std::fmt::Debug)]
pub struct FatalError {
    description: String,
    source: Option<Box<dyn std::error::Error>>,
}

impl FatalError {
    pub fn new(description: String, source: Option<Box<dyn std::error::Error>>) -> Self {
        return Self {
            description,
            source,
        };
    }
}

impl std::error::Error for FatalError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        return self.source.as_ref().map(|e| e.as_ref());
    }
}

impl std::fmt::Display for FatalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return f.write_fmt(format_args!("Non recoverable error: {}", self.description));
    }
}
