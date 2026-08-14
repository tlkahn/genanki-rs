//! Programmatic generation of Anki `.apkg` packages, in Rust.
//!
//! The crate is a port of the Python [genanki](https://github.com/kerrickstaley/genanki)
//! library: you build [`Model`]s, [`Note`]s, [`Deck`]s and a [`Package`], then
//! write a single `.apkg` file that Anki (desktop, AnkiDroid, AnkiWeb) can
//! import. It is **not** affiliated with the Anki project.
//!
//! Feature roadmap and full specification: see repository epic issue #1.
//!
//! # Quickstart
//!
//! Define a note type (a [`Model`] with fields and card templates), create
//! notes, put them in a deck, and write the package:
//!
//! ```
//! use genanki::{Deck, Field, Model, Note, Package, Template};
//!
//! let model = Model::new(1607392319, "Simple Model")
//!     .field(Field::new("Question"))
//!     .field(Field::new("Answer"))
//!     .template(Template::new(
//!         "Card 1",
//!         "{{Question}}",
//!         "{{FrontSide}}<hr id=\"answer\">{{Answer}}",
//!     ));
//!
//! let note = Note::new(model, ["Capital of Argentina", "Buenos Aires"])?;
//!
//! let mut deck = Deck::new(2059400110, "Country Capitals");
//! deck.add_note(note);
//!
//! let dir = tempfile::tempdir()?;
//! let path = dir.path().join("output.apkg");
//! Package::new(deck).write_to_file(&path)?;
//!
//! # Ok::<(), genanki::Error>(())
//! ```
//!
//! # Builtin models
//!
//! The crate ships the five Anki stock note types under the `(genanki)` name
//! suffix (see [`builtin_models`] for why): [`BASIC_MODEL`],
//! [`BASIC_AND_REVERSED_CARD_MODEL`], [`BASIC_OPTIONAL_REVERSED_CARD_MODEL`],
//! [`BASIC_TYPE_IN_THE_ANSWER_MODEL`], and [`CLOZE_MODEL`]. Pass one to
//! [`Note::new`] as `&*MODEL` (the `From<&Model> for Arc<Model>` impl makes
//! this work without an extra clone at the call site):
//!
//! ```
//! use genanki::{CLOZE_MODEL, Deck, Note, Package};
//!
//! let mut deck = Deck::new(2059400110, "Cloze Demos");
//! deck.add_note(Note::new(
//!     &*CLOZE_MODEL,
//!     ["{{c1::Rome}} is the capital of {{c2::Italy}}.", ""],
//! )?);
//!
//! let dir = tempfile::tempdir()?;
//! let path = dir.path().join("cloze.apkg");
//! Package::new(deck).write_to_file(&path)?;
//!
//! # Ok::<(), genanki::Error>(())
//! ```

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

pub use crate::builtin_models::{
    BASIC_AND_REVERSED_CARD_MODEL, BASIC_MODEL, BASIC_OPTIONAL_REVERSED_CARD_MODEL,
    BASIC_TYPE_IN_THE_ANSWER_MODEL, CLOZE_MODEL,
};
pub use crate::card::Card;
pub use crate::deck::Deck;
pub use crate::error::{Error, Result};
pub use crate::guid::guid_for;
pub use crate::model::{Field, Model, ModelType, Template};
pub use crate::note::Note;
pub use crate::package::Package;
pub use crate::req::{ReqEntry, ReqKind};
// BASE91_TABLE: use genanki::guid::BASE91_TABLE (not re-exported at crate root).
