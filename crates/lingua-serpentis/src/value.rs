#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f32),
    Double(f64),
    Text(String),
    Char(char),
    Bool(bool),
}

impl Value {
    pub fn display(&self) -> String {
        match self {
            Value::Int(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Double(f) => f.to_string(),
            Value::Text(s) => s.clone(),
            Value::Char(c) => c.to_string(),
            Value::Bool(b) => (if *b { "verus" } else { "falsus" }).to_string(),
        }
    }
}
