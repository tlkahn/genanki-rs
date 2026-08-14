//! Crate error type and result alias.

use thiserror::Error;

/// Errors produced by this crate.
///
/// Variants will expand in later phases (IO, validation, template req, media, SQL).
#[derive(Debug, Error)]
pub enum Error {
    /// Placeholder variant so the type is usable before real failures exist.
    /// Remove once concrete variants land.
    #[error("internal error: {0}")]
    Internal(&'static str),
}

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
