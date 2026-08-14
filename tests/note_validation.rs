//! Integration tests: note validation (tags + field count) via the public API.
//! Ports `TestTags` / `test_num_fields_*` from Python test_note.py.

use genanki::{Error, Field, Model, Note, Template};

fn simple_model() -> Model {
    Model::new(1376484377, "Simple Model")
        .field(Field::new("Question"))
        .field(Field::new("Answer"))
        .template(Template::new(
            "Card 1",
            "{{Question}}",
            "{{FrontSide}}<hr id=\"answer\">{{Answer}}",
        ))
}

fn three_field_model() -> Model {
    Model::new(1894808898, "Test Model")
        .field(Field::new("Question"))
        .field(Field::new("Answer"))
        .field(Field::new("Extra"))
        .template(Template::new("Card 1", "{{Question}}", "{{Answer}}"))
}

#[test]
fn tags_validate_on_every_mutation_path() {
    let mut n = Note::new(simple_model(), ["Q", "A"])
        .unwrap()
        .with_tags(["foo", "bar", "baz"])
        .unwrap();
    assert_eq!(n.tags(), ["foo", "bar", "baz"]);

    // with_tags rejects spaces.
    let err = Note::new(simple_model(), ["Q", "A"])
        .unwrap()
        .with_tags(["foo", "b ar"])
        .unwrap_err();
    assert!(matches!(err, Error::TagContainsSpace { tag } if tag == "b ar"));

    // set_tag (Python `__setitem__`) rejects spaces.
    n.set_tag(0, "dankey_kang").unwrap();
    assert!(matches!(
        n.set_tag(1, "dankey kang"),
        Err(Error::TagContainsSpace { .. })
    ));

    // add_tag (Python `append`) rejects spaces.
    n.add_tag("sheik_hashtag_melee").unwrap();
    assert!(matches!(
        n.add_tag("king dedede"),
        Err(Error::TagContainsSpace { .. })
    ));

    // extend_tags (Python `extend`) rejects spaces.
    n.extend_tags(["palu", "wolf"]).unwrap();
    assert!(matches!(
        n.extend_tags(["dat fox doe"]),
        Err(Error::TagContainsSpace { .. })
    ));

    // insert_tag (Python `insert`) rejects spaces.
    n.insert_tag(0, "lucina").unwrap();
    assert!(matches!(
        n.insert_tag(0, "nerf joker pls"),
        Err(Error::TagContainsSpace { .. })
    ));
}

#[test]
fn field_count_mismatch_errors_at_construct() {
    // Equal counts are fine.
    let n = Note::new(
        three_field_model(),
        [
            "What is the capital of Taiwan?",
            "Taipei",
            "Taipei was originally inhabited by the Ketagalan people.",
        ],
    )
    .unwrap();
    assert_eq!(n.fields().len(), 3);

    // Fewer fields than the model -> error.
    let err = Note::new(
        three_field_model(),
        ["What is the capital of Taiwan?", "Taipei"],
    )
    .unwrap_err();
    assert!(matches!(
        err,
        Error::FieldCountMismatch {
            model_name,
            model_fields: 3,
            note_fields: 2,
        } if model_name == "Test Model"
    ));

    // More fields than the model -> error.
    let err = Note::new(
        simple_model(),
        ["What is the capital of Taiwan?", "Taipei", "extra field"],
    )
    .unwrap_err();
    assert!(matches!(
        err,
        Error::FieldCountMismatch {
            model_fields: 2,
            note_fields: 3,
            ..
        }
    ));
}

#[test]
fn guid_and_sort_field_defaults_and_overrides() {
    let mut n = Note::new(simple_model(), ["Capital of Argentina", "Buenos Aires"])
        .unwrap()
        .with_guid("custom-guid")
        .with_sort_field("custom-sort");
    assert_eq!(n.guid(), "custom-guid");
    assert_eq!(n.sort_field(), "custom-sort");

    n.set_guid(None);
    n.set_sort_field(None);
    assert_eq!(n.guid(), "HSnG{z%dU<"); // guid_for(["Capital of Argentina", "Buenos Aires"])
    assert_eq!(n.sort_field(), "Capital of Argentina"); // fields[0]
}
