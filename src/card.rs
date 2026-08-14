//! Card representation and scheduling data. (Phase 3)

/// A single generated card of a note.
///
/// Phase 4 maps `suspend` to `queue = -1` (else `0`) and `due` from the note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    /// Ordinal of the template this card was generated from.
    pub ord: i32,
    /// Suspend flag; suspended cards are excluded from review scheduling.
    pub suspend: bool,
}

impl Card {
    /// Create a card for template ordinal `ord` with `suspend = false`.
    #[must_use]
    pub fn new(ord: i32) -> Self {
        Self {
            ord,
            suspend: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_defaults() {
        let mut c = Card::new(2);
        assert_eq!(c.ord, 2);
        assert!(!c.suspend);
        c.suspend = true;
        assert!(c.suspend);
    }
}
