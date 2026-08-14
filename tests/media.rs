//! Media handling in `.apkg` packages: basename map, payload blobs, and the
//! error cases (missing file, basename collision, path dedupe).

mod common;

use common::{entry_names, open_zip, read_entry, simple_model};
use genanki::{Deck, Error, Note, Package};

fn deck_with_one_note() -> Deck {
    let mut deck = Deck::new(1, "d");
    deck.add_note(Note::new(simple_model(), ["a", "b"]).unwrap());
    deck
}

// --- T15: media map + payloads (relative basenames) ---

#[test]
fn media_map_and_payloads() {
    let dir = tempfile::tempdir().unwrap();
    let mp3 = dir.path().join("present.mp3");
    let jpg = dir.path().join("present.jpg");
    std::fs::write(&mp3, b"mp3bytes").unwrap();
    std::fs::write(&jpg, b"jpgbytes").unwrap();

    let pkg = Package::new(deck_with_one_note()).media_files([mp3, jpg]);
    let out = dir.path().join("out.apkg");
    pkg.write_to_file_at(&out, 1_600_000_000.0).unwrap();

    let mut z = open_zip(&out);
    let names = entry_names(&z);
    assert!(
        names.contains("0") && names.contains("1"),
        "numbered media entries: {names:?}"
    );
    assert_eq!(read_entry(&mut z, "0"), b"mp3bytes");
    assert_eq!(read_entry(&mut z, "1"), b"jpgbytes");
    assert_eq!(
        read_entry(&mut z, "media"),
        br#"{"0":"present.mp3","1":"present.jpg"}"#
    );
}

// --- T16: media from subdirectories ---

#[test]
fn media_from_subdirs_basename_only_in_map() {
    let dir = tempfile::tempdir().unwrap();
    let sub1 = dir.path().join("subdir1");
    let sub2 = dir.path().join("subdir2");
    std::fs::create_dir_all(&sub1).unwrap();
    std::fs::create_dir_all(&sub2).unwrap();
    let mp3 = sub1.join("present.mp3");
    let jpg = sub2.join("present.jpg");
    std::fs::write(&mp3, b"mp3bytes").unwrap();
    std::fs::write(&jpg, b"jpgbytes").unwrap();

    let pkg = Package::new(deck_with_one_note()).media_files([mp3, jpg]);
    let out = dir.path().join("out.apkg");
    pkg.write_to_file_at(&out, 1_600_000_000.0).unwrap();

    let mut z = open_zip(&out);
    assert_eq!(read_entry(&mut z, "0"), b"mp3bytes");
    assert_eq!(read_entry(&mut z, "1"), b"jpgbytes");
    assert_eq!(
        read_entry(&mut z, "media"),
        br#"{"0":"present.mp3","1":"present.jpg"}"#,
        "map keys are basenames only"
    );
}

// --- T17: media from absolute paths ---

#[test]
fn media_from_absolute_paths() {
    let dir = tempfile::tempdir().unwrap();
    let abs = dir.path().canonicalize().unwrap().join("present.mp3");
    std::fs::write(&abs, b"absbytes").unwrap();

    let pkg = Package::new(deck_with_one_note()).media_files([abs]);
    let out = dir.path().join("out.apkg");
    pkg.write_to_file_at(&out, 1_600_000_000.0).unwrap();

    let mut z = open_zip(&out);
    assert_eq!(read_entry(&mut z, "0"), b"absbytes");
    assert_eq!(read_entry(&mut z, "media"), br#"{"0":"present.mp3"}"#);
}

// --- T18: missing media file -> error ---

#[test]
fn missing_media_file_errors() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("nope.mp3");
    let pkg = Package::new(deck_with_one_note()).media_files([missing]);
    let out = dir.path().join("out.apkg");
    let err = pkg.write_to_file_at(&out, 1.0).unwrap_err();
    assert!(matches!(err, Error::MediaNotFound { .. }));
    assert!(!out.exists(), "no zip written on media error");
}

// --- T19: basename collision -> error ---

#[test]
fn basename_collision_errors() {
    let dir = tempfile::tempdir().unwrap();
    let sub_a = dir.path().join("a");
    let sub_b = dir.path().join("b");
    std::fs::create_dir_all(&sub_a).unwrap();
    std::fs::create_dir_all(&sub_b).unwrap();
    let a = sub_a.join("foo.png");
    let b = sub_b.join("foo.png");
    std::fs::write(&a, b"1").unwrap();
    std::fs::write(&b, b"2").unwrap();

    let pkg = Package::new(deck_with_one_note()).media_files([a, b]);
    let out = dir.path().join("out.apkg");
    let err = pkg.write_to_file_at(&out, 1.0).unwrap_err();
    match err {
        Error::MediaBasenameCollision {
            basename,
            path_a,
            path_b,
        } => {
            assert_eq!(basename, "foo.png");
            assert!(path_a.ends_with("a/foo.png"), "first-seen path: {path_a:?}");
            assert!(path_b.ends_with("b/foo.png"), "second path: {path_b:?}");
        }
        other => panic!("unexpected {other:?}"),
    }
}

// --- T20: path dedupe ---

#[test]
fn duplicate_path_written_once() {
    let dir = tempfile::tempdir().unwrap();
    let mp3 = dir.path().join("present.mp3");
    std::fs::write(&mp3, b"mp3bytes").unwrap();

    let pkg = Package::new(deck_with_one_note()).media_files([mp3.clone(), mp3]);
    let out = dir.path().join("out.apkg");
    pkg.write_to_file_at(&out, 1_600_000_000.0).unwrap();

    let mut z = open_zip(&out);
    assert_eq!(read_entry(&mut z, "0"), b"mp3bytes");
    assert_eq!(
        read_entry(&mut z, "media"),
        br#"{"0":"present.mp3"}"#,
        "deduped to one media entry"
    );
    let names = entry_names(&z);
    assert_eq!(
        names.len(),
        3,
        "collection.anki2 + media + one blob: {names:?}"
    );
}
