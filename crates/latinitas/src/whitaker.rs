use crate::analyze::normalize;
use crate::model::{Group, Pos, Tables};
use std::collections::HashSet;

/// Width of each of the four stem columns in DICTLINE.GEN.
const STEM_WIDTH: usize = 19;
/// Total width of the four stem columns; the part of speech starts here.
const STEMS_END: usize = STEM_WIDTH * 4;
/// Whitaker's placeholder for "this stem slot is unused".
const NO_STEM: &str = "zzz";
/// Whitaker's declension code for an indeclinable entry.
const INDECLINABLE_DECL: u8 = 9;
/// "Applies regardless of declension/conjugation"; see `merge_wildcard_endings`.
const WILDCARD_DECL: u8 = 0;

/// The three inflecting classes modelled here; everything else is indeclinable.
/// `PRON` and `NUM` are unmapped by decision — see `docs/internals/rationale.md`.
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
    merge_wildcard_endings(&mut t);
    t
}

/// One word per line, `#` starts a comment; entries land in `whole` and so
/// inflect no further. D-18
pub fn add_supplement(t: &mut Tables, text: &str) {
    for line in text.lines() {
        let word = line.split('#').next().unwrap_or("").trim();
        if word.is_empty() {
            continue;
        }
        t.whole.insert(normalize(word));
    }
}

/// Wildcard (declension `0`) endings go to every concrete group of the same `pos`.
fn merge_wildcard_endings(t: &mut Tables) {
    let wildcards: Vec<(Pos, HashSet<String>)> = t
        .endings
        .iter()
        .filter(|(g, _)| g.decl == WILDCARD_DECL)
        .map(|(g, ends)| (g.pos, ends.clone()))
        .collect();
    if wildcards.is_empty() {
        return;
    }

    // groups that own endings, plus those only referenced by a dictionary stem
    let mut concrete: HashSet<Group> =
        t.endings.keys().copied().filter(|g| g.decl != WILDCARD_DECL).collect();
    for groups in t.stems.values() {
        concrete.extend(groups.iter().copied().filter(|g| g.decl != WILDCARD_DECL));
    }

    for g in concrete {
        for (pos, ends) in &wildcards {
            if *pos == g.pos {
                t.endings.entry(g).or_default().extend(ends.iter().cloned());
            }
        }
    }
}

/// UNIQUES.LAT: the word form is flush left, its codes on the next, indented line.
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

    // indeclinable part of speech: the stem is the entire word
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

