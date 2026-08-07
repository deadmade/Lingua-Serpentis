//! A second, independent implementation of the `scribe` formatter, kept only
//! as the randomised oracle for `scriptio`'s tests.
//!
//! This is `lingua-serpentis::value`'s `Value` moved here unchanged but
//! renamed, so nobody mistakes it for production: `#[cfg(test)]`, never
//! compiled into any artefact.
//!
//! **This is a pre-approved, deliberate duplication, not a defect** — spec
//! §10.4 ruled on it explicitly. It exists because it is the *only*
//! randomised oracle for the formatters: golden vectors cover only cases
//! someone thought to write down. It earns its place because it is small,
//! test-only, and the alternative is having no randomised oracle at all.

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
