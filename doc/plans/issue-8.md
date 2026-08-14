# Issue #8: Phase 6 - Hardening + v0.1.0 release readiness

Status: IMPLEMENTED (see PR #15)
Issue: https://github.com/tlkahn/genanki-rs/issues/8
Parent epic: https://github.com/tlkahn/genanki-rs/issues/1
Branch: `issue/8-hardening-v0.1.0`
Method: strict fine-grained TDD (RED -> GREEN -> refactor) for every
  code-backed work item; docs/checklist items use "missing artifact first"
  where applicable

## 1. Goal

Close the v0.1.0 gate: extra assurance on the riskiest pure parsers (cloze +
invalid-HTML), a large-deck smoke path, multi-OS CI, format research note,
manual Anki import record, CHANGELOG, and crates.io **readiness** (not a live
publish in this phase).

After this phase:

- Hand-rolled property / randomized invariant tests cover cloze card ords and
  `find_invalid_html_tags` (no panic; structural invariants; known goldens still
  pass)
- `tests/large_deck.rs` writes ~10k notes, asserts zip/sqlite counts, logs
  wall time (soft sanity - no hard time/memory fail)
- CI matrix: Linux + macOS (`fmt` on Linux only; clippy + test + doc on both)
- Research note documents stay-on-`collection.anki2` unless import breaks
- Manual Anki desktop import procedure + PR record template
- `CHANGELOG.md` for v0.1.0
- crates.io publish checklist + Cargo.toml / README polish for docs.rs
- Epic #1 success-criteria final pass documented on the PR
- **Zero new external dependencies** (runtime or dev)
- `cargo fmt` / `clippy -D warnings` / `test` / `doc` green on the matrix

### Out of scope

- Live `cargo publish` (human gate after checklist; readiness only)
- Python genanki CI sidecar / semantic cross-impl golden job (follow-up; see
  sec. 3.2)
- `proptest`, `arbitrary`, `cargo-fuzz`, libFuzzer targets
- Hard wall-clock or RSS fail thresholds on the 10k smoke (flake risk)
- `collection.anki21` / zstd / protobuf writer support
- Reading existing `.apkg` as a public API
- `write_to_collection_from_addon` (epic non-goal)
- Bit-identical packages with Python genanki
- New domain features beyond hardening

## 2. Current state (code-verified)

| Item | Status |
| ---- | ------ |
| Phases 0-5 | Done and closed (#2-#7 / PR #9-#14); epic #1 still open |
| Cloze parsing | Private regex helpers + `Note::cloze_cards` in `src/note.rs`; unit + `tests/cloze.rs` goldens |
| HTML scanner | Public `genanki::note::find_invalid_html_tags`; unit goldens (valid/invalid/comment/CDATA/issue-28) |
| GUID goldens | `src/guid.rs` table matches Python 0.13.x algorithm (precomputed) |
| Large deck | No 10k path yet |
| CI | `.github/workflows/ci.yml`: **ubuntu-latest only**; fmt, clippy, test, doc |
| CHANGELOG | Missing |
| crates.io | Package name `genanki` 0.1.0; metadata mostly filled; no publish checklist; no `rust-version` / `authors` |
| Anki format note | Missing |
| Manual Anki smoke | Not recorded (structural zip/sqlite only) |
| Python sidecar | Not present (and deferred) |
| Deps | `sha2`, `thiserror`, `serde`/`serde_json`, `regex`, `log`, `rusqlite` (bundled), `zip`, `tempfile` - **dev-deps empty** |
| Test helpers | `tests/common/mod.rs` (`write_pkg`, `open_zip`, `open_collection`, ...) |
| Workspace | `cargo test --all-targets` green on plan-authoring host |

## 3. Locked decisions

| Topic | Decision | Rationale |
| ----- | -------- | --------- |
| External deps this phase | **None** | Confirmed with maintainer (sec. 3.1). |
| Property / fuzz style | **Hand-rolled** deterministic PRNG + invariant tests in-tree | Confirmed. No `proptest` / fuzz crates. |
| Property test location | Unit tests in `src/note.rs` `mod tests` for private cloze helpers + HTML; thin integration optional only if public API needs it | Private regex accessors already unit-tested nearby; keep cycles tight. |
| PRNG | Tiny xorshift64 in the test module (no `rand` crate); fixed seed; N iterations constant | Deterministic CI; zero deps. |
| Python golden CI sidecar | **Skip for v0.1.0**; document as follow-up | Confirmed. GUID acceptance already met by in-repo goldens. |
| Large-deck bounds | Soft sanity: must succeed + correct counts; **eprintln** elapsed; no fail-on-slow | Confirmed. Avoid flaky CI. |
| Large-deck size | **10_000** notes, single deck, `BASIC_MODEL` (1 card each), hermetic timestamp | Issue text "~10k"; BASIC keeps card math trivial (`notes=10000`, `cards=10000`). |
| Large-deck in CI | **Yes, not `#[ignore]`** | 10k simple notes should finish in seconds with bundled sqlite; monitor on first PR. If too slow on macOS runners, fall back to `#[ignore]` in a follow-up fix - do not pre-ignore. |
| CI OS matrix | `ubuntu-latest` + `macos-latest` | Issue acceptance. |
| fmt job | Linux only (once) | rustfmt is platform-stable; saves a macOS minute. |
| clippy + test + doc | Both OSes | Catches OS-specific path/zip/sqlite issues. |
| `collection.anki21` | Research note only: **stay on `collection.anki2`** | Matches Python genanki; Anki still imports anki2 packages. |
| Manual Anki smoke | Human step on the PR; provide `tests/fixtures` recipe or `cargo test` that writes a sample path under `target/` documented in checklist | Cannot automate Anki GUI in CI. |
| crates.io | **Readiness only** - checklist + metadata polish; no `cargo publish` in #8 | Confirmed. |
| CHANGELOG | Keep a Changelog style, `## [0.1.0]` initial release section | Standard for crates.io consumers. |
| Epic criteria pass | PR description checkbox mapping + short `doc/plans/issue-8-acceptance.md` optional; prefer single plan + PR body | Avoid doc sprawl; PR body is the gate record. |
| Branch | `issue/8-hardening-v0.1.0` | Matches prior phase naming. |
| TDD discipline | Every behavior: failing test first, minimal code, refactor while green. Doc artifacts: add failing link/checklist reference first when useful. | Per user request. |

### 3.1 External deps confirmation (explicit)

Maintainer request: minimize external deps; confirm anything we **must** add.

| Crate | Required? | Why |
| ----- | --------- | --- |
| `proptest` / `arbitrary` | **No** | Hand-rolled property tests with an inline PRNG. |
| `rand` | **No** | Inline xorshift64 in `#[cfg(test)]` is enough. |
| `cargo-fuzz` / `libfuzzer-sys` | **No** | Out of scope; heavier than v0.1.0 needs. |
| Python / `genanki` in CI | **No** (deferred) | Not a Rust dep; sidecar skipped for v0.1.0. |
| `criterion` / bench harness | **No** | Large-deck is smoke, not a benchmark. |
| `sha2` / `regex` / `rusqlite` / `zip` / ... | Already present | Reuse only. |
| Anything new in `[dependencies]` or `[dev-dependencies]` | **No** | |

**Bottom line:** this phase adds **zero** crates. If implementors feel pressure
to add a dep, stop and re-confirm - it is not needed for #8.

```toml
# Cargo.toml - no [dependencies] / [dev-dependencies] additions for #8.
# Optional metadata-only keys (authors, rust-version, etc.) are allowed.
```

### 3.2 Deferred: Python genanki semantic sidecar (follow-up)

Issue #8 lists an **optional** CI sidecar. Explicitly **not** implemented now:

- Would need `actions/setup-python`, pin `genanki==0.13.x`, fixture suite,
  unzip both packages, compare `notes.guid` / `cards.ord` / `notes.flds`
  (and possibly tags) under hermetic timestamps
- GUID parity is already locked by `guid_for_goldens_table` (Python-precomputed)
- Structural apkg tests already cover flds/tags/ords/media

Capture in research or publish checklist as "Post-v0.1.0 optional work" so the
issue checkbox can be marked deferred with a link, not silently dropped.

### 3.3 Manual Anki smoke (human gate)

Cannot be a RED/GREEN unit cycle. Treat as release checklist:

1. Build sample via documented command (sec. 5 T9).
2. Import into current Anki desktop (version recorded).
3. Confirm decks/notes/cards/media/cloze render.
4. Paste short result block into the PR description (template in sec. 6.5).

## 4. Algorithm / content reference

### 4.1 Hand-rolled PRNG (tests only)

```rust
/// Deterministic xorshift64* for property tests. Not cryptographic.
struct XorShift64(u64);

impl XorShift64 {
    fn new(seed: u64) -> Self {
        // Avoid zero state.
        Self(seed | 1)
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn next_usize(&mut self, lo: usize, hi_exclusive: usize) -> usize {
        assert!(hi_exclusive > lo);
        let span = hi_exclusive - lo;
        lo + (self.next_u64() as usize % span)
    }
    fn next_bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}
```

Keep this private inside `src/note.rs` `#[cfg(test)]` mod (or a tiny
`tests/common/prng.rs` if shared). Prefer **in `note` unit tests** first so
cloze private helpers stay reachable without `pub(crate)` expansion.

### 4.2 HTML scanner invariants (property)

For many generated `field: String` values:

1. **No panic** calling `find_invalid_html_tags(&field)`.
2. Every returned string `t` satisfies `t.starts_with('<') && t.ends_with('>')`.
3. Every returned `t` is a substring of `field` (and equals some
   `tag_re` match) - i.e. scanner never invents tags.
4. **Oracle on alphabet fragments:** if the generator inserts only known-valid
   tags from a whitelist (`<br>`, `<br/>`, `<h1>`, `</h1>`,
   `<h1 style="x">`, `<!-- c -->`, `<![CDATA[x]]>`, ...), the result is empty.
5. **Oracle on known-invalid inserts:** if the generator inserts `<>`, `< >`,
   `<@h1>`, `<h1@>` as whole tags, each appears in the result (order may follow
   left-to-right finditer).
6. **Idempotent classification:** joining `field` from pieces and scanning
   equals scanning the final string once (sanity).

Generator strategy (mix, not pure random bytes only):

- Alphabet chunks: plain UTF-8 text, newlines, valid tags, invalid tags,
  comments, CDATA, bare `<`, bare `>`, nested-looking noise
- Length 0..=256 for most iters; a few up to ~4k
- Fixed seed (e.g. `0xGENANKI_HTML` as hex literal) and **N = 500** (or 200 if
  debug is slow - tune while green, keep constant in source)

Also retain existing exact goldens (do not delete Phase 3 tests).

### 4.3 Cloze invariants (property)

Public path (preferred for "does the user-visible API hold"):

```rust
fn cloze_ords_for(text: &str) -> Vec<i32> {
    let mut n = Note::new(cloze_model_2_fields(), [text, ""]).unwrap();
    n.cards().unwrap().iter().map(|c| c.ord).collect()
}
```

Invariants per generated `text`:

1. **No panic** / `cards()` is `Ok`.
2. Result is **sorted ascending** and **deduplicated**.
3. If `text` contains no well-formed `{{cN::...}}` with `N > 0` parseable into
   an `i32` ord `N-1` without overflow skip rules, result is `[0]` (Python
   default card).
4. **Constructed markers oracle:** build text by joining plain segments and
   markers `{{c{n}::{body}}}` for chosen `n` in `1..=32` and bodies without
   `}}` (or with controlled newlines for DOTALL). Expected ords =
   sorted unique `(n - 1)` for those n. Must match `cloze_ords_for(text)`.
5. **Hints:** body may contain `::hint` - still one ord.
6. **Legacy qfmt:** separate smaller test that a model whose qfmt uses
   `<%cloze:Text%>` still discovers field names (can stay example-based if
   property over qfmt is awkward).
7. Overflow policy remains as Phase 3: ords that do not fit `i32` after
   `N-1` are skipped; if none remain, `[0]`. Property generator should mostly
   stay in small `n` and include a few explicit overflow cases as fixed tests
   (already present).

Private helper stress (optional extra cycle in unit tests):

- `cloze_ord_re().find_iter` never panics on random strings
- Capture group 1 parses as digits when matched

Multi-field cloze property (smaller N):

- Two fields with independent marker sets; ords = sorted union of both fields'
  `(n-1)` values (empty union => `[0]`).

### 4.4 Large-deck smoke

```text
model = &*BASIC_MODEL  // or simple_model() if wanting zero LazyLock cost
deck  = Deck::new(DECK_ID, "Large Deck Smoke")
for i in 0..10_000:
    deck.add_note(Note::new(model, [format!("Q{i}"), format!("A{i}")])?)
pkg = Package::new(deck)
t0 = Instant::now()
write_to_file_at(path, 1_700_000_000.0)
elapsed = t0.elapsed()
eprintln!("[large_deck] wrote 10000 notes in {elapsed:?}");
open zip + sqlite:
  notes count == 10_000
  cards count == 10_000
  media == {}
  spot-check note 0 flds and note 9999 flds
```

Do **not** assert `elapsed < T`. Optional: assert output file size `> 0` and
`<` some huge bound (e.g. 200 MiB) only as gross sanity - keep loose.

Memory: no RSS API without deps/OS glue. Rely on CI runner not OOM-killing the
job; if OOM appears, reduce to 5k or stream-less patterns in a fix PR.

### 4.5 CI matrix sketch

```yaml
name: ci

on:
  push:
    branches: [main]
  pull_request:

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: -D warnings

jobs:
  fmt:
    name: fmt
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt
      - run: cargo fmt --all -- --check

  test:
    name: clippy + test (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - uses: Swatinem/rust-cache@v2
        with:
          key: ${{ matrix.os }}
      - run: cargo clippy --all-targets --all-features -- -D warnings
      - run: cargo test --all-targets --all-features
      - run: cargo test --doc --all-features
```

Notes:

- Split fmt so macOS does not duplicate rustfmt.
- `fail-fast: false` so both OSes report.
- Cache key includes OS (rust-cache usually handles this; explicit key is fine).

### 4.6 Research note: Anki package formats

Create `doc/research/anki-package-formats.md` (excluded from crate package via
existing `exclude = ["doc/", ...]`):

Contents (concise):

1. What genanki writes today: zip with `collection.anki2` (sqlite3), `media`
   JSON, numbered media blobs - Python genanki v0.13.x parity.
2. Newer Anki desktop may use `collection.anki21` / `collection.anki21b` with
   different scheduling tables / optional zstd; import path still accepts
   legacy `.apkg` with `collection.anki2` as of research date.
3. Decision for v0.1.0: **stay on anki2** unless manual smoke on current Anki
   shows import failure.
4. Revisit triggers: Anki release notes deprecating anki2 import; bug reports.
5. Out of scope: implementing anki21 writer, reading collections.
6. References: Anki source / changelog links (add concrete URLs at implement
   time; verify they resolve).

### 4.7 CHANGELOG.md (initial)

```markdown
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - YYYY-MM-DD

Initial release: Rust port of Python genanki focused on writing Anki `.apkg`
packages (feature parity target: kerrickstaley/genanki v0.13.x).

### Added

- `Model` / `Field` / `Template` builders, `req` computation, front/back + cloze
- `Note` validation (field count, tags), GUID (`guid_for` / base91), cards
- `Deck` + `Package` writer (`collection.anki2`, media map, hermetic timestamp)
- Builtin models: BASIC_*, CLOZE (`LazyLock<Model>`)
- Invalid HTML tag scanner (`note::find_invalid_html_tags`) with non-fatal `log` warnings
- README + rustdoc; CI on Linux and macOS

### Non-goals (v0.1.0)

- Reading/modifying existing `.apkg`
- Anki addon collection write path
- Newer `collection.anki21*` writers
```

Date: use release-prep date or PR-merge date; ISO `YYYY-MM-DD`.

### 4.8 crates.io readiness

**Cargo.toml polish (metadata only, no deps):**

| Field | Action |
| ----- | ------ |
| `name = "genanki"` | Keep; checklist step: `cargo search genanki` / web check availability before human publish |
| `version = "0.1.0"` | Keep until publish |
| `edition = "2024"` | Keep |
| `description` / `license` / `repository` / `homepage` / `documentation` / `readme` / `keywords` / `categories` | Already set; spot-check |
| `exclude` | Keep `doc/`, `.github/`; ensure `tests/` stays in publish if needed for... actually tests are not packaged as runtime; crate package includes `src/`, README, LICENSE by default. Confirm `cargo package --list` looks right |
| `authors` | Add if maintainer wants; optional - ask only if missing blocks nothing (crates.io allows omit) |
| `rust-version` | Optional MSRV pin; only set if we **run** an MSRV CI job. **v0.1.0 decision: omit rust-version** unless we add MSRV matrix (we do not in #8) to avoid lying |
| `license-file` | Not required when `license = "MIT"` and LICENSE present |

**Checklist file:** `doc/publish-checklist.md` (not shipped in crate):

```text
- [ ] cargo fmt / clippy / test / doc green on Linux + macOS CI
- [ ] CHANGELOG.md [0.1.0] date filled
- [ ] README status section no longer says pure WIP if claiming release
- [ ] Manual Anki import recorded on release PR
- [ ] cargo package --allow-dirty / cargo publish --dry-run locally
- [ ] crates.io name `genanki` still available (or fallback name decided)
- [ ] docs.rs build: default features only; bundled rusqlite must build on docs.rs
      (usually OK; if not, document package.metadata.docs.rs)
- [ ] LICENSE MIT present
- [ ] git tag v0.1.0 after publish
- [ ] cargo publish (human; needs crates.io token)
- [ ] Close epic #1 / issue #8 after publish or explicitly leave open until publish
```

**README touch:** Status blurb - once v0.1.0-ready, soften "Work in progress" to
"v0.1.0-ready" / "initial release candidate" without claiming crates.io
published until true.

### 4.9 Sample package for manual Anki import

Add `tests/manual_smoke.rs` **or** document a one-liner using existing APIs.

Prefer a small `#[test]` gated clearly:

```rust
/// Writes target/manual-smoke.apkg for human Anki import (always runs; cheap).
#[test]
fn write_manual_smoke_apkg() {
    // deck with: basic note, reversed, cloze, optional media-less
    // path: std::env::temp_dir() or CARGO_TARGET_TMPDIR / "genanki-manual-smoke.apkg"
    // print path via eprintln so CI logs show it
}
```

Using `std::env::var("CARGO_TARGET_TMPDIR")` or `env!("CARGO_MANIFEST_DIR")`
joined to `target/manual-smoke.apkg` - writing under `target/` is gitignored
and easy for local humans:

```rust
let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("target/manual-smoke.apkg");
```

Contents must exercise: builtin basic, cloze (2 fields), multi-deck **or**
single deck with several note types, unicode, and if easy a tiny media file
created in temp and referenced by basename (optional; media already covered
automatically elsewhere - manual smoke can skip media if file management is
annoying, but **prefer include** one 1x1 png or tiny txt as audio-less image
bytes written to temp).

Issue acceptance: "Cloze + media + multi-deck covered automatically" - already
true via existing tests; manual smoke is import confidence.

## 5. Test plan (RED -> GREEN per item)

### T0 - Baseline green

```text
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo test --doc --all-features
```

Branch from latest `main` (post #7).

### T1 - HTML property: no panic + shape invariants

**RED** - add `html_scanner_property_invariants` in `src/note.rs` tests with
PRNG; initially call a wrong assertion or empty implementation stub only if
needed - against **current** correct scanner the property should mostly **pass
immediately**. TDD twist for pure hardening:

- First add a **deliberately stricter oracle** that fails on a known edge if
  one exists, **or**
- Use "test first" as: write the property test file/claims, run (GREEN on
  correct code), then add at least one **mutation check** locally (break
  scanner, see RED, restore) once during implementation to prove the test
  catches regressions.

Document in PR that property tests were mutation-checked once.

Minimum assertions: sec. 4.2 items 1-5 with N>=200.

**GREEN** - no production change if scanner already correct; keep tests.

### T2 - HTML property: whitelist / blacklist oracles

**RED/GREEN** - generator that only injects whitelist tags => empty invalid
set; generator that injects known bad tags => all found.

### T3 - Cloze property: constructed markers oracle

**RED** - `cloze_property_constructed_markers` (sec. 4.3.4) with N>=200 random
marker layouts.

**GREEN** - should pass on current cloze impl; mutation-check once
(e.g. temporarily break dedupe).

### T4 - Cloze property: multi-field union

**RED/GREEN** - two-field model; union ords.

### T5 - Cloze property: random junk never panics + sorted unique

**RED/GREEN** - pure random bytes/chars mixed with occasional `{{c` fragments.

### T6 - Large-deck smoke integration

**RED** - `tests/large_deck.rs` with 10k notes; assert counts; eprintln time.

**GREEN** - pure usage of existing Package API; no production code unless a
bug appears (e.g. id overflow - unlikely at 10k with ms timestamp ids).

### T7 - Manual smoke artifact test

**RED/GREEN** - `tests/manual_smoke.rs` writes `target/manual-smoke.apkg`
(or target tmp); assert zip layout; eprintln absolute path.

### T8 - CI matrix Linux + macOS

**RED** - edit workflow; push branch; confirm both jobs required.

**GREEN** - YAML as sec. 4.5.

### T9 - Research note exists and is linked

**RED** - add `doc/research/anki-package-formats.md`; link from README
"Non-goals" or "Status" in one sentence; link from publish checklist.

**GREEN** - content per sec. 4.6. Verify Anki still documents anki2 import
(quick web/source check at implement time; cite date).

### T10 - CHANGELOG.md

**RED/GREEN** - add file per sec. 4.7. No code.

### T11 - Publish checklist + Cargo package dry list

**RED/GREEN**

- Add `doc/publish-checklist.md`
- Locally run `cargo package --list` and `cargo publish --dry-run` (no token
  publish); fix metadata if dry-run fails
- Record any `package.metadata.docs.rs` only if dry-run/docs issue appears

### T12 - README status polish for release readiness

**RED/GREEN** - Status section reflects phases 0-6 complete / v0.1.0-ready;
does not claim crates.io published until true. Ensure README example still
matches doctests.

### T13 - Epic success criteria audit (PR body)

Map each epic criterion to evidence:

| Criterion | Evidence |
| --------- | -------- |
| Phases 0-5 complete | Issues #2-#7 closed |
| README example -> apkg opens in Anki | T7 artifact + manual smoke note on PR |
| `cargo test` green Linux + macOS | T8 CI |
| GUID goldens match Python 0.13.x | existing `guid_for_goldens_table` |
| Cloze + media + multi-deck automated | existing tests + T3-T5 hardening |
| Ready to publish crates.io | T10 T11 checklist; dry-run OK |

### T14 - Full workspace gates

Same as T0 plus confirm macOS CI green on the PR.

## 6. Implementation order (fine-grained cycles)

1. **T0** baseline on branch `issue/8-hardening-v0.1.0`.
2. **T1 T2** HTML property tests (mutation-check once).
3. **T3 T4 T5** cloze property tests (mutation-check once).
4. **T6** large-deck smoke.
5. **T7** manual smoke apkg writer test.
6. **T8** CI matrix.
7. **T9** format research note + README link.
8. **T10 T11 T12** CHANGELOG, publish checklist, README status, `cargo publish --dry-run`.
9. **T13** fill PR description with criteria + manual smoke template.
10. **Human:** run Anki import; paste results into PR.
11. **T14** final gates; mark issue checkboxes; plan status -> IMPLEMENTED.

### 6.1 Refactor rules

- Only while green.
- Share PRNG helper if both HTML and cloze tests need it (same module).
- Do not "clean up" unrelated Phase 1-5 code unless clippy forces it.
- No API breaks; additive tests/docs only unless a real bug is found.

### 6.2 If a property test finds a real bug

1. Minimize failing input.
2. Add a dedicated regression golden test (fixed string) that fails.
3. Fix production code (minimal).
4. Keep the property test.
5. Note the fix in CHANGELOG under Fixed if user-visible.

### 6.3 If large-deck is too slow/OOM on CI

1. First try: reuse one `Arc<Model>`, avoid extra clones, capacity-hint
   `Deck` notes vec if such API exists (add `Deck::with_capacity` **only if
   measured need** - YAGNI otherwise).
2. Reduce to 5_000 with comment.
3. Last resort: `#[ignore]` + document in checklist - requires maintainer OK on
   PR (deviates from locked "run in CI").

### 6.4 PR description template (manual smoke + criteria)

```markdown
## Summary
Phase 6 / issue #8: hardening + v0.1.0 readiness.

## Manual Anki smoke
- Anki version:
- OS:
- Package: `target/manual-smoke.apkg` (from `cargo test write_manual_smoke_apkg -- --nocapture`)
- Result: imported OK / problems:
- Notes checked: basic / cloze / (media):

## Epic #1 success criteria
- [ ] Phases 0-5 closed
- [ ] CI Linux + macOS green
- [ ] GUID goldens (link test)
- [ ] Cloze + media + multi-deck automated
- [ ] README + manual import note
- [ ] crates.io ready (checklist); publish deferred to human

## Deps
Zero new crates.
```

## 7. File touch list

| Path | Action |
| ---- | ------ |
| `src/note.rs` | Property tests (HTML + cloze) + tiny test-only PRNG |
| `tests/large_deck.rs` | 10k smoke |
| `tests/manual_smoke.rs` | Sample apkg for human import |
| `tests/common/mod.rs` | Only if shared helpers needed (prefer not) |
| `.github/workflows/ci.yml` | Linux + macOS matrix; fmt job split |
| `doc/research/anki-package-formats.md` | **New** research note |
| `doc/publish-checklist.md` | **New** crates.io checklist |
| `CHANGELOG.md` | **New** v0.1.0 |
| `README.md` | Status polish; link research note / changelog |
| `Cargo.toml` | Metadata-only if dry-run requires; **no new deps** |
| `doc/plans/issue-8.md` | This plan (status -> IMPLEMENTED when done) |

No production changes expected to `package.rs` / `deck.rs` / `guid.rs` /
`req.rs` / `apkg/*` unless tests expose a bug.

## 8. Acceptance mapping

| Acceptance (issue #8) | Deliverable |
| --------------------- | ----------- |
| Property/fuzz tests for cloze regex and HTML scanner | T1-T5 hand-rolled properties |
| Large-deck smoke (~10k) time + memory sanity | T6 soft sanity + eprintln |
| Optional Python golden CI sidecar | **Deferred** (sec. 3.2); noted on issue/PR |
| Research note `collection.anki21` / stay on anki2 | T9 |
| Manual smoke import into Anki desktop; record on PR | T7 + human sec. 6.4 |
| `CHANGELOG.md` for v0.1.0 | T10 |
| crates.io publish checklist | T11 |
| CI on Linux + macOS | T8 |
| Final pass on epic success criteria | T13 |
| Phases 1-5 complete | Already true; restate on PR |
| `cargo test` green Linux + macOS | T8 T14 |
| GUID goldens match Python 0.13.x | Existing tests; cite on PR |
| Cloze + media + multi-deck automated | Existing + T3-T5 |
| README verified + manual note | T12 + human smoke |
| Ready to publish (or published) | Ready = checklist + dry-run; **not** published in #8 |

## 9. Epic / issue checkbox handling

On the PR, explicitly mark the Python sidecar checkbox as:

```text
- [ ] Optional CI sidecar ...  (DEFERRED post-v0.1.0 - see plan sec. 3.2)
```

Do not close #8 until: CI green both OSes, docs landed, manual smoke recorded
(or explicitly waived by maintainer on the PR).

Publish itself may remain open work after #8 closes if "ready to publish"
is accepted without live crates.io - **locked:** readiness satisfies the gate;
closing #8 does not require `cargo publish`. Epic #1 may stay open until
publish if desired.

## 10. Open questions - RESOLVED

| Question | Resolution |
| -------- | ---------- |
| Plan path | `genanki-rs/doc/plans/issue-8.md` |
| New external deps | **None** (confirmed) |
| Property/fuzz approach | Hand-rolled only (confirmed) |
| Python CI sidecar | Skip; document follow-up (confirmed) |
| crates.io | Readiness only; no publish in #8 (confirmed) |
| Large-deck bounds | Soft sanity + log timing (confirmed) |
| Large-deck in CI | Yes by default (10k); fallback plan in sec. 6.3 |
| MSRV / rust-version | Omit unless MSRV CI added (not in #8) |
| authors field | Optional; do not block |
| `find_invalid_html_tags` crate-root re-export | Not required for #8; stays `genanki::note::...` unless README wants it (no change) |

No unresolved blockers. Implement on branch `issue/8-hardening-v0.1.0` with
strict TDD for code-backed items.

## 11. PR checklist (when implementing)

- [ ] Every code work item went RED before GREEN where behavior was new; property tests mutation-checked once each family
- [ ] **Zero** new crates in `Cargo.toml`
- [ ] HTML + cloze property tests landed and green
- [ ] 10k large-deck smoke green; timing logged
- [ ] CI matrix Linux + macOS green
- [ ] Research note: stay on `collection.anki2`
- [ ] CHANGELOG.md v0.1.0
- [ ] `doc/publish-checklist.md` + local `cargo publish --dry-run` OK
- [ ] Manual Anki smoke recorded on PR (or maintainer waiver)
- [ ] Python sidecar explicitly deferred, not silently ignored
- [ ] README status updated
- [ ] Epic success criteria checked off with evidence on PR
- [ ] `cargo fmt`, `clippy -D warnings`, `test`, `doc` green
- [ ] Issue #8 checkboxes updated in PR description
- [ ] Plan status -> IMPLEMENTED + PR link

## 12. Quick reference - commands

```text
# property + unit + integration
cargo test --all-targets --all-features

# doctests
cargo test --doc --all-features

# large deck only (see timing)
cargo test --test large_deck -- --nocapture

# manual smoke artifact path
cargo test --test manual_smoke -- --nocapture

# package / publish readiness (no upload)
cargo package --list
cargo publish --dry-run

# gates
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

## 13. Dependency ledger (v0.1.0 final expected)

Runtime (unchanged):

```toml
sha2 = "0.10"
thiserror = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
regex = "1"
log = "0.4"
rusqlite = { version = "0.37", features = ["bundled"] }
zip = "2"
tempfile = "3"
```

Dev-dependencies: still none.

No optional feature flags required for v0.1.0.
