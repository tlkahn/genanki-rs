# Issue #2: Phase 0 - Scaffolding (CI, crate metadata, module skeleton)

Status: PLAN
Issue: https://github.com/tlkahn/genanki-rs/issues/2
Parent epic: https://github.com/tlkahn/genanki-rs/issues/1
Branch: `issue/2-scaffolding` (or worktree equivalent)

## 1. Goal

Finish repo/crate bootstrap so later phases can land cleanly. No feature
implementation. After this phase:

- CI is green on `main` for a skeleton crate
- `Cargo.toml` is publish-ready (aside from version churn)
- `src/` module tree matches the epic layout (empty stubs OK)
- Crate root forbids `unsafe`
- Basic `Error` / `Result` placeholders exist
- README still points at epic #1 for the full spec

### Out of scope

- Feature implementation (Phases 1-6: GUID, Model, Note, Deck, Package, builtins)
- Real dependency additions (`rusqlite`, `zip`, `serde`, etc.) - defer to the
  phase that first needs them
- Publishing to crates.io
- Integration / package round-trip tests

## 2. Current state (code-verified)

| Item | Status |
| ---- | ------ |
| Repo `tlkahn/genanki-rs` | Done |
| `LICENSE` (MIT) | Done |
| `README.md` pointing at #1 | Done (keep; light polish OK) |
| Library crate skeleton | Partial - still cargo-new defaults |
| `src/lib.rs` | Default `add` demo + test; not the real API surface |
| `Cargo.toml` name | `genanki` |
| `Cargo.toml` edition | `2024` (local rustc 1.95) |
| `Cargo.toml` publish | `publish = ["rsproxy-sparse"]` (local mirror pin; not crates.io-ready) |
| `Cargo.toml` metadata | Missing description, repository, license, keywords, categories, readme |
| Module skeleton | Missing |
| `#![forbid(unsafe_code)]` | Missing |
| `error.rs` | Missing |
| CI | Missing (no `.github/workflows/`) |
| `.gitignore` | Present (`/target`, lockfile, editor junk, `*.apkg`, `*.anki2`) |

### Locked / recommended decisions for Phase 0

