//! Built-in note models shipped with the crate. (Phase 5)
//!
//! The five models are byte-identical to Python genanki v0.13.x
//! `builtin_models.py` (ids, names, Arial fields, templates, CSS). They are
//! exposed as crate-root re-exports ([`crate::BASIC_MODEL`], ...) and as
//! `LazyLock<Model>` statics here.
//!
//! # Why the `(genanki)` suffix?
//!
//! Anki does not assign consistent ids to its own built-in models, so a
//! model literally named `Basic` with a fixed id would, on import, collide
//! with the user's stock `Basic` and get renamed to something like
//! `Basic-123abc`. The `(genanki)` suffix (matching Python genanki) avoids
//! that confusing clash.
//!
//! # Examples
//!
//! ```
//! use genanki::{BASIC_MODEL, Note};
//! let note = Note::new(&*BASIC_MODEL, ["Capital of Argentina", "Buenos Aires"])?;
//! assert_eq!(note.fields().len(), 2);
//! # Ok::<(), genanki::Error>(())
//! ```

use std::sync::LazyLock;

use crate::model::{Field, Model, Template};

/// Shared CSS for the four `Basic`-family models, byte-identical to Python
/// genanki v0.13.x `builtin_models.py` (trailing newline included).
const BASIC_CSS: &str = ".card {\n font-family: arial;\n font-size: 20px;\n text-align: center;\n color: black;\n background-color: white;\n}\n";

/// A field with the builtin font (Python builtins set `font: Arial`; the
/// default [`Field`] font remains Liberation Sans).
fn arial(name: &str) -> Field {
    Field::new(name).font("Arial")
}

/// Shared CSS for the `Cloze` model, byte-identical to Python genanki v0.13.x
/// `builtin_models.py` (Python concatenates two literals; the final
/// `.nightMode .cloze` segment has **no** trailing newline).
const CLOZE_CSS: &str = ".card {\n font-family: arial;\n font-size: 20px;\n text-align: center;\n color: black;\n background-color: white;\n}\n\n.cloze {\n font-weight: bold;\n color: blue;\n}\n.nightMode .cloze {\n color: lightblue;\n}";

/// Anki's stock "Basic" model under the `(genanki)` name (see module docs
/// for the suffix rationale). Fields: `Front`, `Back` (Arial). One card
/// template `Card 1`: `{{Front}}` / `{{FrontSide}}` + answer divider +
/// `{{Back}}`.
pub static BASIC_MODEL: LazyLock<Model> = LazyLock::new(|| {
    Model::new(1559383000, "Basic (genanki)")
        .field(arial("Front"))
        .field(arial("Back"))
        .template(Template::new(
            "Card 1",
            "{{Front}}",
            "{{FrontSide}}\n\n<hr id=answer>\n\n{{Back}}",
        ))
        .css(BASIC_CSS)
});

/// Anki's stock "Basic (and reversed card)" model under the `(genanki)`
/// name. Fields: `Front`, `Back` (Arial). Two card templates: `Card 1` as
/// [`BASIC_MODEL`], plus `Card 2` (`{{Back}}` / `{{FrontSide}}` + divider +
/// `{{Front}}`). Both cards are always generated.
pub static BASIC_AND_REVERSED_CARD_MODEL: LazyLock<Model> = LazyLock::new(|| {
    Model::new(1485830179, "Basic (and reversed card) (genanki)")
        .field(arial("Front"))
        .field(arial("Back"))
        .template(Template::new(
            "Card 1",
            "{{Front}}",
            "{{FrontSide}}\n\n<hr id=answer>\n\n{{Back}}",
        ))
        .template(Template::new(
            "Card 2",
            "{{Back}}",
            "{{FrontSide}}\n\n<hr id=answer>\n\n{{Front}}",
        ))
        .css(BASIC_CSS)
});

