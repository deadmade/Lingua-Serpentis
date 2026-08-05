use crate::model::Tables;

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
    // Only splits that could match are worth hashing. Below `lo` the ending is
    // longer than any ending in the dictionary; above `hi` the stem is longer
    // than any stem. Both lookups already failed for those splits, so skipping
    // them cannot change the answer — it only stops `analyze` from hashing a
    // longer and longer prefix on every iteration, which made it quadratic in
    // the identifier's length (defectus D-23). Against Whitaker's data the two
    // bounds are 18 and 7, so this is constant work however long `w` is; for
    // any word short enough to be Latin the range is the whole of `0..=w.len()`
    // exactly as before. `lo > hi` yields an empty range, which is the
    // pathological case.
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
    fn matches_a_bare_stem_via_the_empty_ending() {
        // Pins the inclusive `0..=w.len()` split. `lux` is a stem whose group
        // allows a zero-length ending, so the ONLY match is at i == w.len(),
        // where the ending is "". A non-inclusive `0..w.len()` would miss it.
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
        // `numer` is a real stem but `zzz` is not a 2nd-declension ending.
        assert!(!analyze(&fixture(), "numerzzz"));
    }

    #[test]
    fn rejects_empty_and_non_ascii_input() {
        let t = fixture();
        assert!(!analyze(&t, ""));
        assert!(!analyze(&t, "café"));
    }

    /// The same fixture plus a stem whose group allows a zero-length ending,
    /// so the differential test below covers the `i == w.len()` split too.
    fn wide_fixture() -> Tables {
        let mut t = fixture();
        let noun3 = Group { pos: Pos::Noun, decl: 3 };
        t.stems.insert("lux".to_string(), vec![noun3]);
        t.endings.insert(noun3, ["", "is"].iter().map(|s| s.to_string()).collect());
        t
    }

    #[test]
    fn narrowing_the_bounds_never_changes_a_verdict() {
        // Defectus D-23: `analyze` used to hash every one of the `w.len() + 1`
        // stem/ending splits, which is O(n^2) in the identifier's length.
        // `narrow_bounds` lets it skip the splits whose stem is longer than any
        // stem in the dictionary, or whose ending is longer than any ending.
        // That is a pure speedup, so the two tables must agree on every word —
        // including one far longer than any Latin word, which is the case the
        // bound exists to make cheap.
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
        // Longer than any dictionary stem, and longer than stem + ending can
        // ever be: under the bound this word costs zero iterations.
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
        // The bounds default to `usize::MAX`, not `0`, so a hand-built `Tables`
        // that never calls `narrow_bounds` behaves exactly as it did before
        // D-23 — merely slow, never wrong. A `0` default would silently reject
        // every word in every such table, including `fixture()` above.
        let t = Tables::new();
        assert_eq!(t.max_stem, usize::MAX);
        assert_eq!(t.max_ending, usize::MAX);
    }
}