| Topic | Decision | Rationale |
| ----- | -------- | --------- |
| crates.io package name | Keep **`genanki`** as the Cargo package name | Epic preference; repo stays `genanki-rs`. `cargo search` shows existing prior art under `genanki-rs` / `genanki-rs-rev`, not bare `genanki`. Final availability re-check at publish (#8). Document the choice in README. |
| Edition | Keep **2024** | Already on crate; toolchain is 1.95. Epic said "2021 or whatever we pin" - 2024 is fine if CI uses a recent stable. |
| MSRV | Pin CI to **stable** only for Phase 0; record aspirational MSRV note in README (`1.85+` if edition 2024 requires it, else whatever stable needs). Do not add `rust-version` until #8 hardening unless CI makes it cheap. | Avoid false MSRV claims before real deps land. |
| `publish` field | Remove `publish = ["rsproxy-sparse"]` (or set `publish = false` until release). Prefer **omit the field** so default crates.io publish works later. | Local sparse mirror pin is environment-specific and blocks normal publish metadata readiness. |
| Dependencies this phase | **None** in `[dependencies]`. Optional: `thiserror` only if we want the real error derive now; otherwise plain enum stub without deps. | Prefer zero runtime deps in Phase 0. `thiserror` can land in #3 with the real error variants. |
| Placeholder modules | `pub mod` with empty/`todo`-free stubs that compile; public re-exports only for types that exist (`Error`, `Result`). Do not re-export unfinished domain types until they have real definitions. | Keeps `cargo build` / clippy clean without fake APIs. |
| Demo `add` API | Delete | Not part of the product surface. |

## 3. Target tree after Phase 0

```text
genanki-rs/
  .github/
    workflows/
      ci.yml
  Cargo.toml
  LICENSE
  README.md
  src/
    lib.rs                 # forbid unsafe, mod declarations, selective re-exports
    error.rs               # Error + Result placeholders
    model.rs               # empty module stub
    note.rs
    card.rs
    deck.rs
    package.rs
    guid.rs
    builtin_models.rs
    req.rs
    apkg/
      mod.rs
      schema.rs
      col.rs
      db.rs
  doc/
    plans/
      issue-2.md           # this plan
```

No `tests/` integration harness required yet (those arrive with the phases that
need them). Unit-test module in `error.rs` or `lib.rs` is enough to prove CI
`cargo test` runs.

## 4. Work items

### 4.1 `Cargo.toml` metadata

Set (adjust URLs if remote differs):

```toml
[package]
name = "genanki"
version = "0.1.0"
edition = "2024"
description = "Generate Anki .apkg decks programmatically"
license = "MIT"
repository = "https://github.com/tlkahn/genanki-rs"
homepage = "https://github.com/tlkahn/genanki-rs"
documentation = "https://docs.rs/genanki"
readme = "README.md"
keywords = ["anki", "flashcards", "apkg", "deck"]
categories = ["encoding", "multimedia"]
exclude = ["doc/", ".github/"]

# Remove: publish = ["rsproxy-sparse"]

[dependencies]
# Intentionally empty in Phase 0. Real deps land per phase (see epic #1).

[dev-dependencies]
# Empty until a phase needs tempfile / etc.
```

Notes:

- `keywords`: max 5 on crates.io - use `anki`, `flashcards`, `apkg`, `deck`
  (drop a fifth or add `learning` if desired).
- `categories`: must be valid crates.io category slugs. `encoding` fits package
  format writing; if review prefers fewer, `multimedia` alone is acceptable.
  Verify against https://crates.io/category_slugs before PR if unsure.
- Do **not** add feature flags yet.

### 4.2 Crate root and module stubs

`src/lib.rs`:

```rust
//! Programmatic generation of Anki `.apkg` packages.
//!
//! Feature roadmap and full specification: see repository epic issue #1.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod error;

// Domain modules (filled in later phases). Public so paths stabilize early.
pub mod apkg;
pub mod builtin_models;
pub mod card;
pub mod deck;
pub mod guid;
pub mod model;
pub mod note;
pub mod package;
pub mod req;

pub use crate::error::{Error, Result};
```

Decision on `#![deny(missing_docs)]`:

- **Preferred for Phase 0:** enable it and put a one-line `//!` on every module
  file plus `///` on `Error` / `Result`. Forces good hygiene before APIs grow.
- **Fallback:** skip the lint deny until #7 docs phase if it creates noise on
  empty stubs. Still write module-level `//!` docs either way.

Each stub file pattern:

```rust
//! Note types and card templates. (Phase 2)
```

`src/apkg/mod.rs`:

```rust
//! SQLite schema, seed `col` row, and DB writers for `.apkg` packages.

pub mod col;
pub mod db;
pub mod schema;
```

`src/error.rs` placeholder (no `thiserror` yet):

```rust
//! Crate error type and result alias.

use std::fmt;

/// Errors produced by this crate.
///
/// Variants will expand in Phase 1+ (IO, validation, template req, media, SQL).
#[derive(Debug)]
pub enum Error {
    /// Placeholder variant so the type is usable before real failures exist.
    /// Remove once concrete variants land.
    Internal(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for Error {}

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = Error::Internal("scaffold");
        assert_eq!(err.to_string(), "internal error: scaffold");
    }
}
```

Do **not** re-export `Model`, `Note`, `Deck`, etc. until those types exist
(Phases 2-4). Empty modules are enough for path stability (`genanki::model::...`).

### 4.3 CI (GitHub Actions, stable)

Create `.github/workflows/ci.yml`:

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
  test:
    name: fmt + clippy + test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install stable toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - name: Cache cargo
        uses: Swatinem/rust-cache@v2

      - name: cargo fmt
        run: cargo fmt --all -- --check

      - name: cargo clippy
        run: cargo clippy --all-targets --all-features -- -D warnings

      - name: cargo test
        run: cargo test --all-targets --all-features
```

Phase 0 scope is **Linux stable only**. Epic success criteria want Linux + macOS
by v0.1.0; matrix expansion is explicitly Phase 6 (#8). Do not over-build CI now.

Optional cheap additions (include if trivial):

- `concurrency:` group to cancel stale PR runs
- `permissions: contents: read`

### 4.4 README touch-up (minimal)

Keep status pointer to #1. Small additions only:

1. State package name: library is published (eventually) as **`genanki`**;
   repository is `genanki-rs`.
2. Keep non-affiliation note and MIT license blurb.
3. Do not expand into full API docs (that is #7).
4. No README code example that does not compile - either omit example until
   Phase 5/7, or keep a commented "planned API" block. Prefer omit.

### 4.5 Housekeeping

- Ensure `cargo fmt` is clean.
- Ensure `cargo clippy --all-targets -- -D warnings` is clean.
- Ensure `cargo test` passes (error display unit test at minimum).
- Leave `.gitignore` as-is unless something obvious is missing (`doc/` should
  **not** be ignored - plans are tracked).
- Do not commit `Cargo.lock` for a pure library (already gitignored - good).

## 5. Implementation order

1. Branch from latest `main`.
2. Rewrite `Cargo.toml` metadata; drop mirror `publish` pin.
3. Replace `src/lib.rs`; add all stub modules + `error.rs`.
4. Run local `fmt` / `clippy` / `test`; fix fallout.
5. Add `.github/workflows/ci.yml`.
6. Light README edit (package name decision + keep #1 link).
7. Open PR against `main` titled/body referencing #2.
8. Confirm CI green; merge; tick issue checkboxes; close #2 when acceptance met.

## 6. Acceptance criteria (map to issue #2)

| Criterion | How verified |
| --------- | ------------ |
| CI green on `main` for skeleton crate | Actions run: fmt, clippy `-D warnings`, test |
| `Cargo.toml` ready for eventual publish | description, repository, license, readme, keywords, categories present; no bad `publish` pin |
| Final crates.io name decided for now | Documented as `genanki` (re-check at #8) |
| Module skeleton matches epic layout | Tree in section 3 exists and compiles |
| `#![forbid(unsafe_code)]` at crate root | Present in `lib.rs` |
| Basic `Error` / `Result` placeholders | `error.rs` + re-export |
| README still points at epic #1 | Link remains |
| No Phase 1+ feature code | PR diff is scaffold-only |

## 7. Test plan (local + CI)

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo doc --no-deps   # optional local check if deny(missing_docs) enabled
```

No `.apkg` golden tests in this phase.

## 8. PR shape

- **One PR** for the whole of Phase 0 (small, cohesive).
- Title: `Phase 0: scaffolding (CI, metadata, module skeleton)`
- Body: checklist mirrored from issue #2; note package name decision; link epic #1.
- Do not bump version beyond `0.1.0`.
- Do not add unrelated refactors.

## 9. Follow-ups (explicitly not this PR)

| Item | Phase / issue |
| ---- | ------------- |
| `guid_for`, base91, schema/col SQL constants, real `Error` variants | #3 |
| `thiserror`, `sha2`, etc. | as needed from #3 onward |
| Model builders / Mustache / `req` | #4 |
| Note / Card | #5 |
| Deck / Package / rusqlite / zip | #6 |
| Builtins + full README/rustdoc | #7 |
| macOS CI matrix, MSRV `rust-version`, crates.io publish | #8 |

## 10. Risks / notes

1. **Name collision on crates.io:** if `genanki` is taken at publish time, rename
   package to `genanki_rs` (underscore) without renaming the repo. Phase 0 only
   needs a documented intent; #8 performs the live check.
2. **Edition 2024 + CI:** `dtolnay/rust-toolchain@stable` must be new enough for
   edition 2024. Current stable (1.85+) is; if Actions ever pins old stable,
   CI will fail loudly - acceptable.
3. **`missing_docs` deny:** can be noisy; drop the deny (keep module docs) if it
   slows scaffolding, and re-enable in #7.
4. **Prior art crates** (`yannickfunk/genanki-rs`, etc.): do not copy code; our
   module layout follows epic #1, not those repos.
5. **Local `publish = ["rsproxy-sparse"]`:** removing it may change how the
   maintainer publishes from a mirror-oriented environment. Prefer project
   correctness (crates.io-ready metadata) over machine-local cargo config; use
   `.cargo/config.toml` (gitignored) for registry mirrors instead of package
   `publish` keys.

## 11. Done definition

Phase 0 is done when the acceptance table in section 6 is true on `main`, issue
#2 checkboxes are updated, and #3 can start without further bootstrap PRs.