/// Anki's stock "Basic (optional reversed card)" model under the `(genanki)`
/// name. Fields: `Front`, `Back`, `Add Reverse` (Arial). Two templates: `Card 1`
/// as [`BASIC_MODEL`]; `Card 2` is emitted only when **both** `Back` and
/// `Add Reverse` are non-empty. The reverse template's `qfmt` is
/// `{{#Add Reverse}}{{Back}}{{/Add Reverse}}`; required-field computation
/// yields `All` over those two field ords (same as Python genanki), so an
/// empty `Back` blanks the whole section body and suppresses Card 2.
pub static BASIC_OPTIONAL_REVERSED_CARD_MODEL: LazyLock<Model> = LazyLock::new(|| {
    Model::new(1382232460, "Basic (optional reversed card) (genanki)")
        .field(arial("Front"))
        .field(arial("Back"))
        .field(arial("Add Reverse"))
        .template(Template::new(
            "Card 1",
            "{{Front}}",
            "{{FrontSide}}\n\n<hr id=answer>\n\n{{Back}}",
        ))
        .template(Template::new(
            "Card 2",
            "{{#Add Reverse}}{{Back}}{{/Add Reverse}}",
            "{{FrontSide}}\n\n<hr id=answer>\n\n{{Front}}",
        ))
        .css(BASIC_CSS)
});

/// Anki's stock "Basic (type in the answer)" model under the `(genanki)`
/// name. Fields: `Front`, `Back` (Arial). One template `Card 1` using the
/// Anki `type:` filter so the answer is typed on the front, then revealed.
pub static BASIC_TYPE_IN_THE_ANSWER_MODEL: LazyLock<Model> = LazyLock::new(|| {
    Model::new(1305534440, "Basic (type in the answer) (genanki)")
        .field(arial("Front"))
        .field(arial("Back"))
        .template(Template::new(
            "Card 1",
            "{{Front}}\n\n{{type:Back}}",
            "{{Front}}\n\n<hr id=answer>\n\n{{type:Back}}",
        ))
        .css(BASIC_CSS)
});

/// Anki's stock "Cloze" model under the `(genanki)` name. Fields: `Text`,
/// `Back Extra` (Arial) - **two fields are required** (see module docs; no
/// single-field auto-pad). One template `Cloze`: `{{cloze:Text}}` /
/// `{{cloze:Text}}<br>` + `{{Back Extra}}`. Cards are generated per unique
/// `{{cN::...}}` deletion.
///
/// # Examples
///
/// Same call shape as the README Cloze section: builtin statics are
/// `LazyLock<Model>`, so pass `&*CLOZE_MODEL` (via `From<&Model> for
/// `Arc<Model>`).
///
/// ```
/// use genanki::{CLOZE_MODEL, Note};
/// let note = Note::new(
///     &*CLOZE_MODEL,
///     ["{{c1::Rome}} is the capital of {{c2::Italy}}", ""],
/// )?;
/// assert_eq!(note.fields().len(), 2);
/// assert_eq!(note.fields()[1], "");
/// # Ok::<(), genanki::Error>(())
/// ```
pub static CLOZE_MODEL: LazyLock<Model> = LazyLock::new(|| {
    Model::new(1550428389, "Cloze (genanki)")
        .model_type(crate::model::ModelType::Cloze)
        .field(arial("Text"))
        .field(arial("Back Extra"))
        .template(Template::new(
            "Cloze",
            "{{cloze:Text}}",
            "{{cloze:Text}}<br>\n{{Back Extra}}",
        ))
        .css(CLOZE_CSS)
});

#[cfg(test)]
mod tests {
    use crate::model::Model;
    use crate::model::ModelType;

    // Python genanki v0.13.0 builtin_models.py. Independent literals - do
    // NOT reference super::BASIC_CSS / super::CLOZE_CSS: corrupting a
    // production const must fail this table, and the table must not silently
    // track it. (concat! keeps the single leading space per line; the plan's
    // `\`-continuation form would strip it via Rust line-continuation.)
    const EXPECTED_BASIC_CSS: &str = concat!(
        ".card {\n",
        " font-family: arial;\n",
        " font-size: 20px;\n",
        " text-align: center;\n",
        " color: black;\n",
        " background-color: white;\n",
        "}\n",
    );

    // Final .nightMode block has NO trailing newline (Python concat).
    const EXPECTED_CLOZE_CSS: &str = concat!(
        ".card {\n",
        " font-family: arial;\n",
        " font-size: 20px;\n",
        " text-align: center;\n",
        " color: black;\n",
        " background-color: white;\n",
        "}\n",
        "\n",
        ".cloze {\n",
        " font-weight: bold;\n",
        " color: blue;\n",
        "}\n",
        ".nightMode .cloze {\n",
        " color: lightblue;\n",
        "}",
    );

