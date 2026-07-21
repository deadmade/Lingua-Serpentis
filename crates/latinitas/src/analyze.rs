/// Fold a word into the orthographic form the dictionary is stored in:
/// lowercase, with `v` written as `u` and `j` as `i`.
pub fn normalize(word: &str) -> String {
    word.chars()
        .map(|c| match c.to_ascii_lowercase() {
            'v' => 'u',
            'j' => 'i',
            other => other,
        })
        .collect()
}

use crate::model::Tables;

/// True when `word` is a recognizable Latin word form: either an indeclinable
/// listed whole, or some split into `stem + ending` where the stem is in the
/// dictionary and the ending is valid for one of that stem's groups.
pub fn analyze(tables: &Tables, word: &str) -> bool {
    let w = normalize(word);
    if w.is_empty() {
        return false;
    }
    // Byte-slicing below assumes one byte per character. Latin words are ASCII,
    // so anything else is not Latin (and would otherwise risk a panic).
    if !w.is_ascii() {
        return false;
    }
    if tables.whole.contains(&w) {
        return true;
    }
    for i in 0..=w.len() {
        let (stem, ending) = (&w[..i], &w[i..]);
        let Some(groups) = tables.stems.get(stem) else {
            continue;
        };
        for g in groups {
            if tables.endings.get(g).is_some_and(|ends| ends.contains(ending)) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Group, Pos, Tables};

    #[test]
    fn lowercases() {
        assert_eq!(normalize("AMARE"), "amare");
    }

    #[test]
    fn maps_v_to_u_and_j_to_i() {
        // Classical Latin has no distinct v/j; the dictionary is stored in i/u
        // form, so both sides are folded the same way.
        assert_eq!(normalize("Verbum"), "uerbum");
        assert_eq!(normalize("Julius"), "iulius");
    }

    #[test]
    fn leaves_other_characters_alone() {
        // Underscores/digits survive normalization so they can fail lookup and
        // correctly produce a warning.
        assert_eq!(normalize("my_var2"), "my_uar2");
    }

    /// A tiny controlled dictionary: the 2nd-declension noun stem `numer-`,
    /// the 1st-conjugation verb stem `am-`, and the indeclinable `et`.
    fn fixture() -> Tables {
        let mut t = Tables::new();
        let noun2 = Group { pos: Pos::Noun, decl: 2 };
        let verb1 = Group { pos: Pos::Verb, decl: 1 };

        t.stems.insert("numer".to_string(), vec![noun2]);
        t.stems.insert("am".to_string(), vec![verb1]);

        t.endings.insert(
            noun2,
            ["us", "i", "o", "um", "e"].iter().map(|s| s.to_string()).collect(),
        );
        t.endings.insert(
            verb1,
            ["o", "as", "at", "are"].iter().map(|s| s.to_string()).collect(),
        );

        t.whole.insert("et".to_string());
        t
    }

    #[test]
    fn accepts_every_inflected_form_of_a_known_noun() {
        let t = fixture();
        for word in ["numerus", "numeri", "numero", "numerum", "numere"] {
            assert!(analyze(&t, word), "expected {word} to be Latin");
        }
    }

    #[test]
    fn accepts_inflected_verb_forms() {
        let t = fixture();
        for word in ["amo", "amas", "amat", "amare"] {
            assert!(analyze(&t, word), "expected {word} to be Latin");
        }
    }

    #[test]
    fn accepts_indeclinable_whole_words() {
        assert!(analyze(&fixture(), "et"));
    }

    #[test]
    fn is_case_insensitive() {
        assert!(analyze(&fixture(), "NUMERUS"));
    }

    #[test]
    fn rejects_non_latin_words() {
        let t = fixture();
        for word in ["result", "myVar", "getData", "xyz", "foobar"] {
            assert!(!analyze(&t, word), "expected {word} to be rejected");
        }
    }

    #[test]
    fn rejects_a_bare_stem_with_an_invalid_ending() {
        // `numer` is a real stem but `zzz` is not a 2nd-declension ending.
        assert!(!analyze(&fixture(), "numerzzz"));
    }

    #[test]
    fn rejects_empty_and_non_ascii_input() {
        let t = fixture();
        assert!(!analyze(&t, ""));
        assert!(!analyze(&t, "café"));
    }
}
