# Issue #7: Phase 5 - Builtin models + docs (BASIC/CLOZE, README, rustdoc)

Status: IMPLEMENTED (see PR #14)
Issue: https://github.com/tlkahn/genanki-rs/issues/7
Parent epic: https://github.com/tlkahn/genanki-rs/issues/1
Branch: `issue/7-builtins-docs`
Method: strict fine-grained TDD (RED -> GREEN -> refactor) per work item below
  (docs/README cycles use "failing doc-test / missing content first" where applicable)

## 1. Goal

Ship stock note types matching Python genanki v0.13.x builtins, plus
user-facing documentation sufficient to generate a deck from the README alone.

After this phase:

- `src/builtin_models.rs` exposes five `LazyLock<Model>` statics with ids,
  names, fields (Arial), templates, CSS, and model types byte-matching
  `genanki/builtin_models.py` (v0.13.0 / master; same content)
- Crate-root re-exports of all five names
- Ergonomics: `impl From<&Model> for Arc<Model>` so
  `Note::new(&*BASIC_MODEL, fields)?` works without an extra owned clone call
  site ceremony beyond `&*`
- Tests: each builtin writes a clean `.apkg`; optional-reverse gating; cloze
  requires 2 fields (no Python-style single-field pad/deprecation)
- README is a full Rust mirror of the Python genanki README concepts
  (quickstart, media, GUID stability, sort_field, cloze, HTML escaping,
  hardcoded model/deck ids, basename media rule, non-goals)
- Crate-level + public-item rustdoc; doc-tests pass (`cargo test --doc`)
- CI runs doctests (currently skipped by `cargo test --all-targets` only)
- **Zero new external dependencies**
- `cargo fmt` / `clippy -D warnings` / `test` / `doc` green

### Out of scope

- Hardening, changelog, crates.io publish, MSRV matrix polish (Phase 6 / #8)
- `write_to_collection_from_addon` (epic non-goal)
- YAML field/template API (epic non-goal)
- CLOZE single-field auto-pad / DeprecationWarning (epic: require 2 fields)
- `examples/` directory / `cargo run --example` binaries (doc-tests only)
- Anki desktop import smoke (still structural zip/sqlite; manual smoke is #8)
- Guaranteeing bit-identical packages with Python
- Changing Phase 1-4 domain behavior except the small `From<&Model>` ergonomics
  impl and docs/CI

## 2. Current state (code-verified)

| Item | Status |
| ---- | ------ |
| Phases 0-4 on `main` | Done (#2-#6 / PR #9-#13) |
| `src/builtin_models.rs` | Stub: `//! Built-in note models shipped with the crate. (Phase 7)` only |
| `src/lib.rs` | `pub mod builtin_models`; **no** crate-root re-exports of builtins; crate docs are one-liner + epic pointer; `#![deny(missing_docs)]` |
| `Model` / `Field` / `Template` | Full builders; `Field::new` default font is `"Liberation Sans"`; builtins must override `.font("Arial")` |
| `Note` | `Note::new(impl Into<Arc<Model>>, fields) -> Result`; field-count fail-fast; no cloze pad |
| `Deck` / `Package` | Write `.apkg` with hermetic `write_to_file_at` |
| `tests/common/mod.rs` | Zip/sqlite helpers (`write_pkg`, `open_zip`, `open_collection`, `col_json`) |
| README.md | Placeholder WIP (status + license only) |
| CI (`.github/workflows/ci.yml`) | `fmt` + `clippy -D warnings` + `cargo test --all-targets --all-features` - **doctests not run** |
| `Cargo.toml` deps | `sha2`, `thiserror`, `serde`/`serde_json`, `regex`, `log`, `rusqlite` (bundled), `zip`, `tempfile` |
| Python reference | `builtin_models.py` + `tests/test_builtin_models.py` + upstream README (v0.13.0 / master identical for builtins) |

## 3. Locked decisions

| Topic | Decision | Rationale |
| ----- | -------- | --------- |
| External deps this phase | **None** | Confirmed with maintainer. Builders + std only. |
| Builtin API shape | **`pub static NAME: LazyLock<Model>`** with Python names | Confirmed. True `const Model` impossible (`String`/`Vec`). `std::sync::LazyLock` (no crate). |
| Note ergonomics | **`impl From<&Model> for Arc<Model>`** in `model.rs` | `Note::new(&*BASIC_MODEL, fields)?` via existing `impl Into<Arc<Model>>`. Clone is explicit inside `From`. |
| Sharing across notes | Callers may `Arc::new((*BASIC_MODEL).clone())` once, or rely on `From<&Model>` per note (clones Model into new Arc each time). Document the cheap pattern: clone static once into `Arc` if writing many notes. | Avoid `LazyLock<Arc<Model>>` type surprise vs Python "the Model object". |
| CSS / templates / ids / names | **Byte-identical to Python v0.13.x** `builtin_models.py` | Including whitespace, `\n` placement, unquoted `hr id=answer`, cloze CSS concat (no trailing newline on final segment). |
| Field font | **`"Arial"`** on every builtin field | Python builtins set `font: Arial`; default Field font stays Liberation Sans. |
| CLOZE fields | **Text + Back Extra** (2 fields); id `1550428389` | Match current Python (not older 1-field prior-art crates). |
| CLOZE single-field | **`Error::FieldCountMismatch`** at `Note::new`; no pad, no warning helper | Epic / Phase 3 decision. Test replaces Python `test_cloze_with_single_field_warns`. |
| Optional reverse gating | Rely on existing front/back `req` + card gen | Empty `Add Reverse` => 1 card; non-empty => 2. Assert via `note.cards()` and/or sqlite card count after write. |
| Re-exports | Crate root: all five statics | Epic API sketch. Module path `genanki::builtin_models::BASIC_MODEL` also works. |
| README scope | **Full Python-README mirror in Rust** | Confirmed. Concepts + quickstart + media + GUID + sort_field + cloze + HTML escape + hardcoded ids + basename rule + non-goals. |
| Examples | **Doc-tests only** (no `examples/` dir) | Confirmed. |
| CI doctests | **Add `cargo test --doc`** (keep existing `--all-targets` job step or run both) | Confirmed. Acceptance requires doc-tests. |
| README doctests | Prefer **crate-level rustdoc examples** that mirror README; README code fences stay illustrative (not `cargo test`ed as markdown). Optionally `include_str!` is **not** required. | Avoids brittle README->doctest tooling; still keep README copy-pasteable and synchronized by review. |
| `missing_docs` | Every new public static + module docs + `From` impl docs | `#![deny(missing_docs)]` already on. |
| TDD discipline | Every behavior: failing test first, minimal impl, refactor. Doc cycles: add failing doctest or integration assertion first when behavior is code-backed. | Per user request. |
| Branch | `issue/7-builtins-docs` | Matches prior phase naming. |

### 3.1 External deps confirmation (explicit)

Maintainer request: minimize external deps; confirm anything we **must** add.

| Crate | Required? | Why |
| ----- | --------- | --- |
| Anything new | **No** | Builtins = `Model`/`Field`/`Template` builders already in-tree. Docs are markdown/rustdoc. |
| `std::sync::LazyLock` | std (1.80+) | Edition 2024 / current CI stable; no `once_cell` / `lazy_static`. |
| `rusqlite` / `zip` / `tempfile` | Already present | Reuse in write smoke tests via `tests/common`. |
| `regex` / `log` / `serde` / ... | Already present | Unchanged. |

**Bottom line:** this phase adds **zero** crates. If implementors feel pressure to add a dep, stop and re-confirm - it is not needed for #7.

```toml
# Cargo.toml - no dependency section changes expected for #7
```

## 4. Algorithm / content reference (must match Python v0.13.x)

### 4.1 Shared CSS (four BASIC_* models)

Exact string (trailing newline included):

```text
.card {
 font-family: arial;
 font-size: 20px;
 text-align: center;
 color: black;
 background-color: white;
}\n
```

Rust source form:

```rust
const BASIC_CSS: &str = "\
.card {\n\
 font-family: arial;\n\
 font-size: 20px;\n\
 text-align: center;\n\
 color: black;\n\
 background-color: white;\n\
}\n";
```

### 4.2 Cloze CSS

Python concatenates two string literals:

```text
.card { ... }\n\n
.cloze {\n font-weight: bold;\n color: blue;\n}\n.nightMode .cloze {\n color: lightblue;\n}
```

Note: **no** trailing `\n` after the nightMode block. Fingerprint in tests with full-string equality.

### 4.3 Five models

| Static | id | name | type | fields (Arial) | templates |
| ------ | -- | ---- | ---- | -------------- | --------- |
| `BASIC_MODEL` | 1559383000 | `Basic (genanki)` | FrontBack | Front, Back | Card 1: q `{{Front}}` / a `{{FrontSide}}\n\n<hr id=answer>\n\n{{Back}}` |
| `BASIC_AND_REVERSED_CARD_MODEL` | 1485830179 | `Basic (and reversed card) (genanki)` | FrontBack | Front, Back | Card 1 as above; Card 2: q `{{Back}}` / a `{{FrontSide}}\n\n<hr id=answer>\n\n{{Front}}` |
| `BASIC_OPTIONAL_REVERSED_CARD_MODEL` | 1382232460 | `Basic (optional reversed card) (genanki)` | FrontBack | Front, Back, Add Reverse | Card 1 as basic; Card 2: q `{{#Add Reverse}}{{Back}}{{/Add Reverse}}` / a reverse afmt |
| `BASIC_TYPE_IN_THE_ANSWER_MODEL` | 1305534440 | `Basic (type in the answer) (genanki)` | FrontBack | Front, Back | Card 1: q `{{Front}}\n\n{{type:Back}}` / a `{{Front}}\n\n<hr id=answer>\n\n{{type:Back}}` |
| `CLOZE_MODEL` | 1550428389 | `Cloze (genanki)` | Cloze | Text, Back Extra | Cloze: q `{{cloze:Text}}` / a `{{cloze:Text}}<br>\n{{Back Extra}}` |

All use `BASIC_CSS` except `CLOZE_MODEL` (cloze CSS). Latex pre/post and `sort_field_index` stay Model defaults.

### 4.4 Construction pattern

```rust
use std::sync::LazyLock;

use crate::model::{Field, Model, ModelType, Template};

fn arial(name: &str) -> Field {
    Field::new(name).font("Arial")
}

pub static BASIC_MODEL: LazyLock<Model> = LazyLock::new(|| {
    Model::new(1559383000, "Basic (genanki)")
        .field(arial("Front"))
        .field(arial("Back"))
        .template(Template::new(
            "Card 1",
            "{{Front}}",
            "{{FrontSide}}\n\n<hr id=answer>\n\n{{Back}}",
        ))
        .css(BASIC_CSS)
});

// ... same for the other four
```

Do **not** invent builder helpers beyond a private `arial` / CSS constants unless tests stay clearer.

### 4.5 `From<&Model> for Arc<Model>`

```rust
// model.rs
impl From<&Model> for std::sync::Arc<Model> {
    fn from(model: &Model) -> Self {
        std::sync::Arc::new(model.clone())
    }
}
```

Enables:

```rust
Note::new(&*BASIC_MODEL, ["Capital of Argentina", "Buenos Aires"])?
```

### 4.6 Optional reverse card semantics (already implemented)

For `BASIC_OPTIONAL_REVERSED_CARD_MODEL`:

- Template 0 `req`: essentially Front required (`all [0]` style after our req engine)
- Template 1 `qfmt` is a conditional on `Add Reverse` wrapping `{{Back}}` -> `any` over involved fields

Card generation (Phase 3):

- Fields `["France", "Paris", ""]` -> **1** card (ord 0 only)
- Fields `["France", "Paris", "y"]` -> **2** cards (ords 0 and 1)

Assert both via `Note::cards()` unit-style tests and optionally sqlite counts after package write.

### 4.7 Cloze two-field rule

```rust
Note::new(&*CLOZE_MODEL, ["{{c1::x}}"])          // Err FieldCountMismatch
Note::new(&*CLOZE_MODEL, ["{{c1::x}}", ""])      // Ok
Note::new(&*CLOZE_MODEL, ["{{c1::x}}", "extra"]) // Ok
```

No `_fix_deprecated_builtin_models_and_warn` port.

## 5. Test plan (RED -> GREEN per item)

Prefer:

- Unit tests in `src/builtin_models.rs` for structural fingerprints (id/name/css/fields/templates/type)
- Unit tests for optional-reverse card counts (can live in builtin module or thin integration)
- Integration `tests/builtin_models.rs` for `.apkg` write smoke (port of Python `test_builtin_models`)
- Existing `tests/common` helpers for zip/sqlite
- Doctests on crate root / module docs for quickstart paths

### T0 - Workspace still green before changes

**Baseline:** `cargo test --all-targets` green on branch point from `main`.

### T1 - Structural fingerprints per builtin (unit)

**RED** (one test per model or one table-driven test):

For each static after force-init (`LazyLock` deref):

- `id`, `name`, `model_type`
- field names **and** each `font == "Arial"`
- template names, `qfmt`, `afmt` exact strings
- `css` exact string (BASIC vs CLOZE)

Start with `BASIC_MODEL` only in the first RED/GREEN cycle; add others one model per cycle (or batch fingerprints once the module skeleton exists - still assert before filling each model body).

**GREEN** - implement that model's `LazyLock` body.

### T2 - `req()` succeeds for every builtin

**RED**

```rust
assert!(BASIC_MODEL.req().is_ok());
// ... all five
```

**GREEN** - should already hold if templates match Python; catches typos in qfmt.

### T3 - `From<&Model> for Arc<Model>`

**RED**

```rust
let arc: Arc<Model> = (&*BASIC_MODEL).into();
assert_eq!(arc.id, 1559383000);
let note = Note::new(&*BASIC_MODEL, ["a", "b"]).unwrap();
assert_eq!(note.model().id, 1559383000);
```

**GREEN** - impl in `model.rs` (+ rustdoc).

### T4 - Optional reverse gating (cards)

**RED**

```rust
let mut n1 = Note::new(&*BASIC_OPTIONAL_REVERSED_CARD_MODEL, ["F", "B", ""]).unwrap();
assert_eq!(n1.cards().unwrap().len(), 1);

let mut n2 = Note::new(&*BASIC_OPTIONAL_REVERSED_CARD_MODEL, ["F", "B", "y"]).unwrap();
assert_eq!(n2.cards().unwrap().len(), 2);
assert_eq!(n2.cards().unwrap()[0].ord, 0);
assert_eq!(n2.cards().unwrap()[1].ord, 1);
```

**GREEN** - no new production code if Phase 3 is correct; this locks the builtin template wiring.

### T5 - Cloze requires 2 fields

**RED**

```rust
let err = Note::new(&*CLOZE_MODEL, ["{{c1::Rome}} is the capital of {{c2::Italy}}"]).unwrap_err();
assert!(matches!(err, Error::FieldCountMismatch { model_fields: 2, note_fields: 1, .. }));

let mut ok = Note::new(&*CLOZE_MODEL, ["{{c1::Rome}} is the capital of {{c2::Italy}}", ""]).unwrap();
assert_eq!(ok.cards().unwrap().len(), 2); // c1, c2
```

**GREEN** - existing `Note::new` validation.

### T6 - Each builtin writes an `.apkg` cleanly (integration)

**RED** - port Python `test_builtin_models`:

```rust
// tests/builtin_models.rs
let mut deck = Deck::new(1598559905, "Country Capitals");
deck.add_note(Note::new(&*BASIC_MODEL, ["Capital of Argentina", "Buenos Aires"]).unwrap());
deck.add_note(Note::new(&*BASIC_AND_REVERSED_CARD_MODEL, ["Costa Rica", "San José"]).unwrap());
deck.add_note(Note::new(&*BASIC_OPTIONAL_REVERSED_CARD_MODEL, ["France", "Paris", "y"]).unwrap());
deck.add_note(Note::new(&*BASIC_TYPE_IN_THE_ANSWER_MODEL, ["Taiwan", "Taipei"]).unwrap());
deck.add_note(Note::new(
    &*CLOZE_MODEL,
    [
        "{{c1::Ottawa}} is the capital of {{c2::Canada}}",
        "Ottawa is in Ontario province.",
    ],
).unwrap());
let pkg = Package::new(deck);
let (_dir, path) = common::write_pkg(&pkg, 1_600_000_000.0);
// zip opens; collection.anki2 opens; notes count == 5
// cards count: 1 + 2 + 2 + 1 + 2 = 8
```

**GREEN** - pure wiring once models exist.

### T7 - Optional reverse card counts in sqlite

**RED** - two notes (empty vs non-empty Add Reverse); assert `cards` rows per `nid` are 1 and 2.

**GREEN**

### T8 - Crate-root re-exports compile

**RED** - integration or doctest:

```rust
use genanki::{
    BASIC_AND_REVERSED_CARD_MODEL, BASIC_MODEL, BASIC_OPTIONAL_REVERSED_CARD_MODEL,
    BASIC_TYPE_IN_THE_ANSWER_MODEL, CLOZE_MODEL,
};
```

**GREEN** - `lib.rs` `pub use`.

### T9 - Crate-level rustdoc quickstart doctest

**RED** - expand `src/lib.rs` module docs with an example that:

1. Builds a custom `Model` **or** uses `BASIC_MODEL`
2. Creates `Note`, `Deck`, `Package`
3. Writes to a temp path (use `tempfile` in doctest - already a crate dep, available to doctests)

Doctest must compile and run under `cargo test --doc`.

Prefer one primary example matching the epic README sketch (custom model) and a second short example using `BASIC_MODEL` + cloze.

**GREEN** - docs + ensure APIs used are public.

### T10 - Builtin module rustdoc

**RED/GREEN** - `//!` module docs explaining `(genanki)` suffix rationale (Anki builtin id clash - see Python module docstring). Short example constructing a note from `BASIC_MODEL`.

### T11 - README full mirror (content review + manual compile check)

Not all README fences need to be doctests. Process:

1. Draft README sections (sec. 6)
2. For each code block, either:
   - Mirror it as a crate/module doctest, or
   - Paste into a temporary `tests/readme_smoke.rs` / doctest during implementation and delete duplication once verified
3. Acceptance: a new user can follow README alone to produce `output.apkg`

**Checklist of README sections** (map from Python):

- [ ] Title, one-paragraph pitch, affiliation disclaimer
- [ ] Status / link to epic optional (keep brief)
- [ ] Notes (concept + `Note::new` example)
- [ ] Models (custom `Model` builder example + unique `model_id` guidance)
- [ ] Generating a Deck/Package (`Deck`, `Package::write_to_file`)
- [ ] Media files (paths on `Package`, basename-only field refs, `[sound:]` / `<img>`)
- [ ] Note GUIDs (`guid_for`, `Note::with_guid`, stability for re-import)
- [ ] `sort_field` / `sort_field_index`
- [ ] Builtin models section (list five; point at `BASIC_MODEL` etc.)
- [ ] Cloze (`CLOZE_MODEL`, **two fields required**, no deprecation pad)
- [ ] FAQ: field HTML escaping (`<`, `>`, `&`)
- [ ] Hardcoded model/deck ids (`1<<30 .. 1<<31` guidance)
- [ ] Non-goals v1: no addon collection write; no YAML template API; write-only
- [ ] License

### T12 - Public item docs gap fill (as touched)

While exporting builtins, skim currently public Phase 4 APIs used in README (`Deck`, `Package`, `Note`, `Model`) for missing examples only if doctests fail or README would be unclear. Do not boil the ocean - #8 can deepen docs.

### T13 - CI runs doctests

**RED** - add step or change test invocation so doctests run in CI:

Option A (minimal additive):

```yaml
- name: cargo test
  run: cargo test --all-targets --all-features

- name: cargo test --doc
  run: cargo test --doc --all-features
```

Option B: single `cargo test --all-features` (unit + integration + doc) **plus** keep clippy on all-targets. Prefer **Option A** to preserve current all-target coverage and explicitly satisfy acceptance.

**GREEN** - workflow edit; confirm locally with `cargo test --doc`.

### T14 - Full workspace gates

```text
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo test --doc --all-features
```

## 6. Implementation order (fine-grained cycles)

Do not batch large green steps. Suggested sequence:

1. **T1 BASIC_MODEL only** - failing fingerprint test -> implement `BASIC_MODEL` + CSS const + `arial` helper.
2. **T1 remaining four models** - one RED/GREEN each (reversed, optional reversed, type-in, cloze).
3. **T2** - `req()` smoke for all five.
4. **T3** - `From<&Model> for Arc<Model>`.
5. **T4 T5** - optional reverse cards + cloze field count (may already pass; keep as regression locks).
6. **T6 T7** - `tests/builtin_models.rs` write smoke + sqlite card counts.
7. **T8** - crate-root re-exports.
8. **T9 T10** - rustdoc + doctests (crate + builtin module).
9. **T11** - README full rewrite (content), cross-check code against doctests.
10. **T13** - CI doctest step.
11. **T12 T14** - docs polish, gates, dead_code/`Phase 7` comment fix (`Phase 5`).
12. **Refactor** only while green (dedupe CSS, table-driven fingerprint tests, shared test helpers).

### Clippy / docs notes

- `LazyLock` statics need `///` docs on each public static (deny missing_docs).
- Avoid `unwrap` in library code; tests/doctests may unwrap or use `?` with `Result` in doctest (`# Ok::<(), genanki::Error>(())` pattern).
- Doctests that write files must use `tempfile::tempdir()` and not leave cwd debris.
- Unicode in tests (`San José`) is fine; file encoding UTF-8.
- `hr id=answer` stays **unquoted** to match Python/Anki stock templates.

## 7. File touch list

| Path | Action |
| ---- | ------ |
| `src/builtin_models.rs` | Five `LazyLock` statics, CSS consts, unit tests, module rustdoc |
| `src/model.rs` | `impl From<&Model> for Arc<Model>` + unit test |
| `src/lib.rs` | Crate-level rustdoc/doctests; `pub use` five builtins |
| `tests/builtin_models.rs` | Integration write smoke + optional reverse sqlite counts |
| `tests/common/mod.rs` | Reuse as-is; export nothing new unless needed |
| `README.md` | Full user guide (Python mirror in Rust) |
| `.github/workflows/ci.yml` | Add `cargo test --doc` step |
| `doc/plans/issue-7.md` | This plan (status -> IMPLEMENTED when done) |
| `Cargo.toml` | **No dep changes** (maybe nothing at all) |

No changes expected to `guid.rs`, `req.rs`, `note.rs` card logic, `package.rs`, or apkg schema unless a doctest reveals a docs-only gap.

## 8. Acceptance mapping

| Acceptance (issue #7) | Tests / deliverables |
| --------------------- | -------------------- |
| All five builtins available and tested | T1 T2 T6 T8 |
| Optional reverse: empty `Add Reverse` => 1 card; non-empty => 2 | T4 T7 |
| Cloze builtin requires 2 fields | T5 |
| Each builtin writes `.apkg` cleanly | T6 |
| README alone is enough to generate a deck | T11 |
| Doc tests pass | T9 T10 T14 |
| Document HTML escaping, hardcoded ids, basename media | T11 README sections |
| Document non-goals (no addon write, no YAML API) | T11 README section |
| Re-export from `lib.rs` | T8 |
| CI runs doctests | T13 |

## 9. Phase 6 handoff notes

Leave ready for #8 without implementing it:

- README + rustdoc exist; #8 can add CHANGELOG, crates.io metadata polish, badges
- Builtin statics are stable public API surface for semver
- No Anki GUI import automation yet (manual smoke note on #8)
- Consider whether docs.rs build needs extra features (none expected)
- Optional later: `examples/` binaries if users request (explicitly skipped here)

## 10. Open questions - RESOLVED

| Question | Resolution |
| -------- | ---------- |
| Plan path | `genanki-rs/doc/plans/issue-7.md` |
| New external deps | **None** (confirmed) |
| API shape | `pub static …: LazyLock<Model>` Python names (confirmed) |
| Note ergonomics | `From<&Model> for Arc<Model>` + `Note::new(&*BASIC_MODEL, …)` |
| CI doctests | Add `cargo test --doc` in #7 (confirmed) |
| README scope | Full Python-mirror in Rust (confirmed) |
| examples/ dir | No; doc-tests only (confirmed) |
| CLOZE 1-field | Error, no pad (epic) |
| CSS/template parity | Byte-match Python v0.13.x |

No unresolved blockers. Implement on branch `issue/7-builtins-docs` with strict TDD.

## 11. PR checklist (when implementing)

- [ ] Every work item went RED before GREEN (code-backed items)
- [ ] **Zero** new crates in `Cargo.toml`
- [ ] Five builtins: ids/names/CSS/templates/fonts match Python
- [ ] Optional reverse gating tested (1 vs 2 cards)
- [ ] Cloze 2-field requirement tested; no deprecation pad
- [ ] `.apkg` write smoke for all builtins in one deck
- [ ] Crate-root re-exports
- [ ] README full guide (media, GUID, cloze, HTML escape, ids, basename, non-goals)
- [ ] rustdoc + `cargo test --doc` green
- [ ] CI runs doctests
- [ ] `cargo fmt`, `clippy -D warnings`, `test --all-targets`, `test --doc` green
- [ ] Issue #7 checkboxes updated in PR description
- [ ] Plan status -> IMPLEMENTED + PR link

## 12. Quick reference - Python -> Rust map

| Python | Rust |
| ------ | ---- |
| `genanki.BASIC_MODEL` | `genanki::BASIC_MODEL` (`LazyLock<Model>`; use `&*BASIC_MODEL`) |
| `genanki.BASIC_AND_REVERSED_CARD_MODEL` | `genanki::BASIC_AND_REVERSED_CARD_MODEL` |
| `genanki.BASIC_OPTIONAL_REVERSED_CARD_MODEL` | `genanki::BASIC_OPTIONAL_REVERSED_CARD_MODEL` |
| `genanki.BASIC_TYPE_IN_THE_ANSWER_MODEL` | `genanki::BASIC_TYPE_IN_THE_ANSWER_MODEL` |
| `genanki.CLOZE_MODEL` | `genanki::CLOZE_MODEL` |
| `Note(model=BASIC_MODEL, fields=[...])` | `Note::new(&*BASIC_MODEL, [...])?` |
| `Note(model=CLOZE_MODEL, fields=[text])` (deprecated pad) | **Error** - pass `[text, extra_or_empty]` |
| `Deck(id, name)` + `Package(deck).write_to_file(path)` | `Deck::new` + `Package::new(deck).write_to_file(path)?` |
| `html.escape` for field text | Caller responsibility; document (e.g. replace `&`/`<`/`>` or use a small helper in app code) |
| `random.randrange(1<<30, 1<<31)` for ids | Generate once, hardcode `i64` literals in app code |

## 13. Exact Python source fingerprints (implement against these)

Ids:

```text
BASIC_MODEL                         1559383000
BASIC_AND_REVERSED_CARD_MODEL       1485830179
BASIC_OPTIONAL_REVERSED_CARD_MODEL  1382232460
BASIC_TYPE_IN_THE_ANSWER_MODEL      1305534440
CLOZE_MODEL                         1550428389
```

Names (include ` (genanki)` suffix):

```text
Basic (genanki)
Basic (and reversed card) (genanki)
Basic (optional reversed card) (genanki)
Basic (type in the answer) (genanki)
Cloze (genanki)
```

Type-in templates:

```text
qfmt: {{Front}}\n\n{{type:Back}}
afmt: {{Front}}\n\n<hr id=answer>\n\n{{type:Back}}
```

Cloze templates:

```text
qfmt: {{cloze:Text}}
afmt: {{cloze:Text}}<br>\n{{Back Extra}}
```

Optional reverse Card 2 qfmt:

```text
{{#Add Reverse}}{{Back}}{{/Add Reverse}}
```
