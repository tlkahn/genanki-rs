//! Integration tests: card generation from `model.req()` (Phase 3 parity).
//! Ports `test_notes_generate_cards_based_on_req__*` from Python test_genanki.py.

use genanki::{Field, Model, Note, Template};

fn cn_model() -> Model {
    Model::new(345678, "Chinese")
        .field(Field::new("Traditional"))
        .field(Field::new("Simplified"))
        .field(Field::new("English"))
        .template(Template::new(
            "Traditional",
            "{{Traditional}}",
            "{{FrontSide}}<hr id=\"answer\">{{English}}",
        ))
        .template(Template::new(
            "Simplified",
            "{{Simplified}}",
            "{{FrontSide}}<hr id=\"answer\">{{English}}",
        ))
}

fn hint_model() -> Model {
    Model::new(456789, "with hint")
        .field(Field::new("Question"))
        .field(Field::new("Hint"))
        .field(Field::new("Answer"))
        .template(Template::new(
            "card1",
            "{{Question}}{{#Hint}}<br>Hint: {{Hint}}{{/Hint}}",
            "{{Answer}}",
        ))
}

#[test]
fn cn_model_generates_cards_based_on_req() {
    // Has the Simplified field: both cards.
    let mut n1 = Note::new(cn_model(), ["中國", "中国", "China"]).unwrap();
    let cards = n1.cards().unwrap();
    assert_eq!(cards.len(), 2);
    assert_eq!(cards[0].ord, 0);
    assert_eq!(cards[1].ord, 1);

    // Simplified empty: only the Traditional card.
    let mut n2 = Note::new(cn_model(), ["你好", "", "hello"]).unwrap();
    let cards = n2.cards().unwrap();
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].ord, 0);
}

#[test]
fn hint_model_generates_one_card_when_q_or_hint_present() {
    let mut n1 = Note::new(hint_model(), ["capital of California", "", "Sacramento"]).unwrap();
    let cards = n1.cards().unwrap();
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].ord, 0);

    let mut n2 = Note::new(
        hint_model(),
        ["capital of Iowa", "French for \"The Moines\"", "Des Moines"],
    )
    .unwrap();
    let cards = n2.cards().unwrap();
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].ord, 0);
}
