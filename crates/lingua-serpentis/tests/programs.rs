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
        munus initium reddit ni {
            ni summa = summare(°XXV, °XXX.M.V.C.VII);
            si (summa minor_est °C) { summa = summa + °C; }
            scribe(summa);
            redde nullus;
        }
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
    let src = "munus initium reddit ni { ni x = \"salve\"; redde nullus; }";
    let err = run(src).unwrap_err();
    assert_eq!(err.line, 1);
    assert!(err.to_string().starts_with("Error in linea 1:"));
}

#[test]
fn char_and_text_comparison_end_to_end() {
    assert_eq!(
        lingua_serpentis::run("munus initium reddit ni { lit c = 'z'; scribe(c); redde nullus; }").unwrap(),
        "z\n"
    );
    assert_eq!(
        lingua_serpentis::run("munus initium reddit ni { scribe(\"apple\" minor_est \"banana\"); redde nullus; }").unwrap(),
        "verus\n"
    );
}

#[test]
fn non_latin_name_warns_but_program_still_runs() {
    let src = "munus initium reddit ni { ni result = °V; scribe(result); redde nullus; }";
    let (output, warnings) = run_with_warnings(src).unwrap();
    // The warning must not change the program's behaviour.
    assert_eq!(output, "V\n");
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].line, Some(1));
    assert!(warnings[0].message.contains("result"));
}

#[test]
fn latin_program_produces_no_warnings() {
    let src = "munus initium reddit ni { ni numerus = °V; scribe(numerus); redde nullus; }";
    let (output, warnings) = run_with_warnings(src).unwrap();
    assert_eq!(output, "V\n");
    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
}

#[test]
fn type_errors_halt_but_their_warnings_survive() {
    // A type error still halts the pipeline. What changed with D-09 is that
    // the Latinity warnings computed along the way are no longer thrown away
    // with it: a program with two problems reports both.
    //
    // The old name and comment here claimed "no warnings are produced at all",
    // which the register read as this test pinning D-09's behaviour as
    // correct. It never did — the assertion was only `is_err()`, and it passes
    // unchanged. It was the prose that was wrong.
    assert!(run_with_warnings("munus initium reddit ni { ni x = \"salve\"; redde nullus; }").is_err());

    let e = lingua_serpentis::run_program_with_warnings(&[(
        "principale.liss",
        "munus initium reddit ni { ni counter = \"salve\"; redde nullus; }",
    )])
    .expect_err("assigning a voc to a ni is a type error");
    assert_eq!(
        e.warnings.iter().map(|w| w.message.as_str()).collect::<Vec<_>>(),
        vec!["nomen 'counter' fortasse non Latinum est"]
    );
}

#[test]
fn mnr_truncates_below_its_uncia_grain() {
    // 1/24 is below mnr's uncia grid, so it is dropped — only the whole prints.
    assert_eq!(run("munus initium reddit ni { mnr x = °I:I/XXIV; scribe(x); redde nullus; }").unwrap(), "I\n");
}

#[test]
fn nr_keeps_the_sub_uncia_part() {
    // nr's scripulum grid holds 1/24 exactly: it is a semuncia.
    assert_eq!(run("munus initium reddit ni { nr y = °I:I/XXIV; scribe(y); redde nullus; }").unwrap(), "I et semuncia\n");
}

#[test]
fn exact_third_prints_same_under_both() {
    // 1/3 = 4/12 is exact on both grids, so mnr and nr agree here.
    assert_eq!(run("munus initium reddit ni { mnr x = °I:I/III; scribe(x); redde nullus; }").unwrap(), "I et triens\n");
    assert_eq!(run("munus initium reddit ni { nr y = °I:I/III; scribe(y); redde nullus; }").unwrap(), "I et triens\n");
}

#[test]
fn representable_fraction_prints_same_under_both() {
    assert_eq!(run("munus initium reddit ni { mnr x = °V:I/II; scribe(x); redde nullus; }").unwrap(), "V et semis\n");
    assert_eq!(run("munus initium reddit ni { nr y = °V:I/II; scribe(y); redde nullus; }").unwrap(), "V et semis\n");
}

#[test]
fn mixing_float_types_is_a_type_error() {
    assert!(run("munus initium reddit ni { mnr a = °I:I/II; nr b = °I:I/IV; scribe(a + b); redde nullus; }").is_err());
}

#[test]
fn nr_arithmetic_and_comparison_run() {
    assert_eq!(run("munus initium reddit ni { nr x = °I:I/IV + °I:I/IV; scribe(x); redde nullus; }").unwrap(), "II et semis\n");
    assert_eq!(run("munus initium reddit ni { scribe(°I:I/II maior_est °I:I/IV); redde nullus; }").unwrap(), "verus\n");
}

#[test]
fn unconstrained_fraction_defaults_to_nr() {
    // A bare fractional literal in an untyped sink defaults to nr.
    assert_eq!(run("munus initium reddit ni { scribe(°I:I/III); redde nullus; }").unwrap(), "I et triens\n");
}

#[test]
fn fraction_divide_by_zero_errors() {
    // (½) / (½ − ½): a runtime zero divisor is an error, not inf.
    assert!(run("munus initium reddit ni { nr z = °I:I/II - °I:I/II; scribe(°I:I/II / z); redde nullus; }").is_err());
}

#[test]
fn fraction_modulo_is_a_type_error() {
    assert!(run("munus initium reddit ni { nr x = °I:I/II % °I:I/IV; redde nullus; }").is_err());
}

