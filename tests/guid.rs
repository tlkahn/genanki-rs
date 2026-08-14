//! Integration tests for the public crate-root re-exports (Phase 1).

#[test]
fn public_path_guid_for() {
    assert_eq!(genanki::guid_for(&["a", "b"]), "q/([o$8RAO");
}

#[test]
fn public_module_path_base91_table() {
    assert_eq!(genanki::guid::BASE91_TABLE.len(), 91);
    assert_eq!(
        genanki::guid::BASE91_TABLE,
        b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!#$%&()*+,-./:;<=>?@[]^_`{|}~"
    );
}
