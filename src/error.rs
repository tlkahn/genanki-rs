//! Crate error type and result alias.

use std::fmt;

/// Errors produced by this crate.
///
/// Variants will expand in Phase 1+ (IO, validation, template req, media, SQL).
#[derive(Debug)]
pub enum Error {
    /// Placeholder variant so the type is usable before real failures exist.
    /// Remove once concrete variants land.
    Internal(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for Error {}

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = Error::Internal("scaffold");
        assert_eq!(err.to_string(), "internal error: scaffold");
    }
}
