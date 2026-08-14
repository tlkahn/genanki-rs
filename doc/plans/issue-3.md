# Issue #3: Phase 1 - GUID + primitives (base91, schema, error type)

Status: IMPLEMENTED
PR: https://github.com/tlkahn/genanki-rs/pull/10
Issue: https://github.com/tlkahn/genanki-rs/issues/3
Parent epic: https://github.com/tlkahn/genanki-rs/issues/1
Branch: `issue/3-guid-primitives`
Method: strict fine-grained TDD (RED -> GREEN -> refactor) per work item below

## 1. Goal

Land foundational utilities and on-disk constants. No package writer, no Model/Note/Deck.

After this phase:

- `guid_for` + Anki base91 alphabet live in `src/guid.rs`, **byte-identical** to Python genanki
- Golden vectors (cross-checked against Python) are locked in unit tests
- `Error` / `Result` use `thiserror` (minimal variants; GUID path is infallible)
- `APKG_SCHEMA` and `APKG_COL` string constants are verbatim from upstream and smoke-tested
- `cargo test` / `clippy` / `fmt` green; CI stays green

### Out of scope

- Model / Note / Card / Deck / Package behavior (Phases 2-4)
- Executing schema SQL against sqlite (Phase 4 / #6)
- Media, zip, rusqlite, serde, regex
- Builtin models, README API examples
- Cross-process Python invocation in CI (goldens are precomputed and committed)

## 2. Current state (code-verified)

| Item | Status |
| ---- | ------ |
| Phase 0 scaffolding on `main` | Done (#2 / PR #9) |
| `src/guid.rs` | Stub module doc only |
| `src/error.rs` | Manual `Error::Internal` + `Display` + `std::error::Error` (no deps) |
| `src/apkg/schema.rs` | Stub module doc only |
| `src/apkg/col.rs` | Stub module doc only |
| `Cargo.toml` `[dependencies]` | Empty |
| `guid_for` public re-export | Not yet (`lib.rs` only re-exports `Error`, `Result`) |
| Python reference pin | Epic cites genanki **v0.13.1**; upstream tags are `v0.13.0` and `v1.13.1`. `util.py` / `apkg_schema.py` / `apkg_col.py` are **byte-identical** across `v0.13.0` and `v1.13.1`. Treat **v0.13.0 content** as the source of truth for this phase (same bytes as v1.13.1 for these three files). |

## 3. Locked decisions

| Topic | Decision | Rationale |
| ----- | -------- | --------- |
| External deps | **`sha2` + `thiserror` only** | SHA-256 is required for GUID parity; std has no hashes. `thiserror` matches issue #3 wording and sets the pattern for later `From` impls. No other deps this phase. |
| `sha2` version | `sha2 = "0.10"` (or current `0.10`/`0.11` compatible - pin latest stable 0.x at implement time) | Pure Rust; widely used; no OpenSSL. |
| `thiserror` version | `thiserror = "2"` (latest 2.x at implement time) | Edition-2024-friendly; keeps derive surface small. |
| `guid_for` signature | `pub fn guid_for(values: &[&str]) -> String` | Simple, explicit, no macros. Call site: `guid_for(&["a", "b"])`. Python accepts any `str()`-able values; Rust v1 takes string slices only (callers stringify first). |
| GUID fallibility | **Infallible** - always returns `String` | Matches Python. No `Result` in the GUID API. |
| `Error` scope this phase | **Minimal**: migrate to `thiserror`, keep a single placeholder variant (rename/docs OK). Do **not** pre-add Io/Sql/Validation variants until a code path produces them. | Avoid dead API surface. Phase 0 test (`error_display`) stays meaningful. |
| Base91 alphabet storage | `pub const BASE91_TABLE: &[u8; 91]` or `&[u8]` of the 91 ASCII bytes; encode via `as char` / `as u8` | Avoid `String` per digit; alphabet is pure ASCII. Public so tests (and later debug) can inspect. |
| Schema / col constants | `pub const APKG_SCHEMA: &str` and `pub const APKG_COL: &str` | Verbatim from Python triple-quoted strings, including newlines and JSON whitespace **as upstream ships them**. Do not "pretty-print" or normalize. |
| Public re-exports | Re-export `guid_for` (and optionally `BASE91_TABLE`) from `lib.rs`. Schema/col stay under `genanki::apkg::{schema,col}` (not crate-root) unless a later phase wants shorter paths. | Matches epic public API sketch (`pub use crate::guid::guid_for`). |
| Golden source | Precomputed via Python 3 `hashlib` against the algorithm in `util.py`; committed as Rust asserts. Document the generator snippet in this plan / test module comment. | No Python runtime in CI. |
| TDD discipline | Every behavior change: write failing test first, then minimal implementation, then refactor. One logical assertion group per cycle when practical. | Per user request for this phase. |

### External deps confirmation (explicit)

| Crate | Required? | Why |
| ----- | --------- | --- |
| `sha2` | **Yes** | SHA-256 of the joined field string. No std equivalent. Vendoring SHA-256 was rejected in favor of the crate. |
| `thiserror` | **Yes (chosen)** | Not strictly required (manual `Error` already works), but selected to match issue #3 and future `#[from]` growth. |
| Anything else | **No** | Schema/col are `&str` constants. Base91 is ~15 lines. No regex/serde/rusqlite/zip this phase. |

If a future review wants zero crates.io deps, the only candidate to drop is `thiserror` (keep manual `Error`). `sha2` stays unless SHA-256 is vendored.

## 4. Algorithm reference (must match exactly)

### 4.1 `guid_for` (from `genanki/util.py`)

```text
1. hash_str = values.joined_with("__")   # Python: '__'.join(str(val) for val in values)
2. digest  = SHA256(hash_str as UTF-8)
3. Take first 8 bytes of digest as big-endian u64:
     hash_int = 0
     for b in digest[0..8]:
       hash_int = (hash_int << 8) + b
4. Base91-encode hash_int with Anki alphabet (below):
     digits = []
     while hash_int > 0:
       digits.push(ALPHABET[hash_int % 91])
       hash_int /= 91
     return digits.reversed().concatenated
5. Edge: if hash_int == 0 after step 3, Python returns "" (loop never runs).
   Practically unreachable for SHA-256 prefixes, but implement the same branch.
```

Rust notes:

- `&[&str]` join: `values.join("__")` (std).
- Empty slice `guid_for(&[])` => join is `""` => same as Python `guid_for()` with no args.
- Do **not** trim, NFC-normalize, or alter Unicode; hash the UTF-8 bytes as given.
- Use `sha2::Sha256` + `Digest` trait; take `hasher.finalize()` / `.digest()` first 8 bytes.
- Prefer `u64` for `hash_int` (8 bytes fit exactly). Use wrapping-free arithmetic; value is always `< 2^64`.

### 4.2 Base91 alphabet (91 chars, exact order)

```text
abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!#$%&()*+,-./:;<=>?@[]^_`{|}~
```

- Length 91.
- **No** `"` (double quote), **no** `\` (backslash), **no** space.
- Order matters; index `0 => 'a'`, not a sorted set.

### 4.3 `APKG_SCHEMA`

Copy verbatim from upstream `apkg_schema.py` `APKG_SCHEMA` triple-quoted string:

- Tables: `col`, `notes`, `cards`, `revlog`, `graves`
- Indexes: `ix_notes_usn`, `ix_cards_usn`, `ix_revlog_usn`, `ix_cards_nid`, `ix_cards_sched`, `ix_revlog_cid`, `ix_notes_csum`
- Do not "fix" the quirky `notes.sfld integer` typing - Anki is picky; match Python.

### 4.4 `APKG_COL`

Copy verbatim from upstream `apkg_col.py` `APKG_COL` raw triple-quoted string (`r'''...'''`):

- Single `INSERT INTO col VALUES(...)` seed row
- Embeds default deck id `1` named `"Default"`, default conf/dconf JSON
- Preserve internal JSON spacing/newlines as upstream has them (raw string content)

Implementation tip: use a Rust raw string `r#"..."#` (or `r###"..."###` if needed) so quotes inside JSON need no escaping. Diff against upstream file after paste.

## 5. Target API surface after Phase 1

```rust
// src/guid.rs
/// Anki base91 alphabet used by [`guid_for`] (91 ASCII bytes, fixed order).
pub const BASE91_TABLE: &[u8; 91] = b"abcdefghijklmnopqrstuvwxyz..."; // full 91

/// Compute a note GUID the same way Python genanki `guid_for` does.
///
/// Fields are joined with `"__"`, SHA-256'd, first 8 bytes taken as a big-endian
/// integer, then encoded with [`BASE91_TABLE`].
pub fn guid_for(values: &[&str]) -> String { ... }

// src/error.rs
/// Errors produced by this crate.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Internal placeholder until concrete failure modes land in later phases.
    #[error("internal error: {0}")]
    Internal(&'static str),
}

pub type Result<T> = std::result::Result<T, Error>;

// src/apkg/schema.rs
/// SQLite DDL executed when creating a new `collection.anki2`.
pub const APKG_SCHEMA: &str = r#"..."#;

// src/apkg/col.rs
/// Seed `INSERT` for the single `col` row (default deck / conf / dconf).
pub const APKG_COL: &str = r#"..."#;

// src/lib.rs additions
pub use crate::guid::guid_for;
// Error/Result already re-exported
```

## 6. TDD plan (fine-grained cycles)

Work in this order. Each cycle: **RED** (failing test / compile failure proving the gap) -> **GREEN** (minimal code) -> **refactor** (names, docs, tiny cleanups; tests stay green).

Do not implement multiple features ahead of their tests. Prefer many small commits or at least clearly separated local steps.

### Cycle 0 - Branch + dep pins (no behavior yet)

1. Branch from latest `main`.
2. Add to `Cargo.toml`:

   ```toml
   [dependencies]
   sha2 = "0.10"       # pin exact latest 0.10.x / 0.11.x chosen at implement time
   thiserror = "2"
   ```

3. `cargo check` still passes with unused deps? If clippy `unused_crate_dependencies` is not enabled, fine. Otherwise a temporary `use sha2 as _;` is **not** desired - move straight into Cycle 1/2 the same session so deps are used.
4. No tests required for "deps exist".

### Cycle 1 - `thiserror` migration of `Error` (behavior-preserving)

**RED**

- Keep existing test `error_display` asserting `Error::Internal("scaffold").to_string() == "internal error: scaffold"`.
- Delete manual `Display` + `std::error::Error` impls and the hand-rolled enum derives in a first edit so the crate fails to compile / test fails - **or** more cleanly: switch the enum to `#[derive(Debug, thiserror::Error)]` with `#[error("internal error: {0}")]` in one step while keeping the test. Strict RED: briefly break the message format and watch the test fail, then fix - optional if too theatrical; minimum bar is **test remains and passes after migration**.

**GREEN**

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Placeholder until concrete failure modes land in later phases.
    #[error("internal error: {0}")]
    Internal(&'static str),
}
```

**Refactor**

- Confirm `Result` alias unchanged.
- Confirm `std::error::Error` is implemented via derive (source trait).
- No new variants.

**Verify:** `cargo test error:: -q`

### Cycle 2 - Base91 alphabet constant

**RED**

Add tests in `src/guid.rs`:

```rust
#[test]
fn base91_table_len_is_91() {
    assert_eq!(BASE91_TABLE.len(), 91);
}

#[test]
fn base91_table_bytes_match_anki_order() {
    const EXPECTED: &[u8; 91] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!#$%&()*+,-./:;<=>?@[]^_`{|}~";
    assert_eq!(BASE91_TABLE, EXPECTED);
}

#[test]
fn base91_table_excludes_quote_backslash_space() {
    assert!(!BASE91_TABLE.contains(&b'"'));
    assert!(!BASE91_TABLE.contains(&b'\\'));
    assert!(!BASE91_TABLE.contains(&b' '));
}
```

Compile fails because `BASE91_TABLE` missing.

**GREEN**

Define `pub const BASE91_TABLE: &[u8; 91] = b"...";`

**Refactor**

- Single source of truth; encoding helper (next cycles) must index this constant, not a private copy.

**Verify:** `cargo test guid:: -q`

### Cycle 3 - `guid_for` golden: simplest multi-field case

**RED**

```rust
#[test]
fn guid_for_a_b_matches_python() {
    // Python genanki util.guid_for("a", "b") on v0.13.0 / v1.13.1
    assert_eq!(guid_for(&["a", "b"]), "q/([o$8RAO");
}
```

Fails: `guid_for` missing.

**GREEN**

Implement full `guid_for` (algorithm section 4.1). It is OK that one implementation lands here even though later cycles add more asserts - do not stub a hard-coded `"q/([o$8RAO"` return.

**Refactor**

- Private helper `fn base91_encode(mut n: u64) -> String` if it clarifies the loop.
- Docs on `guid_for` noting Python parity and join separator.

**Verify:** `cargo test guid:: -q`

### Cycle 4 - More goldens (edge + unicode + multi-field)

**RED then GREEN** by adding asserts one group at a time (or one test function with a table - table-driven is fine if the **first** run fails before impl is complete; after Cycle 3 impl exists, these should pass immediately if algorithm is correct - if any fail, fix algorithm, do not special-case).

Precomputed goldens (Python 3, `hashlib.sha256`, alphabet above):

| Input (`&[&str]` ) | Expected GUID |
| ------------------ | ------------- |
| `["a", "b"]` | `q/([o$8RAO` |
| `[""]` | `ME_YHw2?15` |
| `["", ""]` | `z+VBQ9+v.E` |
| `["hello"]` | `hZ%+.BW-%^` |
| `["Capital of Argentina", "Buenos Aires"]` | `HSnG{z%dU<` |
| `["日本語", "テスト"]` | `C#Zh1EL^|P` |
| `["emoji 😀", "x"]` | `GxV$K=ya?h` |
| `["a", "b", "c"]` | `xEWY5:a/2E` |
| `["only-one"]` | `zRins]8c)T` |
| `["with__double", "underscore"]` | `kE!|u[<uRg` |
| `["\u{1f}"]` (unit separator) | `RisW_+fz{*` |
| `[" leading", "trailing "]` | `r{^OG#`_~o` |
| `["line\nbreak"]` | `r*A<&TFxl@` |
| `[]` (no fields) | same as `[""]` join path -> `ME_YHw2?15` |

Recommended tests:

1. `guid_for_goldens_table` - table-driven over the rows above.
2. `guid_for_empty_slice_matches_single_empty_string` - documents `[]` vs `[""]` both hash the empty join string.
3. `guid_for_is_stable` - call twice, equal.
4. `guid_for_field_order_matters` - `["a","b"] != ["b","a"]`.
5. `guid_for_separator_is_double_underscore` - document that `"a__b"` as single field differs from `["a","b"]` (compute expected via Python when implementing; do not guess).

**Python regenerator** (paste into comment at top of `guid` tests; not run in CI):

```python
# python3 -c '...'  against genanki.util or inlined alphabet+fn from util.py
# Pin note: values below generated with CPython 3 + stdlib hashlib;
# algorithm identical to kerrickstaley/genanki v0.13.0 util.guid_for.
```

Optional: add a tiny ignored test or `#[cfg(False)]` module with the script. Prefer a comment to avoid bitrot of ignored tests.

**Verify:** full `cargo test guid::`

### Cycle 5 - `base91_encode` edge (hash_int == 0)

**RED**

If `base91_encode` is `pub(crate)` or tested via a pure unit:

```rust
#[test]
fn base91_encode_zero_is_empty_string() {
    assert_eq!(base91_encode(0), "");
}
```

If helper stays private, either:

- `#[cfg(test)]` use of a private fn in the same module (allowed), or
- skip dedicated test and rely on documentation comment only.

Prefer testing the private helper in-module - this locks Python's `while hash_int > 0` behavior.

**GREEN** - already satisfied if Cycle 3 loop is correct.

### Cycle 6 - `APKG_SCHEMA` constant

**RED**

```rust
// src/apkg/schema.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_is_non_empty() {
        assert!(!APKG_SCHEMA.trim().is_empty());
    }

    #[test]
    fn schema_creates_core_tables() {
        for table in ["CREATE TABLE col", "CREATE TABLE notes", "CREATE TABLE cards",
                      "CREATE TABLE revlog", "CREATE TABLE graves"] {
            assert!(APKG_SCHEMA.contains(table), "missing {table}");
        }
    }

    #[test]
    fn schema_creates_expected_indexes() {
        for idx in [
            "ix_notes_usn", "ix_cards_usn", "ix_revlog_usn",
            "ix_cards_nid", "ix_cards_sched", "ix_revlog_cid", "ix_notes_csum",
        ] {
            assert!(APKG_SCHEMA.contains(idx), "missing index {idx}");
        }
    }
}
```

**GREEN** - paste verbatim SQL string from upstream.

**Refactor** - ensure leading/trailing newline matches upstream triple-quote content (Python `'''\nCREATE...'''` typically starts with newline). Prefer **exact** upstream interior; smoke tests use `contains` so minor newline trim is OK, but exactness is still the goal for later sqlite exec parity.

**Verify:** `cargo test apkg::schema::`

### Cycle 7 - `APKG_COL` constant

**RED**

```rust
#[test]
fn col_is_non_empty() {
    assert!(!APKG_COL.trim().is_empty());
}

#[test]
fn col_is_insert_into_col() {
    assert!(APKG_COL.contains("INSERT INTO col VALUES"));
}

#[test]
fn col_seed_has_default_deck_name() {
    assert!(APKG_COL.contains("\"name\": \"Default\""));
}
```

**GREEN** - paste verbatim from `apkg_col.py`.

**Verify:** `cargo test apkg::col::`

### Cycle 8 - Public re-export + docs hygiene

**RED**

```rust
// src/lib.rs or a tiny test in guid/lib
#[test]
fn guid_for_is_reexported_at_crate_root() {
    let g = crate::guid_for(&["a", "b"]);
    assert_eq!(g, "q/([o$8RAO");
}
```

Or rely on `pub use` + existing tests via `use genanki::guid_for` in an integration test file.

Prefer **unit tests inside modules** this phase; optional `tests/guid.rs` integration test:

```rust
// tests/guid.rs
#[test]
fn public_path() {
    assert_eq!(genanki::guid_for(&["a", "b"]), "q/([o$8RAO");
}
```

Integration test is a nice acceptance check that the re-export is real - include it (TDD: add file first, fail on missing re-export, then add `pub use`).

**GREEN**

```rust
// lib.rs
pub use crate::guid::guid_for;
```

**Docs**

- Module-level `//!` already required by `deny(missing_docs)`.
- `///` on `guid_for`, `BASE91_TABLE`, `APKG_SCHEMA`, `APKG_COL`, `Error` variants.
- `cargo doc --no-deps` clean.

### Cycle 9 - Full gate + polish

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo doc --no-deps
```

Fix any fallout (unused imports, doc typos). No feature creep.

## 7. File-level diff expectations

| File | Action |
| ---- | ------ |
| `Cargo.toml` | Add `sha2`, `thiserror` |
| `src/lib.rs` | `pub use crate::guid::guid_for;` |
| `src/guid.rs` | Full impl + unit tests (goldens, alphabet) |
| `src/error.rs` | `thiserror` derive; keep `Internal` |
| `src/apkg/schema.rs` | `APKG_SCHEMA` + smoke tests |
| `src/apkg/col.rs` | `APKG_COL` + smoke tests |
| `src/apkg/mod.rs` | Re-export constants? Optional `pub use schema::APKG_SCHEMA` - only if useful; not required |
| `tests/guid.rs` | Optional integration test for crate-root re-export |
| `README.md` | No change required this phase |
| `doc/plans/issue-3.md` | This plan |

## 8. Implementation order (summary checklist)

1. Branch `issue/3-guid-primitives`.
2. Cycle 1: `thiserror` Error migration (test first / preserve `error_display`).
3. Cycle 2: `BASE91_TABLE` tests + const.
4. Cycle 3: `guid_for("a","b")` golden + full impl.
5. Cycle 4: golden table + stability/order tests.
6. Cycle 5: zero-encode edge.
7. Cycle 6: `APKG_SCHEMA` tests + const.
8. Cycle 7: `APKG_COL` tests + const.
9. Cycle 8: crate-root re-export + integration test + docs.
10. Cycle 9: fmt/clippy/test/doc gate.
11. PR -> CI green -> merge -> tick #3 checkboxes -> close #3.

## 9. Acceptance criteria (map to issue #3)

| Criterion | How verified |
| --------- | ------------ |
| `guid_for` + base91 in `src/guid.rs` | Module compiles; public API as above |
| Golden vectors cross-checked vs Python genanki 0.13.x | Table in Cycle 4; values generated from same algorithm as `util.py` |
| `Error` / `Result` with `thiserror` | `error.rs` uses derive; at least variants needed now (`Internal`) |
| APKG schema SQL constants | `APKG_SCHEMA` verbatim; smoke tests for tables/indexes |
| APKG `col` seed insert | `APKG_COL` verbatim; smoke tests |
| Unit tests for GUID edge cases (empty, unicode, multi-field) | Cycle 4 table |
| `guid_for("a", "b")` (Rust: `&["a","b"]`) byte-identical to Python | Assert `q/([o$8RAO` |
| Schema/col compile + unit-smoke-tested | Cycles 6-7 |
| `cargo test` covers guid module | `cargo test guid` / full suite green |

## 10. PR shape

- **One PR** for Phase 1.
- Title: `Phase 1: GUID + primitives (base91, schema, error type)`
- Body:
  - Checklist mirrored from issue #3
  - Note deps added: `sha2`, `thiserror` only
  - Note `guid_for(&[&str])` signature decision
  - Note Python reference pin (`util.py` / schema / col identical on v0.13.0 and v1.13.1)
  - Link epic #1 and this plan path `doc/plans/issue-3.md`
- Do not bump version beyond `0.1.0`.
- Do not add unrelated refactors or Phase 2 types.

## 11. Follow-ups (explicitly not this PR)

| Item | Phase / issue |
| ---- | ------------- |
| Model builders, Mustache subset, `req` | #4 |
| Note / Card / tag validation / cloze | #5 |
| Deck / Package / rusqlite exec of schema+col / zip / media | #6 |
| Expand `Error` with `Io`, `Sqlite`, `Validation`, `Req`, ... + `From` impls | as those phases need them |
| Builtin models + full README | #7 |
| macOS CI matrix, MSRV, publish | #8 |

## 12. Risks / notes

1. **Raw string quoting for `APKG_COL`:** JSON contains many `"` and some `'`-hostile sequences; use `r#"..."#` or more `#` fences. After paste, visually diff against upstream file (or `curl` + checksum the constant body).
2. **Leading newlines in Python triple quotes:** `APKG_SCHEMA = '''\nCREATE...` includes a leading newline in the string value. Prefer exact match; smoke tests should not trim for equality checks if we add a full-string hash later.
3. **Do not "fix" SQL types** (`sfld integer` etc.). Verbatim is a feature.
4. **Unicode / emoji goldens** depend on UTF-8 hashing - Rust `&str` is already UTF-8; do not re-encode via lossy paths.
5. **`sha2` API:** `use sha2::{Digest, Sha256}; let dig = Sha256::digest(hash_str.as_bytes());` then `dig[..8]` as big-endian `u64` via `u64::from_be_bytes(dig[..8].try_into().unwrap())`. Cleaner than a manual shift loop; **must** match the Python big-endian accumulation (same as `from_be_bytes`).
6. **Clippy:** `unwrap` on 8-byte `try_into` is fine and provably safe; or use `let mut b=[0u8;8]; b.copy_from_slice(&dig[..8]);`. Avoid `as u64` byte casts that flip endianness.
7. **Prior art crates:** do not copy from `yannickfunk/genanki-rs` et al.; port from Python reference only.
8. **Issue text vs deps:** issue says "with `thiserror`"; we add it. GUID still needs `sha2` even though issue body only names `thiserror` under Error - both are intentional.

## 13. Done definition

Phase 1 is done when the acceptance table in section 9 is true on `main`, issue #3 checkboxes are updated/closed, and #4 can start without further primitive/GUID work.
