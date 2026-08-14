//! Seed `INSERT` for the single `col` row (`APKG_COL`).

/// Seed `INSERT` for the single `col` row (default deck / conf / dconf).
///
/// Verbatim copy of `APKG_COL` from kerrickstaley/genanki v0.13.0
/// `apkg_col.py` (byte-identical to v1.13.1), including internal JSON
/// whitespace and leading/trailing newlines. Do not re-pretty-print.
pub const APKG_COL: &str = r#"
INSERT INTO col VALUES(
    null,
    1411124400,
    1425279151694,
    1425279151690,
    11,
    0,
    0,
    0,
    '{
        "activeDecks": [
            1
        ],
        "addToCur": true,
        "collapseTime": 1200,
        "curDeck": 1,
        "curModel": "1425279151691",
        "dueCounts": true,
        "estTimes": true,
        "newBury": true,
        "newSpread": 0,
        "nextPos": 1,
        "sortBackwards": false,
        "sortType": "noteFld",
        "timeLim": 0
    }',
    '{}',
    '{
        "1": {
            "collapsed": false,
            "conf": 1,
            "desc": "",
            "dyn": 0,
            "extendNew": 10,
            "extendRev": 50,
            "id": 1,
            "lrnToday": [
                0,
                0
            ],
            "mod": 1425279151,
            "name": "Default",
            "newToday": [
                0,
                0
            ],
            "revToday": [
                0,
                0
            ],
            "timeToday": [
                0,
                0
            ],
            "usn": 0
        }
    }',
    '{
        "1": {
            "autoplay": true,
            "id": 1,
            "lapse": {
                "delays": [
                    10
                ],
                "leechAction": 0,
                "leechFails": 8,
                "minInt": 1,
                "mult": 0
            },
            "maxTaken": 60,
            "mod": 0,
            "name": "Default",
            "new": {
                "bury": true,
                "delays": [
                    1,
                    10
                ],
                "initialFactor": 2500,
                "ints": [
                    1,
                    4,
                    7
                ],
                "order": 1,
                "perDay": 20,
                "separate": true
            },
            "replayq": true,
            "rev": {
                "bury": true,
                "ease4": 1.3,
                "fuzz": 0.05,
                "ivlFct": 1,
                "maxIvl": 36500,
                "minSpace": 1,
                "perDay": 100
            },
            "timer": 0,
            "usn": 0
        }
    }',
    '{}'
);
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn col_is_non_empty() {
        assert!(!APKG_COL.trim().is_empty());
    }

    #[test]
    fn col_is_insert_into_col() {
        assert!(APKG_COL.contains("INSERT INTO col VALUES"));
    }

    #[test]
    fn col_seed_has_default_deck_name() {
        assert!(APKG_COL.contains("\"name\": \"Default\""));
    }

    #[test]
    fn col_preserves_cur_model_token() {
        assert!(
            APKG_COL.contains("\"curModel\": \"1425279151691\""),
            "col seed curModel token drifted"
        );
    }

    #[test]
    fn col_bytes_match_upstream_v0_13_0_fingerprint() {
        // SHA-256 of the APKG_COL string body from kerrickstaley/genanki
        // v0.13.0 apkg_col.py (byte-identical on v1.13.1). Includes leading
        // and trailing newlines from the Python triple-quoted string.
        // Regenerator (not run in CI; paste into test module comments):
        //   python3 - <<'PY'
        //   import hashlib, re, pathlib, sys
        //   text = pathlib.Path(sys.argv[1]).read_text()
        //   body = re.search(r'pub const APKG_COL: &str = r#"(.*?)"#;', text, re.S).group(1)
        //   print(len(body.encode()), hashlib.sha256(body.encode()).hexdigest())
        //   PY
        //   # src/apkg/col.rs APKG_COL
        use sha2::{Digest, Sha256};
        let dig = Sha256::digest(APKG_COL.as_bytes());
        assert_eq!(APKG_COL.len(), 2271, "APKG_COL UTF-8 length drifted");
        assert_eq!(
            format!("{dig:x}"),
            "9ce03b85b9fddde5fcf3e09dfa0962c461b4e593cf38ceb6371f2536bf1cb1db",
            "APKG_COL drifted from genanki v0.13.0"
        );
    }
}
