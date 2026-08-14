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

/// Builtin Basic-family CSS as written into `col.models`; independent literal
/// (Python genanki v0.13.0 builtin_models.py). Keep in sync with the unit
/// `EXPECTED_BASIC_CSS` in `src/builtin_models.rs` (trailing newline included).
pub fn expected_basic_css() -> &'static str {
    concat!(
        ".card {\n",
        " font-family: arial;\n",
        " font-size: 20px;\n",
        " text-align: center;\n",
        " color: black;\n",
        " background-color: white;\n",
        "}\n",
    )
}

/// Builtin Cloze CSS as written into `col.models`; independent literal
/// (Python genanki v0.13.0 builtin_models.py). Keep in sync with the unit
/// `EXPECTED_CLOZE_CSS` in `src/builtin_models.rs`: the final `.nightMode
/// .cloze` segment has **no** trailing newline (Python concatenation).
pub fn expected_cloze_css() -> &'static str {
    concat!(
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
    )
}