/// One INFLECTS rule: `POS decl variant <features...> stem-key ending-length
/// ending flag flag`, with a variable feature count — so it reads from the right.
fn parse_inflect_line(line: &str, t: &mut Tables) {
    let line = line.trim();
    if line.is_empty() || line.starts_with("--") {
        return;
    }
    let toks: Vec<&str> = line.split_whitespace().collect();
    // shortest real rule: POS decl variant stem-key length flag flag
    if toks.len() < 7 {
        return;
    }
    let Some(pos) = inflecting_pos(toks[0]) else {
        return;
    };
    let Ok(decl) = toks[1].parse::<u8>() else {
        return;
    };

    // A trailing scholarly citation sits after the flag columns, either standing
    // alone ("-- G+L 130 1") or glued onto the next word ("--JFW,").
    let core_toks: &[&str] = match toks.iter().position(|tok| tok.starts_with("--")) {
        Some(i) => &toks[..i],
        None => &toks[..],
    };
    // never fires against today's INFLECTS.LAT, but truncation could in
    // principle shrink a line below the minimum shape
    if core_toks.len() < 7 {
        return;
    }

    // drop the two trailing flag columns (age, frequency)
    let core = &core_toks[..core_toks.len() - 2];
    // an ending-length column of 0 means no ending token, so the last token is numeric
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

    /// Correct 19-character stem columns, so no test depends on hand-counted spacing.
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
        assert!(analyze(&t, "amavi"));
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
        let uniques = "est\n\
                       \x20     V      5 1 PRES ACTIVE IND 3 S     X X X A O he/she/it is;\n\
                       agantur\n\
                       \x20     V      3 1 PRES PASSIVE SUB 3 P     F X X E E they are driven;";
        let t = build_tables("", "", uniques);

        assert!(t.whole.contains("est"));
        assert!(t.whole.contains("agantur"));
        // exactly the two flush-left forms, nothing else
        assert_eq!(t.whole.len(), 2);
        assert!(analyze(&t, "est"));
    }

    #[test]
    fn ignores_trailing_citation_comments_in_inflects() {
        // real INFLECTS.LAT line 972
        let infl = "V     1 1 PRES  PASSIVE IND  2 S  2 3 are           B B    --  G+L 130 1";
        let t = build_tables("", infl, "");

        let g = Group { pos: Pos::Verb, decl: 1 };
        let endings = t.endings.get(&g).expect("group should have endings");
        assert_eq!(endings.len(), 1);
        assert!(endings.contains("are"));
        assert!(!endings.contains(""), "bogus empty ending leaked in");
    }

    #[test]
    fn ignores_leaked_comment_words_in_inflects() {
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
        // real INFLECTS.LAT line 409, where the marker is glued on as "--JFW,"
        let infl = "N     3 9 NOM S X  1 2 is        X A  --JFW, 1st stem";
        let t = build_tables("", infl, "");

        let g = Group { pos: Pos::Noun, decl: 3 };
        let endings = t.endings.get(&g).expect("group should have endings");
        assert_eq!(endings.len(), 1);
        assert!(endings.contains("is"));
        assert!(!endings.contains(""));
    }

    #[test]
    fn wildcard_conjugation_endings_reach_every_verb_group() {
        let dict = dict_line(
            &["mon", "mon", "monu", "monit"],
            "V      2 1 TRANS        X X X A O warn, advise;",
        );
        let infl = "V     2 1 PRES  ACTIVE  IND  1 S  1 2 eo        X A\n\
                    V     0 0 PERF  ACTIVE  IND  1 S  3 1 i         X A\n\
                    V     0 0 PERF  ACTIVE  IND  3 S  3 2 it        X A";
        let t = build_tables(&dict, infl, "");

        assert!(analyze(&t, "moneo"), "conjugation-specific ending still works");
        assert!(analyze(&t, "monui"), "wildcard perfect ending must reach V 2");
        assert!(analyze(&t, "monuit"), "wildcard perfect ending must reach V 2");
    }

    #[test]
    fn wildcard_declension_endings_reach_every_adjective_group() {
        let dict = dict_line(
            &["bon", "bon", "melior", "optim"],
            "ADJ    1 1 POS          X X X A O good;",
        );
        let infl = "ADJ   1 1 NOM S M  1 2 us        X A\n\
                    ADJ   0 0 NOM S C COMP  3 2 or    X A\n\
                    ADJ   0 0 NOM S M SUPER 4 2 us    X A";
        let t = build_tables(&dict, infl, "");

        assert!(analyze(&t, "bonus"), "declension-specific ending still works");
        assert!(analyze(&t, "optimus"), "wildcard superlative ending must reach ADJ 1");
    }

    #[test]
    fn wildcard_endings_do_not_cross_parts_of_speech() {
        let dict = dict_line(&["numer", "numer"], "N      2 1 M T          X X X A O number;");
        let infl = "N     2 1 NOM S M  1 2 us        X A\n\
                    V     0 0 PERF  ACTIVE  IND  2 S  3 4 isti      X A";
        let t = build_tables(&dict, infl, "");

        let noun2 = Group { pos: Pos::Noun, decl: 2 };
        assert!(!t.endings[&noun2].contains("isti"), "verb ending leaked onto a noun group");
        assert!(!analyze(&t, "numeristi"));
    }

    #[test]
    fn normalizes_stems_to_dictionary_orthography() {
        let dict = dict_line(&["uerb", "verb"], "N      2 2 N T          X X X A O word;");
        let t = build_tables(&dict, "N     2 2 NOM S N  1 2 um        X A", "");
        assert!(analyze(&t, "verbum"));
        assert!(analyze(&t, "uerbum"));
    }
}
