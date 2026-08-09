use crate::model::Tables;

/// Fold to the dictionary's orthography: lowercase, `v`→`u`, `j`→`i`.
pub fn normalize(word: &str) -> String {
    word.chars()
        .map(|c| match c.to_ascii_lowercase() {
            'v' => 'u',
            'j' => 'i',
            other => other,
        })
        .collect()
}

/// An indeclinable listed whole, or a `stem + ending` split the tables allow.
pub fn analyze(tables: &Tables, word: &str) -> bool {
    let w = normalize(word);
    if w.is_empty() {
        return false;
    }
    // the byte-slicing below assumes one byte per character
    if !w.is_ascii() {
        return false;
    }
    if tables.whole.contains(&w) {
        return true;
    }
    // Splits outside these bounds cannot match; hashing all of them was D-23.
    let lo = w.len().saturating_sub(tables.max_ending);
    let hi = w.len().min(tables.max_stem);
    for i in lo..=hi {
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
    use crate::model::{Group, Pos};

    #[test]
    fn lowercases() {
        assert_eq!(normalize("AMARE"), "amare");
    }

    #[test]
    fn maps_v_to_u_and_j_to_i() {
        assert_eq!(normalize("Verbum"), "uerbum");
        assert_eq!(normalize("Julius"), "iulius");
    }

    #[test]
    fn leaves_other_characters_alone() {
        assert_eq!(normalize("my_var2"), "my_uar2");
    }

    /// The 2nd-declension stem `numer-`, the 1st-conjugation stem `am-`, and the
    /// indeclinable `et`.
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
    fn matches_a_bare_stem_via_the_empty_ending() {
        let mut t = fixture();
        let noun3 = Group { pos: Pos::Noun, decl: 3 };
        t.stems.insert("lux".to_string(), vec![noun3]);
        t.endings.insert(
            noun3,
            ["", "is"].iter().map(|s| s.to_string()).collect(),
        );

        assert!(analyze(&t, "lux"));
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
        assert!(!analyze(&fixture(), "numerzzz"));
    }

    #[test]
    fn rejects_empty_and_non_ascii_input() {
        let t = fixture();
        assert!(!analyze(&t, ""));
        assert!(!analyze(&t, "café"));
    }

    /// `fixture()` plus a stem whose group allows a zero-length ending.
    fn wide_fixture() -> Tables {
        let mut t = fixture();
        let noun3 = Group { pos: Pos::Noun, decl: 3 };
        t.stems.insert("lux".to_string(), vec![noun3]);
        t.endings.insert(noun3, ["", "is"].iter().map(|s| s.to_string()).collect());
        t
    }

    #[test]
    fn narrowing_the_bounds_never_changes_a_verdict() {
        // D-23
        let loose = wide_fixture();
        let mut tight = wide_fixture();
        tight.narrow_bounds();

        let mut words: Vec<String> = [
            "numerus", "numeri", "numero", "numerum", "numere", "lux", "luxis",
            "amo", "amas", "amat", "amare", "et", "NUMERUS", "Amare",
            "result", "myVar", "getData", "xyz", "foobar", "numerzzz", "", "café",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        words.push("a".repeat(10_000));

        for w in &words {
            assert_eq!(
                analyze(&loose, w),
                analyze(&tight, w),
                "narrowed and un-narrowed tables disagree on {w:?}"
            );
        }
    }

    #[test]
    fn an_un_narrowed_table_is_permissive_not_empty() {
        // D-23: a `0` default would silently reject every word.
        let t = Tables::new();
        assert_eq!(t.max_stem, usize::MAX);
        assert_eq!(t.max_ending, usize::MAX);
    }
}
