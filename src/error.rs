//! Crate error type and result alias.

use std::path::PathBuf;

use thiserror::Error;

/// Errors produced by this crate.
///
/// Current variant families (Phase 4): model/template validation
/// ([`Error::TemplateReq`]), tag and field-count validation, deck/media write
/// validation ([`Error::DeckInvalid`], [`Error::MediaNotFound`],
/// [`Error::MediaInvalidPath`], [`Error::MediaBasenameCollision`]), and
/// underlying [`Error::Io`] / [`Error::Sqlite`] / [`Error::Zip`] /
/// [`Error::Json`] failures. [`Error::Internal`] guards structural
/// invariants.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// A structural invariant was violated (e.g. `col.decks` is not a JSON
    /// object). Indicates a bug rather than a user error.
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

    /// A deck failed a write-time validation (e.g. empty name).
    #[error("deck invalid: {reason}")]
    DeckInvalid {
        /// Human-readable reason for the rejection.
        reason: &'static str,
    },

    /// A media file listed in the package does not exist on disk.
    #[error("media file not found: {path}")]
    MediaNotFound {
        /// The missing path.
        path: PathBuf,
    },

    /// A media path has no usable basename (e.g. `..`).
    #[error("media path has no usable basename: {path}")]
    MediaInvalidPath {
        /// The offending path.
        path: PathBuf,
    },

    /// Two distinct media paths share the same basename, which would
    /// silently overwrite in Anki's flat media folder.
    #[error("media basename collision for {basename:?}: {path_a} and {path_b}")]
    MediaBasenameCollision {
        /// The colliding basename.
        basename: String,
        /// First path seen with this basename.
        path_a: PathBuf,
        /// Second (conflicting) path.
        path_b: PathBuf,
    },

    /// Underlying filesystem error (e.g. zip target open).
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Underlying SQLite error.
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),

    /// Underlying zip archive error.
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),

    /// Underlying JSON error (col merge, media map).
    #[error(transparent)]
    Json(#[from] serde_json::Error),
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

    // --- Phase 4 (issue #6) domain variants ---

    #[test]
    fn deck_invalid_error_display() {
        let err = Error::DeckInvalid {
            reason: "name must be non-empty",
        };
        assert_eq!(err.to_string(), "deck invalid: name must be non-empty");
    }

    #[test]
    fn media_not_found_error_display() {
        let err = Error::MediaNotFound {
            path: "/tmp/nope.mp3".into(),
        };
        assert_eq!(err.to_string(), "media file not found: /tmp/nope.mp3");
    }

    #[test]
    fn media_invalid_path_error_display() {
        let err = Error::MediaInvalidPath {
            path: "/tmp/..".into(),
        };
        assert_eq!(
            err.to_string(),
            "media path has no usable basename: /tmp/.."
        );
    }

    #[test]
    fn media_basename_collision_error_display() {
        let err = Error::MediaBasenameCollision {
            basename: "foo.png".into(),
            path_a: "/a/foo.png".into(),
            path_b: "/b/foo.png".into(),
        };
        assert_eq!(
            err.to_string(),
            "media basename collision for \"foo.png\": /a/foo.png and /b/foo.png"
        );
    }

    #[test]
    fn io_sqlite_zip_json_variants_exist() {
        // Transparent `#[from]` variants must exist and map to their sources.
        let io: Error = std::io::Error::other("boom").into();
        assert!(matches!(io, Error::Io(_)));
        let json: Error = serde_json::from_str::<serde_json::Value>("{")
            .unwrap_err()
            .into();
        assert!(matches!(json, Error::Json(_)));
    }
}
