//! Note representation: fields, tags, GUID/sort/due, and card generation, plus
//! the public [`find_invalid_html_tags`] HTML scanner. (Phase 3)

use std::sync::{Arc, LazyLock};

use regex::Regex;

use crate::model::Model;
use crate::{Error, Result};

/// A single note: one row of field values under a shared model.
///
/// Mirrors Python genanki `Note` (v0.13.1): field-count and tag validation
/// happen eagerly; invalid-HTML warnings are non-fatal; GUID and sort field
/// may be overridden; cards are computed lazily and cached.
#[derive(Debug)]
pub struct Note {
    /// The note type this note belongs to (shared, cheap to clone).
    model: Arc<Model>,
    /// Field values in model ordinal order.
    fields: Vec<String>,
    /// Explicit sort-field override; `None` defers to `fields[sort_field_index]`.
    sort_field_override: Option<String>,
    /// Tags; each mutation path validates against U+0020.
    tags: Vec<String>,
    /// Explicit GUID override; `None` computes from `guid_for(fields)`.
    guid_override: Option<String>,
    /// Scheduling due value, propagated to cards at Phase 4 write.
    due: i64,
    /// Lazy card cache; `None` means "needs (re)compute".
    cards: Option<Vec<crate::card::Card>>,
}

impl Note {
    /// Create a note for `model` with `fields`.
    ///
    /// Errors with [`Error::FieldCountMismatch`] if `fields.len()` differs
    /// from `model.fields.len()` (no auto-padding, matching Python genanki's
    /// fail-fast check). Accepts a [`Model`], an [`Arc<Model>`], or a `&Model`
    /// (via [`From<&Model> for Arc<Model>`](std::convert::From), which clones
    /// into a fresh `Arc`). For builtin statics use
    /// `Note::new(&*BASIC_MODEL, fields)?`. When creating many notes from one
    /// model, clone into an `Arc` once and reuse it to avoid repeated deep
    /// clones.
    pub fn new(
        model: impl Into<Arc<Model>>,
        fields: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self> {
        let model = model.into();
        let fields: Vec<String> = fields.into_iter().map(Into::into).collect();
        check_field_count(&model, &fields)?;
        for field in &fields {
            warn_invalid_html(field);
        }
        Ok(Self {
            model,
            fields,
            sort_field_override: None,
            tags: Vec::new(),
            guid_override: None,
            due: 0,
            cards: None,
        })
    }

    /// The note type this note belongs to.
    #[must_use]
    pub fn model(&self) -> &Model {
        &self.model
    }

    /// The shared model handle (cheap `Arc` clone), used by the Phase 4
    /// writers to auto-register note models on the deck.
    #[must_use]
    pub(crate) fn model_arc(&self) -> Arc<Model> {
        Arc::clone(&self.model)
    }

    /// Field values in model ordinal order.
    #[must_use]
    pub fn fields(&self) -> &[String] {
        &self.fields
    }

    /// Tags on this note.
    #[must_use]
    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    /// Consume and set the note's tags (validated).
    ///
    /// Errors with [`Error::TagContainsSpace`] if any tag contains U+0020.
    ///
    /// # Notes
    ///
    /// On validation failure this method **consumes** `self` (standard
    /// fallible builder style): the note is dropped and the error returned.
    /// Prefer [`Self::set_tags`] (or [`Self::add_tag`]) when you must keep
    /// the note across a failed call:
    ///
    /// ```
    /// # use genanki::{Field, Model, Note, Template};
    /// # let model = Model::new(1, "m")
    /// #     .field(Field::new("Q")).field(Field::new("A"))
    /// #     .template(Template::new("c", "{{Q}}", "{{A}}"));
    /// let mut note = Note::new(model, ["Q", "A"]).unwrap();
    /// assert!(note.set_tags(["bad tag"]).is_err());
    /// assert_eq!(note.fields(), ["Q", "A"]); // note retained
    /// ```
    pub fn with_tags(self, tags: impl IntoIterator<Item = impl Into<String>>) -> Result<Self> {
        let tags = collect_and_validate_tags(tags)?;
        Ok(Self { tags, ..self })
    }

    /// Replace all tags (validated).
    ///
    /// Errors with [`Error::TagContainsSpace`] if any tag contains U+0020.
    pub fn set_tags(&mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Result<()> {
        self.tags = collect_and_validate_tags(tags)?;
        Ok(())
    }

    /// Append a single tag (validated).
    ///
    /// Errors with [`Error::TagContainsSpace`] if the tag contains U+0020.
    pub fn add_tag(&mut self, tag: impl Into<String>) -> Result<()> {
        let tag = tag.into();
        validate_tag(&tag)?;
        self.tags.push(tag);
        Ok(())
    }

    /// Replace the tag at `index` (validated). Panics if `index` is out of
    /// bounds (mirroring Python list `__setitem__` raising `IndexError`).
    pub fn set_tag(&mut self, index: usize, tag: impl Into<String>) -> Result<()> {
        let tag = tag.into();
        validate_tag(&tag)?;
        self.tags[index] = tag;
        Ok(())
    }

    /// Append several tags (validated atomically).
    ///
    /// Errors with [`Error::TagContainsSpace`] if any tag contains U+0020;
    /// on error no tags are appended.
    pub fn extend_tags(&mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Result<()> {
        let tags = collect_and_validate_tags(tags)?;
        self.tags.extend(tags);
        Ok(())
    }

    /// Insert a tag at `index` (validated).
    ///
    /// Errors with [`Error::TagContainsSpace`] if the tag contains U+0020.
    ///
    /// Panics if `index > self.tags.len()` (Rust `Vec::insert`).
    /// `index == len` appends. This is **not** Python `list.insert` OOB
    /// clamping; Python never raises on insert bounds.
    pub fn insert_tag(&mut self, index: usize, tag: impl Into<String>) -> Result<()> {
        let tag = tag.into();
        validate_tag(&tag)?;
        self.tags.insert(index, tag);
        Ok(())
    }

    /// Consume and set an explicit GUID override.
    #[must_use]
    pub fn with_guid(mut self, guid: impl Into<String>) -> Self {
        self.guid_override = Some(guid.into());
        self
    }

    /// Consume and set an explicit sort-field override.
    #[must_use]
    pub fn with_sort_field(mut self, sort_field: impl Into<String>) -> Self {
        self.sort_field_override = Some(sort_field.into());
        self
    }

    /// Consume and set the scheduling due value.
    #[must_use]
    pub fn with_due(mut self, due: i64) -> Self {
        self.due = due;
        self
    }

    /// The note GUID: the override if set, else `guid_for(fields)`.
    #[must_use]
    pub fn guid(&self) -> String {
        if let Some(g) = &self.guid_override {
            return g.clone();
        }
        let refs: Vec<&str> = self.fields.iter().map(String::as_str).collect();
        crate::guid_for(&refs)
    }

    /// The sort-field value: the override if set, else
    /// `fields[model.sort_field_index]` (falling back to `fields[0]` when the
    /// index is out of bounds, which a well-formed model should not produce).
    ///
    /// Negative `model.sort_field_index` values are clamped to `0` (Python
    /// would apply negative indexing; well-formed Anki models use
    /// non-negative indices).
    #[must_use]
    pub fn sort_field(&self) -> &str {
        if let Some(sf) = &self.sort_field_override {
            return sf;
        }
        let idx = self.model.sort_field_index.max(0) as usize;
        debug_assert!(idx < self.fields.len(), "sort_field_index out of bounds");
        self.fields
            .get(idx)
            .or_else(|| self.fields.first())
            .map_or("", String::as_str)
    }

    /// The scheduling due value.
    #[must_use]
    pub fn due(&self) -> i64 {
        self.due
    }

    /// Replace the GUID override; `None` restores the computed default.
    pub fn set_guid(&mut self, guid: Option<String>) {
        self.guid_override = guid;
    }

    /// Replace the sort-field override; `None` restores the model-derived default.
    pub fn set_sort_field(&mut self, sort_field: Option<String>) {
        self.sort_field_override = sort_field;
    }

    /// Replace the scheduling due value.
    pub fn set_due(&mut self, due: i64) {
        self.due = due;
    }

    /// Ensure the card cache is computed and return it as a shared slice.
    ///
    /// Front/back models generate one card per `req` entry whose required
    /// fields are satisfied; cloze models generate one card per unique cloze
    /// reference `N` (ord `N-1`). Errors propagate from `model.req()`.
    pub fn cards(&mut self) -> Result<&[crate::card::Card]> {
        if self.cards.is_none() {
            self.cards = Some(self.compute_cards()?);
        }
        Ok(self.cards.as_deref().expect("cache filled above"))
    }

    /// Ensure the card cache is computed and return it mutably, e.g. to set
    /// `cards_mut()[i].suspend = true` (mirrors Python's in-place
    /// `note.cards[i].suspend = True`).
    pub fn cards_mut(&mut self) -> Result<&mut Vec<crate::card::Card>> {
        if self.cards.is_none() {
            self.cards = Some(self.compute_cards()?);
        }
        Ok(self.cards.as_mut().expect("cache filled above"))
    }

    /// Replace all field values (validated), invalidating the card cache.
    ///
    /// Errors with [`Error::FieldCountMismatch`] on count mismatch; emits the
    /// non-fatal invalid-HTML warning per dirty field. Any cached cards are
    /// discarded (including custom `suspend` flags), matching the recomputed
    /// semantics of Python's `cached_property`.
    pub fn set_fields(
        &mut self,
        fields: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<()> {
        let fields: Vec<String> = fields.into_iter().map(Into::into).collect();
        check_field_count(&self.model, &fields)?;
        for field in &fields {
            warn_invalid_html(field);
        }
        self.fields = fields;
        self.cards = None;
        Ok(())
    }
}

impl Note {
    /// Resolve the card list without mutating the cache: the cached cards if
    /// present (preserving custom `suspend` flags), else a fresh compute.
    ///
    /// The Phase 4 writers need this on `&self` because `Deck::write_to_file`
    /// takes the deck by shared reference while notes may already hold a
    /// suspended card cache.
    pub(crate) fn resolved_cards(&self) -> Result<Vec<crate::card::Card>> {
        if let Some(c) = &self.cards {
            return Ok(c.clone());
        }
        self.compute_cards()
    }

    /// Compute cards for the current fields, dispatching on model type.
    fn compute_cards(&self) -> Result<Vec<crate::card::Card>> {
        match self.model.model_type {
            crate::model::ModelType::FrontBack => self.front_back_cards(),
            crate::model::ModelType::Cloze => self.cloze_cards(),
        }
    }
    /// Front/back cards: one per `model.req()` entry whose required fields
    /// are all (or any, per the entry kind) non-empty. Python parity: only
    /// `""` is falsy; whitespace-only fields are non-empty.
    fn front_back_cards(&self) -> Result<Vec<crate::card::Card>> {
        let req = self.model.req()?;
        let mut rv = Vec::new();
        for entry in req {
            let nonempty = |ord: &u32| !self.fields[*ord as usize].is_empty();
            let include = match entry.kind {
                crate::req::ReqKind::All => entry.field_ords.iter().all(nonempty),
                crate::req::ReqKind::Any => entry.field_ords.iter().any(nonempty),
            };
            if include {
                rv.push(crate::card::Card::new(entry.template_ord as i32));
            }
        }
        Ok(rv)
    }

    /// Cloze cards: one card per unique cloze reference `N` (ord `N-1`),
    /// defaulting to a single `ord = 0` card when no references exist.
    ///
    /// Mirrors Python `Note._cloze_cards` (v0.13.1): field names come from
    /// the first template's `qfmt` (`{{...cloze:Name}}` plus the legacy
    /// `<%cloze:Name%>` form); each named field is scanned with DOTALL for
    /// `{{cN::...}}` (hints `::hint` included); ords are deduplicated and
    /// sorted ascending.
    fn cloze_cards(&self) -> Result<Vec<crate::card::Card>> {
        let qfmt = &self
            .model
            .templates
            .first()
            .ok_or(Error::Internal("cloze model has no templates"))?
            .qfmt;

        // Unique field names referenced by cloze filters in the qfmt.
        let mut names: Vec<String> = Vec::new();
        for m in cloze_field_name_re().captures_iter(qfmt) {
            let name = m[1].to_string();
            if !names.contains(&name) {
                names.push(name);
            }
        }
        for m in cloze_field_name_old_re().captures_iter(qfmt) {
            let name = m[1].to_string();
            if !names.contains(&name) {
                names.push(name);
            }
        }

        let mut ords: Vec<i32> = Vec::new();
        for name in names {
            let value = self
                .model
                .fields
                .iter()
                .position(|f| f.name == name)
                .map_or("", |idx| self.fields[idx].as_str());
            for m in cloze_ord_re().captures_iter(value) {
                let Ok(n) = m[1].parse::<i64>() else {
                    continue;
                };
                if n <= 0 {
                    continue;
                }
                let Ok(ord) = i32::try_from(n - 1) else {
                    continue;
                };
                if !ords.contains(&ord) {
                    ords.push(ord);
                }
            }
        }

        if ords.is_empty() {
            ords.push(0);
        }
        ords.sort_unstable();
        Ok(ords.into_iter().map(crate::card::Card::new).collect())
    }
}

/// `{{...cloze:Name}}` - cloze filter field names in a qfmt.
static CLOZE_FIELD_NAME_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{\{[^}]*?cloze:(?:[^}]?:)*(.+?)}}").expect("known-good cloze field name regex")
});

