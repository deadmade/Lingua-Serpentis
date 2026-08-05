// `is_latin` below is the entire public surface. The morphology engine —
// normalization, the stem/ending split, the Whitaker table builder and its
// `Tables`/`Group`/`Pos` types — is an implementation detail that is free to
// change, so these modules stay private (design spec §6.1 and §6.5).
mod analyze;
mod model;
mod whitaker;

use model::Tables;
use std::sync::OnceLock;

/// Whitaker's WORDS data, embedded so the crate is self-contained at runtime.
/// See `data/NOTICE` for provenance and licensing.
const DICTLINE: &str = include_str!("../data/DICTLINE.GEN");
const INFLECTS: &str = include_str!("../data/INFLECTS.LAT");
const UNIQUES: &str = include_str!("../data/UNIQUES.LAT");

/// Our own additions, kept deliberately tiny. See `data/supplement.txt` for the
/// policy governing what may go in it, and defectus D-18 for why it exists.
const SUPPLEMENTUM: &str = include_str!("../data/supplement.txt");

/// The compiled dictionary, built once on first use.
fn tables() -> &'static Tables {
    static TABLES: OnceLock<Tables> = OnceLock::new();
    TABLES.get_or_init(|| {
        let mut t = whitaker::build_tables(DICTLINE, INFLECTS, UNIQUES);
        whitaker::add_supplement(&mut t, SUPPLEMENTUM);
        // Last: the bounds must see every stem and ending that will ever be in
        // the table, or `analyze` would skip a split that could have matched.
        t.narrow_bounds();
        t
    })
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
    fn accepts_comparatives_and_superlatives() {
        // These endings live under the "ADJ 0 0" wildcard group in INFLECTS.
        // Before that group was merged into the concrete declensions, every one
        // of these warned — including the notorious `maximum` accepted but
        // `minimum` rejected split.
        for word in ["melior", "optimus", "altior", "altissimus", "longior",
                     "fortissimus", "minimus", "minimum", "maximum"] {
            assert!(is_latin(word), "expected {word} to be recognized as Latin");
        }
    }

    #[test]
    fn accepts_perfect_system_verb_forms() {
        // The perfect-system personal endings live under the "V 0 0" wildcard
        // group, which is conjugation-independent.
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
        // Defectus D-18. The identifier-checker design §12 left open "whether a
        // handful of common programmer-Latin infinitives used as function names
        // (e.g. `summare`) are present in Whitaker or need a tiny supplemental
        // list", to be settled by whether a real example failed. One does:
        // `examples/calc.liss` declares `munus summare`, and warned.
        assert!(is_latin("summare"));
    }

    #[test]
    fn the_supplement_does_not_open_the_gate() {
        // The supplement is a word list, not a relaxation: everything the
        // checker rejected before it must still be rejected.
        for word in ["result", "getData", "myVar", "counter", "foobar"] {
            assert!(!is_latin(word), "expected {word} to be rejected");
        }
    }

    #[test]
    fn the_dictionary_bounds_stay_small() {
        // Defectus D-23's fix skips stem/ending splits that cannot match,
        // bounded by the longest stem and the longest ending the compiled
        // dictionary actually contains. The speedup is only real while those
        // stay small, and a data-file or supplement change could widen them
        // silently — so pin them. `whole` is exempt: it is matched by one hash
        // of the entire word, not by splitting.
        let t = tables();
        assert_eq!(t.max_stem, 18, "longest stem in DICTLINE.GEN");
        assert_eq!(t.max_ending, 7, "longest ending in INFLECTS.LAT");
    }
}
