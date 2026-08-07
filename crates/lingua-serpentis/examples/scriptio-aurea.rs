//! `scriptio-aurea` — "the golden writing": a one-shot data generator, not a
//! test. It walks a fixed list of `Value`s through `Value::display()` (the
//! `scribe` formatter) and prints one `(kind, payload, expected)` triple per
//! line to stdout.
//!
//! Run once and commit the output as data:
//!
//! ```sh
//! cargo run -p lingua-serpentis --example scriptio-aurea \
//!   > crates/lingua-serpentis/tests/data/scriptio-aurea.txt
//! ```
//!
//! ## Line format
//!
//! Each line is three tab-separated fields — `kind\tpayload\texpected\n`:
//!
//! - `kind` — the `Value` variant's name (`Int`, `Float`, `Double`, `Text`,
//!   `Char`, `Bool`, `List`).
//! - `payload` — `{:?}` (`Debug`) of the value itself: unambiguous and
//!   reconstructible by eye, and — because it is `Debug`, not `Display` —
//!   guaranteed never to contain a raw tab or newline.
//! - `expected` — `Value::display()`'s actual output for that value: the
//!   contract this file pins.
//!
//! A tab is the only delimiter that survives every value in the list below.
//! Rendered fractions contain literal spaces and the word `et`
//! (`"I et triens"`), so a space-separated format is out. Lists render with
//! `", "` and `[`/`]` (`"[I, II, III]"`), so a comma-separated format is out
//! too. Tab appears in none of `Value::display()`'s own vocabulary — not in a
//! Roman numeral, not in Latin fraction words, not in `[`, `]`, `,`, `verus`,
//! `falsus`, or any single `char` — so splitting a line on `\t` needs no
//! escaping and is never ambiguous.

use lingua_serpentis::roman::Roman;
use lingua_serpentis::value::Value;

/// Grain count -> `mnr` fraction value (unciae, 1/12 of the `as`).
fn mnr(n: i64) -> Value {
    Value::Float(Roman::from_i64(n))
}

/// Grain count -> `nr` fraction value (scripula, 1/288 of the `as`).
fn nr(n: i64) -> Value {
    Value::Double(Roman::from_i64(n))
}

fn int(n: i64) -> Value {
    Value::Int(Roman::from_i64(n))
}

fn main() {
    let mut cases: Vec<Value> = Vec::new();

    // -- Int: zero, negative, single-tier, multi-tier, vinculum above M --
    // Same benchmark values `roman.rs`'s own unit tests use, for traceable
    // fidelity: `display_zero_and_negative`, `display_basic_and_subtractive`
    // (2024, 30532) and `display_vinculum_thousands` (4000).
    cases.push(int(0)); // zero
    cases.push(int(-25)); // negative
    cases.push(int(2024)); // single-tier: no vinculum needed (< 4000)
    cases.push(int(30532)); // multi-tier: crosses into the vinculum tier
    cases.push(int(4000)); // vinculum directly above an M (5000 - 1000)

    // -- mnr: every one of the twelve TWELFTHS names, 0 through a whole `as`.
    // Mirrors `fraction.rs`'s `display_every_twelfth_mnr` exactly: k=0 is
    // `nullus`, k=1..=11 are the eleven named twelfths in order, k=12 rolls
    // over to the whole numeral `I`.
    for k in 0..=12 {
        cases.push(mnr(k));
    }

    // -- whole-plus-fraction, both fraction types --
    cases.push(mnr(16)); // 1 as + 4 unciae = "I et triens"
    cases.push(nr(300)); // 1 as + 12 scripula = "I et semuncia"

    // -- nr: the maximal sub-uncia stack --
    // 41 scripula = 24 (uncia) + 12 (semuncia) + 4 (sextula) + 1 (scripulum):
    // every sub-uncia name appears exactly once, greedily, in one value.
    cases.push(nr(41)); // "uncia et semuncia et sextula et scripulum"

    // -- negative fractions, both types --
    cases.push(mnr(-16)); // "-I et triens"
    cases.push(nr(-12)); // "-semuncia"

    // -- Bool: both literals --
    cases.push(Value::Bool(true)); // "verus"
    cases.push(Value::Bool(false)); // "falsus"

    // -- Char: a multi-byte character --
    // U+1F40D SNAKE is 4 bytes in UTF-8 — apt for Lingua Serpentis, and a
    // sharper test of byte-vs-char handling than a 2-byte Latin-1
    // supplement character would be.
    cases.push(Value::Char('\u{1F40D}'));

    // -- Text: identity rendering, for completeness across every Value kind.
    cases.push(Value::Text("Lingua Serpentis".to_string()));

    // -- List: empty, nested, and a list of fractions --
    cases.push(Value::List(vec![])); // "[]"
    cases.push(Value::List(vec![
        Value::List(vec![int(1), int(2)]),
        Value::List(vec![int(3)]),
    ])); // "[[I, II], [III]]"
    cases.push(Value::List(vec![mnr(4), mnr(6)])); // "[triens, semis]"

    for v in &cases {
        let kind = match v {
            Value::Int(_) => "Int",
            Value::Float(_) => "Float",
            Value::Double(_) => "Double",
            Value::Text(_) => "Text",
            Value::Char(_) => "Char",
            Value::Bool(_) => "Bool",
            Value::List(_) => "List",
        };
        println!("{kind}\t{v:?}\t{}", v.display());
    }
}
