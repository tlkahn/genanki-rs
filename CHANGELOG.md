# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-08-14

Initial release: Rust port of Python genanki focused on writing Anki `.apkg`
packages (feature parity target: kerrickstaley/genanki v0.13.x).

### Added

- `Model` / `Field` / `Template` builders, `req` computation, front/back + cloze
- `Note` validation (field count, tags), GUID (`guid_for` / base91), cards
- `Deck` + `Package` writer (`collection.anki2`, media map, hermetic timestamp)
- Builtin models: BASIC_*, CLOZE (`LazyLock<Model>`)
- Invalid HTML tag scanner (`note::find_invalid_html_tags`) with non-fatal `log` warnings
- Hand-rolled property tests for the cloze ord regex and the invalid-HTML scanner
- Large-deck smoke test (~10k notes) and a manual Anki import sample artifact
- README + rustdoc; CI on Linux and macOS

### Non-goals (v0.1.0)

- Reading/modifying existing `.apkg`
- Anki addon collection write path
- Newer `collection.anki21*` writers
