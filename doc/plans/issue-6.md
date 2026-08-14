# Issue #6: Phase 4 - Deck + Package writer (.apkg zip, sqlite, media)

Status: IMPLEMENTED
Issue: https://github.com/tlkahn/genanki-rs/issues/6
Parent epic: https://github.com/tlkahn/genanki-rs/issues/1
Branch: `issue/6-deck-package`
Method: strict fine-grained TDD (RED -> GREEN -> refactor) per work item below

## 1. Goal

Land end-to-end `.apkg` production: `Deck`, `Package`, sqlite `collection.anki2`,
zip layout, and media. After this phase a consumer can build notes/models
(Phases 2-3), put them in a deck, and write a file Anki can import.

After this phase:

- `Deck` lives in `src/deck.rs` (id, name, description, notes, model registry)
- `Package` lives in `src/package.rs` (one or many decks, media file list)
- Write path: temp sqlite -> `APKG_SCHEMA` + `APKG_COL` -> id_gen -> decks /
  models / notes / cards -> zip (`collection.anki2`, `media` JSON, numbered
  blobs)
- Media: relative, absolute, subdir paths; **basename only** in the `media` map
  and note field references; basename collision -> error; missing file -> error;
  path-dedupe + deterministic ordering
- Hermetic `timestamp: Option<f64>` (seconds since epoch) for reproducible ids /
  `mod`
- `Deck::write_to_file` convenience -> `Package`
- Structural tests: open zip + sqlite, assert rows / JSON / media (no full Anki)
- `cargo fmt` / `clippy -D warnings` / `test` / `doc` green

### Out of scope

