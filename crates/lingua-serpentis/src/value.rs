use crate::roman::Roman;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(Roman),
    Float(f32),
    Double(f64),
    Text(String),
    Char(char),
    Bool(bool),
    List(Vec<Value>),
}

impl Value {
    pub fn display(&self) -> String {
        match self {
            Value::Int(r) => r.display(),
            Value::Float(f) => f.to_string(),
            Value::Double(f) => f.to_string(),
            Value::Text(s) => s.clone(),
            Value::Char(c) => c.to_string(),
            Value::Bool(b) => (if *b { "verus" } else { "falsus" }).to_string(),
            Value::List(items) => {
                let inner: Vec<String> = items.iter().map(|v| v.display()).collect();
                format!("[{}]", inner.join(", "))
            }
        }
    }
}
