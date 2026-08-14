//! Programmatic generation of Anki `.apkg` packages.
//!
//! Feature roadmap and full specification: see repository epic issue #1.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod error;

// Domain modules (filled in later phases). Public so paths stabilize early.
pub mod apkg;
pub mod builtin_models;
pub mod card;
pub mod deck;
pub mod guid;
pub mod model;
pub mod note;
pub mod package;
pub mod req;

pub use crate::error::{Error, Result};
pub use crate::guid::guid_for;
pub use crate::model::{Field, Model, ModelType, Template};
pub use crate::req::{ReqEntry, ReqKind};
// BASE91_TABLE: use genanki::guid::BASE91_TABLE (not re-exported at crate root).
