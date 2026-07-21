use crate::analyze::normalize;
use crate::model::{Group, Pos, Tables};

/// Width of each of the four stem columns in DICTLINE.GEN.
const STEM_WIDTH: usize = 19;
/// Total width of the four stem columns; the part of speech starts here.
const STEMS_END: usize = STEM_WIDTH * 4;
/// Whitaker's placeholder for "this stem slot is unused".
const NO_STEM: &str = "zzz";
/// Whitaker's declension code for an indeclinable entry.
const INDECLINABLE_DECL: u8 = 9;

/// Map a Whitaker part-of-speech code onto the three inflecting classes we
/// model. Everything else is indeclinable as far as this checker is concerned.
///
/// Known, accepted limitation: `PRON` and `NUM` are not mapped here, so only
/// their truncated dictionary stems (e.g. `qu`, `h`, `ill`, `du`, `tr`) land in
/// `whole` as bare words — inflected forms like `quae`, `quem`, `duo`, `tres`,
/// `ille` are therefore reported as non-Latin. Routing these truncated 1-3
/// character stems permissively would accept enormous amounts of junk and gut
/// the checker's precision, so this is left as-is.
fn inflecting_pos(code: &str) -> Option<Pos> {
    match code {
        "N" => Some(Pos::Noun),
        "ADJ" => Some(Pos::Adj),
        "V" => Some(Pos::Verb),
        _ => None,
    }
}

/// Compile the three Whitaker data files into lookup tables.
pub fn build_tables(dictline: &str, inflects: &str, uniques: &str) -> Tables {
    let mut t = Tables::new();
    for line in inflects.lines() {
        parse_inflect_line(line, &mut t);
    }
    for line in dictline.lines() {
        parse_dict_line(line, &mut t);
    }
    for line in uniques.lines() {
        parse_unique_line(line, &mut t);
    }
    t
}

/// One line of UNIQUES.LAT. Entries are two lines: the irregular word form
/// flush at column 1, then an indented line of grammatical codes and meaning.
/// Only the flush-left lines are word forms.
fn parse_unique_line(line: &str, t: &mut Tables) {
    if line.starts_with(char::is_whitespace) {
        return;
    }
    let word = line.trim();
    if word.is_empty() || word.starts_with("--") {
        return;
    }
    if word.chars().all(|c| c.is_ascii_alphabetic()) {
        t.whole.insert(normalize(word));
    }
}

/// One DICTLINE entry: four fixed-width stems, then the part of speech and its
/// declension/conjugation number.
fn parse_dict_line(line: &str, t: &mut Tables) {
    if line.len() < STEMS_END || !line.is_ascii() {
        return;
    }
    let stems: Vec<String> = (0..4)
        .map(|i| &line[i * STEM_WIDTH..(i + 1) * STEM_WIDTH])
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && *s != NO_STEM)
        .map(normalize)
        .collect();
    if stems.is_empty() {
        return;
    }

    let mut toks = line[STEMS_END..].split_whitespace();
    let Some(pos_code) = toks.next() else {
        return;
    };

    // Indeclinable part of speech: the stem is the entire word.
    let Some(pos) = inflecting_pos(pos_code) else {
        t.whole.extend(stems);
        return;
    };

    let Some(decl) = toks.next().and_then(|d| d.parse::<u8>().ok()) else {
        return;
    };
    if decl == INDECLINABLE_DECL {
        t.whole.extend(stems);
        return;
    }

    let group = Group { pos, decl };
    for stem in stems {
        let groups = t.stems.entry(stem).or_default();
        if !groups.contains(&group) {
            groups.push(group);
        }
    }
}