fn cloze_field_name_re() -> &'static Regex {
    &CLOZE_FIELD_NAME_RE
}

/// `<%cloze:Name%>` - legacy cloze filter field names in a qfmt.
static CLOZE_FIELD_NAME_OLD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<%cloze:(.+?)%>").expect("known-good legacy cloze regex"));

fn cloze_field_name_old_re() -> &'static Regex {
    &CLOZE_FIELD_NAME_OLD_RE
}

/// `{{cN::...}}` with DOTALL - cloze ord references inside field values.
static CLOZE_ORD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)\{\{c(\d+)::.+?}}").expect("known-good cloze ord regex"));

fn cloze_ord_re() -> &'static Regex {
    &CLOZE_ORD_RE
}

/// Collect tags from any iterator and validate every element.
fn collect_and_validate_tags(
    tags: impl IntoIterator<Item = impl Into<String>>,
) -> Result<Vec<String>> {
    let tags: Vec<String> = tags.into_iter().map(Into::into).collect();
    for tag in &tags {
        validate_tag(tag)?;
    }
    Ok(tags)
}

/// Reject tags containing U+0020 (matching Python `' ' in tag`).
fn validate_tag(tag: &str) -> Result<()> {
    if tag.contains(' ') {
        return Err(Error::TagContainsSpace {
            tag: tag.to_string(),
        });
    }
    Ok(())
}

