# Issue #4: Phase 2 - Model + req (builders, Mustache subset, model JSON)

Status: PLAN
Issue: https://github.com/tlkahn/genanki-rs/issues/4
Parent epic: https://github.com/tlkahn/genanki-rs/issues/1
Branch: `issue/4-model-req`
Method: strict fine-grained TDD (RED -> GREEN -> refactor) per work item below

## 1. Goal

Land note-type definitions, required-field (`req`) computation, and model JSON
serialization. No Note/Card/Deck/Package behavior.

After this phase:

- `Field`, `Template`, `Model`, `ModelType` live in `src/model.rs`
- Consuming builder API matches the epic sketch (`Model::new` + `.field` /
  `.template` / `.css` / ...)
- Defaults match Python genanki 0.13.x (`css`, `FRONT_BACK`, latex pre/post,
  `sort_field_index = 0`, field/template setdefault keys)
- Hand-rolled Mustache **subset** in `src/req.rs` sufficient for `req`
- `req` fixtures (simple / Chinese / hint) match exactly
- Filter tags (`cloze:`, `type:`, ...) resolve to the field name (epic semantics;
  documented divergence from Python/chevron)
- `Model::to_json(timestamp_secs, deck_id) -> Result<serde_json::Value>` shape
  matches Python `Model.to_json` keys
- `ModelType::Cloze` serializes `"type": 1`
- Unit tests green; `cargo fmt` / `clippy -D warnings` / `test` / `doc` green

### Out of scope

