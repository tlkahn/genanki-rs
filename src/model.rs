//! Note types and card templates. (Phase 2)

/// Default LaTeX preamble, byte-identical to Python genanki `Model.DEFAULT_LATEX_PRE`.
pub const DEFAULT_LATEX_PRE: &str = "\\documentclass[12pt]{article}\n\\special{papersize=3in,5in}\n\\usepackage[utf8]{inputenc}\n\\usepackage{amssymb,amsmath}\n\\pagestyle{empty}\n\\setlength{\\parindent}{0in}\n\\begin{document}\n";

/// Default LaTeX postamble, byte-identical to Python genanki `Model.DEFAULT_LATEX_POST`.
pub const DEFAULT_LATEX_POST: &str = "\\end{document}";

/// Note-type category: front/back or cloze.
///
/// Mirrors Python genanki `Model.FRONT_BACK = 0` / `Model.CLOZE = 1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ModelType {
    /// Standard front/back model.
    FrontBack = 0,
    /// Cloze-deletion model.
    Cloze = 1,
}

/// A single field (column) of a note type.
///
/// Defaults match Python genanki `Field` dicts after `Model.to_json` applies
/// `setdefault`: `font` "Liberation Sans", `media` `[]`, `rtl` `false`,
/// `size` 20, `sticky` `false`. `ord` is not stored; it is assigned from the
/// field position at `to_json` time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    /// Field name as shown in the Anki editor.
    pub name: String,
    /// Font used to display the field.
    pub font: String,
    /// Media files attached to the field.
    pub media: Vec<String>,
    /// Right-to-left rendering.
    pub rtl: bool,
    /// Font size in points.
    pub size: u32,
    /// Sticky field (persists focus across notes).
    pub sticky: bool,
}

impl Field {
    /// Create a field with Python-parity defaults.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            font: "Liberation Sans".into(),
            media: Vec::new(),
            rtl: false,
            size: 20,
            sticky: false,
        }
    }

    /// Override the display font.
    #[must_use]
    pub fn font(mut self, font: impl Into<String>) -> Self {
        self.font = font.into();
        self
    }

    /// Attach media files.
    #[must_use]
    pub fn media(mut self, media: Vec<String>) -> Self {
        self.media = media;
        self
    }

    /// Set right-to-left rendering.
    #[must_use]
    pub fn rtl(mut self, rtl: bool) -> Self {
        self.rtl = rtl;
        self
    }

    /// Set the font size in points.
    #[must_use]
    pub fn size(mut self, size: u32) -> Self {
        self.size = size;
        self
    }

    /// Set the sticky flag.
    #[must_use]
    pub fn sticky(mut self, sticky: bool) -> Self {
        self.sticky = sticky;
        self
    }
}

/// A single card template (front/back pair) of a note type.
///
/// Defaults match Python genanki `Template` dicts after `Model.to_json`
/// applies `setdefault`: empty `bqfmt`/`bafmt`/`bfont`, `bsize` 0, `did`
/// `None`. `ord` is not stored; it is assigned from the template position at
/// `to_json` time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    /// Template name as shown in Anki's card-type list.
    pub name: String,
    /// Front (question) template, Mustache-flavored.
    pub qfmt: String,
    /// Back (answer) template.
    pub afmt: String,
    /// Browser-style front template (optional, Anki 2.1.28+).
    pub bqfmt: String,
    /// Browser-style back template (optional).
    pub bafmt: String,
    /// Browser-style font (optional).
    pub bfont: String,
    /// Browser-style font size (optional).
    pub bsize: u32,
    /// Deck override; `None` serializes as JSON `null`.
    pub did: Option<i64>,
}

impl Template {
    /// Create a template from name, question, and answer strings.
    #[must_use]
    pub fn new(name: impl Into<String>, qfmt: impl Into<String>, afmt: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            qfmt: qfmt.into(),
            afmt: afmt.into(),
            bqfmt: String::new(),
            bafmt: String::new(),
            bfont: String::new(),
            bsize: 0,
            did: None,
        }
    }

    /// Override the browser front template.
    #[must_use]
    pub fn bqfmt(mut self, v: impl Into<String>) -> Self {
        self.bqfmt = v.into();
        self
    }

    /// Override the browser back template.
    #[must_use]
    pub fn bafmt(mut self, v: impl Into<String>) -> Self {
        self.bafmt = v.into();
        self
    }

    /// Override the browser font.
    #[must_use]
    pub fn bfont(mut self, v: impl Into<String>) -> Self {
        self.bfont = v.into();
        self
    }

    /// Override the browser font size.
    #[must_use]
    pub fn bsize(mut self, v: u32) -> Self {
        self.bsize = v;
        self
    }

    /// Override the deck id; `None` serializes as JSON `null`.
    #[must_use]
    pub fn did(mut self, did: Option<i64>) -> Self {
        self.did = did;
        self
    }
}

