#[derive(Debug, Clone, PartialEq)]
pub struct LinguaError {
    pub message: String,
    pub line: usize,
}

pub type LResult<T> = Result<T, LinguaError>;

/// A non-fatal style note. Unlike errors, a `Warning` never stops a program
/// from running. `file` is the path relative to the program root (`None` in
/// single-source mode); `line` is `None` for warnings about things that have
/// no line, such as package folder names.
#[derive(Debug, Clone, PartialEq)]
pub struct Warning {
    pub message: String,
    pub file: Option<String>,
    pub line: Option<usize>,
}

impl std::fmt::Display for Warning {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match (&self.file, self.line) {
            (Some(p), Some(l)) => write!(f, "Monitum in '{}', linea {}: {}", p, l, self.message),
            (Some(p), None) => write!(f, "Monitum in '{}': {}", p, self.message),
            (None, Some(l)) => write!(f, "Monitum in linea {}: {}", l, self.message),
            (None, None) => write!(f, "Monitum: {}", self.message),
        }
    }
}

/// An error at program (multi-file) level: optionally attributed to a source
/// file (path relative to the program root) and a line. Structural errors —
/// about the program's shape rather than any source text — carry neither.
#[derive(Debug, Clone, PartialEq)]
pub struct ProgramError {
    pub file: Option<String>,
    pub line: Option<usize>,
    pub message: String,
}

impl ProgramError {
    pub fn structural(message: impl Into<String>) -> Self {
        ProgramError { file: None, line: None, message: message.into() }
    }
    pub fn in_file(file: &str, e: LinguaError) -> Self {
        ProgramError { file: Some(file.to_string()), line: Some(e.line), message: e.message }
    }
}

impl From<LinguaError> for ProgramError {
    fn from(e: LinguaError) -> Self {
        ProgramError { file: None, line: Some(e.line), message: e.message }
    }
}

impl std::fmt::Display for ProgramError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match (&self.file, self.line) {
            (Some(p), Some(l)) => write!(f, "Error in '{}', linea {}: {}", p, l, self.message),
            (Some(p), None) => write!(f, "Error in '{}': {}", p, self.message),
            (None, Some(l)) => write!(f, "Error in linea {}: {}", l, self.message),
            (None, None) => write!(f, "Error: {}", self.message),
        }
    }
}

impl std::fmt::Display for LinguaError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Error in linea {}: {}", self.line, self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_with_line_and_message() {
        let e = LinguaError { message: "res mala".into(), line: 5 };
        assert_eq!(e.to_string(), "Error in linea 5: res mala");
    }

    #[test]
    fn program_error_formats_all_three_shapes() {
        let both = ProgramError {
            file: Some("mathematica/summa.liss".into()), line: Some(5), message: "res mala".into(),
        };
        assert_eq!(both.to_string(), "Error in 'mathematica/summa.liss', linea 5: res mala");
        let line_only: ProgramError = LinguaError { message: "res mala".into(), line: 5 }.into();
        assert_eq!(line_only.to_string(), "Error in linea 5: res mala");
        let structural = ProgramError::structural("munus 'initium' in programmate non inventum");
        assert_eq!(structural.to_string(), "Error: munus 'initium' in programmate non inventum");
    }

    #[test]
    fn in_file_attaches_the_path() {
        let e = ProgramError::in_file("summa.liss", LinguaError { message: "x".into(), line: 3 });
        assert_eq!(e.file.as_deref(), Some("summa.liss"));
        assert_eq!(e.line, Some(3));
    }

    #[test]
    fn warning_formats_with_and_without_file() {
        let plain = Warning { message: "nomen 'result' fortasse non Latinum est".into(), file: None, line: Some(3) };
        assert_eq!(plain.to_string(), "Monitum in linea 3: nomen 'result' fortasse non Latinum est");
        let in_file = Warning { message: "m".into(), file: Some("a.liss".into()), line: Some(2) };
        assert_eq!(in_file.to_string(), "Monitum in 'a.liss', linea 2: m");
        let bare = Warning { message: "nomen fasciculi 'foobar' fortasse non Latinum est".into(), file: None, line: None };
        assert_eq!(bare.to_string(), "Monitum: nomen fasciculi 'foobar' fortasse non Latinum est");
    }
}
