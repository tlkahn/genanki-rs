# genanki-rs: A Rust Library for Generating Anki Decks

`genanki-rs` (published as the crate **`genanki`**) lets you programmatically
generate decks in Rust for [Anki](https://apps.ankiweb.net/), a popular
spaced-repetition flashcard program. It is a port of
[genanki](https://github.com/kerrickstaley/genanki) (Python): same concepts,
same built-in note types, same `.apkg` layout.

*This library and its author(s) are not affiliated/associated with the main
Anki project in any way.*

## Status

Work in progress. See [Issue #1](../../issues/1) for the full feature
specification and roadmap. Phases 1-5 are implemented: GUIDs, models with
`req` computation, notes/cards/cloze, `.apkg` writing, built-in models, and
this documentation. The crate is `write-only`: it produces `.apkg` files, it
does not read them.

## Notes

The basic unit in Anki is the `Note`, which contains a fact to memorize.
`Note`s correspond to one or more `Card`s.

Here's how you create a `Note`:

```rust
let my_note = genanki::Note::new(
    my_model,                                  // Model (or Arc<Model>)
    ["Capital of Argentina", "Buenos Aires"],  // fields, encoded as HTML
)?;
```

You pass in a `Model` (discussed below) and a set of `fields` (encoded as
HTML). The model can be shared: `Note::new` accepts `Model`, `Arc<Model>`,
or `&Model` (the last via `From<&Model> for Arc<Model>`).

## Models

A `Model` defines the fields and cards for a type of `Note`. For example:

```rust
use genanki::{Field, Model, Template};

let my_model = Model::new(1607392319, "Simple Model")
    .field(Field::new("Question"))
    .field(Field::new("Answer"))
    .template(Template::new(
        "Card 1",
        "{{Question}}",
        "{{FrontSide}}<hr id=\"answer\">{{Answer}}",
    ));
```

This note-type has two fields and one card. The card displays the `Question`
field on the front and the `Question` and `Answer` fields on the back,
separated by an `<hr>`. You can also call `.css("...")` on the `Model` to
supply custom CSS.

You need to pass a `model_id` so that Anki can keep track of your model. It's
important that you use a unique `model_id` for each `Model` you define.
Generate one in the range `1 << 30` to `1 << 31` (for example with
`rand::random::<i64>() % (1 << 31) + (1 << 30)` or any other source of
randomness), and **hardcode it** into your model definition:

```rust
// i64 id; Anki convention is a Unix-ms-style timestamp.
let my_model = Model::new(1607392319, "Simple Model") /* ... */;
```

## Generating a Deck/Package

To import your notes into Anki, you need to add them to a `Deck`:

```rust
let mut my_deck = genanki::Deck::new(2059400110, "Country Capitals");
my_deck.add_note(my_note);
```

Once again, you need a unique `deck_id` that you should generate once and
then hardcode into your Rust file.

Then, create a `Package` for your `Deck` and write it to a file:

```rust
genanki::Package::new(my_deck).write_to_file("output.apkg")?;
```

You can then load `output.apkg` into Anki using File -> Import...

For deterministic, byte-identical output (fixed note/card ids and zip entry
times) use `write_to_file_at(path, timestamp_secs)` instead.

## Media Files

To add sounds or images, set the media files on your `Package`:

```rust
let my_package = genanki::Package::new(my_deck)
    .media_files(["sound.mp3", "images/image.jpg"]);
my_package.write_to_file("output.apkg")?;
```

`media_files` takes the path (relative or absolute) to each file. To use them
in notes, first add a field to your model, and reference that field in your
template:

```rust
let my_model = Model::new(1091735104, "Simple Model with Media")
    .field(Field::new("Question"))
    .field(Field::new("Answer"))
    .field(Field::new("MyMedia")) // ADD THIS
    .template(Template::new(
        "Card 1",
        "{{Question}}<br>{{MyMedia}}", // AND THIS
        "{{FrontSide}}<hr id=\"answer\">{{Answer}}",
    ));
```

Then set the `MyMedia` field on your note to `[sound:sound.mp3]` for audio
and `<img src="image.jpg">` for images.

You *cannot* put `<img src="{MyMedia}">` in the template and `image.jpg` in
the field. See these sections in the Anki manual for more information:
[Importing Media](https://docs.ankiweb.net/#/importing?id=importing-media)
and [Media & LaTeX](https://docs.ankiweb.net/#/templates/fields?id=media-amp-latex).

You should only put the filename (aka basename) and not the full path in the
field; `<img src="images/image.jpg">` will *not* work. Media files should
have unique filenames: two distinct paths sharing a basename is an error
(`MediaBasenameCollision`), matching Anki's flat media folder.

## Note GUIDs

`Note`s have a `guid` that uniquely identifies the note. If you import a new
note that has the same GUID as an existing note, the new note will overwrite
the old one (as long as their models have the same fields).

This is an important feature if you want to be able to tweak the
design/content of your notes, regenerate your deck, and import the updated
version into Anki. Your notes need to have stable GUIDs in order for the new
note to replace the existing one.

By default, the GUID is a hash of all the field values (see
`genanki::guid_for`, which is byte-identical to Python's). This may not be
desirable if, for example, you add a new field with additional info that
doesn't change the identity of the note. You can override the GUID to hash
only the fields that identify the note:

```rust
let my_note = genanki::Note::new(my_model, ["a", "b", "extra"])?
    .with_guid(genanki::guid_for(&["a", "b"]));
```

## sort_field

Anki has a value for each `Note` called the `sort_field`. Anki uses this
value to sort the cards in the Browse interface. Anki also is happier if you
avoid having two notes with the same `sort_field`, although this isn't
strictly necessary. By default, the `sort_field` is the first field, but you
can change it per note:

```rust
let my_note = genanki::Note::new(my_model, ["a", "b"])?
    .with_sort_field("my sort key");
```

You can also change the field used for all notes of a model:

```rust
let my_model = Model::new(1607392319, "Simple Model")
    /* .field(...) ... */
    .sort_field_index(1); // 0 = first field, 1 = second, etc.
```

## Builtin Models

Five Anki-stock note types are shipped, byte-identical to Python genanki
v0.13.x `builtin_models.py`, and re-exported at the crate root:

| Static | Model id | Notes |
| ------ | -------- | ----- |
| `BASIC_MODEL` | 1559383000 | `Basic (genanki)`: `Front` / `Back`, one card. |
| `BASIC_AND_REVERSED_CARD_MODEL` | 1485830179 | Always generates the forward **and** the reversed card. |
| `BASIC_OPTIONAL_REVERSED_CARD_MODEL` | 1382232460 | Reversed card only when the `Add Reverse` field is non-empty. |
| `BASIC_TYPE_IN_THE_ANSWER_MODEL` | 1305534440 | `{{type:Back}}`: type the answer on the front. |
| `CLOZE_MODEL` | 1550428389 | Cloze deletions; fields `Text` + `Back Extra` (**two required**). |

All fields use the Arial font like Python's builtins, and the model names
carry the `(genanki)` suffix so Anki does not rename them on import (Anki's
own builtins have inconsistent ids; a plain `Basic` name would collide).

```rust
use genanki::{BASIC_MODEL, Deck, Note, Package};

let mut my_deck = Deck::new(2059400110, "Country Capitals");
my_deck.add_note(Note::new(&*BASIC_MODEL, ["Capital of Argentina", "Buenos Aires"])?);
Package::new(my_deck).write_to_file("output.apkg")?;
```

Note the `&*BASIC_MODEL` deref: the statics are `LazyLock<Model>`, and
`Note::new(&*BASIC_MODEL, ...)` works via `From<&Model> for Arc<Model>`. If
you write many notes, clone the static once into an `Arc<Model>` and reuse
it (cheaper than a per-note clone):

```rust
use std::sync::Arc;
let model = Arc::new((*BASIC_MODEL).clone());
for qa in [("Q1", "A1"), ("Q2", "A2")] {
    deck.add_note(Note::new(Arc::clone(&model), [qa.0, qa.1])?);
}
```

## Cloze

Use `CLOZE_MODEL` with exactly **two** fields: `Text` (containing
`{{cN::...}}` deletions) and `Back Extra` (optional extra shown on the
back; pass `""` if unused):

```rust
let my_note = genanki::Note::new(
    genanki::CLOZE_MODEL,
    ["{{c1::Rome}} is the capital of {{c2::Italy}}", ""],
)?;
```

One card is generated per unique deletion number (`{{c1::...}}` and
`{{c2::...}}` produce two cards). Passing a single field is an error
(`FieldCountMismatch`); unlike old Python genanki versions there is no
automatic empty-second-field padding or deprecation warning.

## FAQ

### My field data is getting garbled

If fields in your notes contain literal `<`, `>`, or `&` characters, you
need to HTML-encode them: field data is HTML, not plain text. There is no
`html.escape` in this crate (the Python equivalent is a stdlib function);
escape in your app code before constructing the note, for example:

```rust
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

let fields = ["AT&T was originally called", "Bell Telephone Company"]
    .map(|f| escape_html(f));
let my_note = genanki::Note::new(my_model, fields)?;
```

This applies even if the content is LaTeX; for example, write
`[latex]r &gt; g[/latex]` rather than `[latex]r > g[/latex]`.

As a safety net, `Note::new` logs a warning (via the `log` facade) when a
field contains tags the crate cannot classify as valid HTML, and
`genanki::note::find_invalid_html_tags` returns the offending substrings.

### How do I generate model/deck ids?

Anki ids are `i64` values. Generate one once in the `1 << 30` to `1 << 31`
range with any randomness source, then hardcode it as a literal in your
program so every deck you produce reuses the same id:

```rust
const MY_MODEL_ID: i64 = 1607392319;
const MY_DECK_ID: i64 = 2059400110;
```

## Non-goals (v1)

- **No Anki-addon collection write.** Python genanki's
  `write_to_collection_from_addon` is not ported; this crate only produces
  `.apkg` files.
- **No YAML template/field API.** Models are built with the Rust builders
  (`Model::new(...).field(...).template(...)`).
- **Write-only.** Reading, merging, or editing existing `.apkg` collections
  is out of scope.
- **No GUI import automation.** Anki desktop/AnkiDroid import the generated
  files themselves; the crate does not drive Anki.

## License

MIT
