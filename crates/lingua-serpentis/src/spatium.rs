/// A half-open byte range within one file's source, and the line it starts on.
///
/// Byte offsets, not `char` indices: the only consumer slices a `&str`
/// (`fragmentum::extract`). `linea` rides along rather than being derived from
/// `principium`, because D-03 makes it non-derivable — see
/// `docs/superpowers/specs/2026-08-09-caret-diagnostics-design.md` §4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spatium {
    pub principium: u32,
    pub finis: u32,
    pub linea: u32,
}

impl Spatium {
    pub fn novum(principium: u32, finis: u32, linea: usize) -> Spatium {
        Spatium { principium, finis, linea: linea as u32 }
    }

    /// From the start of `self` to the end of `alterum`, keeping `self`'s line.
    /// The parser closes an expression's span over its last token this way.
    pub fn ad(self, alterum: Spatium) -> Spatium {
        Spatium { principium: self.principium, finis: alterum.finis, linea: self.linea }
    }

    pub fn linea(&self) -> usize {
        self.linea as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joining_keeps_the_left_start_and_line_and_the_right_end() {
        let sinistrum = Spatium::novum(4, 6, 2);
        let dextrum = Spatium::novum(10, 14, 3);
        assert_eq!(sinistrum.ad(dextrum), Spatium::novum(4, 14, 2));
    }

    #[test]
    fn joining_a_span_with_itself_changes_nothing() {
        let s = Spatium::novum(7, 9, 1);
        assert_eq!(s.ad(s), s);
    }
}
