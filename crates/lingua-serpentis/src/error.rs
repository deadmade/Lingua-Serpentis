#[derive(Debug, Clone, PartialEq)]
pub struct LinguaError {
    pub message: String,
    pub line: usize,
}

pub type LResult<T> = Result<T, LinguaError>;

/// A non-fatal style note. Unlike `LinguaError`, a `Warning` never stops a
/// program from running.
#[derive(Debug, Clone, PartialEq)]
pub struct Warning {
    pub message: String,
    pub line: usize,
}

impl std::fmt::Display for Warning {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Monitum in linea {}: {}", self.line, self.message)
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
    fn warning_formats_with_line_and_message() {
        let w = Warning { message: "nomen 'result' fortasse non Latinum est".into(), line: 3 };
        assert_eq!(
            w.to_string(),
            "Monitum in linea 3: nomen 'result' fortasse non Latinum est"
        );
    }
}
