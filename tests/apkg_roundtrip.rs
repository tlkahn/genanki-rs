//! Structural `.apkg` round-trip tests: write a package, open the zip and the
//! embedded sqlite, and assert rows / JSON / media. No full Anki import.

mod common;

use std::collections::HashSet;

use common::{col_json, entry_names, open_collection, open_zip, simple_model, write_pkg};
use genanki::{Deck, Error, Note, Package};

// --- T8: single-deck package zip layout (no media) ---

#[test]
fn single_deck_zip_layout_no_media() {
    let mut deck = Deck::new(123456, "foodeck");
    deck.add_note(Note::new(simple_model(), ["a", "b"]).unwrap());
    let pkg = Package::new(deck);
    let (_dir, path) = write_pkg(&pkg, 1_600_000_000.0);

    let mut z = open_zip(&path);
    assert_eq!(
        entry_names(&z),
        HashSet::from(["collection.anki2".to_string(), "media".to_string()])
    );
    assert_eq!(common::read_entry(&mut z, "media"), b"{}");
}

// --- T9: notes/cards/decks/models content in sqlite ---

#[test]
fn notes_cards_decks_models_content() {
    let mut deck = Deck::new(123456, "foodeck");
    deck.add_note(Note::new(simple_model(), ["a", "b"]).unwrap());
    let pkg = Package::new(deck);
    let (_dir, path) = write_pkg(&pkg, 1_600_000_000.0);

    let mut z = open_zip(&path);
    let (_dbdir, conn) = open_collection(&mut z);

    let notes: i64 = conn
        .query_row("SELECT count(*) FROM notes", [], |r| r.get(0))
        .unwrap();
    let cards: i64 = conn
        .query_row("SELECT count(*) FROM cards", [], |r| r.get(0))
        .unwrap();
    assert_eq!((notes, cards), (1, 1));

    // col.decks: seeded Default deck preserved, our deck merged in.
    let decks = col_json(&conn, "decks");
    assert_eq!(decks["123456"]["name"], "foodeck");
    assert_eq!(decks["123456"]["id"], 123456);
    assert_eq!(decks["1"]["name"], "Default");
    assert_eq!(decks.as_object().unwrap().len(), 2);

    // col.models: model registered with expected shape.
    let models = col_json(&conn, "models");
    assert_eq!(models["1607392319"]["name"], "Simple Model");
    assert_eq!(models["1607392319"]["flds"][0]["name"], "Question");
    assert_eq!(models["1607392319"]["flds"][1]["name"], "Answer");
    assert_eq!(models["1607392319"]["did"], 123456);

    // notes row.
    let (flds, guid, mid): (String, String, i64) = conn
        .query_row("SELECT flds, guid, mid FROM notes", [], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .unwrap();
    assert_eq!(flds, "a\x1fb");
    assert_eq!(guid, genanki::guid_for(&["a", "b"]));
    assert_eq!(mid, 1607392319);

    // cards row.
    let (ord, did, queue, due): (i32, i64, i64, i64) = conn
        .query_row("SELECT ord, did, queue, due FROM cards", [], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })
        .unwrap();
    assert_eq!((ord, did, queue, due), (0, 123456, 0, 0));
}

// --- T10: description preserved ---

#[test]
fn description_preserved_in_deck_json() {
    let deck = Deck::new(999, "withdesc").with_description("line1\nline2");
    let pkg = Package::new(deck);
    let (_dir, path) = write_pkg(&pkg, 1_600_000_000.0);

    let mut z = open_zip(&path);
    let (_dbdir, conn) = open_collection(&mut z);
    let decks = col_json(&conn, "decks");
    assert_eq!(decks["999"]["desc"], "line1\nline2");
}

// --- T11: multi-deck package ---

