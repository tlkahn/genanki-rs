//! Low-level SQLite writes backing the package writer. (Phase 4)

/// Monotonic id generator shared across notes and cards in package order.
///
/// Mirrors Python genanki's `itertools.count(int(timestamp * 1000))`: the
/// first id is the start value, each subsequent id increments by one.
pub struct IdGen {
    /// The next id to hand out.
    next: i64,
}

impl IdGen {
    /// Create a generator whose first id is `start`.
    #[must_use]
    pub fn new(start: i64) -> Self {
        Self { next: start }
    }

    /// Return the current id and advance.
    pub fn next_id(&mut self) -> i64 {
        let v = self.next;
        self.next = self.next.checked_add(1).expect("id_gen overflow");
        v
    }
}

/// Apply `APKG_SCHEMA` DDL and seed the single `col` row (`APKG_COL`).
///
/// Runs before any note/card inserts; equivalent to Python genanki running
/// `APKG_SCHEMA` then `APKG_COL` on a fresh `collection.anki2`.
pub fn init_schema(conn: &rusqlite::Connection) -> crate::Result<()> {
    conn.execute_batch(crate::apkg::schema::APKG_SCHEMA)?;
    conn.execute_batch(crate::apkg::col::APKG_COL)?;
    Ok(())
}

/// Insert one `notes` row, returning the note id (the id_gen value we
/// inserted, matching Python's `cursor.lastrowid` semantics for explicit ids).
///
/// Column order mirrors Python genanki `Note.write_to_db` (v0.13.0):
/// `id, guid, mid, mod, usn, tags, flds, sfld, csum, flags, data`.
pub(crate) fn insert_note(
    conn: &rusqlite::Connection,
    note: &crate::note::Note,
    timestamp_secs: f64,
    id_gen: &mut IdGen,
) -> crate::Result<i64> {
    let note_id = id_gen.next_id();
    conn.execute(
        "INSERT INTO notes VALUES(?,?,?,?,?,?,?,?,?,?,?)",
        rusqlite::params![
            note_id,
            note.guid(),
            note.model().id,
            timestamp_secs as i64,
            -1i64,
            crate::note::format_tags(note.tags()),
            crate::note::format_fields(note.fields()),
            note.sort_field(),
            0i64, // csum
            0i64, // flags
            "",   // data
        ],
    )?;
    Ok(note_id)
}

