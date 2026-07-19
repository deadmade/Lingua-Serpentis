#[derive(Debug, Clone, PartialEq)]
pub struct LinguaError {
    pub message: String,
    pub line: usize,
}

pub type LResult<T> = Result<T, LinguaError>;

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
}