/// One INFLECTS rule. Layout is
/// `POS decl variant <features...> stem-key ending-length ending flag flag`,
/// with a variable number of feature columns — so we read from the right.
fn parse_inflect_line(line: &str, t: &mut Tables) {
    let line = line.trim();
    if line.is_empty() || line.starts_with("--") {
        return;
    }
    let toks: Vec<&str> = line.split_whitespace().collect();
    // Shortest real rule is: POS decl variant stem-key length flag flag.
    if toks.len() < 7 {
        return;
    }
    let Some(pos) = inflecting_pos(toks[0]) else {
        return;
    };
    let Ok(decl) = toks[1].parse::<u8>() else {
        return;
    };

    // Some lines carry a trailing scholarly citation comment introduced by a
    // `--` token (e.g. "-- G+L 130 1"), which sits AFTER the two flag columns.
    // Some comment markers are glued directly onto the following word instead
    // of standing alone (e.g. "--JFW,", "--PRON", "--quis,"), so match on a
    // `--` prefix rather than exact equality. Strip everything from the first
    // such token onward before dropping the flags, otherwise the
    // right-anchored extraction below misparses the ending.
    let core_toks: &[&str] = match toks.iter().position(|tok| tok.starts_with("--")) {
        Some(i) => &toks[..i],
        None => &toks[..],
    };
    // Defensive re-check: in the current INFLECTS.LAT this never fires (every
    // line long enough to reach here stays long enough after truncation), but
    // a citation-comment truncation could in principle shrink `core_toks`
    // below the minimum shape, so guard it explicitly rather than panic below.
    if core_toks.len() < 7 {
        return;
    }

    // Drop the two trailing flag columns (age, frequency).
    let core = &core_toks[..core_toks.len() - 2];
    // The ending is the last token when it is alphabetic; when the ending-length
    // column is 0 there is no ending token at all, so the last token is numeric.
    let ending = match core.last() {
        Some(last) if last.chars().all(|c| c.is_ascii_alphabetic()) => normalize(last),
        Some(_) => String::new(),
        None => return,
    };

    t.endings.entry(Group { pos, decl }).or_default().insert(ending);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyze::analyze;
    use crate::model::{Group, Pos};

    /// Build a DICTLINE line with correct 19-character stem columns, so the
    /// tests never depend on hand-counted spacing.
    fn dict_line(stems: &[&str], tail: &str) -> String {
        let mut s = String::new();
        for i in 0..4 {
            let stem = stems.get(i).copied().unwrap_or("");
            s.push_str(stem);
            for _ in stem.len()..19 {
                s.push(' ');
            }
        }
        s.push_str(tail);
        s
    }

    #[test]
    fn parses_a_noun_entry_and_its_endings() {
        let dict = dict_line(&["numer", "numer"], "N      2 1 M T          X X X A O number;");
        let infl = "N     2 1 NOM S M  1 2 us        X A\n\
                    N     2 1 GEN S M  2 1 i         X A";
        let t = build_tables(&dict, infl, "");

        let g = Group { pos: Pos::Noun, decl: 2 };
        assert_eq!(t.stems.get("numer"), Some(&vec![g]));
        assert!(t.endings[&g].contains("us"));
        assert!(analyze(&t, "numerus"));
        assert!(analyze(&t, "numeri"));
    }

    #[test]
    fn parses_a_verb_entry_with_four_stems() {
        let dict = dict_line(
            &["am", "am", "amav", "amat"],
            "V      1 1 TRANS        X X X A O love, like;",
        );
        let infl = "V     1 1 PRES  ACTIVE  IND  1 S  1 1 o         X A\n\
                    V     1 1 PRES  ACTIVE  INF  0 X  2 3 are       X A\n\
                    V     1 1 PERF  ACTIVE  IND  1 S  3 1 i         X A";
        let t = build_tables(&dict, infl, "");

        assert!(analyze(&t, "amo"));
        assert!(analyze(&t, "amare"));
        assert!(analyze(&t, "amavi")); // perfect stem + perfect ending
    }

    #[test]
    fn treats_indeclinable_parts_of_speech_as_whole_words() {
        let dict = format!(
            "{}\n{}",
            dict_line(&["et"], "CONJ                    X X X A O and;"),
            dict_line(&["ab"], "PREP   ABL              X X X A O by, from;"),
        );
        let t = build_tables(&dict, "", "");
        assert!(t.whole.contains("et"));
        assert!(analyze(&t, "et"));
        assert!(analyze(&t, "ab"));
    }

    #[test]
    fn handles_zero_length_endings() {
        // 3rd-declension nominatives attach a zero-length ending, so the bare
        // stem is itself a valid word.
        let dict = dict_line(&["rex", "reg"], "N      3 1 M T          X X X A O king;");
        let infl = "N     3 1 NOM S M  1 0           X A\n\
                    N     3 1 GEN S M  2 2 is        X A";
        let t = build_tables(&dict, infl, "");

        assert!(analyze(&t, "rex"));
        assert!(analyze(&t, "regis"));
    }

    #[test]
    fn declension_nine_means_indeclinable() {
        let dict = dict_line(&["a"], "N      9 8 M N          X X X C G year, abb. a.;");
        let t = build_tables(&dict, "", "");
        assert!(t.whole.contains("a"));
        // Declension 9 is indeclinable, so nothing should land in `stems`.
        assert!(!t.stems.contains_key("a"));
    }

    #[test]
    fn skips_placeholder_and_empty_stems() {
        let dict = dict_line(&["numer", "zzz"], "N      2 1 M T          X X X A O number;");
        let t = build_tables(&dict, "N     2 1 NOM S M  1 2 us        X A", "");
        assert!(t.stems.contains_key("numer"));
        assert!(!t.stems.contains_key("zzz"));
    }

    #[test]
    fn skips_comments_and_blank_lines_in_inflects() {
        let t = build_tables("", "-- a comment line\n\nN     2 1 NOM S M  1 2 us        X A", "");
        assert!(t.endings[&Group { pos: Pos::Noun, decl: 2 }].contains("us"));
    }

    #[test]
    fn parses_uniques_as_whole_words() {
        // UNIQUES entries are two lines: the word form flush left, then an
        // indented line of grammatical codes and the English meaning.
        let uniques = "est\n\
                       \x20     V      5 1 PRES ACTIVE IND 3 S     X X X A O he/she/it is;\n\
                       agantur\n\
                       \x20     V      3 1 PRES PASSIVE SUB 3 P     F X X E E they are driven;";
        let t = build_tables("", "", uniques);

        assert!(t.whole.contains("est"));
        assert!(t.whole.contains("agantur"));
        // The indented codes lines must never be mistaken for word forms:
        // exactly the two flush-left forms should be present, nothing else.
        assert_eq!(t.whole.len(), 2);
        assert!(analyze(&t, "est"));
    }

    #[test]
    fn ignores_trailing_citation_comments_in_inflects() {
        // Real INFLECTS.LAT line 972: a trailing "-- G+L 130 1" citation sits
        // AFTER the two flag columns. Without stripping it, the ending is
        // misparsed as "" instead of "are", which then lets any V-1 stem match
        // as a bare word.
        let infl = "V     1 1 PRES  PASSIVE IND  2 S  2 3 are           B B    --  G+L 130 1";
        let t = build_tables("", infl, "");

        let g = Group { pos: Pos::Verb, decl: 1 };
        let endings = t.endings.get(&g).expect("group should have endings");
        // Single-line fixture: pin the exact set, not just membership, so
        // nothing extra (a bogus empty ending or a leaked comment word) has
        // snuck in alongside the real one.
        assert_eq!(endings.len(), 1);
        assert!(endings.contains("are"));
        assert!(!endings.contains(""), "bogus empty ending leaked in");
    }

    #[test]
    fn ignores_leaked_comment_words_in_inflects() {
        // The trailing citation comment can be several words long; none of
        // them (e.g. "exception") may leak into the endings table and be
        // mistaken for a real Latin suffix.
        let infl = "N     1 1 GEN S F  2 2 ae        X A    --  exception to rule";
        let t = build_tables("", infl, "");

        let g = Group { pos: Pos::Noun, decl: 1 };
        let endings = t.endings.get(&g).expect("group should have endings");
        assert_eq!(endings.len(), 1);
        assert!(endings.contains("ae"));
        assert!(!endings.contains("exception"));
    }

    #[test]
    fn strips_attached_comment_markers_not_just_bare_dashes() {
        // Real INFLECTS.LAT line 409: the citation marker "--JFW," is glued
        // directly onto the following word rather than standing alone as a
        // bare "--" token. Matching only exact "--" misses this line
        // entirely: the last token is then "--JFW," itself (non-alphabetic),
        // which injects a bogus empty ending and discards the real ending
        // "is".
        let infl = "N     3 9 NOM S X  1 2 is        X A  --JFW, 1st stem";
        let t = build_tables("", infl, "");

        let g = Group { pos: Pos::Noun, decl: 3 };
        let endings = t.endings.get(&g).expect("group should have endings");
        assert_eq!(endings.len(), 1);
        assert!(endings.contains("is"));
        assert!(!endings.contains(""));
    }

    #[test]
    fn normalizes_stems_to_dictionary_orthography() {
        let dict = dict_line(&["uerb", "verb"], "N      2 2 N T          X X X A O word;");
        let t = build_tables(&dict, "N     2 2 NOM S N  1 2 um        X A", "");
        // Both spellings fold to the same normalized stem.
        assert!(analyze(&t, "verbum"));
        assert!(analyze(&t, "uerbum"));
    }
}
