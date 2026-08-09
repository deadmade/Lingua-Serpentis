use crate::spatium::Spatium;
use crate::fragmentum::Fragmentum;
use crate::pictura::{depinge, Genus};

#[derive(Debug, Clone, PartialEq)]
pub struct LinguaError {
    pub message: String,
    pub line: usize,
    /// Where the diagnostic points. `None` where nothing in the source can be
    /// underlined — a structural complaint about a program rather than a place.
    pub spatium: Option<Spatium>,
}

pub type LResult<T> = Result<T, LinguaError>;

/// A non-fatal style note; never stops a program. `file` is relative to the program root.
#[derive(Debug, Clone, PartialEq)]
pub struct Warning {
    pub message: String,
    pub file: Option<String>,
    pub line: Option<usize>,
    /// Where in the file the name sits. Kept until `lint_program` reaches the source
    /// text and can turn it into a `fragmentum`; never rendered itself.
    pub spatium: Option<Spatium>,
    pub fragmentum: Option<Fragmentum>,
}

impl std::fmt::Display for Warning {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            depinge(
                Genus::Monitum,
                &self.message,
                self.file.as_deref(),
                self.line,
                self.fragmentum.as_ref(),
            )
        )
    }
}

/// Structural errors carry no file and no line. `stdout` and `warnings` carry what the
/// program produced before it failed (D-08, D-09).
#[derive(Debug, Clone, PartialEq)]
pub struct ProgramError {
    pub file: Option<String>,
    pub line: Option<usize>,
    pub message: String,
    pub stdout: String,
    pub warnings: Vec<Warning>,
    pub fragmentum: Option<Fragmentum>,
}

impl ProgramError {
    pub fn structural(message: impl Into<String>) -> Self {
        ProgramError {
            file: None,
            line: None,
            message: message.into(),
            stdout: String::new(),
            warnings: Vec::new(),
            fragmentum: None,
        }
    }
    /// `src` is the file's text, so the line the error points at is taken here —
    /// the last place that has both, since `strip` may clear the name downstream.
    pub fn in_file(file: &str, src: &str, e: LinguaError) -> Self {
        ProgramError {
            file: Some(file.to_string()),
            line: Some(e.line),
            message: e.message,
            stdout: String::new(),
            warnings: Vec::new(),
            fragmentum: e.spatium.and_then(|s| crate::fragmentum::extract(src, s)),
        }
    }
}

impl From<LinguaError> for ProgramError {
    fn from(e: LinguaError) -> Self {
        ProgramError {
            file: None,
            line: Some(e.line),
            message: e.message,
            stdout: String::new(),
            warnings: Vec::new(),
            fragmentum: None,
        }
    }
}

impl std::fmt::Display for ProgramError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            depinge(
                Genus::Erratum,
                &self.message,
                self.file.as_deref(),
                self.line,
                self.fragmentum.as_ref(),
            )
        )
    }
}

impl std::fmt::Display for LinguaError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", depinge(Genus::Erratum, &self.message, None, Some(self.line), None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_with_line_and_message() {
        let e = LinguaError { message: "res mala".into(), line: 5, spatium: None, };
        assert_eq!(e.to_string(), "erratum: res mala\n  in linea V");
    }

    #[test]
    fn program_error_formats_all_three_shapes() {
        let both = ProgramError {
            file: Some("mathematica/summa.liss".into()), line: Some(5), message: "res mala".into(),
            stdout: String::new(), warnings: Vec::new(), fragmentum: None,
        };
        assert_eq!(
            both.to_string(),
            "erratum: res mala\n  in 'mathematica/summa.liss', linea V"
        );
        let line_only: ProgramError = LinguaError { message: "res mala".into(), line: 5, spatium: None, }.into();
        assert_eq!(line_only.to_string(), "erratum: res mala\n  in linea V");
        // Structural: no file, no line, and so no location row at all.
        let structural = ProgramError::structural("munus 'initium' in programmate non inventum");
        assert_eq!(
            structural.to_string(),
            "erratum: munus 'initium' in programmate non inventum"
        );
    }

    #[test]
    fn in_file_attaches_the_path() {
        let e = ProgramError::in_file("summa.liss", "", LinguaError { message: "x".into(), line: 3, spatium: None, });
        assert_eq!(e.file.as_deref(), Some("summa.liss"));
        assert_eq!(e.line, Some(3));
    }

    fn monitum(message: &str, file: Option<&str>, line: Option<usize>) -> Warning {
        Warning {
            message: message.into(),
            file: file.map(str::to_string),
            line,
            spatium: None,
            fragmentum: None,
        }
    }

    #[test]
    fn warning_formats_with_and_without_file() {
        let plain = monitum("nomen 'result' fortasse non Latinum est", None, Some(3));
        assert_eq!(
            plain.to_string(),
            "monitum: nomen 'result' fortasse non Latinum est\n  in linea III"
        );
        let in_file = monitum("m", Some("a.liss"), Some(2));
        assert_eq!(in_file.to_string(), "monitum: m\n  in 'a.liss', linea II");
        let bare = monitum("nomen fasciculi 'foobar' fortasse non Latinum est", None, None);
        assert_eq!(
            bare.to_string(),
            "monitum: nomen fasciculi 'foobar' fortasse non Latinum est"
        );
    }
}
