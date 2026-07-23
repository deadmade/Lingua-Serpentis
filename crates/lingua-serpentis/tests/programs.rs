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
    assert_eq!(run(src).unwrap(), "X\u{0305}X\u{0305}X\u{0305}DXXXII\n"); // 25 + 30507 = 30532 (not < 100, so unchanged)
}

#[test]
fn calc_example_still_runs() {
    let src = include_str!("../../../examples/calc.liss");
    assert_eq!(run(src).unwrap(), "X\u{0305}X\u{0305}X\u{0305}DXXXII\n");
}

#[test]
fn praecisio_example_runs() {
    // mnr grains at the uncia (1/12), nr at the scripulum (1/288); an
    // unconstrained fractional literal defaults to nr.
    let src = include_str!("../../../examples/praecisio.liss");
    assert_eq!(
        run(src).unwrap(),
        "I et triens\nI et triens\nI\nI et semuncia\nII et semis\nI et triens\n"
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
    assert_eq!(output, "V\n");
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].line, 1);
    assert!(warnings[0].message.contains("result"));
}

#[test]
fn latin_program_produces_no_warnings() {
    let src = "ni numerus = °V; scribe(numerus);";
    let (output, warnings) = run_with_warnings(src).unwrap();
    assert_eq!(output, "V\n");
    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
}

#[test]
fn type_errors_still_halt_before_any_linting() {
    // A type error aborts the pipeline, so no warnings are produced at all.
    assert!(run_with_warnings("ni x = \"salve\";").is_err());
}

#[test]
fn mnr_truncates_below_its_uncia_grain() {
    // 1/24 is below mnr's uncia grid, so it is dropped — only the whole prints.
    assert_eq!(run("mnr x = °I:I/XXIV; scribe(x);").unwrap(), "I\n");
}

#[test]
fn nr_keeps_the_sub_uncia_part() {
    // nr's scripulum grid holds 1/24 exactly: it is a semuncia.
    assert_eq!(run("nr y = °I:I/XXIV; scribe(y);").unwrap(), "I et semuncia\n");
}

#[test]
fn exact_third_prints_same_under_both() {
    // 1/3 = 4/12 is exact on both grids, so mnr and nr agree here.
    assert_eq!(run("mnr x = °I:I/III; scribe(x);").unwrap(), "I et triens\n");
    assert_eq!(run("nr y = °I:I/III; scribe(y);").unwrap(), "I et triens\n");
}

#[test]
fn representable_fraction_prints_same_under_both() {
    assert_eq!(run("mnr x = °V:I/II; scribe(x);").unwrap(), "V et semis\n");
    assert_eq!(run("nr y = °V:I/II; scribe(y);").unwrap(), "V et semis\n");
}

#[test]
fn mixing_float_types_is_a_type_error() {
    assert!(run("mnr a = °I:I/II; nr b = °I:I/IV; scribe(a + b);").is_err());
}

#[test]
fn nr_arithmetic_and_comparison_run() {
    assert_eq!(run("nr x = °I:I/IV + °I:I/IV; scribe(x);").unwrap(), "II et semis\n");
    assert_eq!(run("scribe(°I:I/II maior_est °I:I/IV);").unwrap(), "verus\n");
}

#[test]
fn unconstrained_fraction_defaults_to_nr() {
    // A bare fractional literal in an untyped sink defaults to nr.
    assert_eq!(run("scribe(°I:I/III);").unwrap(), "I et triens\n");
}

#[test]
fn fraction_divide_by_zero_errors() {
    // (½) / (½ − ½): a runtime zero divisor is an error, not inf.
    assert!(run("nr z = °I:I/II - °I:I/II; scribe(°I:I/II / z);").is_err());
}

#[test]
fn fraction_modulo_is_a_type_error() {
    assert!(run("nr x = °I:I/II % °I:I/IV;").is_err());
}

#[test]
fn series_example_runs() {
    let src = include_str!("../../../examples/series.liss");
    assert_eq!(run(src).unwrap(), "[I, XX, XXX, XL]\nXCI\n");
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
    assert_eq!(run(src).unwrap(), "III\nII\n");
}
