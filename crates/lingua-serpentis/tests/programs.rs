// Integration tests drive the whole pipeline via the library API.
use lingua_serpentis::run;
use lingua_serpentis::run_with_warnings;

#[test]
fn full_program_with_function_and_if() {
    let src = r#"
        munus summare capit (ni a, ni b) reddit ni {
            ni c = a + b;
            redde c;
        }
        ni summa = summare(°XXV, °XXX.M.V.C.VII);
        si (summa minor_est °C) { summa = summa + °C; }
        scribe(summa);
    "#;
    assert_eq!(run(src).unwrap(), "30532\n"); // 25 + 30507 = 30532 (not < 100, so unchanged)
}

#[test]
fn calc_example_still_runs() {
    let src = include_str!("../../../examples/calc.liss");
    assert_eq!(run(src).unwrap(), "30532\n");
}

#[test]
fn praecisio_example_runs() {
    // Keeps the mnr/nr precision example honest: mnr rounds at f32, nr at f64,
    // and an unconstrained fractional literal defaults to nr.
    let src = include_str!("../../../examples/praecisio.liss");
    assert_eq!(
        run(src).unwrap(),
        "1.3333334\n1.3333333333333333\n2.5\n1.3333333333333333\n"
    );
}

#[test]
fn type_error_reports_latin_and_line() {
    let src = "ni x = \"salve\";";
    let err = run(src).unwrap_err();
    assert_eq!(err.line, 1);
    assert!(err.to_string().starts_with("Error in linea 1:"));
}

#[test]
fn char_and_text_comparison_end_to_end() {
    assert_eq!(lingua_serpentis::run("lit c = 'z'; scribe(c);").unwrap(), "z\n");
    assert_eq!(lingua_serpentis::run("scribe(\"apple\" minor_est \"banana\");").unwrap(), "verus\n");
}

#[test]
fn non_latin_name_warns_but_program_still_runs() {
    let src = "ni result = °V; scribe(result);";
    let (output, warnings) = run_with_warnings(src).unwrap();
    // The warning must not change the program's behaviour.
    assert_eq!(output, "5\n");
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].line, 1);
    assert!(warnings[0].message.contains("result"));
}

#[test]
fn latin_program_produces_no_warnings() {
    let src = "ni numerus = °V; scribe(numerus);";
    let (output, warnings) = run_with_warnings(src).unwrap();
    assert_eq!(output, "5\n");
    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
}

#[test]
fn type_errors_still_halt_before_any_linting() {
    // A type error aborts the pipeline, so no warnings are produced at all.
    assert!(run_with_warnings("ni x = \"salve\";").is_err());
}

#[test]
fn mnr_is_single_precision() {
    // 1 + 1/3 has no exact binary form; f32 and f64 round it differently.
    // This pins that `mnr` really rounds at f32 precision.
    assert_eq!(run("mnr x = °I:I/III; scribe(x);").unwrap(), "1.3333334\n");
}

#[test]
fn nr_is_double_precision() {
    assert_eq!(run("nr y = °I:I/III; scribe(y);").unwrap(), "1.3333333333333333\n");
}

#[test]
fn representable_fraction_prints_same_under_both() {
    // 5.5 is exact in both, so the two types agree here — guards against a test
    // that leans on divergence for a value that does not actually diverge.
    assert_eq!(run("mnr x = °V:I/II; scribe(x);").unwrap(), "5.5\n");
    assert_eq!(run("nr y = °V:I/II; scribe(y);").unwrap(), "5.5\n");
}

#[test]
fn mixing_float_types_is_a_type_error() {
    assert!(run("mnr a = °I:I/II; nr b = °I:I/IV; scribe(a + b);").is_err());
}

#[test]
fn nr_arithmetic_and_comparison_run() {
    // Exercises the interpreter's Double arms end-to-end: 1.25 + 1.25 = 2.5,
    // and 1.5 > 1.25. Without these the eval_binary/cmp Double arms are untested.
    assert_eq!(run("nr x = °I:I/IV + °I:I/IV; scribe(x);").unwrap(), "2.5\n");
    assert_eq!(run("scribe(°I:I/II maior_est °I:I/IV);").unwrap(), "verus\n");
}

#[test]
fn unconstrained_fraction_prints_at_nr_precision() {
    // A bare fractional literal in an untyped sink defaults to nr (f64),
    // so it prints at double precision — not the f32 "1.3333334".
    assert_eq!(run("scribe(°I:I/III);").unwrap(), "1.3333333333333333\n");
}

#[test]
fn series_example_runs() {
    let src = include_str!("../../../examples/series.liss");
    assert_eq!(run(src).unwrap(), "[1, 20, 30, 40]\n91\n");
}

#[test]
fn lists_have_value_semantics() {
    // Passing a list to a function copies it: the caller's list is untouched
    // even though the callee appends to its own copy.
    let src = r#"
        munus auctus capit (ni lista s) reddit ni {
            s = adde(s, °C);
            redde longitudo(s);
        }
        ni lista xs = [°I, °II];
        ni n = auctus(xs);
        scribe(n);
        scribe(longitudo(xs));
    "#;
    assert_eq!(run(src).unwrap(), "3\n2\n");
}
