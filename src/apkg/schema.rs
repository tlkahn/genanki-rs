//! SQLite schema DDL and schema version constants. (Phase 3)

/// SQLite DDL executed when creating a new `collection.anki2`.
///
/// Verbatim copy of `APKG_SCHEMA` from kerrickstaley/genanki v0.13.0
/// `apkg_schema.py` (byte-identical to v1.13.1), including the quirky
/// `notes.sfld integer` typing and leading/trailing newlines. Do not "fix"
/// anything: Anki is picky about this exact DDL.
pub const APKG_SCHEMA: &str = r#"
CREATE TABLE col (
    id              integer primary key,
    crt             integer not null,
    mod             integer not null,
    scm             integer not null,
    ver             integer not null,
    dty             integer not null,
    usn             integer not null,
    ls              integer not null,
    conf            text not null,
    models          text not null,
    decks           text not null,
    dconf           text not null,
    tags            text not null
);
CREATE TABLE notes (
    id              integer primary key,   /* 0 */
    guid            text not null,         /* 1 */
    mid             integer not null,      /* 2 */
    mod             integer not null,      /* 3 */
    usn             integer not null,      /* 4 */
    tags            text not null,         /* 5 */
    flds            text not null,         /* 6 */
    sfld            integer not null,      /* 7 */
    csum            integer not null,      /* 8 */
    flags           integer not null,      /* 9 */
    data            text not null          /* 10 */
);
CREATE TABLE cards (
    id              integer primary key,   /* 0 */
    nid             integer not null,      /* 1 */
    did             integer not null,      /* 2 */
    ord             integer not null,      /* 3 */
    mod             integer not null,      /* 4 */
    usn             integer not null,      /* 5 */
    type            integer not null,      /* 6 */
    queue           integer not null,      /* 7 */
    due             integer not null,      /* 8 */
    ivl             integer not null,      /* 9 */
    factor          integer not null,      /* 10 */
    reps            integer not null,      /* 11 */
    lapses          integer not null,      /* 12 */
    left            integer not null,      /* 13 */
    odue            integer not null,      /* 14 */
    odid            integer not null,      /* 15 */
    flags           integer not null,      /* 16 */
    data            text not null          /* 17 */
);
CREATE TABLE revlog (
    id              integer primary key,
    cid             integer not null,
    usn             integer not null,
    ease            integer not null,
    ivl             integer not null,
    lastIvl         integer not null,
    factor          integer not null,
    time            integer not null,
    type            integer not null
);
CREATE TABLE graves (
    usn             integer not null,
    oid             integer not null,
    type            integer not null
);
CREATE INDEX ix_notes_usn on notes (usn);
CREATE INDEX ix_cards_usn on cards (usn);
CREATE INDEX ix_revlog_usn on revlog (usn);
CREATE INDEX ix_cards_nid on cards (nid);
CREATE INDEX ix_cards_sched on cards (did, queue, due);
CREATE INDEX ix_revlog_cid on revlog (cid);
CREATE INDEX ix_notes_csum on notes (csum);
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_is_non_empty() {
        assert!(!APKG_SCHEMA.trim().is_empty());
    }

    #[test]
    fn schema_creates_core_tables() {
        for table in [
            "CREATE TABLE col",
            "CREATE TABLE notes",
            "CREATE TABLE cards",
            "CREATE TABLE revlog",
            "CREATE TABLE graves",
        ] {
            assert!(APKG_SCHEMA.contains(table), "missing {table}");
        }
    }

    #[test]
    fn schema_creates_expected_indexes() {
        for idx in [
            "ix_notes_usn",
            "ix_cards_usn",
            "ix_revlog_usn",
            "ix_cards_nid",
            "ix_cards_sched",
            "ix_revlog_cid",
            "ix_notes_csum",
        ] {
            assert!(APKG_SCHEMA.contains(idx), "missing index {idx}");
        }
    }
}