- Note / Card / tag validation / cloze card generation (Phase 3 / #5)
- Deck / Package / sqlite / zip / media (Phase 4 / #6)
- Builtin model constants (Phase 5 / #7) - may *exercise* builders in tests, but
  do not land `BASIC_MODEL` etc. as public constants here
- YAML field/template input (Python nicety; epic non-goal)
- Full Mustache at study-time fidelity (Anki renders; we only need `req`)
- `rusqlite`, `zip`, `regex`, Mustache crates

## 2. Current state (code-verified)

| Item | Status |
| ---- | ------ |
| Phase 0 scaffolding on `main` | Done (#2 / PR #9) |
| Phase 1 GUID + primitives on `main` | Done (#3 / PR #10) |
| `src/model.rs` | Stub module doc only |
| `src/req.rs` | Stub module doc only (doc says "Phase 4" - fix to Phase 2) |
| `src/error.rs` | `thiserror`, `Error::Internal` only, `#[non_exhaustive]` |
| `src/lib.rs` | Re-exports `Error`, `Result`, `guid_for` only |
| `Cargo.toml` deps | `sha2`, `thiserror` |
| Python reference | genanki **v0.13.1** (`model.py` `Model`, `_req`, `to_json`) |

## 3. Locked decisions

| Topic | Decision | Rationale |
| ----- | -------- | --------- |
| External deps this phase | **`serde` + `serde_json` only** (in addition to existing `sha2`, `thiserror`) | Nested model JSON with `null`, bools, mixed arrays. Needed again in #6 for `col.models` / `col.decks`. Confirmed with maintainer. |
| Mustache implementation | **Hand-rolled subset in `src/req.rs`** - no `ramhorns` / `upon` / `chevron`-alike | Epic allows custom; keeps deps minimal; surface area for `req` is small. |
| `regex` | **No** | Tag scan is a simple character walker / splitter. |
| Filter tags in templates | **Strip filter prefixes; resolve to field name** | Epic acceptance. Python/chevron looks up the literal key (`cloze:Text`), which makes CLOZE `req` quirky (`[0,1]` both "required"). We deliberately diverge; document + test. |
| Builder style | **Consuming fluent chain** returning `Self` | Epic public API sketch. |
| `to_json` API | `pub fn to_json(&self, timestamp_secs: i64, deck_id: i64) -> Result<serde_json::Value>` | Matches Python `(timestamp, deck_id)`; `Value` is easy to assert in tests and reuse when writing `col.models`. |
| `req` API | `pub fn req(&self) -> Result<Vec<ReqEntry>>` (name flexible) used by `to_json` | Testable without full JSON; mirrors Python `cached_property _req` without lazy-cache complexity (compute on call; cheap). |
| Caching `_req` | **No cache** in v1 | Templates are small; avoid interior mutability. Recompute each call. |
| YAML constructors | **Not supported** | Epic non-goal. |
| Field/Template defaults | Applied at **construction** (`Field::new` / `Template::new`), not mutated during `to_json` | Idiomatic Rust; JSON always sees complete objects. Same observable keys as Python `setdefault`. |
| `Model` validation at build time | **No hard fail** if zero fields/templates at `new`; `req`/`to_json` may error later if a template cannot compute req | Match Python looseness; keep builder flexible. |
| Public re-exports | `Model`, `ModelType`, `Field`, `Template` (and `ReqEntry` / `ReqKind` if public) from crate root | Epic API sketch. |
| Error growth | Add at least one concrete variant for req failure (replace reliance on `Internal` for this path) | Issue acceptance: error if template has no detectable required fields. |
| TDD discipline | Every behavior change: failing test first, minimal impl, refactor. One logical assertion group per cycle when practical. | Per user request. |

### External deps confirmation (explicit)

| Crate | Required? | Why |
| ----- | --------- | --- |
| `serde` + `serde_json` | **Yes (chosen)** | Model JSON for `col.models` entries. Hand-building JSON strings was rejected as brittle (`null` `did`, bools, key order less important but types matter). |
| Mustache crate | **No** | Custom subset only. |
| `regex` | **No** | Not needed for the Mustache subset or `req`. |
| Anything else new | **No** | Builders and defaults are pure Rust. |

If a future review wants zero new deps, the only alternative is a tiny internal `Json` enum + manual stringify. Not planned unless requested.

### Filter / Python divergence (explicit)

| Template `qfmt` | Python genanki 0.13.x `req` (chevron literal keys) | **Our** `req` (filters resolve) |
| --------------- | -------------------------------------------------- | -------------------------------- |
| `{{AField}}` | `[[0,"all",[0]]]` | same |
| Chinese dual | `[[0,"all",[0]],[1,"all",[1]]]` | same |
| Hint section | `[[0,"any",[0,1]]]` | same |
| `{{cloze:Text}}` + field `Back Extra` | `[[0,"all",[0,1]]]` (both; cloze key never hits `Text`) | `[[0,"all",[0]]]` (`Text` only) |
| `{{Front}}\\n\\n{{type:Back}}` | `[[0,"all",[0]]]` | `[[0,"any",[0,1]]]` (see note) |

> Correction (verified against genanki 0.13.1): the `type:Back` row above was
> mispredicted. With filter resolution, blanking Front still renders Back's
> sentinel (via `{{type:Back}}`), so no field is "all"; both fields land in
> the "any" fallback => `[[0,"any",[0,1]]]`. Python yields `[[0,"all",[0]]]`
> only because chevron looks up the literal key `type:Back` (missing -> empty).
> Keep `req_type_in_the_answer_front` asserting `Any [0,1]`.

Upstream fixtures in issue #4 are the first three rows only - those must match **exactly**. Cloze/type rows are extra unit tests documenting our semantics.

## 4. Algorithm reference (must match Python for non-filter cases)

### 4.1 `req` (from `genanki/model.py` `Model._req`)

```text
sentinel = "SeNtInEl"
field_names = [f.name for f in fields]
req = []

for template_ord, template in enumerate(templates):
    # --- strategy "all" ---
    required = []
    for field_ord, name in enumerate(field_names):
        values = {n: sentinel for n in field_names}
        values[name] = ""
        rendered = mustache_render(template.qfmt, values)
        if sentinel not in rendered:
            required.append(field_ord)

    if required:
        req.append([template_ord, "all", required])
        continue

    # --- strategy "any" ---
    required = []
    for field_ord, name in enumerate(field_names):
        values = {n: "" for n in field_names}
        values[name] = sentinel
        rendered = mustache_render(template.qfmt, values)
        if sentinel in rendered:
            required.append(field_ord)

    if not required:
        return Error::TemplateReq { ... }  // include qfmt or template name

    req.append([template_ord, "any", required])

return req
```

Notes:

- Only **`qfmt`** is rendered (not `afmt`).
- Truthiness for sections: non-empty string => truthy; `""` => falsy (Python/chevron for plain strings).
- Field order in `required` follows field ordinal ascending (iteration order).
- Do not special-case cloze model type inside `req`; cloze card ords are Phase 3.

### 4.2 Mustache subset (for `req` only)

Support:

| Syntax | Behavior |
| ------ | -------- |
| `{{name}}` | Interpolate field value (no HTML escape required for `req`; raw copy is fine because sentinel is ASCII) |
| `{{{name}}}` / `{{&name}}` | Same as interpolate for our purposes |
| `{{#name}}...{{/name}}` | Section: if field non-empty, render interior; else skip |
| `{{^name}}...{{/name}}` | Inverted: if field empty, render interior; else skip |
| `{{! ... }}` | Comment: emit nothing |
| Whitespace inside tags | `{{ name }}`, `{{# name }}` trimmed |
| Nested sections | Stack-based; supported |
| Filters | See 4.3 |

Out of scope (emit empty / skip gracefully, do not error):

- Partials `{{> name}}`
- Lists / object context / lambdas
- Delimiter change `{{= =}}`
- Anki-only runtime tokens like `{{FrontSide}}` as magic (look up as ordinary key; absent => empty). Front sides rarely put `FrontSide` in `qfmt`.

Parser approach (suggested, not mandatory):

1. Scan for `{{` ... `}}` (and `{{{` ... `}}}`).
2. Classify tag by first char after trim: `#` section, `^` inverted, `/` close, `!` comment, `&` unescaped, else name.
3. Recursive render of section bodies with the same field map (string-only context).
4. Mismatched close tags: best-effort or error - prefer **hard error** via `Result` only if we surface parse errors; for parity with chevron's leniency, unmatched closings can render as empty. **Decision for implementer:** keep the subset strict enough that the three fixtures + filter tests pass; add a parse-error variant only if tests need it. Prefer not to silently loop forever on bad input.

### 4.3 Filter resolution

When resolving a tag name `raw` against the field map:

```text
1. trim whitespace
2. if map contains exact key raw -> use it
3. else if raw contains ':':
      // Anki filter chain: filter[:filter]*:FieldName
      field = substring after the *last* ':'
      if map contains field -> use it
4. else -> missing (empty string)
```

Examples: `cloze:Text` -> `Text`; `type:Back` -> `Back`; `furigana:Reading` -> `Reading`;
`hint:cloze:Text` -> `Text`.

Exact key wins first so a pathological field literally named `cloze:Text` still works.

### 4.4 Defaults (Python parity)

**Model**

| Field | Default |
| ----- | ------- |
| `css` | `""` |
| `model_type` | `ModelType::FrontBack` (= 0) |
| `latex_pre` | `DEFAULT_LATEX_PRE` (see below) |
| `latex_post` | `DEFAULT_LATEX_POST` = `"\\end{document}"` |
| `sort_field_index` | `0` |

`DEFAULT_LATEX_PRE` (exact bytes from `model.py`):

```text
\documentclass[12pt]{article}
\special{papersize=3in,5in}
\usepackage[utf8]{inputenc}
\usepackage{amssymb,amsmath}
\pagestyle{empty}
\setlength{\parindent}{0in}
\begin{document}
```

(String ends with newline after `\begin{document}` as in Python concatenation.)

**Field** (`Field::new(name)`)

| Key | Default |
| --- | ------- |
| `font` | `"Liberation Sans"` |
| `media` | `[]` |
| `rtl` | `false` |
| `size` | `20` |
| `sticky` | `false` |
| `ord` | assigned at `to_json` time (0..n-1) |

**Template** (`Template::new(name, qfmt, afmt)`)

| Key | Default |
| --- | ------- |
| `bqfmt` | `""` |
| `bafmt` | `""` |
| `bfont` | `""` |
| `bsize` | `0` |
| `did` | `None` (-> JSON `null`) |
| `ord` | assigned at `to_json` time |

### 4.5 `to_json` object shape

```json
{
  "css": "<string>",
  "did": <deck_id number>,
  "flds": [ { "name", "ord", "font", "media", "rtl", "size", "sticky" }, ... ],
  "id": "<model_id as decimal string>",
  "latexPost": "<string>",
  "latexPre": "<string>",
  "latexsvg": false,
  "mod": <timestamp_secs as number>,
  "name": "<string>",
  "req": [ [tmpl_idx, "all"|"any", [field_ords...]], ... ],
  "sortf": <sort_field_index>,
  "tags": [],
  "tmpls": [ { "name", "qfmt", "afmt", "ord", "bafmt", "bqfmt", "bfont", "bsize", "did": null }, ... ],
  "type": 0 | 1,
  "usn": -1,
  "vers": []
}
```

Serde notes:

- `id` is a **string**, not a number (Python `str(self.model_id)`).
- Template `did` is JSON `null` (Python `None`).
- `req` is a heterogeneous array: use `serde_json::json!` or a small custom
  `Serialize` on `ReqEntry`.
- Key names are camelCase where Python uses camelCase (`latexPost`, `latexPre`,
  `latexsvg`, `sortf`).
- Do not add extra keys. Do not omit defaults.

## 5. Target API surface after Phase 2

```rust
// src/model.rs
/// Default LaTeX preamble (Python `Model.DEFAULT_LATEX_PRE`).
pub const DEFAULT_LATEX_PRE: &str = "...";

/// Default LaTeX postamble (Python `Model.DEFAULT_LATEX_POST`).
pub const DEFAULT_LATEX_POST: &str = "\\end{document}";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ModelType {
    FrontBack = 0,
    Cloze = 1,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field { /* name, font, media, rtl, size, sticky */ }

impl Field {
    pub fn new(name: impl Into<String>) -> Self { ... }
    pub fn font(self, font: impl Into<String>) -> Self { ... }
    pub fn media(self, media: Vec<String>) -> Self { ... }
    pub fn rtl(self, rtl: bool) -> Self { ... }
    pub fn size(self, size: u32) -> Self { ... }
    pub fn sticky(self, sticky: bool) -> Self { ... }
    // getters or pub fields - prefer pub fields for simple data, or pub getters
}

#[derive(Debug, Clone, PartialEq)]
pub struct Template { /* name, qfmt, afmt, bqfmt, bafmt, bfont, bsize, did */ }

impl Template {
    pub fn new(name: impl Into<String>, qfmt: impl Into<String>, afmt: impl Into<String>) -> Self { ... }
    pub fn bqfmt(self, v: impl Into<String>) -> Self { ... }
    pub fn bafmt(self, v: impl Into<String>) -> Self { ... }
    pub fn bfont(self, v: impl Into<String>) -> Self { ... }
    pub fn bsize(self, v: u32) -> Self { ... }
    pub fn did(self, did: Option<i64>) -> Self { ... }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Model { /* id, name, fields, templates, css, model_type, latex_*, sort_field_index */ }

impl Model {
    pub fn new(id: i64, name: impl Into<String>) -> Self { ... }
    pub fn field(self, field: Field) -> Self { ... }
    pub fn template(self, template: Template) -> Self { ... }
    pub fn css(self, css: impl Into<String>) -> Self { ... }
    pub fn model_type(self, t: ModelType) -> Self { ... }
    pub fn latex_pre(self, s: impl Into<String>) -> Self { ... }
    pub fn latex_post(self, s: impl Into<String>) -> Self { ... }
    pub fn sort_field_index(self, idx: i32) -> Self { ... }

    /// Compute Anki `req` for each template.
    pub fn req(&self) -> crate::Result<Vec<ReqEntry>> { ... }

    /// Serialize to the object stored under `col.models[model_id]`.
    pub fn to_json(&self, timestamp_secs: i64, deck_id: i64) -> crate::Result<serde_json::Value> { ... }
}

// src/req.rs
/// One template's required-field entry: `[tmpl_idx, "all"|"any", [field_ords...]]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReqEntry {
    pub template_ord: u32,
    pub kind: ReqKind,
    pub field_ords: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReqKind {
    All,
    Any,
}

impl ReqKind {
    pub fn as_str(self) -> &'static str { match self { All => "all", Any => "any" } }
}

/// Render `template` with string field values (Mustache subset). `pub(crate)` is enough
/// if only model/req tests need it; `pub` is OK for advanced callers.
pub fn render(template: &str, fields: &std::collections::BTreeMap<&str, &str>) -> String { ... }
// Prefer BTreeMap for deterministic tests; HashMap also fine if tests don't depend on iter order.
// render itself only does lookups.

// src/error.rs additions
#[error("could not compute required fields for template qfmt: {qfmt}")]
TemplateReq { qfmt: String },
// optional:
// #[error("mustache template error: {0}")]
// Mustache(String),

// src/lib.rs additions
pub use crate::model::{Field, Model, ModelType, Template};
// optionally ReqEntry, ReqKind, DEFAULT_LATEX_*
```

Ownership notes for later phases (do not implement Note yet): `Model` should be
`Clone` so Phase 3 can choose `Arc<Model>` or owned clone freely.

## 6. TDD plan (fine-grained cycles)

Work in this order. Each cycle: **RED** -> **GREEN** -> **refactor**.
Do not implement multiple features ahead of their tests.

### Cycle 0 - Branch + dep pins

1. Branch `issue/4-model-req` from latest `main`.
2. Add to `Cargo.toml`:

   ```toml
   [dependencies]
   sha2 = "0.10"
   thiserror = "2"
   serde = { version = "1", features = ["derive"] }
   serde_json = "1"
   ```

3. No behavior yet. Proceed immediately to Cycle 1 so deps are used the same session.

### Cycle 1 - `Error::TemplateReq`

**RED**

```rust
#[test]
fn template_req_error_display() {
    let err = Error::TemplateReq { qfmt: "{{Nope}}".into() };
    let s = err.to_string();
    assert!(s.contains("required fields"));
    assert!(s.contains("{{Nope}}"));
}
```

**GREEN** - add variant with `thiserror` message; keep `Internal`; keep `#[non_exhaustive]`.

**Verify:** `cargo test error::`

### Cycle 2 - `ModelType`

**RED**

```rust
#[test]
fn model_type_discriminants() {
    assert_eq!(ModelType::FrontBack as u8, 0);
    assert_eq!(ModelType::Cloze as u8, 1);
}
```

**GREEN** - enum in `model.rs` with `#[repr(u8)]`, derives.

### Cycle 3 - `Field::new` defaults + fluent overrides

**RED**

```rust
#[test]
fn field_new_defaults() {
    let f = Field::new("Question");
    assert_eq!(f.name, "Question");
    assert_eq!(f.font, "Liberation Sans");
    assert!(f.media.is_empty());
    assert!(!f.rtl);
    assert_eq!(f.size, 20);
    assert!(!f.sticky);
}

#[test]
fn field_fluent_overrides() {
    let f = Field::new("Q").font("Arial").size(22).rtl(true).sticky(true).media(vec!["a.mp3".into()]);
    assert_eq!(f.font, "Arial");
    assert_eq!(f.size, 22);
    assert!(f.rtl && f.sticky);
    assert_eq!(f.media, vec!["a.mp3"]);
}
```

**GREEN** - struct + builders. Public fields OK (simple data).

### Cycle 4 - `Template::new` defaults + fluent overrides

**RED** - analogous tests for `name`, `qfmt`, `afmt`, empty `bqfmt`/`bafmt`/`bfont`, `bsize == 0`, `did == None`.

**GREEN** - struct + builders.

### Cycle 5 - `Model::new` defaults + consuming chain

**RED**

```rust
#[test]
fn model_new_defaults() {
    let m = Model::new(1607392319, "Simple Model");
    assert_eq!(m.id, 1607392319);
    assert_eq!(m.name, "Simple Model");
    assert!(m.fields.is_empty());
    assert!(m.templates.is_empty());
    assert_eq!(m.css, "");
    assert_eq!(m.model_type, ModelType::FrontBack);
    assert_eq!(m.latex_pre, DEFAULT_LATEX_PRE);
    assert_eq!(m.latex_post, DEFAULT_LATEX_POST);
    assert_eq!(m.sort_field_index, 0);
}

#[test]
fn model_builder_chain() {
    let m = Model::new(1, "M")
        .field(Field::new("Q"))
        .field(Field::new("A"))
        .template(Template::new("Card 1", "{{Q}}", "{{A}}"))
        .css(".card{}")
        .model_type(ModelType::FrontBack)
        .sort_field_index(1)
        .latex_pre("PRE")
        .latex_post("POST");
    assert_eq!(m.fields.len(), 2);
    assert_eq!(m.templates.len(), 1);
    assert_eq!(m.css, ".card{}");
    assert_eq!(m.sort_field_index, 1);
    assert_eq!(m.latex_pre, "PRE");
    assert_eq!(m.latex_post, "POST");
}

#[test]
fn default_latex_pre_matches_python() {
    assert!(DEFAULT_LATEX_PRE.contains(r"\documentclass[12pt]{article}"));
    assert!(DEFAULT_LATEX_PRE.contains(r"\usepackage{amssymb,amsmath}"));
    assert!(DEFAULT_LATEX_PRE.ends_with(r"\begin{document}
") || DEFAULT_LATEX_PRE.ends_with("\\begin{document}\n"));
}
```

Pin the **full exact string** equality against the Python constant in at least one test (copy the exact bytes into the assert expected side, or assert `DEFAULT_LATEX_PRE == "...full..."`).

**GREEN** - `Model` + constants + chain methods.

### Cycle 6 - Mustache: plain interpolation

Put unit tests in `src/req.rs`. Start with `render` only.

**RED**

```rust
#[test]
fn render_interpolates_field() {
    let mut fields = BTreeMap::new();
    fields.insert("AField", "SeNtInEl");
    fields.insert("BField", "");
    assert_eq!(render("{{AField}}", &fields), "SeNtInEl");
}

#[test]
fn render_missing_field_is_empty() {
    let fields = BTreeMap::new();
    assert_eq!(render("{{Nope}}", &fields), "");
}

#[test]
fn render_preserves_surrounding_text() {
    let mut fields = BTreeMap::new();
    fields.insert("Q", "x");
    assert_eq!(render("pre-{{Q}}-post", &fields), "pre-x-post");
}
```

**GREEN** - minimal scanner that only handles `{{name}}`.

### Cycle 7 - Mustache: whitespace trim + unescaped forms

**RED**

```rust
#[test]
fn render_trims_tag_whitespace() {
    let mut fields = BTreeMap::new();
    fields.insert("Q", "x");
    assert_eq!(render("{{ Q }}", &fields), "x");
}

#[test]
fn render_triple_and_amp_unescaped() {
    let mut fields = BTreeMap::new();
    fields.insert("Q", "x");
    assert_eq!(render("{{{Q}}}", &fields), "x");
    assert_eq!(render("{{&Q}}", &fields), "x");
}
```

**GREEN** - extend parser.

### Cycle 8 - Mustache: comments

**RED**

```rust
#[test]
fn render_strips_comments() {
    let mut fields = BTreeMap::new();
    fields.insert("Q", "x");
    assert_eq!(render("{{! ignore me }}{{Q}}", &fields), "x");
}
```

**GREEN**

### Cycle 9 - Mustache: sections + inverted + nesting

**RED**

```rust
#[test]
fn render_section_truthy() {
    let mut fields = BTreeMap::new();
    fields.insert("Hint", "h");
    assert_eq!(render("{{#Hint}}H:{{Hint}}{{/Hint}}", &fields), "H:h");
}

#[test]
fn render_section_falsy_skips() {
    let mut fields = BTreeMap::new();
    fields.insert("Hint", "");
    assert_eq!(render("{{#Hint}}H:{{Hint}}{{/Hint}}X", &fields), "X");
}

#[test]
fn render_inverted() {
    let mut fields = BTreeMap::new();
    fields.insert("A", "");
    assert_eq!(render("{{^A}}no{{/A}}", &fields), "no");
    fields.insert("A", "yes");
    assert_eq!(render("{{^A}}no{{/A}}", &fields), "");
}

#[test]
fn render_nested_sections() {
    let mut fields = BTreeMap::new();
    fields.insert("A", "1");
    fields.insert("B", "2");
    assert_eq!(render("{{#A}}{{#B}}{{A}}{{B}}{{/B}}{{/A}}", &fields), "12");
}
```

**GREEN** - stack-based section rendering. Refactor parser into clear tokenize/render helpers if messy.

### Cycle 10 - Mustache: filter resolution

**RED**

```rust
#[test]
fn render_cloze_filter_resolves_to_field() {
    let mut fields = BTreeMap::new();
    fields.insert("Text", "SeNtInEl");
    fields.insert("Back Extra", "");
    assert_eq!(render("{{cloze:Text}}", &fields), "SeNtInEl");
}

#[test]
fn render_type_filter_resolves_to_field() {
    let mut fields = BTreeMap::new();
    fields.insert("Front", "F");
    fields.insert("Back", "B");
    assert_eq!(render("{{Front}} {{type:Back}}", &fields), "F B");
}

#[test]
fn render_exact_key_wins_over_filter_strip() {
    let mut fields = BTreeMap::new();
    fields.insert("cloze:Text", "LITERAL");
    fields.insert("Text", "FIELD");
    assert_eq!(render("{{cloze:Text}}", &fields), "LITERAL");
}
```

**GREEN** - lookup helper from section 4.3.

### Cycle 11 - `req` fixture: simple (`TEST_MODEL`)

**RED**

```rust
#[test]
fn req_simple_all() {
    let m = Model::new(234567, "foomodel")
        .field(Field::new("AField"))
        .field(Field::new("BField"))
        .template(Template::new(
            "card1",
            "{{AField}}",
            "{{FrontSide}}<hr id=\"answer\">{{BField}}",
        ));
    let req = m.req().unwrap();
    assert_eq!(req, vec![ReqEntry { template_ord: 0, kind: ReqKind::All, field_ords: vec![0] }]);
}
```

**GREEN** - implement `Model::req` on top of `render` (logic may live in `req.rs` as `pub fn compute_req(fields, templates) -> Result<Vec<ReqEntry>>` called by `Model`).

Prefer keeping pure functions in `req.rs` and thin wrappers on `Model` for testability.

### Cycle 12 - `req` fixture: Chinese dual templates

**RED** - build `TEST_CN_MODEL`; assert

```text
[
  ReqEntry { 0, All, [0] },
  ReqEntry { 1, All, [1] },
]
```

**GREEN** - should already pass if Cycle 11 algorithm loops templates correctly; fix if not.

### Cycle 13 - `req` fixture: optional hint (`any`)

**RED** - build `TEST_MODEL_WITH_HINT` with

```text
qfmt = "{{Question}}{{#Hint}}<br>Hint: {{Hint}}{{/Hint}}"
```

Assert `[[0, "any", [0, 1]]]`.

**GREEN** - exercises section falsy path + "any" fallback.

### Cycle 14 - `req` error when nothing required

**RED**

> Correction (verified against genanki 0.13.1): `"static only"` with one
> field does **not** error - Python returns `[[0, "all", [0]]]` because the
> "all" strategy blanks each field in turn and the never-present sentinel
> makes every field "required". The `Error::TemplateReq` path is only
> reachable with **zero fields** (neither strategy loop can run). The original
> test below was replaced accordingly.

```rust
#[test]
fn req_static_only_with_fields_requires_all() {
    let m = Model::new(1, "x")
        .field(Field::new("Q"))
        .template(Template::new("c", "static only", ""));
    let req = m.req().unwrap();
    assert_eq!(req, vec![ReqEntry { template_ord: 0, kind: ReqKind::All, field_ords: vec![0] }]);
}

#[test]
fn req_errors_when_model_has_no_fields() {
    let m = Model::new(1, "x").template(Template::new("c", "static only", ""));
    let err = m.req().unwrap_err();
    match err {
        Error::TemplateReq { qfmt } => assert!(qfmt.contains("static only")),
        other => panic!("unexpected {other:?}"),
    }
}
```

**GREEN** - map the Python `Exception` path to `Error::TemplateReq`.

### Cycle 15 - `req` filter semantics (documented divergence)

**RED**

```rust
#[test]
fn req_cloze_filter_requires_text_only() {
    let m = Model::new(1, "c")
        .model_type(ModelType::Cloze)
        .field(Field::new("Text"))
        .field(Field::new("Back Extra"))
        .template(Template::new("Cloze", "{{cloze:Text}}", "{{cloze:Text}}<br>{{Back Extra}}"));
    let req = m.req().unwrap();
    assert_eq!(req, vec![ReqEntry { template_ord: 0, kind: ReqKind::All, field_ords: vec![0] }]);
    // NOTE: Python genanki 0.13.x yields field_ords [0, 1] because chevron does not strip filters.
}

#[test]
fn req_type_in_the_answer_front() {
    // With filter resolution, `type:Back` carries Back's value on the front,
    // so blanking Front still renders Back's sentinel: no "all" field. Both
    // fields fall to the "any" strategy (either one provides content).
    // NOTE: Python genanki 0.13.x yields [[0, "all", [0]]] because chevron
    // looks up the literal key `type:Back` (missing -> empty).
    let m = Model::new(1, "t")
        .field(Field::new("Front"))
        .field(Field::new("Back"))
        .template(Template::new("c", "{{Front}}\n\n{{type:Back}}", "x"));
    let req = m.req().unwrap();
    assert_eq!(req, vec![ReqEntry { template_ord: 0, kind: ReqKind::Any, field_ords: vec![0, 1] }]);
}
```

**GREEN** - should pass via Cycle 10 lookup; if not, fix resolution.

### Cycle 16 - `to_json` shape (front/back simple model)

**RED**

```rust
#[test]
fn to_json_simple_model_shape() {
    let m = Model::new(234567, "foomodel")
        .field(Field::new("AField"))
        .field(Field::new("BField"))
        .template(Template::new("card1", "{{AField}}", "{{BField}}"));
    let v = m.to_json(1_600_000_000, 123456).unwrap();

    assert_eq!(v["id"], "234567"); // string
    assert_eq!(v["name"], "foomodel");
    assert_eq!(v["did"], 123456);
    assert_eq!(v["mod"], 1_600_000_000);
    assert_eq!(v["type"], 0);
    assert_eq!(v["usn"], -1);
    assert_eq!(v["latexsvg"], false);
    assert_eq!(v["tags"], json!([]));
    assert_eq!(v["vers"], json!([]));
    assert_eq!(v["sortf"], 0);
    assert_eq!(v["css"], "");
    assert_eq!(v["latexPre"], DEFAULT_LATEX_PRE);
    assert_eq!(v["latexPost"], DEFAULT_LATEX_POST);
    assert_eq!(v["req"], json!([[0, "all", [0]]]));

    assert_eq!(v["flds"][0]["name"], "AField");
    assert_eq!(v["flds"][0]["ord"], 0);
    assert_eq!(v["flds"][0]["font"], "Liberation Sans");
    assert_eq!(v["flds"][0]["media"], json!([]));
    assert_eq!(v["flds"][0]["rtl"], false);
    assert_eq!(v["flds"][0]["size"], 20);
    assert_eq!(v["flds"][0]["sticky"], false);
    assert_eq!(v["flds"][1]["ord"], 1);

    assert_eq!(v["tmpls"][0]["name"], "card1");
    assert_eq!(v["tmpls"][0]["ord"], 0);
    assert_eq!(v["tmpls"][0]["qfmt"], "{{AField}}");
    assert_eq!(v["tmpls"][0]["afmt"], "{{BField}}");
    assert_eq!(v["tmpls"][0]["bafmt"], "");
    assert_eq!(v["tmpls"][0]["bqfmt"], "");
    assert_eq!(v["tmpls"][0]["bfont"], "");
    assert_eq!(v["tmpls"][0]["bsize"], 0);
    assert!(v["tmpls"][0]["did"].is_null());
}
```

**GREEN** - implement `to_json` via `serde_json::json!` or typed structs with `Serialize`. Keep key set exact.

Suggested internal approach (either is fine):

1. Build `serde_json::Value` with `json!` macro (fast to write, good for tests), or
2. Private `ModelJson` / `FieldJson` / `TemplateJson` structs with `#[serde(rename = ...)]`.

Prefer (2) if it stays readable; (1) is OK for v1.

### Cycle 17 - `to_json` cloze type + custom latex + sortf

**RED**

```rust
#[test]
fn to_json_cloze_type_is_one() {
    let m = Model::new(9, "c")
        .model_type(ModelType::Cloze)
        .field(Field::new("Text"))
        .field(Field::new("Back Extra"))
        .template(Template::new("Cloze", "{{cloze:Text}}", "{{cloze:Text}}"));
    let v = m.to_json(0, 1).unwrap();
    assert_eq!(v["type"], 1);
    assert_eq!(v["req"], json!([[0, "all", [0]]]));
}

#[test]
fn to_json_custom_latex_and_sortf() {
    let m = Model::new(1, "x")
        .field(Field::new("A"))
        .template(Template::new("c", "{{A}}", ""))
        .latex_pre("PRE")
        .latex_post("POST")
        .sort_field_index(1);
    let v = m.to_json(0, 1).unwrap();
    assert_eq!(v["latexPre"], "PRE");
    assert_eq!(v["latexPost"], "POST");
    assert_eq!(v["sortf"], 1);
}
```

**GREEN** - wire `model_type as u8` (or `as i64` in JSON number), custom fields.

### Cycle 18 - `to_json` propagates req errors

**RED**

```rust
#[test]
fn to_json_propagates_template_req_error() {
    let m = Model::new(1, "x")
        .field(Field::new("Q"))
        .template(Template::new("c", "no fields", ""));
    assert!(matches!(m.to_json(0, 1), Err(Error::TemplateReq { .. })));
}
```

**GREEN** - `to_json` calls `req()?`.

### Cycle 19 - Public re-exports + module docs

**RED** - integration-style unit test or `tests/model_req.rs`:

```rust
#[test]
fn crate_root_reexports_model_api() {
    use genanki::{Field, Model, ModelType, Template};
    let m = Model::new(1, "m")
        .field(Field::new("Q"))
        .template(Template::new("c", "{{Q}}", ""))
        .model_type(ModelType::FrontBack);
    assert!(m.req().is_ok());
}
```

**GREEN**

```rust
// lib.rs
pub use crate::model::{Field, Model, ModelType, Template};
// optional: DEFAULT_LATEX_PRE, DEFAULT_LATEX_POST, ReqEntry, ReqKind
```

Fix `src/req.rs` module doc ("Phase 2", not "Phase 4").
All public items documented (`#![deny(missing_docs)]`).

Optional: re-export `ReqEntry` / `ReqKind` if tests/users need them at crate root; otherwise `genanki::req::ReqEntry` is fine and keeps root surface smaller. **Prefer crate-root re-export of `ReqEntry` + `ReqKind`** if `Model::req` returns them as part of the public signature.

### Cycle 20 - Full gate + polish

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo doc --no-deps
```

Fix fallout. No Phase 3 types, no README overhaul.

## 7. File-level diff expectations

| File | Action |
| ---- | ------ |
| `Cargo.toml` | Add `serde` (derive), `serde_json` |
| `src/lib.rs` | Re-export Model API; keep existing re-exports |
| `src/error.rs` | Add `TemplateReq` (message stable enough for test) |
| `src/model.rs` | Full `Field` / `Template` / `Model` / `ModelType` + defaults + `to_json` + unit tests for builders/JSON |
| `src/req.rs` | Mustache subset `render`, `ReqEntry` / `ReqKind`, `compute_req`, unit tests for fixtures + filters |
| `tests/model_req.rs` | Optional integration test for crate-root re-exports + one end-to-end req fixture |
| `README.md` | No change required (API example still Phase 5/7) |
| `doc/plans/issue-4.md` | This plan |

## 8. Implementation order (summary checklist)

1. Branch `issue/4-model-req`.
2. Cycle 0: add `serde` / `serde_json`.
3. Cycle 1: `Error::TemplateReq`.
4. Cycles 2-5: `ModelType`, `Field`, `Template`, `Model` builders + latex constants.
5. Cycles 6-10: Mustache subset (`render`) including filters.
6. Cycles 11-15: `req` fixtures + error + filter divergence tests.
7. Cycles 16-18: `to_json` shape, cloze type, error propagation.
8. Cycle 19: re-exports + docs.
9. Cycle 20: fmt/clippy/test/doc gate.
10. PR -> CI green -> merge -> tick #4 checkboxes -> close #4.

## 9. Acceptance criteria (map to issue #4)

| Criterion | How verified |
| --------- | ------------ |
| `Field`, `Template`, `Model`, `ModelType` in `src/model.rs` | Types public + tested |
| Builder API (`new`, `.field`, `.template`, `.css`, `.model_type`, `.sort_field_index`, latex pre/post) | Cycle 5 |
| Defaults match Python | Cycles 3-5, 16 |
| Mustache subset for req in `src/req.rs` | Cycles 6-10 |
| `req` simple / cn / hint fixtures exact | Cycles 11-13 |
| Error if template has no detectable required fields | Cycle 14 |
| Serialized model JSON includes `flds`, `tmpls`, `req`, `sortf`, `type`, latex keys, defaults (`bafmt`, font, etc.) | Cycle 16 |
| Cloze `ModelType` serializes `"type": 1` | Cycle 17 |
| Filter resolution per epic (documented Python divergence) | Cycles 10, 15 |
| Deps: only `serde` + `serde_json` added | `Cargo.toml` diff |

## 10. PR shape

- **One PR** for Phase 2.
- Title: `Phase 2: Model + req (builders, Mustache subset, model JSON)`
- Body:
  - Checklist mirrored from issue #4
  - Note deps added: `serde`, `serde_json` only
  - Note filter-resolution divergence from Python/chevron for cloze `req`
  - Note `to_json(...) -> Result<serde_json::Value>` and consuming builders
  - Link epic #1 and this plan path `doc/plans/issue-4.md`
- Do not bump version beyond `0.1.0`.
- Do not land Note/Card/Deck/Package/builtins.

## 11. Follow-ups (explicitly not this PR)

| Item | Phase / issue |
| ---- | ------------- |
| Note / Card, tag validation, cloze card ords, front/back card filtering via `req` | #5 |
| Deck / Package / rusqlite exec / zip / media; writing `col.models` from `to_json` | #6 |
| Builtin models using these builders | #7 |
| README end-to-end example | #7 |
| Broader `Error` variants (`Io`, `Sqlite`, ...) | as those phases need them |

## 12. Risks / notes

1. **Mustache edge cases:** Real Anki templates can be messy. Scope is **req-correctness**, not a general Mustache engine. If a fixture needs another construct, add it with a test - do not expand preemptively.
2. **Section whitespace standing alone:** Some Mustache engines eat whitespace around section tags when the tag is alone on a line. Chevron/`req` fixtures do not depend on that. **Do not** implement standalone-line whitespace eating unless a fixture requires it.
3. **HTML escaping:** Unnecessary for sentinel-based `req`. If `render` is later reused for something user-visible, revisit. Document `render` as req-oriented.
4. **`serde_json` Value key order:** Serde's map order is insertion order for `Map`. Prefer building objects in a stable key order for nicer diffs (optional). Tests should assert keys individually, not full-string equality of pretty JSON.
5. **`id` stringification:** Use ordinary decimal (`234567` -> `"234567"`), not scientific notation. `model.id.to_string()` is correct for `i64`.
6. **`did: null`:** Use `Value::Null`, not omit the key, not `0`.
7. **Prior art crates:** Do not copy from `yannickfunk/genanki-rs` et al. Port behavior from Python reference + this plan.
8. **`req.rs` module doc** currently says "Phase 4" - correct to Phase 2 while touching the file.
9. **Cloze and `req`:** Phase 3 generates cloze cards from `{{cN::...}}` patterns, **not** from `req`. Still emit a sensible `req` in model JSON for Anki.
10. **Clippy:** consuming builders often trigger `must_use` desires - mark chain methods or types with `#[must_use]` where it helps; avoid dead_code on fields used only via JSON.
11. **Edition 2024:** already on crate; no change.

## 13. Done definition

Phase 2 is done when the acceptance table in section 9 is true on `main`, issue #4
checkboxes are updated/closed, and #5 can start without further Model/`req`/model-JSON work.
