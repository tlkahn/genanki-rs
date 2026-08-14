//! Deck (collection) model. (Phase 4)

/// An Anki deck: a named container of notes sharing a model registry.
///
/// Owns its notes by value and a registry of models. The in-memory registry
/// only ever holds explicit [`Deck::add_model`] entries; note models are
/// auto-registered **at write time in the DB only** and do not mutate the
/// deck. Written to an `.apkg` via [`Deck::write_to_file`] (or as part of a
/// [`crate::Package`]).
#[derive(Debug)]
pub struct Deck {
    /// Stable deck id (Anki convention: Unix-ms-style timestamp).
    pub(crate) id: i64,
    /// Deck name as shown in Anki ("Parent::Child" for nested decks).
    pub(crate) name: String,
    /// Deck description text.
    pub(crate) description: String,
    /// Notes in insertion order.
    pub(crate) notes: Vec<crate::note::Note>,
    /// Model registry; auto-populated from notes at write.
    pub(crate) models: std::collections::BTreeMap<i64, std::sync::Arc<crate::model::Model>>,
}

impl Deck {
    /// Create a deck with the given id and name.
    ///
    /// The id is arbitrary; Anki convention is a Unix-ms-style timestamp. A
    /// unique id is strongly recommended: **`id == 1` overwrites the seeded
    /// Default deck** in `col.decks` at write time (only that entry is
    /// replaced; the deck name still comes from this deck).
    #[must_use]
    pub fn new(id: i64, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            description: String::new(),
            notes: Vec::new(),
            models: std::collections::BTreeMap::new(),
        }
    }

    /// The deck id.
    #[must_use]
    pub fn id(&self) -> i64 {
        self.id
    }

    /// The deck name as shown in Anki.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The deck description.
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Consume and set the description.
    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Replace the description.
    pub fn set_description(&mut self, desc: impl Into<String>) {
        self.description = desc.into();
    }

    /// Notes in insertion order.
    #[must_use]
    pub fn notes(&self) -> &[crate::note::Note] {
        &self.notes
    }

    /// Mutable access to notes, e.g. to set `suspend` before write.
    #[must_use]
    pub fn notes_mut(&mut self) -> &mut [crate::note::Note] {
        &mut self.notes
    }

    /// Append a note. Its model is auto-registered at write time.
    pub fn add_note(&mut self, note: crate::note::Note) {
        self.notes.push(note);
    }

    /// Explicitly register a model (kept even if no note uses it).
    pub fn add_model(&mut self, model: impl Into<std::sync::Arc<crate::model::Model>>) {
        let model = model.into();
        self.models.insert(model.id, model);
    }

    /// Model registry: explicit [`Self::add_model`] entries only.
    ///
    /// Note models are **not** added here; auto-registration happens at write
    /// time inside the DB (`col.models`) and never mutates the deck, so a
    /// write via `&self` leaves this map unchanged.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn models(
        &self,
    ) -> &std::collections::BTreeMap<i64, std::sync::Arc<crate::model::Model>> {
        &self.models
    }

    /// Serialize to the object stored under `col.decks[deck_id]`.
    ///
    /// Mirrors Python genanki v0.13.0 `Deck.to_json` key-for-key, including
    /// the hardcoded `mod` 1425278051 (Python has no timestamp parameter
    /// here) and constant scheduling fields.
    #[must_use]
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "collapsed": false,
            "conf": 1,
            "desc": self.description,
            "dyn": 0,
            "extendNew": 0,
            "extendRev": 50,
            "id": self.id,
            "lrnToday": [163, 2],
            "mod": 1425278051,
            "name": self.name,
            "newToday": [163, 2],
            "revToday": [163, 0],
            "timeToday": [163, 23598],
            "usn": -1,
        })
    }

    /// Merge this deck into an open `col`: its JSON into `col.decks`, its
    /// models (explicit registry plus auto-registered note models) into
    /// `col.models`, then insert all notes and their cards.
    ///
    /// Mirrors Python genanki v0.13.0 `Deck.write_to_db`; note ids and card
    /// ids come from the shared `id_gen`. Errors with [`crate::Error::DeckInvalid`]
    /// when the deck name is empty.
    pub(crate) fn write_to_db(
        &self,
        conn: &rusqlite::Connection,
        timestamp_secs: f64,
        id_gen: &mut crate::apkg::db::IdGen,
    ) -> crate::Result<()> {
        if self.name.is_empty() {
            return Err(crate::Error::DeckInvalid {
                reason: "name must be non-empty",
            });
        }

        // Merge deck JSON (preserving the seeded Default deck).
        let decks_raw: String = conn.query_row("SELECT decks FROM col", [], |r| r.get(0))?;
        let mut decks: serde_json::Value = serde_json::from_str(&decks_raw)?;
        decks
            .as_object_mut()
            .ok_or(crate::Error::Internal("col.decks is not an object"))?
            .insert(self.id.to_string(), self.to_json());
        conn.execute("UPDATE col SET decks = ?1", [decks.to_string()])?;

        // Merge model JSON: explicit registry + auto-registered note models.
        let models_raw: String = conn.query_row("SELECT models FROM col", [], |r| r.get(0))?;
        let mut models_json: serde_json::Value = serde_json::from_str(&models_raw)?;
        let mut merged: std::collections::BTreeMap<i64, std::sync::Arc<crate::model::Model>> =
            self.models.clone();
        for note in &self.notes {
            let model = note.model_arc();
            merged.insert(model.id, model);
        }
        {
            let obj = models_json
                .as_object_mut()
                .ok_or(crate::Error::Internal("col.models is not an object"))?;
            for (mid, model) in &merged {
                obj.insert(
                    mid.to_string(),
                    model.to_json(timestamp_secs as i64, self.id)?,
                );
            }
        }
        conn.execute("UPDATE col SET models = ?1", [models_json.to_string()])?;

        // Insert notes then their cards, sharing the id_gen.
        for note in &self.notes {
            let note_id = crate::apkg::db::insert_note(conn, note, timestamp_secs, id_gen)?;
            let due = note.due();
            for card in note.resolved_cards()? {
                crate::apkg::db::insert_card(
                    conn,
                    &card,
                    note_id,
                    self.id,
                    timestamp_secs,
                    due,
                    id_gen,
                )?;
            }
        }
        Ok(())
    }

    /// Write this deck to an `.apkg` file using the current wall-clock time
    /// (the timestamp source for note/card ids and `mod` columns).
    ///
    /// Shorthand for writing a single-deck package (same write engine as
    /// [`crate::Package::write_to_file`]); the deck is borrowed, so it can be
    /// written repeatedly.
    pub fn write_to_file<P: AsRef<std::path::Path>>(&self, path: P) -> crate::Result<()> {
        self.write_to_file_at(path, crate::package::now_secs())
    }

    /// Write this deck to an `.apkg` file with a fixed timestamp (seconds
    /// since Unix epoch).
    ///
    /// Deterministic note/card ids and `mod` columns, plus byte-identical
    /// package bytes across runs (zip entry mtimes are pinned to the same
    /// timestamp). See [`crate::Package::write_to_file_at`] for details.
    pub fn write_to_file_at<P: AsRef<std::path::Path>>(
        &self,
        path: P,
        timestamp_secs: f64,
    ) -> crate::Result<()> {
        crate::package::write_to_file_impl(std::slice::from_ref(self), &[], path, timestamp_secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Field, Model, Template};

    fn simple_model() -> Model {
        Model::new(1607392319, "Simple Model")
            .field(Field::new("Question"))
            .field(Field::new("Answer"))
            .template(Template::new(
                "Card 1",
                "{{Question}}",
                "{{FrontSide}}<hr id=\"answer\">{{Answer}}",
            ))
    }

    #[test]
    fn constructor_and_accessors() {
        let d = Deck::new(123, "foodeck");
        assert_eq!(d.id(), 123);
        assert_eq!(d.name(), "foodeck");
        assert_eq!(d.description(), "");
        assert!(d.notes().is_empty());
    }

    #[test]
    fn description_default_and_setters() {
        let d = Deck::new(1, "d").with_description("hello\nworld");
        assert_eq!(d.description(), "hello\nworld");
        let mut d = d;
        d.set_description("replacement");
        assert_eq!(d.description(), "replacement");
    }

    #[test]
    fn add_note_increases_len() {
        let mut d = Deck::new(123, "foodeck");
        assert_eq!(d.notes().len(), 0);
        d.add_note(crate::note::Note::new(simple_model(), ["a", "b"]).unwrap());
        d.add_note(crate::note::Note::new(simple_model(), ["c", "d"]).unwrap());
        assert_eq!(d.notes().len(), 2);
        assert_eq!(d.notes()[0].fields(), ["a", "b"]);
    }

    #[test]
    fn notes_mut_allows_mutation() {
        let mut d = Deck::new(123, "foodeck");
        d.add_note(crate::note::Note::new(simple_model(), ["a", "b"]).unwrap());
        d.notes_mut()[0].set_tags(["x"]).unwrap();
        assert_eq!(d.notes()[0].tags(), ["x"]);
    }

    #[test]
    fn add_model_registers_in_map() {
        let mut d = Deck::new(123, "foodeck");
        assert!(d.models().is_empty());
        d.add_model(simple_model());
        assert!(d.models().contains_key(&1607392319));
        d.add_model(std::sync::Arc::new(simple_model()));
        assert_eq!(d.models().len(), 1, "same id overwrites (Python dict)");
    }

    // --- T2: Deck JSON shape ---

    #[test]
    fn to_json_full_shape() {
        let d = Deck::new(112233, "foodeck")
            .with_description("This is my great deck.\nIt is so so great.");
        let v = d.to_json();
        assert_eq!(v["name"], "foodeck");
        assert_eq!(v["id"], 112233);
        assert_eq!(v["desc"], "This is my great deck.\nIt is so so great.");
        assert_eq!(v["mod"], 1425278051);
        assert_eq!(v["usn"], -1);
        assert_eq!(v["conf"], 1);
        assert_eq!(v["dyn"], 0);
        assert_eq!(v["collapsed"], false);
        assert_eq!(v["extendNew"], 0);
        assert_eq!(v["extendRev"], 50);
        assert_eq!(v["lrnToday"], serde_json::json!([163, 2]));
        assert_eq!(v["newToday"], serde_json::json!([163, 2]));
        assert_eq!(v["revToday"], serde_json::json!([163, 0]));
        assert_eq!(v["timeToday"], serde_json::json!([163, 23598]));
    }

    #[test]
    fn to_json_empty_description() {
        let d = Deck::new(7, "x");
        assert_eq!(d.to_json()["desc"], "");
    }
}
