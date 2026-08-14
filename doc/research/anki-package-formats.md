# Research note: Anki package formats (v0.1.0 decision)

Status: DECIDED - stay on `collection.anki2` for v0.1.0
Date: 2026-08-14
Issue: https://github.com/tlkahn/genanki-rs/issues/8
Branch: `issue/8-hardening-v0.1.0`

## 1. What this crate writes today

An `.apkg` is a zip archive containing:

- `collection.anki2`: a SQLite3 database (schema in
  `src/apkg/schema.rs` + `src/apkg/col.rs`), matching Python genanki v0.13.x
  byte-for-byte at the package level for the tables this crate writes.
- `media`: a JSON object mapping numbered media ids to basenames
  (`{"0": "smoke.png", ...}`); empty packages write `{}`.
- `0`, `1`, ...: the media payloads themselves, referenced from note fields
  as `[sound:x.mp3]` / `<img src="x.png">`.

This is the "legacy" package format that Python genanki (and therefore this
crate, its port) has always produced.

## 2. Newer Anki storage formats

Anki 2.1.57+ stores collections as `collection.anki21` / `collection.anki21b`
(a richer schema with separate scheduling tables, and an optional zstd
compressed variant). These formats apply to Anki's *own* collection storage.
Importing an `.apkg` remains a separate path:

- Anki desktop still accepts legacy `.apkg` packages containing
  `collection.anki2` as of the research date above.
- The `.apkg` container format itself (zip + `collection.anki2` + `media`) is
  what Anki's import path expects from third-party generators.

## 3. Decision for v0.1.0

**Stay on `collection.anki2`.** Rationale:

1. Python genanki v0.13.x (the parity target) writes anki2; matching it keeps
   the crate's goldens and semantics aligned with the reference implementation.
2. Anki's import path still accepts anki2 packages (research date above).
3. Writing anki21 correctly (scheduling tables, optional zstd) is a separate
   feature with real maintenance cost and no user-visible benefit yet.
4. The manual Anki desktop smoke on the v0.1.0 PR (issue #8) re-verifies the
   import path on current Anki before release.

This decision is deliberately revisit-able: if the manual smoke ever shows an
import failure, or Anki release notes announce deprecation of anki2 import,
the crate should re-evaluate (see next section).

## 4. Revisit triggers

- Anki release notes deprecating or removing legacy `collection.anki2`
  `.apkg` import.
- Bug reports of import failures on supported Anki versions.
- Python genanki itself moving to a newer writer format (parity drift).

## 5. Out of scope (v0.1.0)

- Writing `collection.anki21` / `collection.anki21b` (incl. zstd).
- Reading existing `.apkg` files as a public API.
- Anki addon collection write path (`write_to_collection_from_addon`).

## 6. References (verified resolving 2026-08-14)

- Anki manual, Importing: https://docs.ankiweb.net/importing/intro.html
- Anki source (Rust collection layer; anki21 schema): https://github.com/ankitects/anki/blob/main/rslib/src/collection/mod.rs
- Python genanki (parity target): https://github.com/kerrickstaley/genanki
- Anki ecosystem file formats (community doc): https://anki.tenderapp.com/kb/anki-ecosystem/file-formats
