//! Card template `req` computation and the Mustache subset it needs. (Phase 2)
//!
//! Anki computes which fields are "required" per template by rendering the
//! template's `qfmt` with sentinel values (see [`compute_req`]). This module
//! provides the renderer (a small hand-rolled Mustache subset, not a full
//! engine) plus the [`ReqEntry`] type describing the result.

use std::collections::BTreeMap;

use crate::model::{Field, Template};
use crate::{Error, Result};

/// Sentinel used to probe field presence during `req` computation (matches
/// Python genanki's `sentinel = 'SeNtInEl'`).
const SENTINEL: &str = "SeNtInEl";

/// One template's required-field entry: `[tmpl_idx, "all"|"any", [field_ords...]]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReqEntry {
    /// Ordinal of the template this entry belongs to.
    pub template_ord: u32,
    /// Required-field strategy: "all" or "any".
    pub kind: ReqKind,
    /// Field ordinals required by this template, ascending.
    pub field_ords: Vec<u32>,
}

/// Required-field strategy for a template.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReqKind {
    /// Every listed field must be non-empty.
    All,
    /// At least one listed field must be non-empty.
    Any,
}

impl ReqKind {
    /// JSON string form ("all" / "any").
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ReqKind::All => "all",
            ReqKind::Any => "any",
        }
    }
}

/// Compute the Anki `req` list for a model's fields and templates.
///
/// Mirrors Python genanki `Model._req` (which reimplements Anki's logic): for
/// each template, first try the "all" strategy (each field in turn blanked;
/// if the sentinel disappears, the field is required), then the "any"
/// strategy (each field in turn set to the sentinel; if the sentinel appears,
/// the field suffices). Returns [`Error::TemplateReq`] when a template has no
/// detectable required fields under either strategy.
///
/// Only `qfmt` is rendered. Filter tags such as `{{cloze:Text}}` resolve to
/// the field name (documented divergence from Python/chevron, which looks up
/// the literal key).
#[must_use]
pub fn compute_req(fields: &[Field], templates: &[Template]) -> Result<Vec<ReqEntry>> {
    let field_names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    let mut req = Vec::new();

    for (template_ord, template) in templates.iter().enumerate() {
        // Strategy "all": which fields, when blank, remove all meaningful content?
        let mut required = Vec::new();
        for (field_ord, name) in field_names.iter().copied().enumerate() {
            let mut values: BTreeMap<&str, &str> = BTreeMap::new();
            for n in field_names.iter().copied() {
                values.insert(n, SENTINEL);
            }
            values.insert(name, "");
            let rendered = render(&template.qfmt, &values);
            if !rendered.contains(SENTINEL) {
                required.push(field_ord as u32);
            }
        }

        if !required.is_empty() {
            req.push(ReqEntry {
                template_ord: template_ord as u32,
                kind: ReqKind::All,
                field_ords: required,
            });
            continue;
        }

        // Strategy "any": which fields, when present alone, provide content?
        for (field_ord, name) in field_names.iter().copied().enumerate() {
            let mut values: BTreeMap<&str, &str> = BTreeMap::new();
            for n in field_names.iter().copied() {
                values.insert(n, "");
            }
            values.insert(name, SENTINEL);
            let rendered = render(&template.qfmt, &values);
            if rendered.contains(SENTINEL) {
                required.push(field_ord as u32);
            }
        }

        if required.is_empty() {
            return Err(Error::TemplateReq {
                qfmt: template.qfmt.clone(),
            });
        }
        req.push(ReqEntry {
            template_ord: template_ord as u32,
            kind: ReqKind::Any,
            field_ords: required,
        });
    }

    Ok(req)
}

/// Render `template` with string field values (req-oriented Mustache subset).
///
/// Supported: `{{name}}`, `{{{name}}}` / `{{&name}}`, sections `{{#name}}` /
/// `{{^name}}` with nesting, comments `{{! ... }}`, and whitespace inside
/// tags. Filter-style names (`cloze:Text`) resolve to the field after the
/// last `:` when no exact key matches. Unknown/out-of-scope constructs render
/// as empty. No HTML escaping: this is for sentinel-based `req` only.
#[must_use]
pub fn render(template: &str, fields: &BTreeMap<&str, &str>) -> String {
    render_into(template, fields)
}

