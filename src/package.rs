//! `.apkg` package writer (SQLite + zip archive). (Phase 4)

use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::{Error, Result};

/// A collection of decks written to a single `.apkg` file, optionally with
/// media files.
///
/// Media files are embedded by basename (matching Anki's flat media folder);
/// two distinct paths sharing a basename is an error
/// ([`Error::MediaBasenameCollision`]). Writes are hermetic via
/// [`Self::write_to_file_at`] or wall-clock via [`Self::write_to_file`].
#[derive(Debug)]
pub struct Package {
    /// Decks in package order; written into one shared `col` row.
    pub(crate) decks: Vec<crate::Deck>,
    /// Media files in first-seen order (after path dedupe at write).
    pub(crate) media_files: Vec<PathBuf>,
}

impl Package {
    /// A package containing a single deck.
    #[must_use]
    pub fn new(deck: crate::Deck) -> Self {
        Self {
            decks: vec![deck],
            media_files: Vec::new(),
        }
    }

    /// A package containing many decks.
    #[must_use]
    pub fn from_decks(decks: impl IntoIterator<Item = crate::Deck>) -> Self {
        Self {
            decks: decks.into_iter().collect(),
            media_files: Vec::new(),
        }
    }

    /// Consume and set the media file list (first-seen order preserved after
    /// path dedupe at write time).
    #[must_use]
    pub fn media_files(mut self, files: impl IntoIterator<Item = impl Into<PathBuf>>) -> Self {
        self.media_files = files.into_iter().map(Into::into).collect();
        self
    }

    /// Append a media file path.
    pub fn add_media_file(&mut self, path: impl Into<PathBuf>) {
        self.media_files.push(path.into());
    }

    /// Decks in package order.
    #[must_use]
    pub fn decks(&self) -> &[crate::Deck] {
        &self.decks
    }

    /// Mutable access to decks (e.g. to suspend notes before write).
    #[must_use]
    pub fn decks_mut(&mut self) -> &mut [crate::Deck] {
        &mut self.decks
    }

    /// Write the `.apkg` using the current wall-clock time.
    ///
    /// See [`Self::write_to_file_at`] for the hermetic variant.
    pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        self.write_to_file_at(path, now_secs())
    }

    /// Write the `.apkg` with a fixed timestamp (seconds since Unix epoch).
    ///
    /// The timestamp drives the shared id generator (starting at
    /// `timestamp * 1000`) and the `mod` column of notes/cards/models, so a
    /// fixed value yields reproducible output.
    pub fn write_to_file_at<P: AsRef<Path>>(&self, path: P, timestamp_secs: f64) -> Result<()> {
        write_to_file_impl(&self.decks, &self.media_files, path, timestamp_secs)
    }
}

/// Shared write engine for [`Package`] and [`crate::Deck`] convenience
/// methods: temp sqlite -> `collection.anki2` + `media` JSON + numbered
/// blobs inside a zip.
pub(crate) fn write_to_file_impl(
    decks: &[crate::Deck],
    media_files: &[PathBuf],
    path: impl AsRef<Path>,
    timestamp_secs: f64,
) -> Result<()> {
    let media_plan = plan_media(media_files)?;

    // 1. Build the sqlite `collection.anki2` in a temp file.
    let tmp = tempfile::NamedTempFile::new()?;
    let conn = rusqlite::Connection::open(tmp.path())?;
    write_db(&conn, decks, timestamp_secs)?;
    conn.close().map_err(|(_, e)| e)?;

    // 2. Zip: collection.anki2, media JSON, then numbered media blobs.
    let file = std::fs::File::create(path.as_ref())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();

    zip.start_file("collection.anki2", options)?;
    {
        let mut src = std::fs::File::open(tmp.path())?;
        std::io::copy(&mut src, &mut zip)?;
    }

    let media_json = media_json_value(&media_plan)?;
    zip.start_file("media", options)?;
    zip.write_all(media_json.to_string().as_bytes())?;

    for (idx, media_path) in media_plan.iter().enumerate() {
        zip.start_file(idx.to_string(), options)?;
        let mut src = std::fs::File::open(media_path)?;
        std::io::copy(&mut src, &mut zip)?;
    }

    zip.finish()?;
    Ok(())
}

