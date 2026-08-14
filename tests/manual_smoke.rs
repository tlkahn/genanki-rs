//! Phase 6 (issue #8) manual-smoke artifact: write a small `.apkg` under
//! `target/` that a human can import into Anki desktop to confirm the crate
//! produces importable packages (plan sec. 4.9, 3.3). Always runs and is
//! cheap; the absolute path is printed so CI logs and the PR record can cite
//! it. Media is included (a 1x1 transparent PNG) to exercise the numbered
//! blob + `media` map path in a human-visible way.

mod common;

use std::path::PathBuf;

use common::{entry_names, open_zip};
use genanki::{BASIC_MODEL, CLOZE_MODEL, Deck, Note, Package};

/// 1x1 transparent PNG (67 bytes) referenced from the basic note's back so
/// the imported card visibly renders media.
const PIXEL_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
    0x42, 0x60, 0x82,
];

/// Where the smoke package lands: `target/manual-smoke.apkg` (gitignored).
fn smoke_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/manual-smoke.apkg")
}

#[test]
fn write_manual_smoke_apkg() {
    // Media file: a tiny PNG written into a temp dir (Anki imports it into
    // its own media folder; we only need the basename in the map).
    let media_dir = tempfile::tempdir().unwrap();
    let png_path = media_dir.path().join("smoke.png");
    std::fs::write(&png_path, PIXEL_PNG).unwrap();

    let mut deck = Deck::new(1_700_000_002, "Manual Smoke Deck");

    // Basic note with unicode and a media reference in the answer.
    deck.add_note(
        Note::new(
            &*BASIC_MODEL,
            [
                "Ünïcödé question: capital of France?",
                "Paris <img src=\"smoke.png\"> (éàü)",
            ],
        )
        .unwrap(),
    );

    // Cloze note (two required fields: Text + Back Extra).
    deck.add_note(
        Note::new(
            &*CLOZE_MODEL,
            [
                "{{c1::Berlin}} is the capital of {{c2::Germany}}. 東京",
                "Extra on the back: 東京",
            ],
        )
        .unwrap(),
    );

    let pkg = Package::new(deck).media_files([png_path]);
    let path = smoke_path();
    pkg.write_to_file(&path).unwrap();

    // Structural sanity so the artifact is verified automatically too.
    let mut z = open_zip(&path);
    assert_eq!(
        entry_names(&z),
        std::collections::HashSet::from([
            "collection.anki2".to_string(),
            "media".to_string(),
            "0".to_string(),
        ])
    );
    assert_eq!(common::read_entry(&mut z, "media"), br#"{"0":"smoke.png"}"#);
    assert_eq!(common::read_entry(&mut z, "0"), PIXEL_PNG);

    eprintln!(
        "[manual_smoke] wrote {} for manual Anki import (File -> Import...)",
        path.display()
    );
}
