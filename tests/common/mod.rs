//! Shared structural-test helpers for the `.apkg` round-trip suites.
//!
//! Compiled into each integration-test crate; items unused by one crate are
//! expected, so dead-code is allowed here.
#![allow(dead_code)]

use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use genanki::{Field, Model, Template};
use rusqlite::Connection;
use zip::ZipArchive;

pub fn simple_model() -> Model {
    Model::new(1607392319, "Simple Model")
        .field(Field::new("Question"))
        .field(Field::new("Answer"))
        .template(Template::new(
            "Card 1",
            "{{Question}}",
            "{{FrontSide}}<hr id=\"answer\">{{Answer}}",
        ))
}

/// Write a package with a fixed timestamp inside a fresh temp dir.
pub fn write_pkg(pkg: &genanki::Package, ts: f64) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("out.apkg");
    pkg.write_to_file_at(&path, ts).unwrap();
    (dir, path)
}

pub fn open_zip(path: &Path) -> ZipArchive<File> {
    ZipArchive::new(File::open(path).unwrap()).unwrap()
}

pub fn entry_names(z: &ZipArchive<File>) -> HashSet<String> {
    (0..z.len())
        .map(|i| z.name_for_index(i).unwrap().to_string())
        .collect()
}

pub fn read_entry(z: &mut ZipArchive<File>, name: &str) -> Vec<u8> {
    let mut f = z.by_name(name).unwrap();
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).unwrap();
    buf
}

/// Extract `collection.anki2` from the zip into a temp file and open it.
pub fn open_collection(z: &mut ZipArchive<File>) -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let bytes = read_entry(z, "collection.anki2");
    let db_path = dir.path().join("collection.anki2");
    std::fs::write(&db_path, bytes).unwrap();
    let conn = Connection::open(&db_path).unwrap();
    (dir, conn)
}

pub fn col_json(conn: &Connection, col: &str) -> serde_json::Value {
    let raw: String = conn
        .query_row(&format!("SELECT {col} FROM col"), [], |r| r.get(0))
        .unwrap();
    serde_json::from_str(&raw).unwrap()
}