// --- Invalid-HTML tag scanner ---
//
// Python genanki v0.13.1 uses one lookahead regex:
//   r'<(?!/?[a-zA-Z0-9]+(?: .*|/?)>|!--|!\[CDATA\[)(?:.|\n)*?>'
// The `regex` crate has no lookahead, so the plan specifies an equivalent
// two-pass scan (verified byte-for-byte against Python `findall` on all
// goldens, including the issue-28 LaTeX and CDATA/comment cases):
//   1. find every tag with `(?s)<.*?>`;
//   2. a tag is valid when its body (everything after `<`, including the
//      closing `>`) matches `^/?[a-zA-Z0-9]+(?: .*|/?)>$` or starts with
//      `!--` or `![CDATA[`; otherwise it is invalid.

/// `(?s)<.*?>` - every tag from `<` to the first `>`, spanning newlines.
static TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<.*?>").expect("known-good tag regex"));

fn tag_re() -> &'static Regex {
    &TAG_RE
}

/// `^/?[a-zA-Z0-9]+(?: .*|/?)>$` applied to a tag body that includes the
/// closing `>`. No DOTALL: `.*` must not span newlines (Python parity).
static VALID_BODY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^/?[a-zA-Z0-9]+(?: .*|/?)>$").expect("known-good body regex"));

fn valid_body_re() -> &'static Regex {
    &VALID_BODY_RE
}

/// Return the invalid HTML tag substrings in `field` (Python findall parity).
///
/// A tag is valid if its body (after `<`, including `>`) is `[a-zA-Z0-9]+`
/// with optional leading `/` and optional trailing `/`, optionally followed
/// by a space and attributes, or starts with `!--` (HTML comment) or
/// `![CDATA[` (CDATA section). Everything else is reported as invalid.
#[must_use]
pub fn find_invalid_html_tags(field: &str) -> Vec<String> {
    let mut invalid = Vec::new();
    for m in tag_re().find_iter(field) {
        let tag = m.as_str();
        let body = &tag[1..];
        if valid_body_re().is_match(body) || body.starts_with("!--") || body.starts_with("![CDATA[")
        {
            continue;
        }
        invalid.push(tag.to_string());
    }
    invalid
}

/// Non-fatal warning for a field containing invalid HTML tags.
///
/// Message mirrors Python genanki's `warnings.warn(...)` text from
/// `Note._check_invalid_html_tags_in_fields`.
fn warn_invalid_html(field: &str) {
    let invalid = find_invalid_html_tags(field);
    if !invalid.is_empty() {
        log::warn!(
            "Field contained the following invalid HTML tags. Make sure you are calling html.escape() if your field data isn't already HTML-encoded: {}",
            invalid.join(" ")
        );
    }
}

/// Join fields with the Anki unit-separator (`\x1f`) - the `flds` DB column
/// format Python genanki produces from `Note._format_fields`.
///
/// Consumed by the Phase 4 DB writers (issue #6).
#[must_use]
pub(crate) fn format_fields(fields: &[String]) -> String {
    fields.join("\x1f")
}

/// Tags joined with single spaces and wrapped in spaces - the `tags` DB
/// column format Python genanki produces from `Note._format_tags`.
///
/// Consumed by the Phase 4 DB writers (issue #6).
#[must_use]
pub(crate) fn format_tags(tags: &[String]) -> String {
    format!(" {} ", tags.join(" "))
}