#[test]
fn multi_deck_package_writes_both_decks() {
    let mut d1 = Deck::new(123456, "foodeck");
    d1.add_note(Note::new(simple_model(), ["a", "b"]).unwrap());
    let mut d2 = Deck::new(654321, "bardeck");
    d2.add_note(Note::new(simple_model(), ["c", "d"]).unwrap());
    let pkg = Package::from_decks([d1, d2]);
    let (_dir, path) = write_pkg(&pkg, 1_600_000_000.0);

    let mut z = open_zip(&path);
    let (_dbdir, conn) = open_collection(&mut z);

    let decks = col_json(&conn, "decks");
    assert_eq!(decks["123456"]["name"], "foodeck");
    assert_eq!(decks["654321"]["name"], "bardeck");
    assert_eq!(decks["1"]["name"], "Default");
    assert_eq!(decks.as_object().unwrap().len(), 3);

    let notes: i64 = conn
        .query_row("SELECT count(*) FROM notes", [], |r| r.get(0))
        .unwrap();
    let cards: i64 = conn
        .query_row("SELECT count(*) FROM cards", [], |r| r.get(0))
        .unwrap();
    assert_eq!((notes, cards), (2, 2));
}

// --- T12: nested deck name as plain string ---

#[test]
fn nested_deck_name_plain_string() {
    let deck = Deck::new(1, "Parent::Child");
    let pkg = Package::new(deck);
    let (_dir, path) = write_pkg(&pkg, 1_600_000_000.0);

    let mut z = open_zip(&path);
    let (_dbdir, conn) = open_collection(&mut z);
    let decks = col_json(&conn, "decks");
    assert_eq!(decks["1"]["name"], "Parent::Child");
}

// --- T13: fixed timestamp => deterministic ids ---

#[test]
fn fixed_timestamp_zero_gives_deterministic_ids() {
    let mut deck = Deck::new(123456, "foodeck");
    deck.add_note(Note::new(simple_model(), ["a", "b"]).unwrap());
    let pkg = Package::new(deck);
    let (_dir, path) = write_pkg(&pkg, 0.0);

    let mut z = open_zip(&path);
    let (_dbdir, conn) = open_collection(&mut z);

    let note_id: i64 = conn
        .query_row("SELECT id FROM notes", [], |r| r.get(0))
        .unwrap();
    let card_ids: Vec<i64> = conn
        .prepare("SELECT id FROM cards ORDER BY id")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(note_id, 0, "first id = timestamp*1000");
    assert_eq!(card_ids, vec![1]);

    let note_mod: i64 = conn
        .query_row("SELECT mod FROM notes", [], |r| r.get(0))
        .unwrap();
    let card_mod: i64 = conn
        .query_row("SELECT mod FROM cards", [], |r| r.get(0))
        .unwrap();
    assert_eq!((note_mod, card_mod), (0, 0), "mod = timestamp as i64");
}

#[test]
fn fixed_timestamp_fractional_scales_by_thousand() {
    let mut deck = Deck::new(123456, "foodeck");
    deck.add_note(Note::new(simple_model(), ["a", "b"]).unwrap());
    let pkg = Package::new(deck);
    let (_dir, path) = write_pkg(&pkg, 1000.5);

    let mut z = open_zip(&path);
    let (_dbdir, conn) = open_collection(&mut z);

    let note_id: i64 = conn
        .query_row("SELECT id FROM notes", [], |r| r.get(0))
        .unwrap();
    assert_eq!(note_id, 1_000_500);
}

// --- T14: "now" timestamp => modern card ids ---

#[test]
fn now_timestamp_gives_modern_ids() {
    let mut deck = Deck::new(1, "d");
    deck.add_note(Note::new(simple_model(), ["a", "b"]).unwrap());
    let pkg = Package::new(deck);

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("out.apkg");
    pkg.write_to_file(&path).unwrap(); // real now

    let mut z = open_zip(&path);
    let (_dbdir, conn) = open_collection(&mut z);
    let card_id: i64 = conn
        .query_row("SELECT id FROM cards", [], |r| r.get(0))
        .unwrap();
    assert!(
        card_id > 1_577_836_800_000,
        "card id {card_id} must be past Jan 1 2020 UTC ms"
    );
}

// --- T21: empty name rejected at write ---

#[test]
fn empty_deck_name_rejected_at_write() {
    let deck = Deck::new(1, "");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("out.apkg");
    let err = deck.write_to_file(&path).unwrap_err();
    assert!(matches!(err, Error::DeckInvalid { .. }));
    assert!(!path.exists(), "no file written on error");
}

// --- T25: Deck::write_to_file convenience ---

