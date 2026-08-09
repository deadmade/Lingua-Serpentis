use crate::spatium::Spatium;

/// The one source line a diagnostic points at, ready to render. Extracted where
/// the file's text is still in hand, not at render time — see
/// `docs/superpowers/specs/2026-08-09-caret-diagnostics-design.md` §5.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fragmentum {
    /// 1-based.
    pub linea: usize,
    /// The source line: tabs already substituted, a trailing `\r` already gone.
    pub textus: String,
    /// 0-based display column of the first caret. May equal `textus`'s length,
    /// which is the end-of-line insertion point a missing `;` is reported at.
    pub caret: usize,
    /// Caret count, never zero.
    pub latitudo: usize,
}

/// Longer than this and the snippet is dropped rather than elided: an elision
/// marker needs a re-based caret, and that is a second family of edge cases.
const LONGITUDO_MAXIMA: usize = 200;

/// The greatest `char` boundary of `s` at or below `i`, and never past its end.
fn ad_terminum(s: &str, i: usize) -> usize {
    let mut i = i.min(s.len());
    while !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// `None` whenever there is nothing worth drawing — see the design's §5 table.
/// One column per `char`, and one space per tab, so that the caret is counted
/// from the same iteration that renders the line.
pub fn extract(src: &str, spatium: Spatium) -> Option<Fragmentum> {
    if src.is_empty() || spatium.linea() == 0 || spatium.principium as usize >= src.len() {
        return None;
    }
    // Floored onto a `char` boundary before anything slices with it. A span from
    // the lexer already lands on one; `extract` still must not be a way to panic
    // the compiler, and every offset below is derived from this one.
    let principium = ad_terminum(src, spatium.principium as usize);

    let initium = src[..principium].rfind('\n').map_or(0, |i| i + 1);
    let finis = src[principium..].find('\n').map_or(src.len(), |i| principium + i);
    let cruda = src[initium..finis].strip_suffix('\r').unwrap_or(&src[initium..finis]);
    if cruda.chars().count() > LONGITUDO_MAXIMA {
        return None;
    }

    // A span crossing lines underlines its first line only, so the width is
    // clamped at the end of that line rather than run on into the next.
    let terminus = (spatium.finis as usize).min(finis).max(principium);
    // Both ends are floored onto a `char` boundary before slicing. A well-formed
    // span from the lexer already lands on one, but `extract` must not be a way
    // to panic the compiler: stripping the `\r` above shortens the line under
    // offsets taken against the raw source, and a caller could hand over a
    // `finis` mid-character.
    let a = ad_terminum(cruda, principium - initium);
    let b = ad_terminum(cruda, terminus - initium).max(a);
    let caret = cruda[..a].chars().count();
    let latitudo = cruda[a..b].chars().count().max(1);

    Some(Fragmentum {
        linea: spatium.linea(),
        textus: cruda.replace('\t', " "),
        caret,
        latitudo,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sp(principium: u32, finis: u32, linea: usize) -> Spatium {
        Spatium::novum(principium, finis, linea)
    }

    #[test]
    fn the_caret_sits_under_the_span_it_was_given() {
        let src = "ni x = y + °I;";
        let f = extract(src, sp(7, 8, 1)).unwrap();
        assert_eq!(f.textus, "ni x = y + °I;");
        assert_eq!(f.caret, 7);
        assert_eq!(f.latitudo, 1);
        assert_eq!(&f.textus.chars().nth(f.caret).unwrap().to_string(), "y");
    }

    #[test]
    fn the_caret_column_is_chars_and_not_bytes() {
        // `mathēmatica` puts a two-byte char before the span; a byte column
        // would point one place too far right.
        let src = "ni mathēmatica = ō;";
        let byte = src.find('ō').unwrap() as u32;
        let f = extract(src, sp(byte, byte + 2, 1)).unwrap();
        assert_eq!(f.textus.chars().nth(f.caret), Some('ō'));
    }

    #[test]
    fn a_caret_under_a_degree_sign_lands_on_the_degree_sign() {
        // `°` is two bytes and one column, and starts every numeral in the
        // language, so it is on the caret path of much of the corpus.
        let src = "redde °XL;";
        let byte = src.find('°').unwrap() as u32;
        let f = extract(src, sp(byte, byte + 4, 1)).unwrap();
        assert_eq!(f.textus.chars().nth(f.caret), Some('°'));
        assert_eq!(f.latitudo, 3, "°XL is three columns");
    }

    #[test]
    fn a_line_in_the_middle_of_a_file_is_found_by_its_offset() {
        let src = "prima\nsecunda\ntertia\n";
        let byte = src.find("secunda").unwrap() as u32;
        let f = extract(src, sp(byte, byte + 7, 2)).unwrap();
        assert_eq!(f.textus, "secunda");
        assert_eq!(f.caret, 0);
        assert_eq!(f.latitudo, 7);
    }

    #[test]
    fn a_span_crossing_lines_underlines_only_its_first_line() {
        // D-03's multi-line literal: the span runs to the closing quote two
        // lines down, and the caret run stops at the end of the first.
        let src = "voc s = \"prima\nsecunda\";";
        let byte = src.find('"').unwrap() as u32;
        let f = extract(src, sp(byte, 23, 1)).unwrap();
        assert_eq!(f.textus, "voc s = \"prima");
        assert_eq!(f.caret, 8);
        assert_eq!(f.caret + f.latitudo, f.textus.chars().count());
    }

    #[test]
    fn a_tab_becomes_one_space_and_counts_one_column() {
        let src = "\t\tredde x;";
        let byte = src.find('x').unwrap() as u32;
        let f = extract(src, sp(byte, byte + 1, 1)).unwrap();
        assert_eq!(f.textus, "  redde x;");
        assert_eq!(f.textus.chars().nth(f.caret), Some('x'));
    }

    #[test]
    fn a_carriage_return_is_stripped_so_the_caret_row_does_not_overprint() {
        let src = "ni x = °I;\r\nni y = °II;";
        let f = extract(src, sp(3, 4, 1)).unwrap();
        assert_eq!(f.textus, "ni x = °I;");
        assert!(!f.textus.ends_with('\r'));
    }

    #[test]
    fn a_zero_width_span_still_gets_one_caret() {
        let src = "redde;";
        let f = extract(src, sp(5, 5, 1)).unwrap();
        assert_eq!(f.latitudo, 1);
    }

    #[test]
    fn an_empty_file_has_nothing_to_draw() {
        assert_eq!(extract("", sp(0, 0, 1)), None);
    }

    #[test]
    fn a_span_at_or_past_the_end_has_nothing_to_draw() {
        let src = "redde x;";
        assert_eq!(extract(src, sp(8, 8, 1)), None);
        assert_eq!(extract(src, sp(99, 99, 1)), None);
    }

    #[test]
    fn line_zero_has_nothing_to_draw() {
        // `Parser::line()` returns 0 on an empty token stream.
        assert_eq!(extract("redde x;", sp(0, 1, 0)), None);
    }

    #[test]
    fn a_very_long_line_is_dropped_rather_than_elided() {
        let brevis = "x".repeat(LONGITUDO_MAXIMA);
        let longa = "x".repeat(LONGITUDO_MAXIMA + 1);
        assert!(extract(&brevis, sp(0, 1, 1)).is_some());
        assert_eq!(extract(&longa, sp(0, 1, 1)), None);
    }

    #[test]
    fn an_empty_line_renders_as_itself_with_no_trailing_space() {
        let src = "prima\n\ntertia";
        // The span sits on the empty second line's newline position.
        let f = extract(src, sp(6, 6, 2)).unwrap();
        assert_eq!(f.textus, "");
        assert_eq!(f.caret, 0);
    }

    #[test]
    fn a_span_on_a_newline_points_at_the_end_of_the_line_before_it() {
        // Where `expectatum: ';'` belongs: the token that provoked it is on the
        // next line, and the caret marks the position the `;` should occupy.
        let src = "redde x\nredde y;";
        let f = extract(src, sp(7, 8, 1)).unwrap();
        assert_eq!(f.textus, "redde x");
        assert_eq!(f.caret, 7, "one past the last column");
        assert_eq!(f.latitudo, 1);
    }

    #[test]
    fn the_caret_never_starts_past_the_end_of_the_line_it_is_under() {
        // The invariant `judge`'s `caret aligns` rule checks at run time. The
        // caret may sit *at* the end — that is the end-of-line insertion point
        // above — but never beyond it, and the run adds at most that one column.
        // Deliberately includes a CRLF line and two multi-byte characters, and
        // deliberately does *not* skip offsets that fall mid-character: a span
        // `extract` cannot slice must yield a fragment or nothing, never a panic.
        let src = "ni x = y;\r\nni z = °I;\n\tabc\n\nmathēmatica\n";
        for principium in 0..src.len() as u32 {
            // Counted over bytes, not by slicing: `principium` may be mid-character.
            let linea = src.bytes().take(principium as usize).filter(|b| *b == b'\n').count() + 1;
            let Some(f) = extract(src, sp(principium, principium + 2, linea)) else {
                continue;
            };
            let longitudo = f.textus.chars().count();
            assert!(
                f.caret <= longitudo,
                "{principium}: caret {} past {:?}",
                f.caret,
                f.textus
            );
            assert!(
                f.latitudo >= 1 && f.caret + f.latitudo <= longitudo + 1,
                "{principium}: caret {} + {} over {:?}",
                f.caret,
                f.latitudo,
                f.textus
            );
        }
    }
}
