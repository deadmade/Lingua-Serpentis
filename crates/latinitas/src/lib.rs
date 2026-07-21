pub mod analyze;
pub mod model;
pub mod whitaker;

use model::Tables;
use std::sync::OnceLock;

/// Whitaker's WORDS data, embedded so the crate is self-contained at runtime.
/// See `data/NOTICE` for provenance and licensing.
const DICTLINE: &str = include_str!("../data/DICTLINE.GEN");
const INFLECTS: &str = include_str!("../data/INFLECTS.LAT");
const UNIQUES: &str = include_str!("../data/UNIQUES.LAT");

/// The compiled dictionary, built once on first use.
fn tables() -> &'static Tables {
    static TABLES: OnceLock<Tables> = OnceLock::new();
    TABLES.get_or_init(|| whitaker::build_tables(DICTLINE, INFLECTS, UNIQUES))
}

/// True when `word` is a recognizable Latin word form — any inflection of a
/// dictionary entry, or an indeclinable word. Case- and orthography-insensitive
/// (`v`/`u` and `j`/`i` are equivalent). Empty or non-ASCII input is `false`.
pub fn is_latin(word: &str) -> bool {
    analyze::analyze(tables(), word)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_common_latin_nouns_in_several_cases() {
        for word in ["numerus", "numeri", "numerum", "rex", "regis"] {
            assert!(is_latin(word), "expected {word} to be recognized as Latin");
        }
    }

    #[test]
    fn accepts_common_latin_verbs_and_adjectives() {
        for word in ["amo", "amare", "bonus", "bona"] {
            assert!(is_latin(word), "expected {word} to be recognized as Latin");
        }
    }

    #[test]
    fn accepts_words_used_in_the_example_programs() {
        for word in ["summa", "pecunia"] {
            assert!(is_latin(word), "expected {word} to be recognized as Latin");
        }
    }

    #[test]
    fn accepts_irregular_forms_from_the_uniques_table() {
        // "memento" (imperative of `meminisse`, "remember!") is a flush-left
        // entry in UNIQUES.LAT.
        assert!(is_latin("memento"));
    }

    #[test]
    fn is_case_insensitive() {
        assert!(is_latin("AMO"));
        assert!(is_latin("Numerus"));
    }

    #[test]
    fn rejects_english_programmer_names() {
        for word in ["result", "getData", "myVar", "counter", "foobar"] {
            assert!(!is_latin(word), "expected {word} to be rejected");
        }
    }

    #[test]
    fn rejects_empty_input() {
        assert!(!is_latin(""));
    }
}