#[test]
fn negative_fractions_print_signed() {
    // Unary minus on a fraction: mnr drops the 1/24 (truncation toward zero) and
    // keeps the sign; nr keeps it as a semuncia. -(1 + 1/24).
    assert_eq!(run("munus initium reddit ni { mnr n = -°I:I/XXIV; scribe(n); redde nullus; }").unwrap(), "-I\n");
    assert_eq!(run("munus initium reddit ni { nr n = -°I:I/XXIV; scribe(n); redde nullus; }").unwrap(), "-I et semuncia\n");
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
    let src = "munus auctus capit (ni lista s) reddit ni { s = adde(s, °C); redde longitudo(s); } munus initium reddit ni { ni lista xs = [°I, °II]; ni n = auctus(xs); scribe(n); scribe(longitudo(xs)); redde nullus; }";
    assert_eq!(run(src).unwrap(), "III\nII\n");
}

use lingua_serpentis::run_program_with_warnings;

#[test]
fn multi_file_program_with_package_runs() {
    let files = [
        ("principale.liss",
         "affer mathematica;\nmunus initium reddit ni { scribe(mathematica.summa(°XXV, °XVII)); redde nullus; }"),
        ("mathematica/summa.liss",
         "munus summa capit (ni a, ni b) reddit ni { redde a + b; }"),
    ];
    let (output, code, warnings) = run_program_with_warnings(&files).unwrap();
    assert_eq!(output, "XLII\n");
    assert_eq!(code, 0);
    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
}

#[test]
fn same_folder_files_need_no_import() {
    let files = [
        ("auxilia.liss", "munus quinque reddit ni { redde °V; }"),
        ("principale.liss", "munus initium reddit ni { scribe(quinque()); redde nullus; }"),
    ];
    assert_eq!(run_program_with_warnings(&files).unwrap().0, "V\n");
}

#[test]
fn program_without_initium_is_a_bare_error() {
    let files = [("a.liss", "munus f reddit ni { redde nullus; }")];
    let e = run_program_with_warnings(&files).unwrap_err();
    assert_eq!(e.to_string(), "Error: munus 'initium' in programmate non inventum");
}

#[test]
fn multi_file_error_names_file_and_line() {
    let files = [
        ("principale.liss",
         "affer mathematica;\nmunus initium reddit ni { redde mathematica.summa(°I, °II); }"),
        ("mathematica/summa.liss",
         "// prima linea\nmunus summa capit (ni a, ni b) reddit ni { redde \"salve\"; }"),
    ];
    let e = run_program_with_warnings(&files).unwrap_err();
    assert!(e.to_string().starts_with("Error in 'mathematica/summa.liss', linea 2:"),
        "got: {e}");
}

#[test]
fn exit_status_surfaces_through_the_pipeline() {
    let files = [("a.liss", "munus initium reddit ni { redde °V; }")];
    assert_eq!(run_program_with_warnings(&files).unwrap().1, 5);
}

#[test]
fn import_cycle_runs() {
    let files = [
        ("a.liss", "affer par;\nmunus initium reddit ni { scribe(par.aequum(°X)); redde nullus; }"),
        ("par/p.liss",
         "affer impar;\nmunus aequum capit (ni n) reddit ni { si (n par_est nullus) { redde °I; } redde impar.imparum(n - °I); }"),
        ("impar/i.liss",
         "affer par;\nmunus imparum capit (ni n) reddit ni { si (n par_est nullus) { redde nullus; } redde par.aequum(n - °I); }"),
    ];
    // 10 is even → aequum returns I
    assert_eq!(run_program_with_warnings(&files).unwrap().0, "I\n");
}

#[test]
fn non_latin_package_name_warns_but_runs() {
    let files = [
        ("principale.liss",
         "affer foobar;\nmunus initium reddit ni { scribe(foobar.quinque()); redde nullus; }"),
        ("foobar/x.liss", "munus quinque reddit ni { redde °V; }"),
    ];
    let (output, _, warnings) = run_program_with_warnings(&files).unwrap();
    assert_eq!(output, "V\n");
    assert!(warnings.iter().any(|w| w.message.contains("fasciculi 'foobar'")),
        "got: {warnings:?}");
}

#[test]
fn top_level_statement_is_now_an_error() {
    let e = run("scribe(°I);").unwrap_err();
    assert!(e.message.contains("nihil nisi declarationes"), "got: {}", e.message);
    assert_eq!(e.line, 1);
}

#[test]
fn single_file_program_still_needs_initium() {
    let e = run("munus f reddit ni { redde °I; }").unwrap_err();
    assert!(e.message.contains("initium"), "got: {}", e.message);
}

#[test]
fn intra_package_unqualified_calls_run() {
    // Two files in the same package call each other with bare (unqualified)
    // names — the checker validates this against the package's own function
    // table, and the interpreter must resolve it there too, not against the
    // root namespace (regression test for the package bare-call bug).
    let files = [
        ("principale.liss",
         "affer mathematica;\nmunus initium reddit ni { scribe(mathematica.decem()); redde nullus; }"),
        ("mathematica/quinque.liss", "munus quinque reddit ni { redde °V; }"),
        ("mathematica/decem.liss", "munus decem reddit ni { redde quinque() + quinque(); }"),
    ];
    let (output, code, warnings) = run_program_with_warnings(&files).unwrap();
    assert_eq!(output, "X\n");
    assert_eq!(code, 0);
    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
}

#[test]
fn computatrum_example_runs() {
    let files = [
        ("principale.liss", include_str!("../../../examples/computatrum/principale.liss")),
        ("mathematica/summa.liss", include_str!("../../../examples/computatrum/mathematica/summa.liss")),
        ("mathematica/multiplicatio.liss", include_str!("../../../examples/computatrum/mathematica/multiplicatio.liss")),
    ];
    let (output, code, warnings) = run_program_with_warnings(&files).unwrap();
    assert_eq!(output, "XLII\nXLII\n");
    assert_eq!(code, 0);
    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
}
