//! Randomised differential test: `scriptio`'s free functions must agree with
//! `speculum::Value::display` — the interpreter's original, independent
//! formatter — on every value proptest can generate. Golden vectors only
//! cover cases someone thought to write down; this is what catches the rest.

use crate::roman::Roman;
use crate::scriptio;
use crate::speculum::Value;
use proptest::prelude::*;

fn roman_strategy() -> impl Strategy<Value = Roman> {
    (any::<bool>(), proptest::collection::vec(0u8..10, 0..12))
        .prop_map(|(negative, digits)| {
            Roman::from_digits(negative, &digits).expect("digits 0..=9 are always in range")
        })
}

fn text_strategy() -> impl Strategy<Value = String> {
    proptest::collection::vec(any::<char>(), 0..12).prop_map(|cs| cs.into_iter().collect())
}

fn value_strategy() -> impl Strategy<Value = Value> {
    prop_oneof![
        roman_strategy().prop_map(Value::Int),
        roman_strategy().prop_map(Value::Float),
        roman_strategy().prop_map(Value::Double),
        text_strategy().prop_map(Value::Text),
        any::<char>().prop_map(Value::Char),
        any::<bool>().prop_map(Value::Bool),
    ]
}

/// `scriptio`'s rendering of `v`, or `None` for `List` — `scriptio` has no
/// list formatter (it is `ekind`-driven and lands in `liss-abi`, Task 8), so
/// there is nothing to compare for that variant.
fn scribed(v: &Value) -> Option<String> {
    let mut s = String::new();
    match v {
        Value::Int(r) => scriptio::scribe_int(r, &mut s),
        Value::Float(c) => scriptio::scribe_frac(c, 12, &mut s),
        Value::Double(c) => scriptio::scribe_frac(c, 288, &mut s),
        Value::Text(t) => scriptio::scribe_text(t, &mut s),
        Value::Char(c) => scriptio::scribe_char(*c, &mut s),
        Value::Bool(b) => scriptio::scribe_bool(*b, &mut s),
        Value::List(_) => return None,
    }
    Some(s)
}

proptest! {
    #[test]
    fn scriptio_agrees_with_speculum(v in value_strategy()) {
        if let Some(got) = scribed(&v) {
            prop_assert_eq!(got, v.display());
        }
    }
}
