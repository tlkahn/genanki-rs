//! Integration tests for the public crate-root re-exports (Phase 2).

use genanki::{Field, Model, ModelType, ReqEntry, ReqKind, Template};

#[test]
fn crate_root_reexports_model_api() {
    let m = Model::new(1, "m")
        .field(Field::new("Q"))
        .template(Template::new("c", "{{Q}}", ""))
        .model_type(ModelType::FrontBack);
    assert!(m.req().is_ok());
}

#[test]
fn crate_root_req_types_roundtrip_via_to_json() {
    use serde_json::json;
    let m = Model::new(2, "m")
        .field(Field::new("Q"))
        .field(Field::new("A"))
        .template(Template::new("c", "{{Q}}", "{{A}}"));
    let v = m.to_json(0, 1).unwrap();
    assert_eq!(v["req"], json!([[0, "all", [0]]]));
    let req = m.req().unwrap();
    assert_eq!(
        req,
        vec![ReqEntry {
            template_ord: 0,
            kind: ReqKind::All,
            field_ords: vec![0],
        }]
    );
}
