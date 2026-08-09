// `is_latin` is the entire public surface; the morphology engine stays private (design §6.1, §6.5).
mod analyze;
mod model;
mod whitaker;

use model::Tables;
use std::sync::OnceLock;

/// Whitaker's WORDS data, embedded; see `data/NOTICE` for provenance and licensing.
const DICTLINE: &str = include_str!("../data/DICTLINE.GEN");
const INFLECTS: &str = include_str!("../data/INFLECTS.LAT");
const UNIQUES: &str = include_str!("../data/UNIQUES.LAT");

/// Our own additions; `data/supplement.txt` carries the policy. D-18
const SUPPLEMENTUM: &str = include_str!("../data/supplement.txt");

fn tables() -> &'static Tables {
    static TABLES: OnceLock<Tables> = OnceLock::new();
    TABLES.get_or_init(|| {
        let mut t = whitaker::build_tables(DICTLINE, INFLECTS, UNIQUES);
        whitaker::add_supplement(&mut t, SUPPLEMENTUM);
        t.narrow_bounds();
        t
    })
}

/// Case- and orthography-insensitive (`v`/`u` and `j`/`i`); empty input is `false`.
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
        assert!(is_latin("memento"));
    }

    #[test]
    fn is_case_insensitive() {
        assert!(is_latin("AMO"));
        assert!(is_latin("Numerus"));
    }

    #[test]
    fn accepts_comparatives_and_superlatives() {
        for word in ["melior", "optimus", "altior", "altissimus", "longior",
                     "fortissimus", "minimus", "minimum", "maximum"] {
            assert!(is_latin(word), "expected {word} to be recognized as Latin");
        }
    }

    #[test]
    fn accepts_perfect_system_verb_forms() {
        for word in ["amavi", "amavit", "vidi", "vidit", "monui", "monuit"] {
            assert!(is_latin(word), "expected {word} to be recognized as Latin");
        }
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

    #[test]
    fn accepts_the_supplemented_programmer_infinitive() {
        // D-18: `examples/calc.liss` declares `munus summare`, and warned.
        assert!(is_latin("summare"));
    }

    #[test]
    fn the_supplement_does_not_open_the_gate() {
        for word in ["result", "getData", "myVar", "counter", "foobar"] {
            assert!(!is_latin(word), "expected {word} to be rejected");
        }
    }

    #[test]
    fn the_dictionary_bounds_stay_small() {
        // D-23
        let t = tables();
        assert_eq!(t.max_stem, 18, "longest stem in DICTLINE.GEN");
        assert_eq!(t.max_ending, 7, "longest ending in INFLECTS.LAT");
    }
}