/// Recursive renderer; section bodies are rendered on the same field map.
fn render_into(template: &str, fields: &BTreeMap<&str, &str>) -> String {
    let mut out = String::new();
    let mut rest = template;
    loop {
        let Some(start) = rest.find("{{") else {
            out.push_str(rest);
            return out;
        };
        out.push_str(&rest[..start]);
        let after_open = &rest[start + 2..];
        // Triple-brace form `{{{name}}}`.
        let (tag_body, after_close) = if let Some(b) = after_open.strip_prefix('{') {
            match b.find("}}}") {
                Some(rel) => (&b[..rel], &b[rel + 3..]),
                None => {
                    out.push_str(rest);
                    return out;
                }
            }
        } else {
            match after_open.find("}}") {
                Some(rel) => (&after_open[..rel], &after_open[rel + 2..]),
                None => {
                    out.push_str(rest);
                    return out;
                }
            }
        };
        let raw = tag_body.trim();
        let sigil = raw.as_bytes().first();
        match sigil {
            Some(b'!') => {
                // Comment: emit nothing.
                rest = after_close;
            }
            Some(b'#') | Some(b'^') => {
                let name = raw[1..].trim();
                let truthy = lookup(name, fields).is_some_and(|v| !v.is_empty());
                match find_section_body(after_close, name) {
                    Some((interior, tail)) => {
                        let render_section = if sigil == Some(&b'#') { truthy } else { !truthy };
                        if render_section {
                            out.push_str(&render_into(interior, fields));
                        }
                        rest = tail;
                    }
                    // Unmatched open: lenient, skip the tag itself and keep scanning.
                    None => rest = after_close,
                }
            }
            Some(b'/') => {
                // Unmatched close: lenient, emit nothing.
                rest = after_close;
            }
            Some(b'&') => {
                let name = raw[1..].trim();
                if let Some(v) = lookup(name, fields) {
                    out.push_str(v);
                }
                rest = after_close;
            }
            _ => {
                if let Some(v) = lookup(raw, fields) {
                    out.push_str(v);
                }
                rest = after_close;
            }
        }
    }
}

/// Locate the interior and tail of the `{{#name}}...{{/name}}` section that
/// starts at `after_open`, honoring nesting. Returns `(interior, tail)` where
/// `interior` is the text between the open and matching close tag and `tail`
/// is everything after the close tag.
fn find_section_body<'a>(after_open: &'a str, name: &str) -> Option<(&'a str, &'a str)> {
    let mut depth = 0usize;
    let mut rest = after_open;
    loop {
        let start = rest.find("{{")?;
        let after = &rest[start + 2..];
        let (tag_body, tail) = if let Some(b) = after.strip_prefix('{') {
            match b.find("}}}") {
                Some(rel) => (&b[..rel], &b[rel + 3..]),
                None => return None,
            }
        } else {
            match after.find("}}") {
                Some(rel) => (&after[..rel], &after[rel + 2..]),
                None => return None,
            }
        };
        let raw = tag_body.trim();
        match raw.as_bytes().first() {
            Some(b'#') if raw[1..].trim() == name => depth += 1,
            Some(b'/') if raw[1..].trim() == name => {
                if depth == 0 {
                    // `start` is relative to `rest`; convert to `after_open` offset.
                    let interior_end = after_open.len() - rest.len() + start;
                    return Some((&after_open[..interior_end], tail));
                }
                depth -= 1;
            }
            _ => {}
        }
        rest = tail;
    }
}

