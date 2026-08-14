//! Note GUID generation (`guid_for`) and Anki base91 alphabet.

use sha2::Digest;

/// Anki base91 alphabet used by [`guid_for`] (91 ASCII bytes, fixed order).
///
/// Identical to `genanki.util.BASE91_TABLE` (v0.13.0 / v1.13.1): lowercase,
/// uppercase, digits, then punctuation. No space, double quote, or backslash.
pub const BASE91_TABLE: &[u8; 91] =
    b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!#$%&()*+,-./:;<=>?@[]^_`{|}~";

/// Compute a note GUID the same way Python genanki `guid_for` does.
///
/// Fields are joined with `"__"`, the result is SHA-256'd as UTF-8, the first
/// 8 bytes of the digest are read as a big-endian integer, and that integer is
/// encoded with [`BASE91_TABLE`] (Anki's base91). Byte-identical to
/// `genanki.util.guid_for` for the same values.
///
/// An empty slice joins to the empty string, same as Python's no-arg call.
///
/// Fields are joined with `"__"` before hashing. A single field value that
/// itself contains `"__"` is therefore indistinguishable from multiple fields
/// (same as Python genanki). Choose identity fields with that in mind.
///
/// # Examples
///
/// ```
/// use genanki::guid_for;
/// assert_eq!(guid_for(&["a", "b"]), "q/([o$8RAO");
/// // A field that embeds "__" joins identically to two fields (Python parity):
/// assert_eq!(guid_for(&["a__b"]), guid_for(&["a", "b"]));
/// ```
pub fn guid_for(values: &[&str]) -> String {
    let hash_str = values.join("__");

    // SHA-256 of the joined string; first 8 bytes as a big-endian integer.
    let digest = sha2::Sha256::digest(hash_str.as_bytes());
    let mut hash_bytes = [0u8; 8];
    hash_bytes.copy_from_slice(&digest[..8]);
    let hash_int = u64::from_be_bytes(hash_bytes);

    base91_encode(hash_int)
}

/// Encode a non-negative integer in Anki's base91 (most significant digit first).
///
/// Mirrors Python's `while hash_int > 0` loop: zero encodes to the empty string.
fn base91_encode(mut n: u64) -> String {
    const RADIX: u64 = BASE91_TABLE.len() as u64;
    let mut digits_reversed = Vec::new();
    while n > 0 {
        digits_reversed.push(BASE91_TABLE[(n % RADIX) as usize] as char);
        n /= RADIX;
    }
    digits_reversed.iter().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base91_table_len_is_91() {
        assert_eq!(BASE91_TABLE.len(), 91);
    }

    #[test]
    fn base91_table_bytes_match_anki_order() {
        const EXPECTED: &[u8; 91] =
            b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!#$%&()*+,-./:;<=>?@[]^_`{|}~";
        assert_eq!(BASE91_TABLE, EXPECTED);
    }

    #[test]
    fn base91_table_excludes_quote_backslash_space() {
        assert!(!BASE91_TABLE.contains(&b'"'));
        assert!(!BASE91_TABLE.contains(&b'\\'));
        assert!(!BASE91_TABLE.contains(&b' '));
    }

    #[test]
    fn guid_for_a_b_matches_python() {
        // Python genanki util.guid_for("a", "b") on v0.13.0 / v1.13.1.
        assert_eq!(guid_for(&["a", "b"]), "q/([o$8RAO");
    }

    #[test]
    fn guid_for_goldens_table() {
        // Precomputed with CPython 3 + stdlib hashlib against the algorithm in
        // kerrickstaley/genanki v0.13.0 `util.py` (byte-identical to v1.13.1).
        // Regenerator (not run in CI):
        //   python3 - <<'PY'
        //   import hashlib
        //   T = list("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!#$%&()*+,-./:;<=>?@[]^_`{|}~")
        //   def g(*v):
        //       s = '__'.join(str(x) for x in v)
        //       h = hashlib.sha256(s.encode('utf-8')).digest()[:8]
        //       n = int.from_bytes(h, 'big')
        //       r = []
        //       while n > 0:
        //           r.append(T[n % 91]); n //= 91
        //       return ''.join(reversed(r))
        //   print(g('a','b'))
        //   PY
        let cases: &[(&[&str], &str)] = &[
            (&["a", "b"], "q/([o$8RAO"),
            (&[""], "ME_YHw2?15"),
            (&["", ""], "z+VBQ9+v.E"),
            (&["hello"], "hZ%+.BW-%^"),
            (&["Capital of Argentina", "Buenos Aires"], "HSnG{z%dU<"),
            (&["日本語", "テスト"], "C#Zh1EL^|P"),
            (&["emoji 😀", "x"], "GxV$K=ya?h"),
            (&["a", "b", "c"], "xEWY5:a/2E"),
            (&["only-one"], "zRins]8c)T"),
            (&["with__double", "underscore"], "kE!|u[<uRg"),
            (&["\u{1f}"], "RisW_+fz{*"),
            (&[" leading", "trailing "], "r{^OG#`_~o"),
            (&["line\nbreak"], "r*A<&TFxl@"),
            (&[], "ME_YHw2?15"),
        ];
        for (values, expected) in cases {
            assert_eq!(guid_for(values), *expected, "values: {values:?}");
        }
    }

    #[test]
    fn guid_for_empty_slice_matches_single_empty_string() {
        // Both hash the empty join string (Python: guid_for() == guid_for("")).
        assert_eq!(guid_for(&[]), guid_for(&[""]));
    }

    #[test]
    fn guid_for_is_stable() {
        assert_eq!(guid_for(&["a", "b"]), guid_for(&["a", "b"]));
    }

    #[test]
    fn guid_for_field_order_matters() {
        assert_ne!(guid_for(&["a", "b"]), guid_for(&["b", "a"]));
    }

    #[test]
    fn guid_for_separator_is_double_underscore() {
        // A single field containing "__" joins to the same string as two fields:
        // ['a','b'].join("__") == ['a__b'].join("__") == "a__b".
        assert_eq!(guid_for(&["a__b"]), guid_for(&["a", "b"]));
    }

    #[test]
    fn base91_encode_zero_is_empty_string() {
        // Python's `while hash_int > 0` loop never runs for 0.
        assert_eq!(base91_encode(0), "");
    }

    #[test]
    fn base91_encode_small_values() {
        // Characterization of the alphabet positions: BASE91_TABLE[1] == 'b',
        // and 91 == 1 * 91 + 0 encodes as 'a' then 'b', reversed -> "ba".
        assert_eq!(base91_encode(1), "b");
        assert_eq!(base91_encode(91), "ba");
    }
}
