//! Integration smoke: all five builtin models write a clean `.apkg`.
//! Port of Python genanki `tests/test_builtin_models.py`.

mod common;

use common::{open_collection, open_zip, write_pkg};
use genanki::{
    BASIC_AND_REVERSED_CARD_MODEL, BASIC_MODEL, BASIC_OPTIONAL_REVERSED_CARD_MODEL,
    BASIC_TYPE_IN_THE_ANSWER_MODEL, CLOZE_MODEL, Deck, Note, Package,
};

#[test]
fn all_builtins_write_apkg() {
    let mut deck = Deck::new(1598559905, "Country Capitals");

    deck.add_note(Note::new(&*BASIC_MODEL, ["Capital of Argentina", "Buenos Aires"]).unwrap());
    deck.add_note(Note::new(&*BASIC_AND_REVERSED_CARD_MODEL, ["Costa Rica", "San José"]).unwrap());
    deck.add_note(
        Note::new(
            &*BASIC_OPTIONAL_REVERSED_CARD_MODEL,
            ["France", "Paris", "y"],
        )
        .unwrap(),
    );
    deck.add_note(Note::new(&*BASIC_TYPE_IN_THE_ANSWER_MODEL, ["Taiwan", "Taipei"]).unwrap());
    deck.add_note(
        Note::new(
            &*CLOZE_MODEL,
            [
                "{{c1::Ottawa}} is the capital of {{c2::Canada}}",
                "Ottawa is in Ontario province.",
            ],
        )
        .unwrap(),
    );

    let pkg = Package::new(deck);
    let (_dir, path) = write_pkg(&pkg, 1_600_000_000.0);

    // Zip opens; collection opens; 5 notes, 8 cards
    // (1 + 2 + 2 + 1 + 2 as documented in the plan).
    let mut z = open_zip(&path);
    let (_dbdir, conn) = open_collection(&mut z);

    let notes: i64 = conn
        .query_row("SELECT count(*) FROM notes", [], |r| r.get(0))
        .unwrap();
    let cards: i64 = conn
        .query_row("SELECT count(*) FROM cards", [], |r| r.get(0))
        .unwrap();
    assert_eq!((notes, cards), (5, 8));

    // All five model ids registered in col.models.
    let models = common::col_json(&conn, "models");
    for id in [
        1559383000u64,
        1485830179,
        1382232460,
        1305534440,
        1550428389,
    ] {
        assert!(
            models.get(id.to_string()).is_some(),
            "model {id} missing from col.models"
        );
    }
}

#[test]
fn builtin_models_json_fingerprint() {
    // Byte-level fingerprint of the builtin model JSON as written into
    // col.models (Python genanki v0.13.0 builtin_models.py). Locks the
    // serialization path, not only the in-memory builder state.
    let mut deck = Deck::new(1598559905, "Country Capitals");
    deck.add_note(Note::new(&*BASIC_MODEL, ["Capital of Argentina", "Buenos Aires"]).unwrap());
    deck.add_note(
        Note::new(
            &*CLOZE_MODEL,
            [
                "{{c1::Ottawa}} is the capital of {{c2::Canada}}",
                "Ottawa is in Ontario province.",
            ],
        )
        .unwrap(),
    );

    let pkg = Package::new(deck);
    let (_dir, path) = write_pkg(&pkg, 1_600_000_000.0);

    let mut z = open_zip(&path);
    let (_dbdir, conn) = open_collection(&mut z);
    let models = common::col_json(&conn, "models");

    let basic = &models["1559383000"];
    assert_eq!(basic["name"], "Basic (genanki)");
    assert_eq!(basic["type"], 0);
    assert_eq!(basic["css"], common::expected_basic_css());
    assert_eq!(basic["tmpls"][0]["qfmt"], "{{Front}}");
    assert_eq!(
        basic["tmpls"][0]["afmt"],
        "{{FrontSide}}\n\n<hr id=answer>\n\n{{Back}}"
    );
    assert_eq!(basic["flds"][0]["font"], "Arial");

    let cloze = &models["1550428389"];
    assert_eq!(cloze["name"], "Cloze (genanki)");
    assert_eq!(cloze["type"], 1);
    let css = cloze["css"].as_str().unwrap();
    assert_eq!(css, common::expected_cloze_css());
    assert!(!css.ends_with('\n'));
    assert_eq!(cloze["tmpls"][0]["qfmt"], "{{cloze:Text}}");
    assert_eq!(
        cloze["tmpls"][0]["afmt"],
        "{{cloze:Text}}<br>\n{{Back Extra}}"
    );
}

#[test]
fn cloze_readme_snippet_compiles_and_builds_note() {
    // Mirrors the README Cloze section (fixed `&*CLOZE_MODEL` form). Fails if
    // someone "simplifies" the call site back to `CLOZE_MODEL` without a
    // compiling alternative.
    let my_note = Note::new(
        &*CLOZE_MODEL,
        ["{{c1::Rome}} is the capital of {{c2::Italy}}", ""],
    )
    .unwrap();
    assert_eq!(my_note.fields().len(), 2);
    assert_eq!(my_note.fields()[1], "");
}

#[test]
fn optional_reverse_card_counts_in_sqlite() {
    // Empty Add Reverse => 1 card; non-empty => 2 cards, per note.
    let mut deck = Deck::new(1598559905, "Country Capitals");
    deck.add_note(
        Note::new(
            &*BASIC_OPTIONAL_REVERSED_CARD_MODEL,
            ["France", "Paris", ""],
        )
        .unwrap(),
    );
    deck.add_note(
        Note::new(
            &*BASIC_OPTIONAL_REVERSED_CARD_MODEL,
            ["Germany", "Berlin", "y"],
        )
        .unwrap(),
    );

    let pkg = Package::new(deck);
    let (_dir, path) = write_pkg(&pkg, 1_600_000_000.0);

    let mut z = open_zip(&path);
    let (_dbdir, conn) = open_collection(&mut z);

    // Per-note card counts via join on note id (ord set from the card list).
    let rows: Vec<(String, i64)> = conn
        .prepare(
            "SELECT n.flds, count(c.id) FROM notes n JOIN cards c ON c.nid = n.id GROUP BY n.id ORDER BY n.id",
        )
        .unwrap()
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();

    assert_eq!(rows.len(), 2);
    let counts: Vec<i64> = rows.iter().map(|(_, c)| *c).collect();
    assert_eq!(counts, vec![1, 2]);
    assert!(rows.iter().any(|(f, c)| f.contains("Paris") && *c == 1));
    assert!(rows.iter().any(|(f, c)| f.contains("Berlin") && *c == 2));
}
