# crates.io publish checklist (v0.1.0)

Readiness gate for the v0.1.0 release. **Not** an instruction to publish from
issue #8 (locked decision: readiness only; `cargo publish` is a human step
after this checklist is green).

See also: [Anki package formats research note](./research/anki-package-formats.md).

## Pre-publish checks

- [ ] `cargo fmt --all -- --check` green (Linux CI)
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` green on Linux + macOS
- [ ] `cargo test --all-targets --all-features` green on Linux + macOS
- [ ] `cargo test --doc --all-features` green on Linux + macOS
- [ ] `CHANGELOG.md` `[0.1.0]` section present with the release date filled
- [ ] README Status section says v0.1.0-ready (not "pure WIP", not "published")
- [ ] Manual Anki desktop import of `target/manual-smoke.apkg` recorded on the
      release PR (`cargo test --test manual_smoke -- --nocapture` to write it)
- [ ] `cargo package --list` reviewed: no `doc/`, no `.github/`, tests fine to ship
- [ ] `cargo publish --dry-run` passes locally (no token needed)
- [ ] crates.io name `genanki` still available (`cargo search genanki` / web check),
      or fallback name decided
- [ ] docs.rs build: default features only; bundled rusqlite must build on docs.rs
      (usually OK; if not, document `package.metadata.docs.rs` in Cargo.toml)
- [ ] `LICENSE` (MIT) present at repo root
- [ ] `Cargo.toml` metadata spot-check: description / license / repository /
      homepage / documentation / readme / keywords / categories / exclude

## Publish steps (human, after #8 closes)

- [ ] `cargo publish` (requires a crates.io token; dry-run first)
- [ ] Verify the docs.rs build for the published version
- [ ] `git tag v0.1.0` and push the tag
- [ ] Close epic #1 (and this issue) or explicitly leave open until publish is done

## Deferred (post-v0.1.0 optional work)

- [ ] Python genanki CI sidecar: generate goldens with Python and compare
      semantic sqlite content (GUIDs, ords, flds) - see plan sec. 3.2
