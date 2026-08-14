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

    /// A tag contained a space (U+0020), which Anki does not allow.
    ///
    /// Mirrors Python genanki's `ValueError` from `_TagList._validate_tag`;
    /// raised from every tag mutation path.
    #[error("tag contains a space (U+0020), which is not allowed: {tag:?}")]
    TagContainsSpace {
        /// The offending tag value.
        tag: String,
    },

    /// The note's field count did not match its model's field count.
    ///
    /// Mirrors Python genanki's `ValueError` from
    /// `Note._check_number_model_fields_matches_num_fields`.
    #[error(
        "number of fields in model does not match note: model {model_name:?} has {model_fields} fields, note has {note_fields}"
    )]
    FieldCountMismatch {
        /// The model's name.
        model_name: String,
        /// Number of fields defined on the model.
        model_fields: usize,
        /// Number of fields supplied on the note.
        note_fields: usize,
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
    fn tag_contains_space_error_display() {
        let err = Error::TagContainsSpace { tag: "b ar".into() };
        let s = err.to_string();
        assert!(s.contains("space"));
        assert!(s.contains("b ar"));
    }

    #[test]
    fn field_count_mismatch_error_display() {
        let err = Error::FieldCountMismatch {
            model_name: "Test Model".into(),
            model_fields: 3,
            note_fields: 2,
        };
        let s = err.to_string();
        assert!(s.contains("Test Model"));
        assert!(s.contains("has 3 fields"));
        assert!(s.contains("note has 2"));
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