/// A note type: field set, card templates, styling, and model metadata.
///
/// Defaults match Python genanki `Model.__init__`: empty `css`, `model_type`
/// [`ModelType::FrontBack`], [`DEFAULT_LATEX_PRE`]/[`DEFAULT_LATEX_POST`], and
/// `sort_field_index` 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    /// Model id (Anki uses Unix-ms-style timestamps as ids).
    pub id: i64,
    /// Model name shown in Anki.
    pub name: String,
    /// Fields in ordinal order.
    pub fields: Vec<Field>,
    /// Card templates in ordinal order.
    pub templates: Vec<Template>,
    /// CSS shared by all templates of this model.
    pub css: String,
    /// Front/back or cloze.
    pub model_type: ModelType,
    /// LaTeX preamble for `[latex]` blocks.
    pub latex_pre: String,
    /// LaTeX postamble for `[latex]` blocks.
    pub latex_post: String,
    /// Index of the sort field (Anki default 0).
    pub sort_field_index: i32,
}

impl Model {
    /// Create a model with Python-parity defaults; add fields/templates via
    /// the consuming chain builders.
    #[must_use]
    pub fn new(id: i64, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            fields: Vec::new(),
            templates: Vec::new(),
            css: String::new(),
            model_type: ModelType::FrontBack,
            latex_pre: DEFAULT_LATEX_PRE.into(),
            latex_post: DEFAULT_LATEX_POST.into(),
            sort_field_index: 0,
        }
    }

    /// Append a field.
    #[must_use]
    pub fn field(mut self, field: Field) -> Self {
        self.fields.push(field);
        self
    }

    /// Append a card template.
    #[must_use]
    pub fn template(mut self, template: Template) -> Self {
        self.templates.push(template);
        self
    }

    /// Override the model CSS.
    #[must_use]
    pub fn css(mut self, css: impl Into<String>) -> Self {
        self.css = css.into();
        self
    }

    /// Set front/back or cloze.
    #[must_use]
    pub fn model_type(mut self, t: ModelType) -> Self {
        self.model_type = t;
        self
    }

    /// Override the LaTeX preamble.
    #[must_use]
    pub fn latex_pre(mut self, s: impl Into<String>) -> Self {
        self.latex_pre = s.into();
        self
    }

    /// Override the LaTeX postamble.
    #[must_use]
    pub fn latex_post(mut self, s: impl Into<String>) -> Self {
        self.latex_post = s.into();
        self
    }

    /// Override the sort field index.
    #[must_use]
    pub fn sort_field_index(mut self, idx: i32) -> Self {
        self.sort_field_index = idx;
        self
    }

    /// Compute Anki's `req` (required fields per template) for this model.
    ///
    /// Errors with [`crate::Error::TemplateReq`] if any template's `qfmt`
    /// contains no detectable field references.
    pub fn req(&self) -> crate::Result<Vec<crate::req::ReqEntry>> {
        crate::req::compute_req(&self.fields, &self.templates)
    }

    /// Serialize to the object stored under `col.models[model_id]`.
    ///
    /// Mirrors Python genanki `Model.to_json(timestamp, deck_id)` key-for-key:
    /// `id` is a decimal string, template `did` is `null`, `mod` is the
    /// timestamp, and `req` is computed on the fly (errors if a template has
    /// no computable required fields). Field and template `ord` are assigned
    /// from their positions.
    pub fn to_json(&self, timestamp_secs: i64, deck_id: i64) -> crate::Result<serde_json::Value> {
        let req = self.req()?;

        let flds: Vec<serde_json::Value> = self
            .fields
            .iter()
            .enumerate()
            .map(|(ord, f)| {
                serde_json::json!({
                    "name": f.name,
                    "ord": ord,
                    "font": f.font,
                    "media": f.media,
                    "rtl": f.rtl,
                    "size": f.size,
                    "sticky": f.sticky,
                })
            })
            .collect();

        let tmpls: Vec<serde_json::Value> = self
            .templates
            .iter()
            .enumerate()
            .map(|(ord, t)| {
                serde_json::json!({
                    "name": t.name,
                    "qfmt": t.qfmt,
                    "afmt": t.afmt,
                    "ord": ord,
                    "bafmt": t.bafmt,
                    "bqfmt": t.bqfmt,
                    "bfont": t.bfont,
                    "bsize": t.bsize,
                    "did": t.did,
                })
            })
            .collect();

        let req_json: Vec<serde_json::Value> = req
            .iter()
            .map(|e| serde_json::json!([e.template_ord, e.kind.as_str(), e.field_ords]))
            .collect();

        Ok(serde_json::json!({
            "css": self.css,
            "did": deck_id,
            "flds": flds,
            "id": self.id.to_string(),
            "latexPost": self.latex_post,
            "latexPre": self.latex_pre,
            "latexsvg": false,
            "mod": timestamp_secs,
            "name": self.name,
            "req": req_json,
            "sortf": self.sort_field_index,
            "tags": [],
            "tmpls": tmpls,
            "type": self.model_type as u8,
            "usn": -1,
            "vers": [],
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;

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
        let f = Field::new("Q")
            .font("Arial")
            .size(22)
            .rtl(true)
            .sticky(true)
            .media(vec!["a.mp3".into()]);
        assert_eq!(f.font, "Arial");
        assert_eq!(f.size, 22);
        assert!(f.rtl && f.sticky);
        assert_eq!(f.media, vec!["a.mp3"]);
    }

    #[test]
    fn model_type_discriminants() {
        assert_eq!(ModelType::FrontBack as u8, 0);
        assert_eq!(ModelType::Cloze as u8, 1);
    }

    #[test]
    fn template_new_defaults() {
        let t = Template::new("card1", "{{Q}}", "{{A}}");
        assert_eq!(t.name, "card1");
        assert_eq!(t.qfmt, "{{Q}}");
        assert_eq!(t.afmt, "{{A}}");
        assert_eq!(t.bqfmt, "");
        assert_eq!(t.bafmt, "");
        assert_eq!(t.bfont, "");
        assert_eq!(t.bsize, 0);
        assert_eq!(t.did, None);
    }

    #[test]
    fn template_fluent_overrides() {
        let t = Template::new("c", "{{Q}}", "{{A}}")
            .bqfmt("BQ")
            .bafmt("BA")
            .bfont("Arial")
            .bsize(18)
            .did(Some(42));
        assert_eq!(t.bqfmt, "BQ");
        assert_eq!(t.bafmt, "BA");
        assert_eq!(t.bfont, "Arial");
        assert_eq!(t.bsize, 18);
        assert_eq!(t.did, Some(42));
    }

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
        assert!(
            DEFAULT_LATEX_PRE.ends_with(
                r"\begin{document}
"
            ) || DEFAULT_LATEX_PRE.ends_with("\\begin{document}\n")
        );
        assert_eq!(
            DEFAULT_LATEX_PRE,
            "\\documentclass[12pt]{article}\n\\special{papersize=3in,5in}\n\\usepackage[utf8]{inputenc}\n\\usepackage{amssymb,amsmath}\n\\pagestyle{empty}\n\\setlength{\\parindent}{0in}\n\\begin{document}\n"
        );
        assert_eq!(DEFAULT_LATEX_POST, "\\end{document}");
    }

    #[test]
    fn req_static_only_with_fields_requires_all() {
        // Python parity: with fields present, a qfmt with no field refs still
        // yields "all" (blanking any field removes the never-present sentinel).
        let m = Model::new(1, "x")
            .field(Field::new("Q"))
            .template(Template::new("c", "static only", ""));
        let req = m.req().unwrap();
        assert_eq!(
            req,
            vec![crate::req::ReqEntry {
                template_ord: 0,
                kind: crate::req::ReqKind::All,
                field_ords: vec![0],
            }]
        );
    }

    #[test]
    fn req_errors_when_model_has_no_fields() {
        // With zero fields neither strategy loop can run; Python raises its
        // equivalent Exception here. This is the reachable error path.
        let m = Model::new(1, "x").template(Template::new("c", "static only", ""));
        let err = m.req().unwrap_err();
        match err {
            Error::TemplateReq { qfmt } => assert!(qfmt.contains("static only")),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn req_cloze_filter_requires_text_only() {
        let m = Model::new(1, "c")
            .model_type(ModelType::Cloze)
            .field(Field::new("Text"))
            .field(Field::new("Back Extra"))
            .template(Template::new(
                "Cloze",
                "{{cloze:Text}}",
                "{{cloze:Text}}<br>{{Back Extra}}",
            ));
        let req = m.req().unwrap();
        assert_eq!(
            req,
            vec![crate::req::ReqEntry {
                template_ord: 0,
                kind: crate::req::ReqKind::All,
                field_ords: vec![0],
            }]
        );
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
        assert_eq!(
            req,
            vec![crate::req::ReqEntry {
                template_ord: 0,
                kind: crate::req::ReqKind::Any,
                field_ords: vec![0, 1],
            }]
        );
    }

    #[test]
    fn to_json_simple_model_shape() {
        use serde_json::json;
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

    #[test]
    fn to_json_cloze_type_is_one() {
        use serde_json::json;
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

    #[test]
    fn to_json_propagates_template_req_error() {
        let m = Model::new(1, "x").template(Template::new("c", "no fields", ""));
        assert!(matches!(m.to_json(0, 1), Err(Error::TemplateReq { .. })));
    }
}