/// Fail-fast field-count check shared by `new` and `set_fields`.
fn check_field_count(model: &Model, fields: &[String]) -> Result<()> {
    if fields.len() != model.fields.len() {
        return Err(Error::FieldCountMismatch {
            model_name: model.name.clone(),
            model_fields: model.fields.len(),
            note_fields: fields.len(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Field, Model, Template};

    fn simple_model() -> Model {
        Model::new(1376484377, "Simple Model")
            .field(Field::new("Question"))
            .field(Field::new("Answer"))
            .template(Template::new(
                "Card 1",
                "{{Question}}",
                "{{FrontSide}}<hr id=\"answer\">{{Answer}}",
            ))
    }

    #[test]
    fn new_matches_field_count_ok() {
        let m = simple_model();
        let n = Note::new(m, ["Capital of Argentina", "Buenos Aires"]).unwrap();
        assert_eq!(n.model().name, "Simple Model");
        assert_eq!(n.fields(), ["Capital of Argentina", "Buenos Aires"]);
    }

    #[test]
    fn new_accepts_arc_model() {
        let n = Note::new(std::sync::Arc::new(simple_model()), ["a", "b"]).unwrap();
        assert_eq!(n.model().name, "Simple Model");
    }

    #[test]
    fn new_too_few_fields_errors() {
        let m = Model::new(1894808898, "Test Model")
            .field(Field::new("Question"))
            .field(Field::new("Answer"))
            .field(Field::new("Extra"))
            .template(Template::new("Card 1", "{{Question}}", "{{Answer}}"));
        let err = Note::new(m, ["Q?", "A"]).unwrap_err();
        match err {
            Error::FieldCountMismatch {
                model_name,
                model_fields,
                note_fields,
            } => {
                assert_eq!(model_name, "Test Model");
                assert_eq!(model_fields, 3);
                assert_eq!(note_fields, 2);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn new_too_many_fields_errors() {
        let m = simple_model();
        let err = Note::new(m, ["Q?", "A", "extra"]).unwrap_err();
        assert!(matches!(
            err,
            Error::FieldCountMismatch {
                model_fields: 2,
                note_fields: 3,
                ..
            }
        ));
    }

    // --- Tags (T3) ---

    #[test]
    fn with_tags_ok_and_space_errors() {
        let m = simple_model();
        let n = Note::new(m, ["Q", "A"])
            .unwrap()
            .with_tags(["foo", "bar"])
            .unwrap();
        assert_eq!(n.tags(), ["foo", "bar"]);
        let err = Note::new(simple_model(), ["Q", "A"])
            .unwrap()
            .with_tags(["foo", "b ar"])
            .unwrap_err();
        assert!(matches!(err, Error::TagContainsSpace { ref tag } if tag == "b ar"));
    }

    #[test]
    fn set_tags_ok_and_space_errors() {
        let mut n = Note::new(simple_model(), ["Q", "A"]).unwrap();
        n.set_tags(["foo", "bar"]).unwrap();
        assert_eq!(n.tags(), ["foo", "bar"]);
        let err = n.set_tags(["foo", " baz"]).unwrap_err();
        assert!(matches!(err, Error::TagContainsSpace { .. }));
    }

    #[test]
    fn add_tag_ok_and_space_errors() {
        let mut n = Note::new(simple_model(), ["Q", "A"])
            .unwrap()
            .with_tags(["foo", "bar"])
            .unwrap();
        n.add_tag("baz").unwrap();
        assert_eq!(n.tags(), ["foo", "bar", "baz"]);
        let err = n.add_tag("king dedede").unwrap_err();
        assert!(matches!(err, Error::TagContainsSpace { .. }));
    }

    #[test]
    fn set_tag_ok_and_space_errors() {
        let mut n = Note::new(simple_model(), ["Q", "A"])
            .unwrap()
            .with_tags(["foo", "bar", "baz"])
            .unwrap();
        n.set_tag(0, "dankey_kang").unwrap();
        assert_eq!(n.tags(), ["dankey_kang", "bar", "baz"]);
        let err = n.set_tag(1, "dankey kang").unwrap_err();
        assert!(matches!(err, Error::TagContainsSpace { .. }));
        assert_eq!(n.tags(), ["dankey_kang", "bar", "baz"]); // unchanged
    }

    #[test]
    fn extend_tags_ok_and_space_errors() {
        let mut n = Note::new(simple_model(), ["Q", "A"])
            .unwrap()
            .with_tags(["foo", "bar"])
            .unwrap();
        n.extend_tags(["palu", "wolf"]).unwrap();
        assert_eq!(n.tags(), ["foo", "bar", "palu", "wolf"]);
        let err = n.extend_tags(["dat fox doe"]).unwrap_err();
        assert!(matches!(err, Error::TagContainsSpace { .. }));
    }

    #[test]
    fn insert_tag_ok_and_space_errors() {
        let mut n = Note::new(simple_model(), ["Q", "A"])
            .unwrap()
            .with_tags(["foo", "bar", "baz"])
            .unwrap();
        n.insert_tag(0, "lucina").unwrap();
        assert_eq!(n.tags(), ["lucina", "foo", "bar", "baz"]);
        let err = n.insert_tag(0, "nerf joker pls").unwrap_err();
        assert!(matches!(err, Error::TagContainsSpace { .. }));
    }

    // --- GUID / sort field / due (T4-T6) ---

    #[test]
    fn guid_defaults_to_guid_for_fields() {
        let m = simple_model();
        let n = Note::new(m, ["Capital of Argentina", "Buenos Aires"]).unwrap();
        assert_eq!(
            n.guid(),
            crate::guid_for(&["Capital of Argentina", "Buenos Aires"])
        );
        assert_eq!(n.guid(), "HSnG{z%dU<"); // precomputed golden
    }

    #[test]
    fn guid_override_and_clear() {
        let m = simple_model();
        let mut n = Note::new(m, ["Capital of Argentina", "Buenos Aires"])
            .unwrap()
            .with_guid("custom");
        assert_eq!(n.guid(), "custom");
        n.set_guid(None);
        assert_eq!(n.guid(), "HSnG{z%dU<");
    }

    #[test]
    fn sort_field_default_index_zero() {
        let m = simple_model();
        let n = Note::new(m, ["Q-field", "A-field"]).unwrap();
        assert_eq!(n.sort_field(), "Q-field");
    }

    #[test]
    fn sort_field_from_model_index() {
        let m = Model::new(987123, "with sort field index")
            .field(Field::new("AField"))
            .field(Field::new("BField"))
            .template(Template::new("card1", "{{AField}}", "{{BField}}"))
            .sort_field_index(1);
        let n = Note::new(m, ["a", "b"]).unwrap();
        assert_eq!(n.sort_field(), "b");
    }

    #[test]
    fn sort_field_override() {
        let m = simple_model();
        let mut n = Note::new(m, ["Q-field", "A-field"])
            .unwrap()
            .with_sort_field("x");
        assert_eq!(n.sort_field(), "x");
        n.set_sort_field(None);
        assert_eq!(n.sort_field(), "Q-field");
    }

    #[test]
    fn due_default_and_set() {
        let m = simple_model();
        let n = Note::new(m, ["Q", "A"]).unwrap();
        assert_eq!(n.due(), 0);
        let n = n.with_due(42);
        assert_eq!(n.due(), 42);
        let mut n = n;
        n.set_due(-7);
        assert_eq!(n.due(), -7);
    }

    // --- Invalid-HTML scanner goldens (T7) ---

    #[test]
    fn html_ok_tags() {
        assert_eq!(find_invalid_html_tags("<h1>"), Vec::<String>::new());
        assert_eq!(find_invalid_html_tags(" <h1> "), Vec::<String>::new());
        assert_eq!(
            find_invalid_html_tags("<h1>test</h1>"),
            Vec::<String>::new()
        );
        assert_eq!(find_invalid_html_tags("<br>"), Vec::<String>::new());
        assert_eq!(find_invalid_html_tags("<br/>"), Vec::<String>::new());
        assert_eq!(find_invalid_html_tags("<br />"), Vec::<String>::new());
        assert_eq!(
            find_invalid_html_tags("<h1 style=\"color: red\">STOP</h1>"),
            Vec::<String>::new()
        );
        assert_eq!(find_invalid_html_tags("<TD></Td>"), Vec::<String>::new());
    }

    #[test]
    fn html_invalid_tags() {
        assert_eq!(find_invalid_html_tags(" hello <> goodbye"), ["<>"]);
        assert_eq!(find_invalid_html_tags(" hello < > goodbye"), ["< >"]);
        assert_eq!(find_invalid_html_tags("<@h1>"), ["<@h1>"]);
        assert_eq!(find_invalid_html_tags("<h1@>"), ["<h1@>"]);
    }

    #[test]
    fn html_comments_and_cdata_ok() {
        assert_eq!(
            find_invalid_html_tags("<!-- here is a comment -->"),
            Vec::<String>::new()
        );
        assert_eq!(
            find_invalid_html_tags("<![CDATA[ here is some cdata ]]>"),
            Vec::<String>::new()
        );
        assert_eq!(
            find_invalid_html_tags("<![CDATA[multi\nline cdata]]>"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn html_issue_28_latex_golden() {
        let latex = "[latex]\n\\schemestart\n\\chemfig{*6(--(<OH)-(<:Br)---)}\n\\arrow{->[?]}\n\\chemfig{*6(--(<[:30]{O}?)(<:H)-?[,{>},](<:H)---)}\n\\schemestop\n[/latex]";
        assert_eq!(
            find_invalid_html_tags(latex),
            ["<OH)-(<:Br)---)}\n\\arrow{->", "<[:30]{O}?)(<:H)-?[,{>",]
        );
    }

    // --- HTML warn wiring (T8) ---

    /// Minimal capturing `log::Log`; no extra dev-dependency (plan sec. 3.1).
    struct CaptureLogger;
    static LOGS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
    static LOGGER: CaptureLogger = CaptureLogger;

    impl log::Log for CaptureLogger {
        fn enabled(&self, metadata: &log::Metadata) -> bool {
            metadata.level() <= log::Level::Warn
        }

        fn log(&self, record: &log::Record) {
            if self.enabled(record.metadata()) {
                LOGS.lock().unwrap().push(format!("{}", record.args()));
            }
        }

        fn flush(&self) {}
    }

    /// Install the capture logger once and serialize warn-capture tests.
    fn install_capture_logger() -> std::sync::MutexGuard<'static, ()> {
        static SETUP: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let guard = SETUP.lock().unwrap();
        if log::set_logger(&LOGGER).is_ok() {
            log::set_max_level(log::LevelFilter::Warn);
        }
        LOGS.lock().unwrap().clear();
        guard
    }

    #[test]
    fn warns_on_invalid_html_at_construct() {
        let _guard = install_capture_logger();
        let n = Note::new(simple_model(), ["Capital of <$> Argentina", "Buenos Aires"]).unwrap();
        assert_eq!(n.fields()[0], "Capital of <$> Argentina");
        let logs = LOGS.lock().unwrap().clone();
        assert_eq!(logs.len(), 1);
        assert!(logs[0].contains("invalid HTML tags"));
        assert!(logs[0].contains("<$>"));
        assert!(
            logs[0].contains("html.escape()"),
            "warn text must match Python prose: {}",
            logs[0]
        );
        assert!(
            logs[0].contains("isn't"),
            "warn text must match Python prose: {}",
            logs[0]
        );
    }

    #[test]
    fn guid_override_not_scanned_for_html() {
        let _guard = install_capture_logger();
        let n = Note::new(simple_model(), ["Capital of Argentina", "Buenos Aires"])
            .unwrap()
            .with_guid("<@h1>not-a-field");
        assert_eq!(n.guid(), "<@h1>not-a-field");
        let logs = LOGS.lock().unwrap().clone();
        assert!(logs.is_empty(), "guid must never be scanned: {logs:?}");
    }

    // --- Front/back card generation (T9) ---

    fn cn_model() -> Model {
        Model::new(345678, "Chinese")
            .field(Field::new("Traditional"))
            .field(Field::new("Simplified"))
            .field(Field::new("English"))
            .template(Template::new(
                "Traditional",
                "{{Traditional}}",
                "{{FrontSide}}<hr id=\"answer\">{{English}}",
            ))
            .template(Template::new(
                "Simplified",
                "{{Simplified}}",
                "{{FrontSide}}<hr id=\"answer\">{{English}}",
            ))
    }

    fn hint_model() -> Model {
        Model::new(456789, "with hint")
            .field(Field::new("Question"))
            .field(Field::new("Hint"))
            .field(Field::new("Answer"))
            .template(Template::new(
                "card1",
                "{{Question}}{{#Hint}}<br>Hint: {{Hint}}{{/Hint}}",
                "{{Answer}}",
            ))
    }

    #[test]
    fn front_back_cards_cn_model() {
        // Both fields present -> both cards.
        let mut n1 = Note::new(cn_model(), ["中國", "中国", "China"]).unwrap();
        let cards = n1.cards().unwrap();
        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].ord, 0);
        assert_eq!(cards[1].ord, 1);

        // Simplified empty -> only the Traditional card.
        let mut n2 = Note::new(cn_model(), ["你好", "", "hello"]).unwrap();
        let cards = n2.cards().unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].ord, 0);
    }

    #[test]
    fn front_back_cards_hint_model() {
        // Q present, Hint empty -> card (any).
        let mut n1 = Note::new(hint_model(), ["capital of California", "", "Sacramento"]).unwrap();
        let cards = n1.cards().unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].ord, 0);

        // Q and Hint present -> card.
        let mut n2 = Note::new(
            hint_model(),
            ["capital of Iowa", "French for \"The Moines\"", "Des Moines"],
        )
        .unwrap();
        let cards = n2.cards().unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].ord, 0);
    }

    #[test]
    fn whitespace_only_field_is_nonempty() {
        // Python truthiness: only "" is falsy; " " is truthy.
        let mut n = Note::new(cn_model(), [" ", "", "x"]).unwrap();
        let cards = n.cards().unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].ord, 0);
    }

    // --- Cloze card generation (T10) ---

    fn cloze_model() -> Model {
        Model::new(998877661, "My Cloze Model")
            .model_type(crate::model::ModelType::Cloze)
            .field(Field::new("Text"))
            .field(Field::new("Extra"))
            .template(Template::new(
                "My Cloze Card",
                "{{cloze:Text}}",
                "{{cloze:Text}}<br>{{Extra}}",
            ))
    }

    fn multi_field_cloze_model() -> Model {
        Model::new(1047194615, "Multi Field Cloze Model")
            .model_type(crate::model::ModelType::Cloze)
            .field(Field::new("Text1"))
            .field(Field::new("Text2"))
            .template(Template::new(
                "Cloze",
                "{{cloze:Text1}} and {{cloze:Text2}}",
                "{{cloze:Text1}} and {{cloze:Text2}}",
            ))
    }

    fn cloze_ords(fields: &[&str]) -> Vec<i32> {
        let mut n = Note::new(cloze_model(), fields.iter().copied()).unwrap();
        n.cards().unwrap().iter().map(|c| c.ord).collect()
    }

    #[test]
    fn cloze_single_deletion() {
        assert_eq!(cloze_ords(&["NOTE ONE: {{c1::single deletion}}", ""]), [0]);
    }

    #[test]
    fn cloze_three_deletions() {
        assert_eq!(
            cloze_ords(&[
                "NOTE TWO: {{c1::1st deletion}} {{c2::2nd deletion}} {{c3::3rd deletion}}",
                ""
            ]),
            [0, 1, 2]
        );
    }

    #[test]
    fn cloze_hint_deletion() {
        assert_eq!(
            cloze_ords(&["NOTE THREE: {{c1::1st deletion::C1-CLOZE}}", ""]),
            [0]
        );
    }

    #[test]
    fn cloze_repeated_reference_dedupes() {
        assert_eq!(
            cloze_ords(&[
                "NOTE FOUR: {{c1::1st deletion}} foo {{c2::2nd deletion}} bar {{c1::3rd deletion}}",
                ""
            ]),
            [0, 1]
        );
    }

    #[test]
    fn cloze_multi_field_union() {
        let fields = [
            "{{c1::Berlin}} is the capital of {{c2::Germany}}",
            "{{c3::Paris}} is the capital of {{c4::France}}",
        ];
        let mut n = Note::new(multi_field_cloze_model(), fields).unwrap();
        let ords: Vec<i32> = n.cards().unwrap().iter().map(|c| c.ord).collect();
        assert_eq!(ords, [0, 1, 2, 3]);
    }

    #[test]
    fn cloze_indices_do_not_start_at_one() {
        assert_eq!(
            cloze_ords(&[
                "{{c2::Mitochondria}} are the {{c3::powerhouses}} of the cell",
                ""
            ]),
            [1, 2]
        );
    }

    #[test]
    fn cloze_newlines_in_deletion() {
        assert_eq!(
            cloze_ords(&[
                "{{c1::Washington, D.C.}} is the capital of {{c2::the\nUnited States\nof America}}",
                ""
            ]),
            [0, 1]
        );
    }

    #[test]
    fn cloze_no_markers_defaults_to_zero() {
        assert_eq!(cloze_ords(&["no cloze markers at all", ""]), [0]);
    }

    #[test]
    fn cloze_ignores_missing_named_field() {
        // A qfmt referencing a cloze field the model lacks: Python looks up
        // the index, finds -1, and treats the value as empty.
        let m = Model::new(1, "c")
            .model_type(crate::model::ModelType::Cloze)
            .field(Field::new("Text"))
            .template(Template::new(
                "Cloze",
                "{{cloze:Text}} {{cloze:Missing}}",
                "x",
            ));
        let mut n = Note::new(m, ["{{c1::Berlin}}"]).unwrap();
        let ords: Vec<i32> = n.cards().unwrap().iter().map(|c| c.ord).collect();
        assert_eq!(ords, [0]);
    }

    #[test]
    fn cloze_hint_prefixed_and_legacy_qfmt_forms() {
        // `{{hint:cloze:Text}}` resolves to field Text (Python parity for
        // `(?:[^}]?:)*`); the legacy `<%cloze:Text%>` form also names the
        // field. Both still produce cards from cN references in the value.
        for qfmt in ["{{hint:cloze:Text}}", "<%cloze:Text%>"] {
            let m = Model::new(1, "c")
                .model_type(crate::model::ModelType::Cloze)
                .field(Field::new("Text"))
                .template(Template::new("Cloze", qfmt, "x"));
            let mut n = Note::new(m, ["{{c1::Berlin}} {{c2::Paris}}"]).unwrap();
            let ords: Vec<i32> = n.cards().unwrap().iter().map(|c| c.ord).collect();
            assert_eq!(ords, [0, 1], "qfmt: {qfmt}");
        }
    }

    #[test]
    fn cloze_oversized_ord_is_skipped_not_panic() {
        // 40 nines: parses as digits but overflows i32/i64 ord path; must not panic.
        let huge = format!("{{{{c{}::x}}}}", "9".repeat(40));
        assert_eq!(cloze_ords(&[huge.as_str(), ""]), [0]); // overflow-only -> default 0
    }

    #[test]
    fn cloze_oversized_ord_does_not_drop_valid_siblings() {
        let fields = format!("{{{{c1::ok}}}} {{{{c{}::nope}}}}", "9".repeat(40));
        assert_eq!(cloze_ords(&[fields.as_str(), ""]), [0]);
    }

    #[test]
    fn cloze_ord_just_inside_i32_max_is_kept() {
        // n - 1 == i32::MAX  =>  n == i32::MAX as i64 + 1
        let n = i32::MAX as i64 + 1;
        let field = format!("{{{{c{n}::edge}}}}");
        assert_eq!(cloze_ords(&[field.as_str(), ""]), [i32::MAX]);
    }

    #[test]
    fn cloze_ord_just_outside_i32_max_is_skipped() {
        // n - 1 == i32::MAX as i64 + 1  => does not fit i32
        let n = i32::MAX as i64 + 2;
        let field = format!("{{{{c{n}::edge}}}}");
        assert_eq!(cloze_ords(&[field.as_str(), ""]), [0]);
    }

    // --- Cache, suspend, invalidation (T11) ---

    #[test]
    fn suspend_on_cached_cards_and_invalidate_on_set_fields() {
        let mut note = Note::new(cn_model(), ["中國", "中国", "China"]).unwrap();
        assert_eq!(note.cards().unwrap().len(), 2);
        note.cards_mut().unwrap()[1].suspend = true;
        assert!(note.cards().unwrap()[1].suspend);

        // Same content still invalidates: cards are recomputed fresh and the
        // custom suspend flag is cleared (documented cache semantics).
        note.set_fields(["中國", "中国", "China"]).unwrap();
        assert!(!note.cards().unwrap()[1].suspend);
    }

    #[test]
    fn resolved_cards_prefers_cache_and_computes_without_mutating() {
        let mut note = Note::new(cn_model(), ["中國", "中国", "China"]).unwrap();

        // No cache yet: computes on &self and does not populate the cache.
        let cards = note.resolved_cards().unwrap();
        assert_eq!(cards.len(), 2);
        assert!(!cards[1].suspend);

        // Suspend via the mutating cache path; resolved_cards must keep it.
        note.cards_mut().unwrap()[1].suspend = true;
        let cards = note.resolved_cards().unwrap();
        assert!(cards[1].suspend, "must prefer cached suspended cards");
    }

    #[test]
    fn set_fields_validates_count_and_warns_html() {
        let _guard = install_capture_logger();
        let mut note = Note::new(simple_model(), ["Q", "A"]).unwrap();
        let err = note.set_fields(["Q"]).unwrap_err();
        assert!(matches!(
            err,
            Error::FieldCountMismatch {
                model_fields: 2,
                note_fields: 1,
                ..
            }
        ));
        // Invalid HTML in the replacement triggers the non-fatal warn.
        note.set_fields(["Q <$> x", "A"]).unwrap();
        let logs = LOGS.lock().unwrap().clone();
        assert_eq!(logs.len(), 1);
        assert!(logs[0].contains("<$>"));
        assert!(
            logs[0].contains("html.escape()"),
            "warn text must match Python prose: {}",
            logs[0]
        );
        assert!(
            logs[0].contains("isn't"),
            "warn text must match Python prose: {}",
            logs[0]
        );
    }

    // --- Formatting helpers (T12) ---

    #[test]
    fn format_fields_joins_with_unit_separator() {
        let fields = vec!["a".to_string(), "b".to_string()];
        assert_eq!(format_fields(&fields), "a\x1fb");
        assert_eq!(format_fields(&[]), "");
    }

    #[test]
    fn format_tags_wraps_with_spaces() {
        let tags = vec!["foo".to_string(), "bar".to_string()];
        assert_eq!(format_tags(&tags), " foo bar ");
        assert_eq!(format_tags(&[]), "  ");
    }

    // --- Phase 6 hardening: hand-rolled property tests (issue #8) ---
    //
    // Deterministic PRNG + generators, zero new crates (plan sec. 4.1-4.3).
    // HTML properties call `find_invalid_html_tags` directly (pure, so no log
    // output); cloze properties build fields from alphabets without `<`/`>`
    // so `Note::new` never emits the warn path and cannot race the
    // log-capture tests above.

    /// Deterministic xorshift64* PRNG for property tests (not cryptographic).
    struct XorShift64(u64);

    impl XorShift64 {
        fn new(seed: u64) -> Self {
            Self(seed | 1) // avoid the all-zero state
        }
        fn next_u64(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            x
        }
        fn next_usize(&mut self, lo: usize, hi_exclusive: usize) -> usize {
            assert!(hi_exclusive > lo);
            let span = hi_exclusive - lo;
            lo + (self.next_u64() as usize % span)
        }
        fn next_bool(&mut self) -> bool {
            self.next_u64() & 1 == 1
        }
    }

    /// Iteration counts kept as constants so CI results are reproducible.
    const HTML_PROP_ITERS: usize = 500;
    const CLOZE_PROP_ITERS: usize = 200;

    /// Whole known-valid HTML tags for the whitelist oracle (plan sec. 4.2 #4).
    const HTML_VALID_TAGS: &[&str] = &[
        "<br>",
        "<br/>",
        "<br />",
        "<h1>",
        "</h1>",
        "<h1 style=\"x\">",
        "<td>",
        "</Td>",
        "<div class=\"box\">",
        "<img src=\"a.png\">",
        "<!-- comment -->",
        "<![CDATA[x]]>",
    ];

    /// Whole known-invalid tags for the blacklist oracle (plan sec. 4.2 #5).
    const HTML_BAD_TAGS: &[&str] = &["<>", "< >", "<@h1>", "<h1@>"];

    /// Wide text alphabet (may contain `<`/`>`/`{`/`}`) for the mixed mode.
    const TEXT_ALPHABET: &[char] = &[
        'a', 'b', 'c', 'd', 'e', 'f', ' ', '\n', '1', '2', '3', '(', ')', '[', ']', '.', ':', '!',
        '?', 'é', 'ü', '中', '文',
    ];

    /// Text without `<`/`>`/`{`/`}`: safe for the HTML oracles (cannot merge
    /// into tags) and the cloze marker oracle (cannot form `{{cN::`).
    const SAFE_ALPHABET: &[char] = &[
        'a', 'b', 'c', 'd', 'e', ' ', '\n', '1', '2', '3', '.', ',', '!', '?', 'é', 'ü',
    ];

    fn gen_text(rng: &mut XorShift64, max_len: usize, alphabet: &[char]) -> String {
        let len = rng.next_usize(0, max_len + 1);
        (0..len)
            .map(|_| alphabet[rng.next_usize(0, alphabet.len())])
            .collect()
    }

    enum HtmlMode {
        /// Safe text + whole valid tags only; expect zero invalid tags.
        Whitelist,
        /// Safe text + whole known-invalid tags only; expect exactly those.
        Blacklist,
        /// Everything: text, valid/invalid tags, bare `<`/`>`, `{{c` noise.
        Mixed,
    }

    fn gen_html_field(rng: &mut XorShift64, mode: HtmlMode, target: usize) -> String {
        let mut s = String::new();
        while s.len() < target {
            let piece: String = match mode {
                HtmlMode::Whitelist => {
                    if rng.next_bool() {
                        HTML_VALID_TAGS[rng.next_usize(0, HTML_VALID_TAGS.len())].to_string()
                    } else {
                        gen_text(rng, 12, SAFE_ALPHABET)
                    }
                }
                HtmlMode::Blacklist => {
                    if rng.next_bool() {
                        HTML_BAD_TAGS[rng.next_usize(0, HTML_BAD_TAGS.len())].to_string()
                    } else {
                        gen_text(rng, 12, SAFE_ALPHABET)
                    }
                }
                HtmlMode::Mixed => match rng.next_usize(0, 7) {
                    0 => gen_text(rng, 12, TEXT_ALPHABET),
                    1 => HTML_VALID_TAGS[rng.next_usize(0, HTML_VALID_TAGS.len())].to_string(),
                    2 => HTML_BAD_TAGS[rng.next_usize(0, HTML_BAD_TAGS.len())].to_string(),
                    3 => "<".to_string(),
                    4 => ">".to_string(),
                    5 => "{{c".to_string(),
                    _ => "}}".to_string(),
                },
            };
            s.push_str(&piece);
        }
        s
    }

    #[test]
    fn html_scanner_property_invariants() {
        let mut rng = XorShift64::new(0x5EED_BA5E_C0FF_EE01);
        for i in 0..HTML_PROP_ITERS {
            // Mostly short fields; a few up to ~4k to stress non-anchored scans.
            let target = if i % 20 == 0 { 4000 } else { 256 };
            let field = gen_html_field(&mut rng, HtmlMode::Mixed, target);
            let invalid = find_invalid_html_tags(&field);
            for t in &invalid {
                assert!(
                    t.starts_with('<') && t.ends_with('>'),
                    "reported tag has wrong shape: {t:?}"
                );
                assert!(field.contains(t), "scanner invented {t:?} for {field:?}");
                // Idempotent classification: re-scanning a reported tag
                // reproduces exactly itself (tags end at the first `>`).
                assert_eq!(
                    find_invalid_html_tags(t),
                    vec![t.clone()],
                    "reclassification of {t:?} is not idempotent"
                );
            }
            // Every reported tag must be one of the scanner's own tag matches
            // (the scanner never invents tags beyond `tag_re`).
            let found: Vec<String> = tag_re()
                .find_iter(&field)
                .map(|m| m.as_str().to_string())
                .collect();
            for t in &invalid {
                assert!(found.contains(t), "reported tag not a tag_re match: {t:?}");
            }
        }
    }

    #[test]
    fn html_scanner_property_whitelist_oracle() {
        let mut rng = XorShift64::new(0xBAD0_C0DE_5EED_0001);
        for i in 0..HTML_PROP_ITERS {
            let target = if i % 20 == 0 { 4000 } else { 256 };
            let field = gen_html_field(&mut rng, HtmlMode::Whitelist, target);
            assert_eq!(
                find_invalid_html_tags(&field),
                Vec::<String>::new(),
                "whitelist-only field reported invalid tags: {field:?}"
            );
        }
    }

    #[test]
    fn html_scanner_property_blacklist_oracle() {
        let mut rng = XorShift64::new(0xDEAD_BEEF_CAFE_0002);
        for _ in 0..HTML_PROP_ITERS {
            let field = gen_html_field(&mut rng, HtmlMode::Blacklist, 256);
            // Rebuild the expected list from the pieces: every `<` starts a
            // whole bad tag (safe text has no `<`/`>`), closed at the next `>`.
            let mut expected: Vec<String> = Vec::new();
            let mut rest = field.as_str();
            while let Some(pos) = rest.find('<') {
                let (head, tail) = rest.split_at(pos);
                assert!(
                    !head.contains('>'),
                    "safe text cannot contain '>': {field:?}"
                );
                let end = tail.find('>').expect("whole bad tag is closed") + 1;
                expected.push(tail[..end].to_string());
                rest = &tail[end..];
            }
            assert!(!rest.contains('>'), "trailing '>' without '<': {field:?}");
            assert_eq!(
                find_invalid_html_tags(&field),
                expected,
                "blacklist field mismatch: {field:?}"
            );
        }
    }

    /// Public-path ords for the two-field cloze model (plan sec. 4.3).
    fn cloze_ords_for(text: &str) -> Vec<i32> {
        let mut n = Note::new(cloze_model(), [text, ""]).unwrap();
        n.cards().unwrap().iter().map(|c| c.ord).collect()
    }

    /// Body text for constructed cloze markers: no `{`/`}`/`<`/`>` so it can
    /// neither form nested markers nor trigger the HTML warn path; newlines
    /// exercise the DOTALL ord regex.
    ///
    /// Bodies are forced **non-empty**: Python genanki's `.+?}}` ord regex
    /// requires at least one body char, so an empty body (`{{cN::}}`) makes
    /// the lazy match swallow following text up to the next `}}` (parity
    /// behavior pinned by `cloze_empty_body_swallows_following_text`); the
    /// constructed-marker oracle below therefore stays on well-formed bodies.
    fn gen_cloze_body(rng: &mut XorShift64) -> String {
        let s = gen_text(rng, 10, SAFE_ALPHABET);
        if s.is_empty() { "x".to_string() } else { s }
    }

    /// Build a field from plain text plus `{{cN::body}}` / `{{cN::body::hint}}`
    /// markers (`n` in 1..=32). Returns the field and its expected ords
    /// (sorted unique `n-1`; empty means the field has no markers).
    fn gen_cloze_field(rng: &mut XorShift64) -> (String, Vec<i32>) {
        let mut s = String::new();
        let mut ords: Vec<i32> = Vec::new();
        let pieces = rng.next_usize(1, 9);
        for _ in 0..pieces {
            match rng.next_usize(0, 4) {
                0 | 3 => s.push_str(&gen_text(rng, 8, SAFE_ALPHABET)),
                1 => {
                    let n = rng.next_usize(1, 33) as i64;
                    let ord = (n - 1) as i32;
                    let body = gen_cloze_body(rng);
                    s.push_str(&format!("{{{{c{n}::{body}}}}}"));
                    if !ords.contains(&ord) {
                        ords.push(ord);
                    }
                }
                _ => {
                    let n = rng.next_usize(1, 33) as i64;
                    let ord = (n - 1) as i32;
                    let body = gen_cloze_body(rng);
                    s.push_str(&format!("{{{{c{n}::{body}::{body}}}}}")); // hint form
                    if !ords.contains(&ord) {
                        ords.push(ord);
                    }
                }
            }
        }
        ords.sort_unstable();
        (s, ords)
    }

    #[test]
    fn cloze_property_constructed_markers() {
        let mut rng = XorShift64::new(0xC10E_5E20_2400_0001);
        for _ in 0..CLOZE_PROP_ITERS {
            let (text, mut expected) = gen_cloze_field(&mut rng);
            if expected.is_empty() {
                expected.push(0); // Python default card when no markers
            }
            assert_eq!(cloze_ords_for(&text), expected, "field: {text:?}");
        }
    }

    #[test]
    fn cloze_empty_body_swallows_following_text() {
        // Python parity pin: `.+?}}` needs >= 1 body char, so an empty body
        // makes the lazy match consume text up to the *next* `}}`, hiding the
        // second marker. Verified against Python genanki v0.13.x's
        // `re.finditer(r"\{\{c(\d+)::.+?\}\}", value, re.DOTALL)`.
        assert_eq!(
            cloze_ords_for("{{c17::}} text {{c16::x}} and {{c5::y}}"),
            [4, 16]
        );
        // Non-empty bodies are independent of what follows.
        assert_eq!(
            cloze_ords_for("{{c17::x}} text {{c16::y}} and {{c5::z}}"),
            [4, 15, 16]
        );
    }

    #[test]
    fn cloze_property_multi_field_union() {
        let mut rng = XorShift64::new(0xC10E_5E20_2400_0002);
        for _ in 0..CLOZE_PROP_ITERS {
            let (t1, e1) = gen_cloze_field(&mut rng);
            let (t2, e2) = gen_cloze_field(&mut rng);
            let mut n = Note::new(multi_field_cloze_model(), [t1.as_str(), t2.as_str()]).unwrap();
            let ords: Vec<i32> = n.cards().unwrap().iter().map(|c| c.ord).collect();
            let mut expected = e1;
            expected.extend(e2);
            expected.sort_unstable();
            expected.dedup();
            if expected.is_empty() {
                expected.push(0);
            }
            assert_eq!(ords, expected, "fields: {t1:?} / {t2:?}");
        }
    }

    /// Random junk: wide char alphabet plus occasional `{{c` fragments.
    fn gen_cloze_junk(rng: &mut XorShift64) -> String {
        const JUNK: &[char] = &[
            'a', 'b', 'c', '{', '}', ':', '1', '2', '3', ' ', '\n', 'é', '中', '!', '?', 'x', 'y',
        ];
        let len = rng.next_usize(0, 300);
        let mut s = String::new();
        for i in 0..len {
            if i % 7 == 0 && rng.next_bool() {
                s.push_str("{{c");
            } else {
                s.push(JUNK[rng.next_usize(0, JUNK.len())]);
            }
        }
        s
    }

    #[test]
    fn cloze_property_random_junk_sorted_unique_no_panic() {
        let mut rng = XorShift64::new(0xC10E_5E20_2400_0003);
        for _ in 0..CLOZE_PROP_ITERS {
            let text = gen_cloze_junk(&mut rng);
            let ords = cloze_ords_for(&text);
            let mut sorted = ords.clone();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(ords, sorted, "ords must be sorted+deduped: {ords:?}");
            assert!(ords.iter().all(|&o| o >= 0));
            assert!(!ords.is_empty(), "cloze cards always have a default card");
        }
    }

    #[test]
    fn cloze_ord_re_matches_only_digit_groups() {
        let mut rng = XorShift64::new(0xC10E_5E20_2400_0004);
        for _ in 0..CLOZE_PROP_ITERS {
            let text = gen_cloze_junk(&mut rng);
            for caps in cloze_ord_re().captures_iter(&text) {
                let digits = &caps[1];
                assert!(
                    digits.bytes().all(|b| b.is_ascii_digit()),
                    "ord group must be digits: {digits:?}"
                );
            }
        }
    }
}