/// Apply schema + seed, then write every deck into the open connection.
///
/// Mirrors Python genanki `Package.write_to_db`: schema and seed first, then
/// per-deck writes sharing one id generator, all inside a transaction.
pub(crate) fn write_db(
    conn: &rusqlite::Connection,
    decks: &[crate::Deck],
    timestamp_secs: f64,
) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute_batch(crate::apkg::schema::APKG_SCHEMA)?;
    tx.execute_batch(crate::apkg::col::APKG_COL)?;
    let mut id_gen = crate::apkg::db::IdGen::new((timestamp_secs * 1000.0) as i64);
    for deck in decks {
        deck.write_to_db(&tx, timestamp_secs, &mut id_gen)?;
    }
    tx.commit()?;
    Ok(())
}

/// Path-dedupe (first-seen wins) and validate media files.
///
/// Errors with [`Error::MediaNotFound`] for a missing path, [`Error::MediaInvalidPath`]
/// when a path has no usable basename, and [`Error::MediaBasenameCollision`]
/// when two distinct paths share a basename.
fn plan_media(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut out: Vec<PathBuf> = Vec::new();
    let mut seen_paths: HashSet<PathBuf> = HashSet::new();
    let mut basename_owner: HashMap<String, PathBuf> = HashMap::new();

    for p in paths {
        if !seen_paths.insert(p.clone()) {
            continue; // path-dedupe, keep first
        }
        if !p.is_file() {
            return Err(Error::MediaNotFound { path: p.clone() });
        }
        let base = p
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| Error::MediaInvalidPath { path: p.clone() })?
            .to_string();
        if let Some(prev) = basename_owner.get(&base) {
            if prev != p {
                return Err(Error::MediaBasenameCollision {
                    basename: base,
                    path_a: prev.clone(),
                    path_b: p.clone(),
                });
            }
        } else {
            basename_owner.insert(base, p.clone());
        }
        out.push(p.clone());
    }
    Ok(out)
}

/// Build the `media` map: `{"0": basename0, "1": basename1, ...}` in
/// first-seen index order (empty package -> `{}`).
fn media_json_value(media_plan: &[PathBuf]) -> Result<serde_json::Value> {
    let mut map = serde_json::Map::new();
    for (idx, path) in media_plan.iter().enumerate() {
        let base = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| Error::MediaInvalidPath { path: path.clone() })?
            .to_string();
        map.insert(idx.to_string(), serde_json::Value::String(base));
    }
    Ok(serde_json::Value::Object(map))
}

/// Seconds since Unix epoch from the wall clock.
pub(crate) fn now_secs() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_media_dedupes_paths_preserving_first_seen() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.mp3");
        std::fs::write(&a, b"x").unwrap();
        let planned = plan_media(&[a.clone(), a.clone()]).unwrap();
        assert_eq!(planned, vec![a]);
    }

    #[test]
    fn plan_media_missing_file_errors() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope.mp3");
        let err = plan_media(&[missing]).unwrap_err();
        assert!(matches!(err, Error::MediaNotFound { .. }));
    }

    #[test]
    fn plan_media_basename_collision_errors() {
        let dir = tempfile::tempdir().unwrap();
        let sub_a = dir.path().join("a");
        let sub_b = dir.path().join("b");
        std::fs::create_dir_all(&sub_a).unwrap();
        std::fs::create_dir_all(&sub_b).unwrap();
        let a = sub_a.join("foo.png");
        let b = sub_b.join("foo.png");
        std::fs::write(&a, b"1").unwrap();
        std::fs::write(&b, b"2").unwrap();
        let err = plan_media(&[a.clone(), b.clone()]).unwrap_err();
        match err {
            Error::MediaBasenameCollision {
                basename,
                path_a,
                path_b,
            } => {
                assert_eq!(basename, "foo.png");
                assert_eq!(path_a, a);
                assert_eq!(path_b, b);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn media_json_value_index_order() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("present.mp3");
        let b = dir.path().join("present.jpg");
        std::fs::write(&a, b"x").unwrap();
        std::fs::write(&b, b"x").unwrap();
        let planned = plan_media(&[a, b]).unwrap();
        let v = media_json_value(&planned).unwrap();
        assert_eq!(
            v,
            serde_json::json!({"0": "present.mp3", "1": "present.jpg"})
        );
    }

    #[test]
    fn media_json_value_empty_is_empty_object() {
        assert_eq!(media_json_value(&[]).unwrap(), serde_json::json!({}));
    }
}