    /// One row of the byte-exact fingerprint table for the five builtins.
    /// `css` is `EXPECTED_BASIC_CSS` for the four Basic-family models and
    /// `EXPECTED_CLOZE_CSS` for the cloze model (Python concatenates two
    /// literals there).
    struct Expect {
        model: &'static Model,
        id: i64,
        name: &'static str,
        model_type: ModelType,
        fields: &'static [(&'static str, &'static str)],
        templates: &'static [(&'static str, &'static str, &'static str)],
        css: &'static str,
    }

    const BASIC_AFMT: &str = "{{FrontSide}}\n\n<hr id=answer>\n\n{{Back}}";
    const REVERSED_AFMT: &str = "{{FrontSide}}\n\n<hr id=answer>\n\n{{Front}}";

    #[test]
    fn builtin_fingerprints() {
        let expectations: &[Expect] = &[
            Expect {
                model: &super::BASIC_MODEL,
                id: 1559383000,
                name: "Basic (genanki)",
                model_type: ModelType::FrontBack,
                fields: &[("Front", "Arial"), ("Back", "Arial")],
                templates: &[("Card 1", "{{Front}}", BASIC_AFMT)],
                css: EXPECTED_BASIC_CSS,
            },
            Expect {
                model: &super::BASIC_AND_REVERSED_CARD_MODEL,
                id: 1485830179,
                name: "Basic (and reversed card) (genanki)",
                model_type: ModelType::FrontBack,
                fields: &[("Front", "Arial"), ("Back", "Arial")],
                templates: &[
                    ("Card 1", "{{Front}}", BASIC_AFMT),
                    ("Card 2", "{{Back}}", REVERSED_AFMT),
                ],
                css: EXPECTED_BASIC_CSS,
            },
            Expect {
                model: &super::BASIC_OPTIONAL_REVERSED_CARD_MODEL,
                id: 1382232460,
                name: "Basic (optional reversed card) (genanki)",
                model_type: ModelType::FrontBack,
                fields: &[
                    ("Front", "Arial"),
                    ("Back", "Arial"),
                    ("Add Reverse", "Arial"),
                ],
                templates: &[
                    ("Card 1", "{{Front}}", BASIC_AFMT),
                    (
                        "Card 2",
                        "{{#Add Reverse}}{{Back}}{{/Add Reverse}}",
                        REVERSED_AFMT,
                    ),
                ],
                css: EXPECTED_BASIC_CSS,
            },
            Expect {
                model: &super::BASIC_TYPE_IN_THE_ANSWER_MODEL,
                id: 1305534440,
                name: "Basic (type in the answer) (genanki)",
                model_type: ModelType::FrontBack,
                fields: &[("Front", "Arial"), ("Back", "Arial")],
                templates: &[(
                    "Card 1",
                    "{{Front}}\n\n{{type:Back}}",
                    "{{Front}}\n\n<hr id=answer>\n\n{{type:Back}}",
                )],
                css: EXPECTED_BASIC_CSS,
            },
            Expect {
                model: &super::CLOZE_MODEL,
                id: 1550428389,
                name: "Cloze (genanki)",
                model_type: ModelType::Cloze,
                fields: &[("Text", "Arial"), ("Back Extra", "Arial")],
                templates: &[(
                    "Cloze",
                    "{{cloze:Text}}",
                    "{{cloze:Text}}<br>\n{{Back Extra}}",
                )],
                css: EXPECTED_CLOZE_CSS,
            },
        ];

        for e in expectations {
            let m = e.model;
            assert_eq!(m.id, e.id, "{}", e.name);
            assert_eq!(m.name, e.name);
            assert_eq!(m.model_type, e.model_type, "{}", e.name);
            assert_eq!(
                m.fields
                    .iter()
                    .map(|f| (f.name.as_str(), f.font.as_str()))
                    .collect::<Vec<_>>(),
                e.fields,
                "field fingerprint of {}",
                e.name
            );
            assert_eq!(
                m.templates
                    .iter()
                    .map(|t| (t.name.as_str(), t.qfmt.as_str(), t.afmt.as_str()))
                    .collect::<Vec<_>>(),
                e.templates,
                "template fingerprint of {}",
                e.name
            );
            assert_eq!(m.css, e.css, "css fingerprint of {}", e.name);
        }

        // Python parity: BASIC family ends with the trailing newline, the
        // cloze literal does not (Python concatenation artifact).
        assert!(EXPECTED_BASIC_CSS.ends_with('\n'));
        assert!(!EXPECTED_CLOZE_CSS.ends_with('\n'));
    }

    #[test]
    fn all_builtins_compute_req() {
        // Guards template typos: every builtin's qfmt must produce req entries.
        for m in [
            &*super::BASIC_MODEL,
            &*super::BASIC_AND_REVERSED_CARD_MODEL,
            &*super::BASIC_OPTIONAL_REVERSED_CARD_MODEL,
            &*super::BASIC_TYPE_IN_THE_ANSWER_MODEL,
            &*super::CLOZE_MODEL,
        ] {
            let req = m.req().unwrap_or_else(|e| panic!("{}: {e:?}", m.name));
            assert!(!req.is_empty(), "{} has empty req", m.name);
        }
    }

    #[test]
    fn optional_reversed_req_gating() {
        // Card 2's qfmt wraps {{Back}} in a section on Add Reverse. The
        // sentinel strategy reports kind All over [Back, Add Reverse] (blanking
        // either empties the whole render), matching Python genanki's
        // chevron-based _req. Note-level card gating (T4) therefore requires
        // both non-empty: empty Add Reverse -> 1 card, non-empty -> 2.
        let req = super::BASIC_OPTIONAL_REVERSED_CARD_MODEL.req().unwrap();
        assert_eq!(req.len(), 2);
        assert_eq!(req[0].kind, crate::req::ReqKind::All);
        assert_eq!(req[0].field_ords, vec![0]);
        assert_eq!(req[1].kind, crate::req::ReqKind::All);
        assert_eq!(req[1].field_ords, vec![1, 2]);
    }

    #[test]
    fn optional_reversed_empty_back_suppresses_card2() {
        // req Card2 = All[Back, Add Reverse]: empty Back blanks the section
        // body, so the reverse card is suppressed even with Add Reverse set.
        let mut n = crate::Note::new(
            &*super::BASIC_OPTIONAL_REVERSED_CARD_MODEL,
            ["France", "", "y"],
        )
        .unwrap();
        let cards = n.cards().unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].ord, 0);
    }

    #[test]
    fn optional_reversed_card_gating() {
        // Empty Add Reverse => 1 card; non-empty => 2 cards.
        let mut n1 =
            crate::Note::new(&*super::BASIC_OPTIONAL_REVERSED_CARD_MODEL, ["F", "B", ""]).unwrap();
        let cards = n1.cards().unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].ord, 0);

        let mut n2 =
            crate::Note::new(&*super::BASIC_OPTIONAL_REVERSED_CARD_MODEL, ["F", "B", "y"]).unwrap();
        let cards = n2.cards().unwrap();
        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].ord, 0);
        assert_eq!(cards[1].ord, 1);
    }

    #[test]
    fn cloze_requires_two_fields() {
        let err = crate::Note::new(
            &*super::CLOZE_MODEL,
            ["{{c1::Rome}} is the capital of {{c2::Italy}}"],
        )
        .unwrap_err();
        match err {
            crate::Error::FieldCountMismatch {
                model_fields,
                note_fields,
                ..
            } => {
                assert_eq!(model_fields, 2);
                assert_eq!(note_fields, 1);
            }
            other => panic!("unexpected {other:?}"),
        }

        // Two fields (even an empty second) is fine; cards follow the cloze refs.
        let mut ok = crate::Note::new(
            &*super::CLOZE_MODEL,
            ["{{c1::Rome}} is the capital of {{c2::Italy}}", ""],
        )
        .unwrap();
        assert_eq!(ok.cards().unwrap().len(), 2);
        assert_eq!(ok.cards().unwrap()[0].ord, 0);
        assert_eq!(ok.cards().unwrap()[1].ord, 1);
    }
}
