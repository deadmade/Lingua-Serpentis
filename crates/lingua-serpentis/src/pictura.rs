use crate::fragmentum::Fragmentum;
use crate::roman::Roman;

/// Which word opens the block. The two differ in nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Genus {
    Erratum,
    Monitum,
}

impl Genus {
    fn verbum(self) -> &'static str {
        match self {
            Genus::Erratum => "erratum",
            Genus::Monitum => "monitum",
        }
    }
}

fn numeralis(n: usize) -> String {
    Roman::from_usize(n).display()
}

/// One diagnostic, in the three-level grammar of
/// `docs/superpowers/specs/2026-08-09-caret-diagnostics-design.md` §3. No trailing
/// newline: the caller separates diagnostics, and no line ends in whitespace.
///
/// Each level is independent of the one below, which is what lets the compiled image
/// emit the first two and stop.
pub fn depinge(
    genus: Genus,
    nuntius: &str,
    scida: Option<&str>,
    linea: Option<usize>,
    fragmentum: Option<&Fragmentum>,
) -> String {
    let mut out = format!("{}: {}", genus.verbum(), nuntius);

    match (scida, linea) {
        (Some(s), Some(l)) => out.push_str(&format!("\n  in '{}', linea {}", s, numeralis(l))),
        (Some(s), None) => out.push_str(&format!("\n  in '{}'", s)),
        (None, Some(l)) => out.push_str(&format!("\n  in linea {}", numeralis(l))),
        (None, None) => {}
    }

    if let Some(f) = fragmentum {
        let num = numeralis(f.linea);
        // The `|` column is shared by all four rows and set by the numeral's width,
        // which is not two: line 88 is `LXXXVIII`.
        let vacuum = " ".repeat(num.chars().count() + 2);
        out.push_str(&format!("\n{vacuum}|"));
        out.push_str(&format!("\n {num} |"));
        if !f.textus.is_empty() {
            out.push(' ');
            out.push_str(&f.textus);
        }
        out.push_str(&format!("\n{vacuum}| "));
        out.push_str(&" ".repeat(f.caret));
        out.push_str(&"^".repeat(f.latitudo.max(1)));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frag(linea: usize, textus: &str, caret: usize, latitudo: usize) -> Fragmentum {
        Fragmentum { linea, textus: textus.to_string(), caret, latitudo }
    }

    #[test]
    fn the_whole_grammar_with_every_level_present() {
        let f = frag(3, "ni x = y + °I;", 7, 1);
        assert_eq!(
            depinge(Genus::Erratum, "variabilis ignota: 'y'", Some("principale.liss"), Some(3), Some(&f)),
            "\
erratum: variabilis ignota: 'y'
  in 'principale.liss', linea III
     |
 III | ni x = y + °I;
     |        ^"
        );
    }

    #[test]
    fn the_four_shapes_a_location_can_take() {
        let d = |s, l| depinge(Genus::Erratum, "res mala", s, l, None);
        assert_eq!(d(Some("a.liss"), Some(5)), "erratum: res mala\n  in 'a.liss', linea V");
        assert_eq!(d(Some("a.liss"), None), "erratum: res mala\n  in 'a.liss'");
        assert_eq!(d(None, Some(5)), "erratum: res mala\n  in linea V");
        assert_eq!(d(None, None), "erratum: res mala");
    }

    #[test]
    fn a_warning_differs_in_one_word() {
        assert_eq!(
            depinge(Genus::Monitum, "nomen 'xs' fortasse non Latinum est", None, None, None),
            "monitum: nomen 'xs' fortasse non Latinum est"
        );
    }

    #[test]
    fn the_gutter_widens_with_the_numeral() {
        // Line 88 is `LXXXVIII`: eight columns, and nothing may assume two.
        let f = frag(88, "redde x;", 6, 1);
        let out = depinge(Genus::Erratum, "res mala", None, Some(88), Some(&f));
        let canales: Vec<&str> = out.lines().skip(2).collect();
        let barra: Vec<usize> = canales.iter().map(|l| l.find('|').unwrap()).collect();
        assert_eq!(barra, vec![10, 10, 10], "every gutter row shares one column");
        assert!(canales[1].starts_with(" LXXXVIII |"));
    }

    #[test]
    fn the_caret_lands_under_the_column_it_names() {
        let f = frag(1, "ni x = y + °I;", 11, 3);
        let out = depinge(Genus::Erratum, "res mala", None, Some(1), Some(&f));
        let lineae: Vec<&str> = out.lines().collect();
        let fons = lineae[3];
        let cursus = lineae[4];
        let sub = cursus.find('^').unwrap();
        assert_eq!(fons.chars().nth(sub), Some('°'), "the caret sits under the numeral");
        assert_eq!(cursus.matches('^').count(), 3, "`°I;` is three columns");
    }

    #[test]
    fn no_line_ends_in_whitespace() {
        // No `.gitattributes` in this repository: a blessed fixture with a trailing
        // space is one editor-on-save from a diff nobody meant to make.
        for f in [frag(2, "", 0, 1), frag(2, "redde x;", 8, 1), frag(2, "\tx", 1, 1)] {
            let out = depinge(Genus::Monitum, "res", Some("a.liss"), Some(2), Some(&f));
            for linea in out.lines() {
                assert_eq!(linea.trim_end(), linea, "trailing space in {linea:?}");
            }
        }
    }

    #[test]
    fn an_empty_source_line_still_draws_its_gutter() {
        let f = frag(2, "", 0, 1);
        let out = depinge(Genus::Erratum, "res mala", None, Some(2), Some(&f));
        assert_eq!(out, "erratum: res mala\n  in linea II\n    |\n II |\n    | ^");
    }

    #[test]
    fn the_gutter_numeral_is_the_fragments_line() {
        // `judge`'s `caret aligns` compares the two; they must be written the same way.
        let f = frag(14, "redde x;", 0, 5);
        let out = depinge(Genus::Erratum, "res mala", Some("a.liss"), Some(14), Some(&f));
        assert!(out.contains("linea XIV"));
        assert!(out.contains("\n XIV | redde x;"));
    }
}
