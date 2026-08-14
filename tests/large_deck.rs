//! Phase 6 (issue #8) large-deck smoke: write ~10k notes, assert zip/sqlite
//! counts, log wall time. Soft sanity only - no hard time/memory thresholds
//! (plan sec. 4.4); if CI ever OOMs, the follow-up is to reduce the count,
//! not to flake on timers.

mod common;

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use common::{entry_names, open_collection, open_zip};
use genanki::{BASIC_MODEL, Deck, Note, Package};

/// Note/card count for the smoke deck. BASIC_MODEL yields exactly one card
/// per note, so `notes == cards == NOTE_COUNT` keeps the math trivial.
const NOTE_COUNT: usize = 10_000;

/// Hermetic timestamp (2023-11-14) so ids are deterministic and the write is
/// byte-reproducible across runs.
const TS: f64 = 1_700_000_000.0;

#[test]
fn large_deck_10k_notes_writes_and_counts() {
    // Reuse one Arc<Model> for all notes (plan sec. 6.3: avoid per-note clones).
    let model = Arc::new((*BASIC_MODEL).clone());

    let mut deck = Deck::new(1_700_000_001, "Large Deck Smoke");
    for i in 0..NOTE_COUNT {
        deck.add_note(Note::new(Arc::clone(&model), [format!("Q{i}"), format!("A{i}")]).unwrap());
    }

    let pkg = Package::new(deck);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large.apkg");

    let t0 = Instant::now();
    pkg.write_to_file_at(&path, TS).unwrap();
    let elapsed = t0.elapsed();
    eprintln!("[large_deck] wrote {NOTE_COUNT} notes in {elapsed:?}");

    // Gross sanity only: the file exists and is far below a huge bound.
    let size = std::fs::metadata(&path).unwrap().len();
    assert!(size > 0);
    assert!(
        size < 200 * 1024 * 1024,
        "unexpectedly huge package ({size} bytes)"
    );

    let mut z = open_zip(&path);
    assert_eq!(
        entry_names(&z),
        HashSet::from(["collection.anki2".to_string(), "media".to_string(),])
    );
    assert_eq!(common::read_entry(&mut z, "media"), b"{}");

    let (_dbdir, conn) = open_collection(&mut z);
    let notes: i64 = conn
        .query_row("SELECT count(*) FROM notes", [], |r| r.get(0))
        .unwrap();
    let cards: i64 = conn
        .query_row("SELECT count(*) FROM cards", [], |r| r.get(0))
        .unwrap();
    assert_eq!((notes, cards), (NOTE_COUNT as i64, NOTE_COUNT as i64));

    // Spot-check first and last note flds (BASIC_MODEL: Front/Back).
    let first: String = conn
        .query_row("SELECT flds FROM notes ORDER BY id LIMIT 1", [], |r| {
            r.get(0)
        })
        .unwrap();
    let last: String = conn
        .query_row("SELECT flds FROM notes ORDER BY id DESC LIMIT 1", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(first, "Q0\x1fA0");
    assert_eq!(last, format!("Q{}\x1fA{}", NOTE_COUNT - 1, NOTE_COUNT - 1));
}