- Builtin model constants / README polish (Phase 5 / #7)
- `write_to_collection_from_addon` (Python/aqt only; epic non-goal)
- Reading or modifying existing `.apkg` as a public API (write-only library)
- Newer Anki formats beyond `collection.anki2` (stick to upstream)
- Anki desktop import in CI (structural sqlite/zip is enough for v1)
- CLOZE single-field auto-pad (already rejected in #5)
- Guaranteeing bit-identical packages with Python (semantic parity only)

## 2. Current state (code-verified)

| Item | Status |
| ---- | ------ |
| Phases 0-3 on `main` | Done (#2-#5 / PR #9-#12) |
| `src/deck.rs` | Stub module doc only |
| `src/package.rs` | Stub module doc only |
| `src/apkg/db.rs` | Stub module doc only ("Phase 6" - fix to Phase 4) |
| `src/apkg/{schema,col}.rs` | Verbatim `APKG_SCHEMA` / `APKG_COL` constants + fingerprint tests |
| `src/note.rs` | Full Note/Card gen; `pub(crate) format_fields` / `format_tags` ready |
| `src/card.rs` | `Card { ord, suspend }` |
| `src/model.rs` | `Model::to_json(timestamp_secs, deck_id) -> Result<Value>` |
| `src/error.rs` | `Internal`, `TemplateReq`, `TagContainsSpace`, `FieldCountMismatch` |
| `src/lib.rs` | Re-exports Note/Card/Model/...; **not** yet `Deck` / `Package` |
| `Cargo.toml` deps | `sha2`, `thiserror`, `serde`, `serde_json`, `regex`, `log` |
| `Cargo.toml` dev-deps | Empty |
| Python reference | genanki **v0.13.0** `deck.py` / `package.py` / `card.py` / `note.py`
  `write_to_db` (byte-same behavior as v0.13.1 / v1.13.1 for these paths) |

## 3. Locked decisions

| Topic | Decision | Rationale |
| ----- | -------- | --------- |
| External deps this phase | **`rusqlite` (bundled) + `zip` + `tempfile`** | Confirmed with maintainer (sec. 3.1). |
| SQLite crate | `rusqlite` with `features = ["bundled"]` | Cross-platform; no system libsqlite. Used for write + structural tests. |
| Zip crate | `zip` (default features OK; write + read in tests) | Pure Rust; powers production write and apkg open in tests. |
| Temp files | `tempfile` as **runtime** dependency | Matches Python `tempfile.mkstemp` pattern; confirmed. |
| Timestamp API | `Option<f64>` seconds since Unix epoch (Python parity) | `None` => `now()`; `Some(t)` => hermetic. `id_gen` starts at `(t * 1000.0) as i64`; `mod = t as i64`. Not `SystemTime` (harder to pin exactly in tests). |
| `id_gen` | Monotonic `i64` counter, start = `(timestamp_secs * 1000.0) as i64` | Python: `itertools.count(int(timestamp * 1000))`. Shared across notes and cards in package order. |
| Deck validation | `id: i64` and `name: String` always set by `Deck::new(id, name)`; no Option | Python allows `Deck()` then fails at write with `TypeError`. Rust makes invalid decks unrepresentable via constructor. Still test that empty name is rejected at write (or at `new`) - see sec. 3.2. |
| Empty name | **Reject at write** with `Error::DeckInvalid` if `name.is_empty()` | Mirrors "name required". Id is always present as `i64`. |
| Model registry | `BTreeMap<i64, Arc<Model>>` on Deck | Deterministic iteration; auto-register from notes at write; `add_model` for explicit register. |
| Note ownership in Deck | `Vec<Note>` by value | Notes are mutated for suspend before write; Deck owns them. |
| Media storage | `Vec<PathBuf>` on Package, order preserved after path-dedupe | Epic: path-dedupe; basename unique. |
| Media dedupe key | **Full path** (after `canonicalize` attempt? **No** - string/path equality as given) | Same `PathBuf` twice -> one entry. Different paths, same basename -> **error**. Do not canonicalize (symlink surprises; Python does not). |
| Media basename | `path.file_name()`; error if path has no file name (e.g. `..`) | Python `os.path.basename`. |
| Media missing on disk | `Error::MediaNotFound { path }` before/at zip write | Clearer than raw IO. |
| Basename collision | `Error::MediaBasenameCollision { basename, path_a, path_b }` | Epic open decision resolved: error. |
| Media map JSON | Object with string keys `"0"`, `"1"`, ... and basename values; empty => `{}` | Python `json.dumps({idx: basename})`. Use `serde_json::Map` insertion in index order. |
| Zip entry compression | Default `zip` deflate is fine (Anki accepts); do not special-case STORE | Match common tooling; not bit-identical to Python. |
| Zip contents | Exactly: `collection.anki2`, `media`, plus `"0".."N-1"` for each media blob | No extra entries. |
| Temp sqlite lifecycle | `tempfile::NamedTempFile` (or Builder suffix), write schema/rows, commit, close, then `zip.start_file("collection.anki2")` from path; temp auto-cleaned | Python mkstemp + unlink is implicit via OS; `tempfile` drops on close. Prefer explicit `keep` only if debugging. |
| In-memory sqlite? | **Not required** this phase (tempfile chosen) | Could revisit later to avoid disk. |
| DB write helpers location | `src/apkg/db.rs` for low-level execute/insert; `deck.rs` / `package.rs` / `note` write orchestration | Matches proposed layout. Note/card row insert can live as `pub(crate)` methods on Note/Card or free functions in `db.rs` - prefer free functions taking `&Note`/`&Card` to avoid bloating domain types with rusqlite types. |
| `Note` HTML re-check at write | **Optional re-warn** (Python re-checks in `write_to_db`); skip re-check in v1 (already warned at construct) | Avoid double warnings; document. Field count already enforced. |
| `col.decks` merge | Read seed JSON from `col`, insert/overwrite key `deck_id.to_string()`, write back | Preserve Default deck from `APKG_COL`. |
| `col.models` merge | Same pattern; keys = `model.id.to_string()` (serde_json object keys are strings) | Python int keys become strings via `json.dumps`. |
| Model JSON at write | `model.to_json(timestamp as i64, deck_id)?` | Already implemented in Phase 2. |
| Deck JSON | Fixed shape from Python `Deck.to_json` (sec. 4.2); `mod` stays constant `1425278051` like Python (not write timestamp) | Bit-parity with Python deck blob constants. |
| Multi-deck | `Package` holds `Vec<Deck>`; each `write_to_db` merges into same `col` | Shared `id_gen` across decks. |
| Nested deck names | Plain string `"Parent::Child"` - no special parsing | Anki convention; store as-is. |
| Public re-exports | `Deck`, `Package` from crate root | Epic API sketch. |
| Error growth | Add IO/Sql/Zip/Json/Media/Deck variants via `#[from]` where clean | See sec. 4.10. Keep `#[non_exhaustive]`. |
| TDD discipline | Every behavior change: failing test first, minimal impl, refactor. One logical assertion group per cycle when practical. | Per user request. |

### 3.1 External deps confirmation (explicit)

Maintainer request: minimize external deps; confirm anything we **must** add.

| Crate | Required? | Why |
| ----- | --------- | --- |
| `rusqlite` + `bundled` | **Yes (confirmed)** | Must execute `APKG_SCHEMA` / `APKG_COL` and insert notes/cards. No pure-Rust SQLite substitute is realistic for this. Bundled avoids system libsqlite. |
| `zip` | **Yes (confirmed)** | `.apkg` is a zip. Hand-rolled STORE+CRC32 rejected in favor of the crate (write + test read). |
| `tempfile` | **Yes as runtime dep (confirmed)** | Maintainer chose runtime over std-only temp paths. Used for temp `collection.anki2` during write. |
| `serde` / `serde_json` | Already present | `col.decks` / `col.models` / `media` JSON. |
| `sha2` / `thiserror` / `regex` / `log` | Already present | Reuse. |
| Anything else new | **No** | No `walkdir`, no `chrono` (use `std::time` + `f64`), no compression extras beyond `zip` defaults. |

Versions (pin latest compatible at implement time):

```toml
[dependencies]
rusqlite = { version = "0.32", features = ["bundled"] }  # or current 0.3x
zip = "2"       # or current 1.x/2.x stable at implement time
tempfile = "3"

[dev-dependencies]
# none required: rusqlite + zip already available to tests via the crate under test
```

**Bottom line:** this phase adds exactly three crates (`rusqlite`, `zip`, `tempfile`). All three were explicitly accepted.

### 3.2 Deck id/name validation nuance

Python:

```python
deck = genanki.Deck()          # deck_id=None, name=None
deck.write_to_file(...)        # TypeError
```

Rust constructors should not allow `None`. Locked API:

```rust
Deck::new(id: i64, name: impl Into<String>) -> Self
```

Acceptance "Missing deck id/name -> error" is satisfied by:

1. Type system: no Deck without id/name fields set.
2. Write-time check: reject **empty** `name` (`Error::DeckInvalid`).
3. Optional unit test documenting that `new` is the only constructor (no default).

Do **not** add `Deck::default()` that leaves invalid state.

## 4. Algorithm reference (must match Python v0.13.x semantics)

### 4.1 `Deck` shape

```rust
pub struct Deck {
    id: i64,
    name: String,
    description: String,                 // default ""
    notes: Vec<Note>,
    models: BTreeMap<i64, Arc<Model>>, // explicit + auto from notes
}
```

```rust
impl Deck {
    pub fn new(id: i64, name: impl Into<String>) -> Self;
    pub fn with_description(self, desc: impl Into<String>) -> Self;
    pub fn set_description(&mut self, desc: impl Into<String>);

    pub fn add_note(&mut self, note: Note);
    pub fn add_model(&mut self, model: impl Into<Arc<Model>>);

    pub fn id(&self) -> i64;
    pub fn name(&self) -> &str;
    pub fn description(&self) -> &str;
    pub fn notes(&self) -> &[Note];
    pub fn notes_mut(&mut self) -> &mut [Note]; // for suspend before write

    /// Convenience: `Package::new(self).write_to_file(path)`.
    pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()>;
    pub fn write_to_file_at<P: AsRef<Path>>(&self, path: P, timestamp_secs: f64) -> Result<()>;
}
```

Ownership note: `write_to_file` takes `&self` but needs `&mut Note` for
`cards()` lazy cache. Options:

1. Require cards already computed (call `note.cards()` before `add_note`), or
2. Take `&mut self`, or
3. Interior compute: `Note::cards` currently needs `&mut self`.

**Locked:** `write_to_file` / package write take `&mut self` on Package/Deck
**or** compute cards via a method that can run on `&Note` by not caching.

Prefer minimal API churn on Note: add `pub(crate) fn cards_for_write(&self) -> Result<Vec<Card>>` that computes without touching cache (or clones cache if present). Simplest path that preserves `&self` write:

```rust
// on Note
pub(crate) fn resolved_cards(&self) -> Result<Vec<Card>> {
    if let Some(c) = &self.cards { return Ok(c.clone()); }
    self.compute_cards() // make compute_cards accessible
}
```

Suspend workflow:

```rust
let mut note = Note::new(...)?;
note.cards_mut()?[1].suspend = true;
deck.add_note(note); // suspended state stored in cache inside Note
```

So write **must** prefer cached cards when present. `resolved_cards` above does that.

### 4.2 Deck JSON (`to_json`)

Exact Python shape (constant scheduling fields included):

```json
{
  "collapsed": false,
  "conf": 1,
  "desc": "<description>",
  "dyn": 0,
  "extendNew": 0,
  "extendRev": 50,
  "id": <deck_id>,
  "lrnToday": [163, 2],
  "mod": 1425278051,
  "name": "<name>",
  "newToday": [163, 2],
  "revToday": [163, 0],
  "timeToday": [163, 23598],
  "usn": -1
}
```

Build with `serde_json::json!`. `id` is a JSON number (Python keeps int).

### 4.3 `Deck::write_to_db` (orchestration)

```text
1. if name.is_empty(): return Err(DeckInvalid { reason: "name must be non-empty" })
2. decks = json::from_str( SELECT decks FROM col )
3. decks[id.to_string()] = deck.to_json()
4. UPDATE col SET decks = dumps(decks)

5. for note in notes: models.insert(note.model().id, note.model_arc())
6. models_json = json::from_str( SELECT models FROM col )
7. for (mid, model) in models:
     models_json[mid.to_string()] = model.to_json(timestamp as i64, deck.id)?
8. UPDATE col SET models = dumps(models_json)

9. for note in notes:
     insert_note_and_cards(cursor, note, timestamp, deck.id, id_gen)?
```

Model registry values: store `Arc<Model>` so note models and explicit
`add_model` share identity. When the same id is added twice, last write wins
(Python dict).

### 4.4 Note + Card DB rows

**notes** (11 columns), match Python `Note.write_to_db`:

| col | value |
| --- | ----- |
| id | `id_gen.next()` |
| guid | `note.guid()` |
| mid | `note.model().id` |
| mod | `timestamp as i64` |
| usn | `-1` |
| tags | `format_tags(note.tags())` -> `" " + join + " "` |
| flds | `format_fields(note.fields())` -> join `\x1f` |
| sfld | `note.sort_field()` (text bound into integer-typed column - SQLite affinity) |
| csum | `0` |
| flags | `0` |
| data | `""` |

Then `note_id = that id` (do not rely on `last_insert_rowid` alone if we set id
explicitly - using the same value we inserted is clearer).

**cards** (18 columns), match Python `Card.write_to_db`:

| col | value |
| --- | ----- |
| id | `id_gen.next()` |
| nid | note_id |
| did | deck_id |
| ord | card.ord |
| mod | `timestamp as i64` |
| usn | `-1` |
| type | `0` |
| queue | `-1` if `card.suspend` else `0` |
| due | `note.due()` |
| ivl, factor, reps, lapses, left, odue, odid, flags | `0` |
| data | `""` |

SQL:

```sql
INSERT INTO notes VALUES(?,?,?,?,?,?,?,?,?,?,?);
INSERT INTO cards VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?);
```

### 4.5 `Package` shape

```rust
pub struct Package {
    decks: Vec<Deck>,
    media_files: Vec<PathBuf>,
}

impl Package {
    /// One deck.
    pub fn new(deck: Deck) -> Self;
    /// Many decks.
    pub fn from_decks(decks: impl IntoIterator<Item = Deck>) -> Self;

    pub fn media_files(mut self, files: impl IntoIterator<Item = impl Into<PathBuf>>) -> Self;
    pub fn add_media_file(&mut self, path: impl Into<PathBuf>);

    pub fn decks(&self) -> &[Deck];
    pub fn decks_mut(&mut self) -> &mut [Deck];

    pub fn write_to_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.write_to_file_at(path, now_secs())
    }

    pub fn write_to_file_at<P: AsRef<Path>>(
        &mut self,
        path: P,
        timestamp_secs: f64,
    ) -> Result<()>;
}
```

`&mut self` so deck notes can resolve cards (if we mutate caches). If
`resolved_cards` is `&self`, Package write can take `&self`. **Prefer `&self`
write** if `resolved_cards` works on `&Note`; then Deck convenience is `&self`
too. Lock at implement time once Note helper lands; tests should not care.

### 4.6 `Package::write_to_file_at`

```text
1. media_plan = plan_media(&self.media_files)?
   // path-dedupe preserving first-seen order
   // basename collision -> error
   // each path must exist and be a file -> MediaNotFound

2. tmp = tempfile::NamedTempFile::new()?  // or Builder::new().suffix(".anki2")
   conn = rusqlite::Connection::open(tmp.path())?
   write_to_db(&conn, &self.decks, timestamp_secs)?
   conn.close()?  // flush

3. zip = ZipWriter::new(File::create(out_path)?)
   zip.start_file("collection.anki2", options)?
   copy tmp -> zip
   zip.start_file("media", options)?
   write media JSON map {"0": basename0, ...}  // or {}
   for (idx, path) in media_plan:
     zip.start_file(idx.to_string(), options)?
     copy file bytes -> zip
   zip.finish()?

4. tmp dropped / unlinked
```

`write_to_db`:

```text
conn.execute_batch(APKG_SCHEMA)?
conn.execute_batch(APKG_COL)?   // or execute the INSERT
id_gen = IdGen::new((timestamp_secs * 1000.0) as i64)
for deck in decks:
  deck.write_to_db(&conn, timestamp_secs, &mut id_gen)?
// commit if transaction used
```

Prefer a single explicit transaction around all deck writes for speed/atomicity:

```rust
let tx = conn.unchecked_transaction()?;
// ... schema already applied outside or inside
tx.commit()?;
```

Schema+col must run before inserts. Python runs schema/col then per-deck writes
then `conn.commit()`.

### 4.7 `id_gen`

```rust
struct IdGen { next: i64 }
impl IdGen {
    fn new(start: i64) -> Self { Self { next: start } }
    fn next(&mut self) -> i64 {
        let v = self.next;
        self.next = self.next.checked_add(1).expect("id_gen overflow");
        v
    }
}
```

Start value: `(timestamp_secs * 1000.0) as i64` - same truncation toward zero
as Python `int(timestamp * 1000)` for non-negative timestamps.

### 4.8 Media planning

```text
fn plan_media(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
  let mut out: Vec<PathBuf> = Vec::new();
  let mut seen_paths: HashSet<PathBuf> = HashSet::new();
  let mut basename_owner: HashMap<String, PathBuf> = HashMap::new();

  for p in paths {
    if !seen_paths.insert(p.clone()) {
      continue; // path-dedupe, keep first
    }
    if !p.is_file() {
      return Err(MediaNotFound { path: p.clone() });
    }
    let base = p.file_name()
      .and_then(|s| s.to_str())
      .ok_or_else(|| MediaInvalidPath { path: p.clone() })?
      .to_string();
    if let Some(prev) = basename_owner.get(&base) {
      if prev != p {
        return Err(MediaBasenameCollision { basename: base, path_a: prev.clone(), path_b: p.clone() });
      }
    } else {
      basename_owner.insert(base, p.clone());
    }
    out.push(p.clone());
  }
  Ok(out)
}
```

Deterministic ordering = first-seen order after path dedupe (stable, no
BTreeMap re-sort required). Issue mentioned `BTreeMap` as an option; **first-seen
stable order** matches Python list order better and stays deterministic.

### 4.9 `now_secs`

```rust
fn now_secs() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}
```

### 4.10 Error variants to add

```rust
#[error("deck invalid: {reason}")]
DeckInvalid { reason: &'static str },

#[error("media file not found: {path}")]
MediaNotFound { path: PathBuf },

#[error("media path has no usable basename: {path}")]
MediaInvalidPath { path: PathBuf },

#[error(
  "media basename collision for {basename:?}: {path_a} and {path_b}"
)]
MediaBasenameCollision {
    basename: String,
    path_a: PathBuf,
    path_b: PathBuf,
},

#[error(transparent)]
Io(#[from] std::io::Error),

#[error(transparent)]
Sqlite(#[from] rusqlite::Error),

#[error(transparent)]
Zip(#[from] zip::result::ZipError),

#[error(transparent)]
Json(#[from] serde_json::Error),
```

Notes:

- `thiserror` + `#[from]` keeps call sites clean.
- Display tests for new domain variants (`DeckInvalid`, media trio).
- IO/Sqlite/Zip/Json covered implicitly by `#[from]` + one smoke mapping test optional.
- Remove or keep `Internal` - keep for unexpected branches; do not use it for
  the paths above.

### 4.11 Structural test helper

```rust
// tests/apkg_roundtrip.rs (or tests/support + suites)
struct ApkgProbe {
    // temp dir holding extracted or open zip
}

fn write_and_open(pkg: &mut Package, ts: f64) -> (TempDir, Connection, MediaMap) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("out.apkg");
    pkg.write_to_file_at(&path, ts).unwrap();
    let file = File::open(&path).unwrap();
    let mut zip = zip::ZipArchive::new(file).unwrap();
    // read collection.anki2 to a temp file or to_vec + open bytes
    // rusqlite::Connection::open_from_memory? -> write bytes to temp and open
    ...
}
```

Opening sqlite from zip entry: copy `collection.anki2` bytes to
`tempfile::NamedTempFile`, `Connection::open(path)`.

Assert helpers:

- zip entry names set
- `media` JSON value
- `SELECT count(*) FROM notes/cards`
- `SELECT flds, tags, sfld, guid, mid FROM notes`
- `SELECT ord, queue, due, nid FROM cards ORDER BY id`
- `SELECT decks, models FROM col` -> parse JSON

## 5. Test plan (RED -> GREEN per item)

Prefer unit tests next to `deck.rs` / `package.rs` / `apkg/db.rs` for pure
pieces; integration suite `tests/apkg_roundtrip.rs` (and optionally
`tests/media.rs`) for zip+sqlite. Build small models inline (do not wait on
Phase 5 builtins).

Shared fixture:

```rust
fn simple_model() -> Model {
    Model::new(1607392319, "Simple Model")
        .field(Field::new("Question"))
        .field(Field::new("Answer"))
        .template(Template::new(
            "Card 1",
            "{{Question}}",
            "{{FrontSide}}<hr id=\"answer\">{{Answer}}",
        ))
}
```

### T0 - Deps compile + re-export smoke

**RED/GREEN**

- Add `rusqlite`, `zip`, `tempfile` to `Cargo.toml`.
- Empty `Deck`/`Package` types compiling; `pub use` from `lib.rs`.
- `cargo test` still green for Phases 1-3.

### T1 - `Deck` constructor + description + notes/models registry

**RED**

- `Deck::new(123, "foodeck")` accessors
- `with_description` / default `""`
- `add_note` increases `notes().len()`
- `add_model` registers in map; `add_note` does not need to register until write
  (auto-register tested at write)

**GREEN** - `src/deck.rs` data structure only (no IO).

### T2 - Deck JSON shape

**RED**

```rust
let d = Deck::new(112233, "foodeck")
    .with_description("This is my great deck.\nIt is so so great.");
let v = d.to_json();
assert_eq!(v["name"], "foodeck");
assert_eq!(v["id"], 112233);
assert_eq!(v["desc"], "This is my great deck.\nIt is so so great.");
assert_eq!(v["mod"], 1425278051);
assert_eq!(v["usn"], -1);
assert_eq!(v["conf"], 1);
// spot-check lrnToday / extendRev etc.
```

**GREEN** - `Deck::to_json`.

### T3 - Error variant displays

**RED/GREEN** - `DeckInvalid`, `MediaNotFound`, `MediaBasenameCollision`,
`MediaInvalidPath` Display strings.

### T4 - Schema + col execute on fresh sqlite

**RED**

```rust
let tmp = tempfile::NamedTempFile::new().unwrap();
let conn = Connection::open(tmp.path()).unwrap();
apkg::db::init_schema(&conn).unwrap();
let n: i64 = conn.query_row("SELECT count(*) FROM col", [], |r| r.get(0)).unwrap();
assert_eq!(n, 1);
let decks: String = conn.query_row("SELECT decks FROM col", [], |r| r.get(0)).unwrap();
assert!(decks.contains("Default"));
```

**GREEN** - `apkg::db::init_schema` runs `APKG_SCHEMA` + `APKG_COL` via
`execute_batch`.

### T5 - id_gen monotonic

**RED**

```rust
let mut g = IdGen::new(1000);
assert_eq!(g.next(), 1000);
assert_eq!(g.next(), 1001);
```

**GREEN**

### T6 - Insert one note + one card row (unit, no zip)

**RED**

- init schema
- insert note with known fields/tags/guid/due
- insert card ord=0 suspend=false
- assert `flds` uses `\x1f`, tags have surrounding spaces, `queue=0`, `due=...`

**GREEN** - `db::insert_note` / `insert_card` or orchestration helper.

### T7 - Suspend -> queue = -1; due propagates

**RED**

- two cards, second suspended
- `queue` values `[0, -1]`
- `due` equals `note.due()` on both

**GREEN**

### T8 - Single-deck package zip layout (no media)

**RED**

```rust
let mut deck = Deck::new(123456, "foodeck");
deck.add_note(Note::new(simple_model(), ["a", "b"]).unwrap());
let mut pkg = Package::new(deck);
let path = ...;
pkg.write_to_file_at(&path, 1_600_000_000.0).unwrap();

let z = ZipArchive::new(File::open(path).unwrap()).unwrap();
let names: HashSet<_> = (0..z.len()).map(|i| z.name_for_index(i).unwrap().to_string()).collect();
assert_eq!(names, hashset!{"collection.anki2", "media"});
// media file content == "{}"
```

**GREEN** - end-to-end write path.

### T9 - Notes/cards/decks/models content in sqlite

**RED** (same package as T8 or dedicated)

- 1 note, 1 card
- `col.decks` has key `"123456"` with name `foodeck`
- `col.models` has key `"1607392319"` with expected field names
- note `flds == "a\x1fb"`, `guid == guid_for(&["a","b"])`
- card `ord == 0`, `did == 123456`

**GREEN**

### T10 - Description preserved

**RED** - deck with multiline description; read `col.decks` JSON `desc`.

**GREEN**

### T11 - Multi-deck package

**RED**

```rust
let mut d1 = Deck::new(123456, "foodeck");
let mut d2 = Deck::new(654321, "bardeck");
d1.add_note(...);
d2.add_note(...);
Package::from_decks([d1, d2]).write_to_file_at(...)?;
// col.decks has both ids + Default
// notes count == 2
```

**GREEN**

### T12 - Nested deck name string

**RED** - `Deck::new(1, "Parent::Child")` survives into `col.decks` name field.

**GREEN** - no special casing required beyond storing the string.

### T13 - Fixed timestamp => deterministic ids

**RED**

```rust
// timestamp = 0.0
// first note id = 0, first card id = 1 (one-card note)
// timestamp = 1000.5 -> id_gen start = 1_000_500
pkg.write_to_file_at(path, 0.0)?;
// SELECT id FROM notes -> 0; SELECT id FROM cards ORDER BY id -> [1] or [1,2]
```

Also assert `mod` columns equal `0` when timestamp is `0.0`.

**GREEN**

### T14 - "Now" timestamp => modern card ids

**RED**

```rust
pkg.write_to_file(path)?; // real now
let card_id: i64 = ...;
assert!(card_id > 1_577_836_800_000); // Jan 1 2020 UTC ms
```

**GREEN** - default timestamp path uses `now_secs()`.

### T15 - Media map + payloads (relative basenames)

**RED**

- write tiny files `present.mp3`, `present.jpg` in temp dir
- note fields reference basenames only
- `Package::new(deck).media_files([mp3, jpg])`
- zip has entries `0`, `1` with exact bytes
- `media` JSON `{"0":"present.mp3","1":"present.jpg"}` (order = first-seen)

**GREEN**

### T16 - Media from subdirs

**RED** - paths `subdir1/present.mp3`, `subdir2/present.jpg`; map basenames only;
payloads correct.

**GREEN**

### T17 - Media from absolute paths

**RED** - absolute temp paths; same basename behavior.

**GREEN**

### T18 - Missing media file -> error

**RED**

```rust
let err = pkg.write_to_file_at(path, 1.0).unwrap_err();
assert!(matches!(err, Error::MediaNotFound { .. }));
```

**GREEN** - `plan_media` existence check.

### T19 - Basename collision -> error

**RED**

```rust
// /tmp/a/foo.png and /tmp/b/foo.png both listed
assert!(matches!(err, Error::MediaBasenameCollision { .. }));
```

**GREEN**

### T20 - Path dedupe

**RED** - same `PathBuf` twice in media list -> one zip entry, media JSON length 1.

**GREEN**

### T21 - Empty name rejected at write

**RED**

```rust
let deck = Deck::new(1, "");
let err = deck.write_to_file(path).unwrap_err();
assert!(matches!(err, Error::DeckInvalid { .. }));
```

**GREEN**

### T22 - Models auto-registered from notes; explicit add_model

**RED**

- note with model M => `col.models` contains M.id after write without
  `add_model`
- `add_model` with unused model still appears in `col.models` (Python keeps
  explicit registry entries even if no notes use them - verify: yes, Python
  merges `self.models` after adding note models)

**GREEN**

### T23 - Latex pre/post + sortf round-trip into model JSON in sqlite

**RED** - custom model; after write, `models[id].latexPre/Post/sortf` match.

**GREEN** - already in `Model::to_json`; this is integration glue.

### T24 - Tags + flds formatting integration

**RED** - note with tags `["foo","bar"]` => `tags` column `" foo bar "`; two
fields => `\x1f` join.

**GREEN** - uses existing `format_fields` / `format_tags`.

### T25 - `Deck::write_to_file` convenience

**RED** - single call writes valid apkg equivalent to Package path.

**GREEN**

### T26 - Full workspace gates

```text
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo test --doc
```

## 6. Implementation order (fine-grained cycles)

Do not batch large green steps. Suggested sequence:

1. **Deps** - add `rusqlite` (bundled), `zip`, `tempfile` to `Cargo.toml`.
2. **Errors** - new variants + Display tests (T3).
3. **Deck data + JSON** - T1 T2 (no IO).
4. **IdGen** - T5.
5. **apkg::db::init_schema** - T4.
6. **Note/Card insert helpers** - T6 T7; wire `format_fields` / `format_tags`;
   add `Note::resolved_cards` (`pub(crate)`).
7. **Deck::write_to_db** - merge decks/models JSON + insert notes (unit tests
   against open `Connection`) - pieces of T9 T10 T22 T23 T24.
8. **Package write zip (no media)** - T8 T9 T10 T11 T12 T13 T14 T25.
9. **Media planning + zip blobs** - T15 T16 T17 T18 T19 T20.
10. **Empty name / validation** - T21.
11. **Re-exports, docs, dead_code cleanup** on format helpers.
12. **Gates** - T26.
13. **Refactor** only while green (extract test helpers, shrink duplication,
    module docs, fix `apkg/db.rs` phase comment).

### Transaction / connection details

- Use `Connection::execute_batch(APKG_SCHEMA)` - schema string is multiple
  statements.
- `APKG_COL` is a single INSERT (with embedded newlines); `execute_batch` or
  `execute` both work.
- Bind `sfld` as text (`note.sort_field()`); SQLite stores it despite integer
  type affinity - matches Python.
- Disable foreign keys etc. not required (schema has none).

### Clippy / edition notes

- Edition 2024 already in Cargo.toml - keep.
- `#![forbid(unsafe_code)]` remains; `rusqlite` bundled uses unsafe internally
  but our crate code stays safe.
- Avoid `unwrap` in library code; tests may unwrap.

## 7. File touch list

| Path | Action |
| ---- | ------ |
| `Cargo.toml` | add `rusqlite` (bundled), `zip`, `tempfile` |
| `src/error.rs` | new variants + tests |
| `src/deck.rs` | `Deck`, JSON, write_to_db, write_to_file |
| `src/package.rs` | `Package`, media plan, zip write |
| `src/apkg/db.rs` | `init_schema`, note/card inserts, maybe id_gen |
| `src/apkg/mod.rs` | export db items as needed; fix module docs |
| `src/note.rs` | `pub(crate) resolved_cards`; drop `allow(dead_code)` on formatters when used |
| `src/card.rs` | docs only (queue mapping lives at insert site) |
| `src/lib.rs` | re-export `Deck`, `Package` |
| `tests/apkg_roundtrip.rs` | structural zip+sqlite suite |
| `tests/media.rs` | optional split of media cases |
| `doc/plans/issue-6.md` | this plan (status -> IMPLEMENTED when done) |

No changes expected to `guid.rs` / `req.rs` / `model.rs` / schema constants
beyond consumption.

## 8. Acceptance mapping

| Acceptance (issue #6) | Tests |
| --------------------- | ----- |
| Single-deck `.apkg` has valid zip layout | T8 T9 |
| Multi-deck package writes both deck entries | T11 |
| Description preserved in deck JSON | T10 |
| Media map + payloads for subdir and absolute paths | T15 T16 T17 |
| Missing media file on disk -> error | T18 |
| Missing deck id/name -> error | T21 (+ type system for id) |
| Fixed timestamp => deterministic note/card ids | T13 |
| "Now" timestamp => modern card ids (not epoch) | T14 |
| `due` and `suspend` reflected in `cards` rows | T7 T9 |
| Notes: `flds` joined with `\x1f`, tags with surrounding spaces | T6 T24 |
| Models auto-registered from notes | T22 |
| Nested deck names as plain strings | T12 |

## 9. Phase 5 handoff notes

Leave ready for #7 without implementing it:

- `Package::write_to_file` works with any `Model`/`Note`/`Deck`
- Builtin constants can be thin wrappers over existing `Model::new` builders
- README example in epic should compile once builtins land (or with inline model)
- Structural test helpers in `tests/` can be reused for builtin write smoke
- Media rules ready to document in README (basename-only field refs)

## 10. Open questions - RESOLVED

| Question | Resolution |
| -------- | ---------- |
| Plan path | `genanki-rs/doc/plans/issue-6.md` |
| `rusqlite` | Yes, `bundled` (confirmed) |
| `zip` | Yes, crate (confirmed) |
| `tempfile` | Yes, **runtime** dep (confirmed) |
| Timestamp type | `Option<f64>` / `write_to_file_at(..., f64)` seconds |
| Deck id/name Option | Non-optional fields via `Deck::new`; empty name err at write |
| Media dedupe | By path, first-seen order; basename collision errors |
| Note cards during write | `resolved_cards()` prefers cache (preserves suspend) |
| HTML re-scan at write | Skip (already at construct) |
| Addon write path | Out of scope |

No unresolved blockers. Implement on branch `issue/6-deck-package` with strict TDD.

## 11. PR checklist (when implementing)

- [ ] Every work item went RED before GREEN
- [ ] Only `rusqlite` (bundled) + `zip` + `tempfile` added (plus existing deps)
- [ ] No builtin models / README feature creep (Phase 5)
- [ ] No `write_to_collection_from_addon`
- [ ] Basename collision + missing media errors covered
- [ ] Hermetic timestamp + modern-now ids covered
- [ ] Suspend/due/flds/tags asserted in sqlite
- [ ] Multi-deck + description covered
- [ ] `cargo fmt`, `clippy -D warnings`, `test`, `doc` green
- [ ] Issue #6 checkboxes updated in PR description
- [ ] Plan status -> IMPLEMENTED + PR link

## 12. Quick reference - Python -> Rust map

| Python | Rust |
| ------ | ---- |
| `Deck(id, name, description=...)` | `Deck::new(id, name).with_description(...)` |
| `deck.add_note(note)` | `deck.add_note(note)` |
| `deck.add_model(model)` | `deck.add_model(model)` |
| `deck.write_to_file(path)` | `deck.write_to_file(path)?` |
| `Package(deck)` | `Package::new(deck)` |
| `Package([d1,d2], media_files=[...])` | `Package::from_decks([d1,d2]).media_files([...])` |
| `pkg.write_to_file(path, timestamp=None)` | `pkg.write_to_file(path)?` / `write_to_file_at(path, ts)?` |
| `itertools.count(int(ts*1000))` | `IdGen::new((ts * 1000.0) as i64)` |
| `os.path.basename(path)` | `path.file_name()` |
| note `cards[i].suspend = True` | `note.cards_mut()?[i].suspend = true` before `add_note` |
