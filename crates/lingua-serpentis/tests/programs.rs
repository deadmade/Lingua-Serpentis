// Integration tests drive the whole pipeline via the library API.
use lingua_serpentis::run;

#[test]
fn full_program_with_function_and_if() {
    let src = r#"
        munus summare :: ni -> ni -> ni
        summare(a, b) {
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
