//! Integration tests: cloze card generation via the public API.
//! Ports test_cloze.py from Python genanki.

use genanki::{Field, Model, ModelType, Note, Template};

fn cloze_model() -> Model {
    Model::new(998877661, "My Cloze Model")
        .model_type(ModelType::Cloze)
        .field(Field::new("Text"))
        .field(Field::new("Extra"))
        .template(Template::new(
            "My Cloze Card",
            "{{cloze:Text}}",
            "{{cloze:Text}}<br>{{Extra}}",
        ))
}

fn multi_field_cloze_model() -> Model {
    Model::new(1047194615, "Multi Field Cloze Model")
        .model_type(ModelType::Cloze)
        .field(Field::new("Text1"))
        .field(Field::new("Text2"))
        .template(Template::new(
            "Cloze",
            "{{cloze:Text1}} and {{cloze:Text2}}",
            "{{cloze:Text1}} and {{cloze:Text2}}",
        ))
}

fn cloze_ords(fields: &[&str]) -> Vec<i32> {
    let mut n = Note::new(cloze_model(), fields.iter().copied()).unwrap();
    n.cards().unwrap().iter().map(|c| c.ord).collect()
}

#[test]
fn cloze_single_deletion() {
    assert_eq!(cloze_ords(&["NOTE ONE: {{c1::single deletion}}", ""]), [0]);
}

#[test]
fn cloze_three_deletions() {
    assert_eq!(
        cloze_ords(&[
            "NOTE TWO: {{c1::1st deletion}} {{c2::2nd deletion}} {{c3::3rd deletion}}",
            ""
        ]),
        [0, 1, 2]
    );
}

#[test]
fn cloze_hint_deletion() {
    assert_eq!(
        cloze_ords(&["NOTE THREE: {{c1::1st deletion::C1-CLOZE}}", ""]),
        [0]
    );
}

#[test]
fn cloze_repeated_reference_dedupes() {
    assert_eq!(
        cloze_ords(&[
            "NOTE FOUR: {{c1::1st deletion}} foo {{c2::2nd deletion}} bar {{c1::3rd deletion}}",
            ""
        ]),
        [0, 1]
    );
}

#[test]
fn cloze_multi_field_union() {
    let mut n = Note::new(
        multi_field_cloze_model(),
        [
            "{{c1::Berlin}} is the capital of {{c2::Germany}}",
            "{{c3::Paris}} is the capital of {{c4::France}}",
        ],
    )
    .unwrap();
    let ords: Vec<i32> = n.cards().unwrap().iter().map(|c| c.ord).collect();
    assert_eq!(ords, [0, 1, 2, 3]);
}

#[test]
fn cloze_indicies_do_not_start_at_one() {
    assert_eq!(
        cloze_ords(&[
            "{{c2::Mitochondria}} are the {{c3::powerhouses}} of the cell",
            ""
        ]),
        [1, 2]
    );
}

#[test]
fn cloze_newlines_in_deletion() {
    assert_eq!(
        cloze_ords(&[
            "{{c1::Washington, D.C.}} is the capital of {{c2::the\nUnited States\nof America}}",
            ""
        ]),
        [0, 1]
    );
}

#[test]
fn cloze_no_markers_defaults_to_zero() {
    assert_eq!(cloze_ords(&["no cloze markers at all", ""]), [0]);
}
