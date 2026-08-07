use crate::roman::Roman;
use liss_runtime::scriptio;

/// The tree-walking interpreter's own tagged runtime representation. This
/// type is *not* the thing spec §5.5 describes deleting — that is about the
/// backend under construction, which has no interpreter and nothing tagged.
/// `interp.rs` still walks a tree and still needs a runtime tag until it is
/// deleted itself (final task of this milestone), so `Value` stays here.
///
/// What *did* move is the formatter: `display` below no longer carries its
/// own copy of the rendering logic. It delegates to `liss_runtime::scriptio`
/// — the surviving, monomorphic, sink-based formatter — so there is exactly
/// one implementation of "how a language value prints" in production. The
/// *other* copy, `liss_runtime::speculum` (test-only), is the independent
/// oracle `scriptio` is checked against; this `Value` is not it.
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

/// Run `f` against a fresh buffer and return what it wrote — the adapter
/// between `scriptio`'s sink-based API and this method's `String`-returning
/// one.
fn rendered(f: impl FnOnce(&mut String)) -> String {
    let mut s = String::new();
    f(&mut s);
    s
}

impl Value {
    pub fn display(&self) -> String {
        match self {
            Value::Int(r) => rendered(|s| scriptio::scribe_int(r, s)),
            Value::Float(c) => rendered(|s| scriptio::scribe_frac(c, 12, s)),
            Value::Double(c) => rendered(|s| scriptio::scribe_frac(c, 288, s)),
            Value::Text(s) => s.clone(),
            Value::Char(c) => rendered(|s| scriptio::scribe_char(*c, s)),
            Value::Bool(b) => rendered(|s| scriptio::scribe_bool(*b, s)),
            Value::List(items) => {
                let inner: Vec<String> = items.iter().map(|v| v.display()).collect();
                format!("[{}]", inner.join(", "))
            }
        }
    }
}