/// Insert one `cards` row.
///
/// Column order mirrors Python genanki `Card.write_to_db` (v0.13.0):
/// `id, nid, did, ord, mod, usn, type, queue, due, ivl, factor, reps,
/// lapses, left, odue, odid, flags, data`. `queue` is `-1` when suspended.
pub(crate) fn insert_card(
    conn: &rusqlite::Connection,
    card: &crate::card::Card,
    note_id: i64,
    deck_id: i64,
    timestamp_secs: f64,
    due: i64,
    id_gen: &mut IdGen,
) -> crate::Result<()> {
    let queue = if card.suspend { -1i64 } else { 0i64 };
    conn.execute(
        "INSERT INTO cards VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
        rusqlite::params![
            id_gen.next_id(),
            note_id,
            deck_id,
            card.ord,
            timestamp_secs as i64,
            -1i64,
            0i64, // type (0 = non-Cloze)
            queue,
            due,
            0i64, // ivl
            0i64, // factor
            0i64, // reps
            0i64, // lapses
            0i64, // left
            0i64, // odue
            0i64, // odid
            0i64, // flags
            "",   // data
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_gen_starts_at_given_value_and_increments() {
        let mut g = IdGen::new(1000);
        assert_eq!(g.next_id(), 1000);
        assert_eq!(g.next_id(), 1001);
        assert_eq!(g.next_id(), 1002);
    }

    #[test]
    fn id_gen_zero_start() {
        let mut g = IdGen::new(0);
        assert_eq!(g.next_id(), 0);
        assert_eq!(g.next_id(), 1);
    }

    #[test]
    fn init_schema_creates_col_with_default_seed() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let conn = rusqlite::Connection::open(tmp.path()).unwrap();
        init_schema(&conn).unwrap();

        let n: i64 = conn
            .query_row("SELECT count(*) FROM col", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1, "exactly one col row seeded");

        let decks: String = conn
            .query_row("SELECT decks FROM col", [], |r| r.get(0))
            .unwrap();
        assert!(decks.contains("Default"), "seed default deck: {decks}");

        // All schema tables exist.
        for table in ["col", "notes", "cards", "revlog", "graves"] {
            let cnt: i64 = conn
                .query_row(
                    "SELECT count(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [table],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(cnt, 1, "table {table} missing");
        }
    }

    fn simple_model() -> crate::model::Model {
        crate::model::Model::new(1, "m")
            .field(crate::model::Field::new("Q"))
            .field(crate::model::Field::new("A"))
            .template(crate::model::Template::new("c", "{{Q}}", "{{A}}"))
    }

    #[test]
    fn insert_note_and_card_rows() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let conn = rusqlite::Connection::open(tmp.path()).unwrap();
        init_schema(&conn).unwrap();

        let mut note = crate::note::Note::new(simple_model(), ["hello", "world"])
            .unwrap()
            .with_tags(["foo", "bar"])
            .unwrap()
            .with_due(5);
        let card = note.cards().unwrap()[0].clone(); // ord 0, not suspended

        let mut id_gen = IdGen::new(1_000);
        let note_id = insert_note(&conn, &note, 123.0, &mut id_gen).unwrap();
        insert_card(&conn, &card, note_id, 42, 123.0, note.due(), &mut id_gen).unwrap();

        let n = conn
            .query_row(
                "SELECT id, guid, mid, mod, usn, tags, flds, sfld, csum, flags, data FROM notes",
                [],
                |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, i64>(2)?,
                        r.get::<_, i64>(3)?,
                        r.get::<_, i64>(4)?,
                        r.get::<_, String>(5)?,
                        r.get::<_, String>(6)?,
                        r.get::<_, String>(7)?,
                        r.get::<_, i64>(8)?,
                        r.get::<_, i64>(9)?,
                        r.get::<_, String>(10)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(n.0, 1_000, "note id from id_gen");
        assert_eq!(n.1, note.guid());
        assert_eq!(n.2, 1, "mid from model id");
        assert_eq!(n.3, 123, "mod = timestamp as i64");
        assert_eq!(n.4, -1, "usn");
        assert_eq!(n.5, " foo bar ", "tags wrapped in spaces");
        assert_eq!(n.6, "hello\x1fworld", "flds unit-separated");
        assert_eq!(n.7, "hello", "sfld = sort field text");
        assert_eq!(n.8, 0, "csum");
        assert_eq!(n.9, 0, "flags");
        assert_eq!(n.10, "", "data");

        let c = conn
            .query_row(
                "SELECT id, nid, did, ord, mod, usn, type, queue, due, ivl, factor, reps, lapses, left, odue, odid, flags, data FROM cards",
                [],
                |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, i64>(1)?,
                        r.get::<_, i64>(2)?,
                        r.get::<_, i64>(3)?,
                        r.get::<_, i64>(4)?,
                        r.get::<_, i64>(5)?,
                        r.get::<_, i64>(6)?,
                        r.get::<_, i64>(7)?,
                        r.get::<_, i64>(8)?,
                        r.get::<_, i64>(9)?,
                        r.get::<_, i64>(10)?,
                        r.get::<_, i64>(11)?,
                        r.get::<_, i64>(12)?,
                        r.get::<_, i64>(13)?,
                        r.get::<_, i64>(14)?,
                        r.get::<_, i64>(15)?,
                        r.get::<_, i64>(16)?,
                        r.get::<_, String>(17)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(c.0, 1_001, "card id next from id_gen");
        assert_eq!(c.1, note_id, "nid links to note");
        assert_eq!(c.2, 42, "did = deck id");
        assert_eq!(c.3, 0, "ord");
        assert_eq!(c.4, 123, "mod = timestamp as i64");
        assert_eq!(c.5, -1, "usn");
        assert_eq!(c.6, 0, "type");
        assert_eq!(c.7, 0, "queue 0 when not suspended");
        assert_eq!(c.8, 5, "due from note");
        for v in [c.9, c.10, c.11, c.12, c.13, c.14, c.15, c.16] {
            assert_eq!(v, 0, "zeroed scheduling column");
        }
        assert_eq!(c.17, "", "data");
    }

    #[test]
    fn suspend_sets_queue_neg_one_and_due_propagates() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let conn = rusqlite::Connection::open(tmp.path()).unwrap();
        init_schema(&conn).unwrap();

        // Two templates -> two cards.
        let model = crate::model::Model::new(1, "cn")
            .field(crate::model::Field::new("Trad"))
            .field(crate::model::Field::new("Simpl"))
            .field(crate::model::Field::new("Eng"))
            .template(crate::model::Template::new("t0", "{{Trad}}", "x"))
            .template(crate::model::Template::new("t1", "{{Simpl}}", "x"));
        let mut note = crate::note::Note::new(model, ["a", "b", "c"])
            .unwrap()
            .with_due(9);
        note.cards_mut().unwrap()[1].suspend = true;

        let mut id_gen = IdGen::new(0);
        let note_id = insert_note(&conn, &note, 1.0, &mut id_gen).unwrap();
        let due = note.due();
        for card in note.cards().unwrap() {
            insert_card(&conn, card, note_id, 7, 1.0, due, &mut id_gen).unwrap();
        }

        let rows: Vec<(i64, i64, i64)> = conn
            .prepare("SELECT ord, queue, due FROM cards ORDER BY id")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .collect::<std::result::Result<_, _>>()
            .unwrap();
        assert_eq!(rows, vec![(0, 0, 9), (1, -1, 9)]);
    }
}
