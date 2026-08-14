//! Crate error type and result alias.

use thiserror::Error;

/// Errors produced by this crate.
///
/// Variants will expand in later phases (IO, validation, template req, media, SQL).
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// Placeholder variant so the type is usable before real failures exist.
    /// Remove once concrete variants land.
    #[error("internal error: {0}")]
    Internal(&'static str),

    /// Could not compute required fields for a template's `qfmt`.
    ///
    /// Mirrors Python genanki's `Exception` raised from `Model._req` when no
    /// field (under either the "all" or "any" strategy) is detectable.
    #[error("could not compute required fields for template qfmt: {qfmt}")]
    TemplateReq {
        /// The template's `qfmt` string (interpolated into the error message).
        qfmt: String,
    },
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

    #[test]
    fn template_req_error_display() {
        let err = Error::TemplateReq {
            qfmt: "{{Nope}}".into(),
        };
        let s = err.to_string();
        assert!(s.contains("required fields"));
        assert!(s.contains("{{Nope}}"));
    }
}
