use crate::roman::Roman;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(Roman),
    Float(Roman),
    Double(Roman),
    Text(String),
    Char(char),
    Bool(bool),
    List(Vec<Value>),
}

impl Value {
    pub fn display(&self) -> String {
        match self {
            Value::Int(r) => r.display(),
            Value::Float(c) => crate::fraction::display(c, &crate::fraction::uncia_grain()),
            Value::Double(c) => crate::fraction::display(c, &crate::fraction::scripulum_grain()),
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