/// Resolve a tag name against the field map, applying Anki filter semantics.
///
/// Exact key wins first; otherwise a name containing `:` is treated as a
/// filter chain (`filter[:filter]*:FieldName`) and the part after the last
/// `:` is looked up. Missing names resolve to empty.
fn lookup<'a>(raw: &str, fields: &'a BTreeMap<&str, &str>) -> Option<&'a str> {
    if let Some(v) = fields.get(raw) {
        return Some(v);
    }
    if let Some(idx) = raw.rfind(':') {
        let field = &raw[idx + 1..];
        if let Some(v) = fields.get(field) {
            return Some(v);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields<'a>(pairs: &[(&'a str, &'a str)]) -> BTreeMap<&'a str, &'a str> {
        pairs.iter().copied().collect()
    }

    #[test]
    fn render_interpolates_field() {
        let fields = fields(&[("AField", "SeNtInEl"), ("BField", "")]);
        assert_eq!(render("{{AField}}", &fields), "SeNtInEl");
    }

    #[test]
    fn render_missing_field_is_empty() {
        let fields = fields(&[]);
        assert_eq!(render("{{Nope}}", &fields), "");
    }

    #[test]
    fn render_preserves_surrounding_text() {
        let fields = fields(&[("Q", "x")]);
        assert_eq!(render("pre-{{Q}}-post", &fields), "pre-x-post");
    }

    #[test]
    fn render_trims_tag_whitespace() {
        let fields = fields(&[("Q", "x")]);
        assert_eq!(render("{{ Q }}", &fields), "x");
    }

    #[test]
    fn render_triple_and_amp_unescaped() {
        let fields = fields(&[("Q", "x")]);
        assert_eq!(render("{{{Q}}}", &fields), "x");
        assert_eq!(render("{{&Q}}", &fields), "x");
    }

    #[test]
    fn render_strips_comments() {
        let fields = fields(&[("Q", "x")]);
        assert_eq!(render("{{! ignore me }}{{Q}}", &fields), "x");
    }

    #[test]
    fn render_section_truthy() {
        let fields = fields(&[("Hint", "h")]);
        assert_eq!(render("{{#Hint}}H:{{Hint}}{{/Hint}}", &fields), "H:h");
    }

    #[test]
    fn render_section_falsy_skips() {
        let fields = fields(&[("Hint", "")]);
        assert_eq!(render("{{#Hint}}H:{{Hint}}{{/Hint}}X", &fields), "X");
    }

    #[test]
    fn render_inverted() {
        let mut map = fields(&[("A", "")]);
        assert_eq!(render("{{^A}}no{{/A}}", &map), "no");
        map.insert("A", "yes");
        assert_eq!(render("{{^A}}no{{/A}}", &map), "");
    }

    #[test]
    fn render_nested_sections() {
        let fields = fields(&[("A", "1"), ("B", "2")]);
        assert_eq!(render("{{#A}}{{#B}}{{A}}{{B}}{{/B}}{{/A}}", &fields), "12");
    }

    #[test]
    fn render_cloze_filter_resolves_to_field() {
        let fields = fields(&[("Text", "SeNtInEl"), ("Back Extra", "")]);
        assert_eq!(render("{{cloze:Text}}", &fields), "SeNtInEl");
    }

    #[test]
    fn render_type_filter_resolves_to_field() {
        let fields = fields(&[("Front", "F"), ("Back", "B")]);
        assert_eq!(render("{{Front}} {{type:Back}}", &fields), "F B");
    }

    #[test]
    fn render_exact_key_wins_over_filter_strip() {
        let fields = fields(&[("cloze:Text", "LITERAL"), ("Text", "FIELD")]);
        assert_eq!(render("{{cloze:Text}}", &fields), "LITERAL");
    }

    // --- compute_req fixtures (Python genanki test_Model_req*) ---

    use crate::model::{Field, Template};
    use crate::Error;

    fn simple_model() -> (Vec<Field>, Vec<Template>) {
        (
            vec![Field::new("AField"), Field::new("BField")],
            vec![Template::new(
                "card1",
                "{{AField}}",
                "{{FrontSide}}<hr id=\"answer\">{{BField}}",
            )],
        )
    }

    fn cn_model() -> (Vec<Field>, Vec<Template>) {
        (
            vec![
                Field::new("Traditional"),
                Field::new("Simplified"),
                Field::new("English"),
            ],
            vec![
                Template::new("Traditional", "{{Traditional}}", "{{FrontSide}}<hr id=\"answer\">{{English}}"),
                Template::new("Simplified", "{{Simplified}}", "{{FrontSide}}<hr id=\"answer\">{{English}}"),
            ],
        )
    }

    fn hint_model() -> (Vec<Field>, Vec<Template>) {
        (
            vec![
                Field::new("Question"),
                Field::new("Hint"),
                Field::new("Answer"),
            ],
            vec![Template::new(
                "card1",
                "{{Question}}{{#Hint}}<br>Hint: {{Hint}}{{/Hint}}",
                "{{Answer}}",
            )],
        )
    }

    #[test]
    fn req_simple_all() {
        let (fields, templates) = simple_model();
        let req = compute_req(&fields, &templates).unwrap();
        assert_eq!(
            req,
            vec![ReqEntry {
                template_ord: 0,
                kind: ReqKind::All,
                field_ords: vec![0],
            }]
        );
    }

    #[test]
    fn req_cn_dual_templates() {
        let (fields, templates) = cn_model();
        let req = compute_req(&fields, &templates).unwrap();
        assert_eq!(
            req,
            vec![
                ReqEntry {
                    template_ord: 0,
                    kind: ReqKind::All,
                    field_ords: vec![0],
                },
                ReqEntry {
                    template_ord: 1,
                    kind: ReqKind::All,
                    field_ords: vec![1],
                },
            ]
        );
    }

    #[test]
    fn req_hint_any() {
        let (fields, templates) = hint_model();
        let req = compute_req(&fields, &templates).unwrap();
        assert_eq!(
            req,
            vec![ReqEntry {
                template_ord: 0,
                kind: ReqKind::Any,
                field_ords: vec![0, 1],
            }]
        );
    }
}
