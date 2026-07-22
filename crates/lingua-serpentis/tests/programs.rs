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
