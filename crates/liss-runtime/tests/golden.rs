//! Golden vectors captured from the interpreter (spec §10.4); see docs/internals/testing.md.

use liss_runtime::roman::Roman;
use liss_runtime::scriptio::{scribe_bool, scribe_char, scribe_frac, scribe_int, scribe_text};

const DATA: &str = include_str!("data/scriptio-aurea.txt");

fn rendered(f: impl FnOnce(&mut String)) -> String {
    let mut s = String::new();
    f(&mut s);
    s
}

fn parse_roman(payload: &str) -> Roman {
    let negative = payload.contains("negative: true");
    let start = payload.find('[').expect("tiers has an opening bracket");
    let end = payload.find(']').expect("tiers has a closing bracket");
    let digits: Vec<u8> = payload[start + 1..end]
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.parse().expect("a digit 0..=9"))
        .collect();
    Roman::from_digits(negative, &digits).expect("golden digits are always 0..=9")
}

fn parse_char(payload: &str) -> char {
    let start = payload.find('\'').expect("opening quote");
    let rest = &payload[start + 1..];
    let end = rest.rfind('\'').expect("closing quote");
    rest[..end].chars().next().expect("exactly one char")
}

fn parse_text(payload: &str) -> String {
    let start = payload.find('"').expect("opening quote");
    let end = payload.rfind('"').expect("closing quote");
    payload[start + 1..end].to_string()
}

#[test]
fn scriptio_matches_the_golden_vectors() {
    let mut checked = 0;
    for (i, line) in DATA.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        let mut fields = line.splitn(3, '\t');
        let kind = fields.next().expect("kind field");
        let payload = fields.next().expect("payload field");
        let expected = fields.next().expect("expected field");
        let got = match kind {
            "Int" => rendered(|s| scribe_int(&parse_roman(payload), s)),
            "Float" => rendered(|s| scribe_frac(&parse_roman(payload), 12, s)),
            "Double" => rendered(|s| scribe_frac(&parse_roman(payload), 288, s)),
            "Bool" => rendered(|s| scribe_bool(payload.contains("true"), s)),
            "Char" => rendered(|s| scribe_char(parse_char(payload), s)),
            "Text" => rendered(|s| scribe_text(&parse_text(payload), s)),
            // No list formatter in `scriptio`.
            "List" => continue,
            other => panic!("line {}: unknown kind {other:?}", i + 1),
        };
        assert_eq!(got, expected, "line {} ({kind}): {payload}", i + 1);
        checked += 1;
    }
    assert!(checked >= 20, "expected at least 20 non-list rows checked, got {checked}");
}

#[test]
fn golden_vectors_exercise_every_subuncia_denomination() {
    for name in ["semuncia", "sicilicus", "sextula", "dimidia sextula", "scripulum"] {
        assert!(DATA.contains(name), "no golden vector renders {name:?}");
    }
}