#[test]
fn deck_write_to_file_convenience_matches_package() {
    let mut deck = Deck::new(123456, "foodeck");
    deck.add_note(Note::new(simple_model(), ["a", "b"]).unwrap());

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("out.apkg");
    deck.write_to_file(&path).unwrap();

    let mut z = open_zip(&path);
    assert_eq!(
        entry_names(&z),
        HashSet::from(["collection.anki2".to_string(), "media".to_string()])
    );
    let (_dbdir, conn) = open_collection(&mut z);
    let notes: i64 = conn
        .query_row("SELECT count(*) FROM notes", [], |r| r.get(0))
        .unwrap();
    let decks = col_json(&conn, "decks");
    assert_eq!(notes, 1);
    assert_eq!(decks["123456"]["name"], "foodeck");
}

// --- T22: models auto-registered from notes; explicit registry kept ---

#[test]
fn models_auto_registered_and_explicit_registry_kept() {
    let mut deck = Deck::new(123456, "foodeck");
    deck.add_note(Note::new(simple_model(), ["a", "b"]).unwrap());
    let unused = genanki::Model::new(987654321, "Unused Model")
        .field(genanki::Field::new("X"))
        .template(genanki::Template::new("c", "{{X}}", ""));
    deck.add_model(unused);
    let pkg = Package::new(deck);
    let (_dir, path) = write_pkg(&pkg, 1_600_000_000.0);

    let mut z = open_zip(&path);
    let (_dbdir, conn) = open_collection(&mut z);
    let models = col_json(&conn, "models");
    assert_eq!(
        models["1607392319"]["name"], "Simple Model",
        "auto-registered from note without add_model"
    );
    assert_eq!(
        models["987654321"]["name"], "Unused Model",
        "explicit registry entry kept even without notes"
    );
}

// --- T23: latex pre/post + sortf round-trip into model JSON ---

#[test]
fn latex_pre_post_and_sortf_round_trip_into_model_json() {
    let model = genanki::Model::new(555, "custom")
        .field(genanki::Field::new("Q"))
        .field(genanki::Field::new("A"))
        .template(genanki::Template::new("c", "{{Q}}", "{{A}}"))
        .latex_pre("PRE")
        .latex_post("POST")
        .sort_field_index(1);
    let mut deck = Deck::new(123456, "foodeck");
    deck.add_note(Note::new(model, ["a", "b"]).unwrap());
    let pkg = Package::new(deck);
    let (_dir, path) = write_pkg(&pkg, 1_600_000_000.0);

    let mut z = open_zip(&path);
    let (_dbdir, conn) = open_collection(&mut z);
    let models = col_json(&conn, "models");
    assert_eq!(models["555"]["latexPre"], "PRE");
    assert_eq!(models["555"]["latexPost"], "POST");
    assert_eq!(models["555"]["sortf"], 1);
}

// --- T24: tags + flds formatting integration ---

#[test]
fn tags_and_flds_formatting_in_sqlite() {
    let mut deck = Deck::new(123456, "foodeck");
    let note = Note::new(simple_model(), ["one", "two"])
        .unwrap()
        .with_tags(["foo", "bar"])
        .unwrap();
    deck.add_note(note);
    let pkg = Package::new(deck);
    let (_dir, path) = write_pkg(&pkg, 1_600_000_000.0);

    let mut z = open_zip(&path);
    let (_dbdir, conn) = open_collection(&mut z);
    let (tags, flds): (String, String) = conn
        .query_row("SELECT tags, flds FROM notes", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap();
    assert_eq!(tags, " foo bar ", "tags wrapped in spaces");
    assert_eq!(flds, "one\x1ftwo", "flds unit-separated");
}

// --- T1 (review 5291226535): zip-phase failure must not truncate an
// existing destination (atomic publish). Unix-only: uses chmod 0 to make a
// file that passes `is_file()` but fails `File::open`. ---

#[cfg(unix)]
#[test]
fn failed_write_preserves_existing_destination() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.apkg");
    std::fs::write(&out, b"SENTINEL_APKG").unwrap();

    // Media path exists as a regular file (plan_media ok) but cannot be read.
    let media = dir.path().join("secret.mp3");
    std::fs::write(&media, b"bytes").unwrap();
    let mut perms = std::fs::metadata(&media).unwrap().permissions();
    perms.set_mode(0o000);
    std::fs::set_permissions(&media, perms).unwrap();

    let mut deck = Deck::new(1, "d");
    deck.add_note(Note::new(simple_model(), ["a", "b"]).unwrap());
    let pkg = Package::new(deck).media_files([media.clone()]);
    let err = pkg.write_to_file_at(&out, 1_600_000_000.0).unwrap_err();
    assert!(
        matches!(err, Error::Io(_)),
        "open/read failure maps to Io: {err:?}"
    );

    assert_eq!(
        std::fs::read(&out).unwrap(),
        b"SENTINEL_APKG",
        "destination must not be truncated on zip-phase failure"
    );

    // Cleanup so tempdir can remove the file.
    let mut perms = std::fs::metadata(&media).unwrap().permissions();
    perms.set_mode(0o600);
    std::fs::set_permissions(&media, perms).unwrap();
}

