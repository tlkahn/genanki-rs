# Issue #5: Phase 3 - Note + Card (validation, cloze/front-back generation)

Status: IMPLEMENTED (see PR #12)
Issue: https://github.com/tlkahn/genanki-rs/issues/5
Parent epic: https://github.com/tlkahn/genanki-rs/issues/1
Branch: `issue/5-note-card`
Method: strict fine-grained TDD (RED -> GREEN -> refactor) per work item below

## 1. Goal

Land notes, validation, GUID/sort/due/tags, and card generation (front/back +
cloze). No Deck/Package/sqlite/zip/media.

After this phase:

- `Card` lives in `src/card.rs` (`ord`, `suspend`)
- `Note` lives in `src/note.rs` with `Arc<Model>`, fields, tags, guid, sort
  field, due, and lazy-cached cards
- Tag validation rejects U+0020 on construct and every mutation path
- Field-count mismatch errors at fallible `Note::new`
- Invalid HTML tag scan (non-fatal `log::warn!`); GUID content is never scanned
- Default GUID = `guid_for(fields...)`; explicit override preserved
- Sort field: override or `fields[model.sort_field_index]`
- Front/back cards from `model.req()` (`all` / `any` + non-empty fields)
- Cloze cards: field names from first template `qfmt`, `{{cN::...}}` (DOTALL,
  hints), unique `N-1` ords, default `{0}` if none
- Suspend flag on `Card`
- Unit/integration tests ported from `test_note.py`, `test_cloze.py`, and the
  card-req cases in `test_genanki.py`
- `cargo fmt` / `clippy -D warnings` / `test` / `doc` green

### Out of scope

- Deck / Package / sqlite / zip / media (Phase 4 / #6)
- `Card::write_to_db` / `Note::write_to_db` (Phase 4) - only pure domain +
  `pub(crate)` field/tag formatting helpers if useful
- Builtin model constants (Phase 5 / #7)
- CLOZE single-field auto-pad / deprecation (epic: do **not** auto-pad; require
  correct field count)
- Subclass/`@property guid` Python trick (`test_Note_with_guid_property`)
- Anki import round-trips (need Package writer)

## 2. Current state (code-verified)

| Item | Status |
| ---- | ------ |
| Phase 0 scaffolding on `main` | Done (#2 / PR #9) |
| Phase 1 GUID + primitives on `main` | Done (#3 / PR #10) |
| Phase 2 Model + req on `main` | Done (#4 / PR #11) |
| `src/note.rs` | Stub module doc only |
| `src/card.rs` | Stub module doc only |
| `src/error.rs` | `thiserror`, `Internal` + `TemplateReq`, `#[non_exhaustive]` |
| `src/lib.rs` | Re-exports `Error`, `Result`, `guid_for`, `Model`/`Field`/`Template`/`ModelType`, `ReqEntry`/`ReqKind` |
| `Model` | `Clone` + builders; `req()` / `to_json()` work |
| `Cargo.toml` deps | `sha2`, `thiserror`, `serde`, `serde_json` |
| Python reference | genanki **v0.13.1** (latest `note.py` has CDATA/comment HTML fix and `if not card_ords`) |

## 3. Locked decisions

| Topic | Decision | Rationale |
| ----- | -------- | --------- |
| External deps this phase | **`regex` + `log`** (in addition to existing) | Confirmed with maintainer. See sec. 3.1. |
| HTML scanner engine | **Two-pass with `regex` crate** (no lookaround). Not `fancy-regex`. | Standard `regex` has no negative lookahead; two-pass matches Python goldens (verified). |
| Cloze parser engine | **`regex`**, patterns from Python `note.py` | Closer to upstream; cloze patterns need no lookaround. |
| HTML warnings | **`log::warn!`** + public `find_invalid_html_tags` | Facade is zero-cost without a logger; finder is what tests assert. |
| Field-count check | **Fallible `Note::new` only** (fail-fast) | Issue preference; write path is Phase 4. Document that callers must construct valid notes. |
| Model ownership | **`Arc<Model>`** via `impl Into<Arc<Model>>` | Epic proposal; cheap share across many notes; `Model: Clone` already. |
| Cards API | **Lazy-cache `Option<Vec<Card>>`**; `cards()` / `cards_mut()`; invalidate when fields change | Matches Python `cached_property` + in-place `suspend = True`. |
| Cloze card order | **Sort ords ascending** before building `Vec<Card>` | Python set order is arbitrary; tests use sets/sorted. Deterministic Rust API. |
| Empty cloze | **Emit one card with `ord = 0`** if no `{{cN::...}}` found | Latest Python: `if not card_ords: card_ords = {0}` (fixes dead `== {}` check in older tags). |
| Tag API | **Validated methods** (`with_tags` / `set_tags` / `add_tag` / ...), not a public `TagList` type | Idiomatic Rust; every mutation path returns `Result` and is tested. |
| GUID API | `Option<String>` override; `guid()` returns `String` (computed or clone) | Matches Python property. |
| Sort field API | `Option<String>` override; `sort_field()` returns `&str` | Index comes from `model.sort_field_index`. |
| `due` | `i64`, default `0` | Propagates to cards at Phase 4 write; stored on note now. |
| DB writers | **Not this phase** | `pub(crate) format_fields` / `format_tags` OK as pure helpers for #6. |
| Public re-exports | `Note`, `Card` from crate root | Epic API sketch. |
| Error growth | Add `TagContainsSpace`, `FieldCountMismatch` (replace `Internal` for these paths) | Concrete, testable. |
| TDD discipline | Every behavior change: failing test first, minimal impl, refactor. One logical assertion group per cycle when practical. | Per user request. |

### 3.1 External deps confirmation (explicit)

Maintainer request: minimize external deps; confirm anything we **must** add.

| Crate | Required? | Why |
| ----- | --------- | --- |
| `regex` | **Chosen (not strictly mandatory)** | Cloze + HTML can be hand-rolled (Phase 2 style). Maintainer preferred `regex` for Python parity and lower parser risk. **Cannot** express Python's HTML negative lookahead in `regex`; use two-pass instead (sec. 4.5). Alternative rejected: `fancy-regex` (heavier, lookaround). |
| `log` | **Chosen (not strictly mandatory)** | Could expose only `find_invalid_html_tags` / a callback with zero deps. Maintainer preferred the `log` facade + public finder. No `tracing`. |
| `fancy-regex` | **No** | Avoid. |
| `thiserror` / `sha2` / `serde` / `serde_json` | Already present | Reuse. |
| Anything else new | **No** | `Arc` is std. No `cached_property` equivalent crate. No dev-dep required for log capture (custom `log::Log` in tests). |

**Bottom line:** this phase adds exactly two crates (`regex`, `log`). Neither is forced by the language; both were explicitly accepted.

Versions (pin latest compatible at implement time):

```toml
regex = "1"
log = "0.4"
```

## 4. Algorithm reference (must match Python v0.13.1 semantics)

### 4.1 `Card`

```rust
pub struct Card {
    pub ord: i32,
    pub suspend: bool, // default false
}

impl Card {
    pub fn new(ord: i32) -> Self { Self { ord, suspend: false } }
}
```

Phase 4 will map `suspend -> queue = -1 else 0` and `due` from the note. No DB code here.

### 4.2 `Note` shape

```rust
pub struct Note {
    model: Arc<Model>,
    fields: Vec<String>,
    sort_field_override: Option<String>,
    tags: Vec<String>,
    guid_override: Option<String>,
    due: i64,
    /// Lazy card cache; `None` means "needs (re)compute".
    cards: Option<Vec<Card>>,
}
```

Suggested constructors / builders (names flexible if tests stay clear):

```rust
impl Note {
    /// Fallible: field count must equal `model.fields.len()`; tags validated.
    pub fn new(
        model: impl Into<Arc<Model>>,
        fields: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self>;

    pub fn with_tags(self, tags: impl IntoIterator<Item = impl Into<String>>) -> Result<Self>;
    pub fn with_guid(self, guid: impl Into<String>) -> Self;
    pub fn with_sort_field(self, sf: impl Into<String>) -> Self;
    pub fn with_due(self, due: i64) -> Self;

    pub fn model(&self) -> &Model;
    pub fn fields(&self) -> &[String];
    pub fn tags(&self) -> &[String];
    pub fn due(&self) -> i64;

    pub fn guid(&self) -> String;           // override or guid_for(&field_refs)
    pub fn sort_field(&self) -> &str;     // override or fields[sort_field_index]

    pub fn set_fields(&mut self, fields: impl IntoIterator<Item = impl Into<String>>) -> Result<()>;
    pub fn set_tags(&mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Result<()>;
    pub fn add_tag(&mut self, tag: impl Into<String>) -> Result<()>;
    pub fn set_tag(&mut self, index: usize, tag: impl Into<String>) -> Result<()>; // optional but covers Python __setitem__
    // extend_tags / insert_tag if cheap; each must validate

    pub fn set_guid(&mut self, guid: Option<String>);
    pub fn set_sort_field(&mut self, sf: Option<String>);
    pub fn set_due(&mut self, due: i64);

    /// Ensure cache, return shared slice. Computes via front/back or cloze.
    pub fn cards(&mut self) -> Result<&[Card]>;
    /// Ensure cache, return mutable vec (for `cards_mut()[i].suspend = true`).
    pub fn cards_mut(&mut self) -> Result<&mut Vec<Card>>;
}
```

Invalidation: any path that changes `fields` (or model, if ever mutable) sets
`self.cards = None`. Tag/guid/sort/due changes do **not** invalidate cards.

On `Note::new` / `set_fields`: after field-count check, run HTML scan and
`log::warn!` if needed (non-fatal).

### 4.3 Tag validation

```text
for each tag:
  if tag contains U+0020 (' '):
    return Error::TagContainsSpace { tag }
```

Only ASCII space, matching Python `' ' in tag`. Do not reject tabs/newlines
unless a test demands it (Python does not).

### 4.4 Field count

```text
if fields.len() != model.fields.len():
  return Error::FieldCountMismatch {
    model_name, model_fields, note_fields
  }
```

Error message should be informative (model name + both counts). No auto-pad.

### 4.5 Invalid HTML tags

Python (v0.13.1):

```python
_INVALID_HTML_TAG_RE = re.compile(
  r'<(?!/?[a-zA-Z0-9]+(?: .*|/?)>|!--|!\[CDATA\[)(?:.|\n)*?>'
)
```

**`regex` crate cannot compile this** (no `?!` lookaround). Equivalent two-pass
(verified byte-for-byte against Python `findall` on all goldens including
issue-28 latex and CDATA/comments):

```text
1. Find all tags with (?s)<.*?>
2. For each match t (including < and >):
     body = t[1..]            # after '<'
     if body matches ^/?[a-zA-Z0-9]+(?: .*|/?)$   # note: body includes final '>'
        OR body starts with "!--"
        OR body starts with "![CDATA["
     then OK (skip)
     else INVALID (collect t)
```

More precisely, the "valid tag" check is on `body` where `body` is everything
after `<` **including** the closing `>`:

```text
valid_tag_body = ^/?[a-zA-Z0-9]+(?: .*|/?)>$
```

Public API:

```rust
/// Return invalid HTML tag substrings in `field` (Python findall parity).
pub fn find_invalid_html_tags(field: &str) -> Vec<String>;
```

Warn path (called from `Note::new` / `set_fields`):

```text
for field in fields:
  invalid = find_invalid_html_tags(field)
  if !invalid.is_empty():
    log::warn!(
      "Field contained the following invalid HTML tags. Make sure you are \
       calling html escaping if your field data is not already HTML-encoded: {}",
      invalid.join(" ")
    )
```

**Never** pass `guid` (override or computed) through the scanner.

GUID-override acceptance: constructing a note whose **fields** are clean but
whose **guid** contains `<>` / `<@h1>` must not emit HTML warnings. Test via a
capturing `log::Log` + dirty guid + clean fields.

### 4.6 GUID and sort field

```text
guid():
  match guid_override {
    Some(g) => g.clone(),
    None => {
      let refs: Vec<&str> = fields.iter().map(String::as_str).collect();
      guid_for(&refs)
    }
  }

sort_field():
  match sort_field_override {
    Some(s) => s.as_str(),
    None => fields[model.sort_field_index as usize].as_str()
            // panic/error if index OOB? Prefer debug_assert + fallback to
            // fields[0] or Error - decide at implement time; Python would
            // IndexError. Fallible sort_field() -> Result<&str> is OK if
            // index can be OOB; models default to 0.
  }
```

### 4.7 Front/back card generation

```text
rv = []
for entry in model.req()? :   // ReqEntry { template_ord, kind, field_ords }
  let nonempty = |ord| !fields[ord].is_empty()   // whitespace-only is nonempty
  include = match kind {
    ReqKind::All => field_ords.all(nonempty),
    ReqKind::Any => field_ords.any(nonempty),
  }
  if include { rv.push(Card::new(entry.template_ord as i32)) }
return rv
```

Truthiness: only `""` is empty/falsy. `" "` is truthy (Python non-empty string).

Fixtures (from upstream / Phase 2 models):

| Case | Expect |
| ---- | ------ |
| Chinese: `['中國','中国','China']` | ords `[0, 1]` |
| Chinese: `['你好','','hello']` | ords `[0]` only (simplified empty) |
| Hint: `['capital of California','','Sacramento']` | ord `[0]` (`any` Q or Hint) |
| Hint: `['capital of Iowa','French for "The Moines"','Des Moines']` | ord `[0]` |

### 4.8 Cloze card generation

From Python `Note._cloze_cards` (v0.13.1):

```text
1. qfmt = model.templates[0].qfmt   // first template only
2. cloze field names = unique(
     regex findall r"\{\{[^}]*?cloze:(?:[^}]?:)*(.+?)\}\}" on qfmt
     + regex findall r"<%cloze:(.+?)%>" on qfmt
   )
3. card_ords = empty set
4. for name in cloze field names:
     idx = index of field with that name, or missing
     value = fields[idx] if found else ""
     for m in regex findall r"\{\{c(\d+)::.+?\}\}" on value with DOTALL:
       n = m as int
       if n > 0: card_ords.insert(n - 1)
5. if card_ords is empty: card_ords = {0}
6. return Cards for sorted(card_ords)
```

Notes:

- Hints `{{c1::text::hint}}` are covered by `::.+?` (non-greedy still reaches
  final `}}` correctly for common cases; match Python).
- Newlines inside deletion require DOTALL (`(?s)` or `dot_matches_new_line(true)`).
- Multi-field: union ords across all cloze-named fields.
- Missing template[0]: return error or empty-with-default-0; prefer a clear
  `Error` variant or reuse `Internal` only if tests never hit it. Practical
  cloze models always have >=1 template.

### 4.9 Formatting helpers (optional this phase, used by #6)

```rust
pub(crate) fn format_fields(fields: &[String]) -> String {
    fields.join("\x1f")
}
pub(crate) fn format_tags(tags: &[String]) -> String {
    format!(" {} ", tags.join(" "))
}
```

Unit-test these if landed; otherwise defer entirely to #6.

### 4.10 Error variants to add

```rust
#[error("tag contains a space (U+0020), which is not allowed: {tag:?}")]
TagContainsSpace { tag: String },

#[error(
  "number of fields in model does not match note: model {model_name:?} has \
   {model_fields} fields, note has {note_fields}"
)]
FieldCountMismatch {
  model_name: String,
  model_fields: usize,
  note_fields: usize,
},
```

Keep `#[non_exhaustive]`. Do not add IO/SQL variants yet.

## 5. Test plan (RED -> GREEN per item)

Prefer unit tests in `src/note.rs` / `src/card.rs` for tight cycles; add
`tests/cloze.rs`, `tests/note_validation.rs` (or similar) for crate-root
parity suites. Helper to build simple/Chinese/hint/cloze models inline (do not
depend on Phase 5 builtins).

### T0 - Skeleton compile

**RED/GREEN:** module docs already compile. After first types land, re-export
`Note`/`Card` from `lib.rs` and add a smoke test that paths resolve.

### T1 - `Card` defaults

**RED**

```rust
let c = Card::new(2);
assert_eq!(c.ord, 2);
assert!(!c.suspend);
c.suspend = true;
assert!(c.suspend);
```

**GREEN** - `src/card.rs` struct + `new`.

### T2 - `Note::new` field count

**RED**

- 2-field model + 2 fields -> Ok
- 3-field model + 2 fields -> `FieldCountMismatch`
- 2-field model + 3 fields -> `FieldCountMismatch`
- error `Display` contains counts / name

**GREEN** - fallible `new`, `Error` variant.

### T3 - Tags validation (all mutation paths)

**RED** (mirror `TestTags`)

- `with_tags(["foo","bar"])` ok; `with_tags(["foo","b ar"])` err
- `set_tags` ok / space err
- `add_tag` ok / space err
- `set_tag(0, ...)` ok / space err
- `extend_tags` / `insert_tag` if implemented

**GREEN** - shared `validate_tag` used everywhere.

### T4 - GUID default + override

**RED**

- default `note.guid() == guid_for(&["Capital of Argentina", "Buenos Aires"])`
- `with_guid("custom")` -> `guid() == "custom"`
- `set_guid(None)` restores computed default

**GREEN** - override field + `guid_for` on fields.

### T5 - Sort field default + override

**RED**

- default index 0 -> `fields[0]`
- model `sort_field_index(1)` -> `fields[1]`
- `with_sort_field("x")` overrides

**GREEN**

### T6 - `due` default + set

**RED** - default 0; `with_due(42)` / `set_due` round-trip.

**GREEN**

### T7 - HTML scanner goldens (`find_invalid_html_tags`)

**RED** - port `TestFindInvalidHtmlTagsInField` + latest comment/CDATA:

| Input | Expect |
| ----- | ------ |
| `<h1>` | `[]` |
| ` <h1> ` | `[]` |
| `<h1>test</h1>` | `[]` |
| `<br>`, `<br/>`, `<br />` | `[]` |
| `<h1 style="color: red">STOP</h1>` | `[]` |
| `<TD></Td>` | `[]` |
| ` hello <> goodbye` | `["<>"]` |
| ` hello < > goodbye` | `["< >"]` |
| `<@h1>` | `["<@h1>"]` |
| `<h1@>` | `["<h1@>"]` |
| issue-28 latex fixture | two specific invalid spans (copy exact strings from Python test) |
| `<!-- here is a comment -->` | `[]` |
| `<![CDATA[ here is some cdata ]]>` | `[]` |

**GREEN** - two-pass `regex` implementation in `note.rs` (or small private
module).

### T8 - HTML warn on construct; GUID not scanned

**RED**

- Capturing logger: note with field `Capital of <$> Argentina` emits one warn
  whose text mentions invalid HTML tags and `<$>`.
- Note with clean fields + `with_guid("<@h1>not-a-field")` emits **no** warn.
- Finder still unit-tested independently so logger-less CI stays meaningful.

**GREEN** - call scan from `new`/`set_fields`; never scan guid.

Capturing logger sketch (no extra dev-dep):

```rust
struct Capture;
static LOGS: Mutex<Vec<String>> = ...;
impl log::Log for Capture {
  fn enabled(&self, m: &Metadata) -> bool { m.level() <= Level::Warn }
  fn log(&self, r: &Record) { LOGS.lock().push(format!("{}", r.args())); }
  fn flush(&self) {}
}
// set_logger once; LevelFilter::Warn
```

### T9 - Front/back cards from `req` (Chinese + hint)

**RED** - build models matching `TEST_CN_MODEL` / `TEST_MODEL_WITH_HINT`;
assert ords as in sec. 4.7.

**GREEN** - `_front_back_cards` using `model.req()?`.

### T10 - Cloze suite

**RED** - port `test_cloze.py`:

| Fields (Text, Extra) | Expect ords |
| -------------------- | ----------- |
| `NOTE ONE: {{c1::single deletion}}`, `` | `{0}` |
| three clozes c1/c2/c3 | `[0,1,2]` |
| `{{c1::1st deletion::C1-CLOZE}}` (hint) | `{0}` |
| c1, c2, c1 again | `[0,1]` |
| multi-field model c1..c4 across two fields | `[0,1,2,3]` |
| only c2 and c3 | `[1,2]` |
| newlines inside `{{c2::the\nUnited States\nof America}}` | `[0,1]` |
| no cloze markers at all | `{0}` (default) |

**GREEN** - cloze regex path + sorted ords.

### T11 - Suspend on cached cards

**RED**

```rust
let mut note = Note::new(cn_model, ["中國", "中国", "China"])?;
assert_eq!(note.cards()?.len(), 2);
note.cards_mut()?[1].suspend = true;
assert!(note.cards()?[1].suspend);
// field change invalidates and clears custom suspend (document this)
note.set_fields(["中國", "中国", "China"])?; // same content still invalidates
assert!(!note.cards()?[1].suspend); // recomputed fresh
```

**GREEN** - cache + invalidate on `set_fields`.

### T12 - Crate-root re-exports + formatting helpers (if landed)

**RED/GREEN** - `use genanki::{Note, Card};` smoke; optional
`format_fields` / `format_tags` asserts (`\x1f`, leading/trailing space).

### T13 - Full workspace gates

```text
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo test --doc
```

## 6. Implementation order (fine-grained cycles)

Do not batch large green steps. Suggested sequence:

1. **Deps** - add `regex`, `log` to `Cargo.toml` (empty use is ok briefly; clippy
   deny unused will force wiring quickly).
2. **Errors** - `TagContainsSpace`, `FieldCountMismatch` + display tests (RED/GREEN).
3. **Card** - T1.
4. **Note::new + fields + Arc model** - T2; HTML/tags deferred (empty tags, no
   warn yet).
5. **Tags** - T3.
6. **guid / sort_field / due** - T4 T5 T6.
7. **HTML finder** - T7 (pure function, no Note wiring).
8. **HTML warn wiring** - T8.
9. **Front/back cards** - T9.
10. **Cloze cards** - T10.
11. **Cache + suspend + invalidate** - T11.
12. **Re-exports + helpers + gates** - T12 T13.
13. **Refactor** only while green (dedupe model fixtures, private modules,
    regex `LazyLock`/`OnceLock` for compiled patterns).

### Regex compilation

Prefer process-wide compiled patterns:

```rust
use std::sync::OnceLock;
fn tag_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?s)<.*?>").unwrap())
}
```

Same for cloze field-name and `{{cN::...}}` patterns. `unwrap` on known-good
literals is acceptable; avoid silently ignoring compile failure.

## 7. File touch list

| Path | Action |
| ---- | ------ |
| `Cargo.toml` | add `regex`, `log` |
| `src/error.rs` | new variants + tests |
| `src/card.rs` | `Card` type |
| `src/note.rs` | `Note`, HTML finder, card gen, tests |
| `src/lib.rs` | re-export `Note`, `Card`; module docs if needed |
| `tests/cloze.rs` | optional integration suite |
| `tests/note_validation.rs` | optional integration suite |
| `tests/card_req.rs` | optional front/back req card suite |
| `doc/plans/issue-5.md` | this plan (status -> IMPLEMENTED when done) |

No changes to `model.rs` / `req.rs` expected unless a tiny accessor is needed
(fields/templates already `pub`).

## 8. Acceptance mapping

| Acceptance (issue #5) | Tests |
| --------------------- | ----- |
| Tag space -> error on all mutation paths | T3 |
| Field count mismatch -> error | T2 |
| Cloze suite parity | T10 |
| Front/back empty required suppresses card (Chinese) | T9 |
| Hint model generates card when Q or Hint present | T9 |
| HTML scanner goldens (ok, br, comments, CDATA, invalid) | T7 |
| GUID override does not trigger HTML warnings | T8 |
| Suspend support | T11 |
| Default + override GUID / sort field | T4 T5 |

## 9. Phase 4 handoff notes

Leave ready for #6 without implementing it:

- `Note` exposes `guid()`, `sort_field()`, `fields()`, `tags()`, `due()`,
  `model()`, `cards_mut()` / `cards()`
- `Card { ord, suspend }` ready for `queue` mapping
- Optional `format_fields` / `format_tags` match Python DB strings
- HTML already warned at construct; Phase 4 write need not re-warn (may
  re-scan if desired for parity with Python `write_to_db` timing - optional)
- Field count already enforced; write can assume valid notes or re-check
  defensively

## 10. Open questions - RESOLVED

| Question | Resolution |
| -------- | ---------- |
| Plan path | `genanki-rs/doc/plans/issue-5.md` (not lit) |
| `regex` vs hand-roll | Add `regex`; HTML via two-pass (no `fancy-regex`) |
| warn mechanism | `log` facade + public finder |
| field count timing | fallible `Note::new` |
| model ownership | `Arc<Model>` + `Into<Arc<Model>>` |
| cards/suspend | lazy cache, `cards`/`cards_mut`, invalidate on field change |

No unresolved blockers. Implement on branch `issue/5-note-card` with strict TDD.

## 11. PR checklist (when implementing)

- [ ] Every work item went RED before GREEN
- [ ] Only `regex` + `log` added (plus existing deps)
- [ ] No `write_to_db` / Package / Deck feature creep
- [ ] Cloze default ord 0 when empty
- [ ] HTML comment + CDATA goldens pass
- [ ] GUID not scanned for HTML
- [ ] `cargo fmt`, `clippy -D warnings`, `test`, `doc` green
- [ ] Issue #5 checkboxes updated in PR description
- [ ] Plan status -> IMPLEMENTED + PR link