// --- T2 (review 5291226535): a successful write atomically replaces an
// existing destination file. ---

#[test]
fn successful_write_replaces_preexisting_file() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.apkg");
    std::fs::write(&out, b"OLD").unwrap();

    let mut deck = Deck::new(123456, "foodeck");
    deck.add_note(Note::new(simple_model(), ["a", "b"]).unwrap());
    Package::new(deck)
        .write_to_file_at(&out, 1_600_000_000.0)
        .unwrap();

    let bytes = std::fs::read(&out).unwrap();
    assert_ne!(bytes, b"OLD");
    let mut z = open_zip(&out);
    assert!(entry_names(&z).contains("collection.anki2"));
    assert!(entry_names(&z).contains("media"));
}

// --- T3 (review 5291226535): fixed timestamp => byte-identical packages ---
// across time (pinned zip mtimes, not wall clock). Sleep > DOS 2s resolution
// between writes so any wall-clock leak changes bytes.

#[test]
fn fixed_timestamp_writes_are_byte_identical() {
    let mut deck = Deck::new(123456, "foodeck");
    deck.add_note(Note::new(simple_model(), ["a", "b"]).unwrap());
    let pkg = Package::new(deck);

    let dir = tempfile::tempdir().unwrap();
    let p1 = dir.path().join("a.apkg");
    let p2 = dir.path().join("b.apkg");
    pkg.write_to_file_at(&p1, 1_600_000_000.0).unwrap();
    std::thread::sleep(std::time::Duration::from_secs(2)); // > DOS 2s resolution
    pkg.write_to_file_at(&p2, 1_600_000_000.0).unwrap();

    let b1 = std::fs::read(&p1).unwrap();
    let b2 = std::fs::read(&p2).unwrap();
    assert_eq!(
        b1, b2,
        "pinned zip mtimes + hermetic ids must yield identical bytes"
    );
}

// --- T7 end-to-end: suspend via cached cards -> queue = -1 ---

#[test]
fn suspended_cached_card_writes_queue_neg_one() {
    // Two templates -> two cards; suspend the second before adding to the
    // deck (the cached card list must survive the write via resolved_cards).
    let model = genanki::Model::new(345678, "cn")
        .field(genanki::Field::new("Trad"))
        .field(genanki::Field::new("Simpl"))
        .field(genanki::Field::new("Eng"))
        .template(genanki::Template::new("t0", "{{Trad}}", "x"))
        .template(genanki::Template::new("t1", "{{Simpl}}", "x"));
    let mut note = Note::new(model, ["a", "b", "c"]).unwrap().with_due(9);
    note.cards_mut().unwrap()[1].suspend = true;

    let mut deck = Deck::new(123456, "foodeck");
    deck.add_note(note);
    let pkg = Package::new(deck);
    let (_dir, path) = write_pkg(&pkg, 1_600_000_000.0);

    let mut z = open_zip(&path);
    let (_dbdir, conn) = open_collection(&mut z);
    let rows: Vec<(i32, i64, i64)> = conn
        .prepare("SELECT ord, queue, due FROM cards ORDER BY id")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(rows, vec![(0, 0, 9), (1, -1, 9)]);
}
