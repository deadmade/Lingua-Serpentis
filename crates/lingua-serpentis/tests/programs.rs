//! Whole programs, compiled and run: the pipeline end to end.

mod support;

use lingua_serpentis::Rendered;
use support::proc::temp_dir;

/// One single-source program; its diagnostics name no file (`in linea N`).
fn curre(fons: &str) -> Rendered {
    let d = temp_dir("liss-programs");
    let p = d.write("fons.liss", fons);
    support::curre(&p).unwrap_or_else(|e| panic!("{e}"))
}

/// A multi-file program given as (program-relative path, source) pairs.
fn curre_fasciculos(scidae: &[(&str, &str)]) -> Rendered {
    let d = temp_dir("liss-programs");
    for (via, fons) in scidae {
        d.write(via, fons);
    }
    support::curre(d.path()).unwrap_or_else(|e| panic!("{e}"))
}

/// What a program printed, having required exit 0 and an empty stderr.
fn emissio(r: &Rendered) -> &str {
    emissio_cum_monitis(r, "")
}

/// The expected lines are the same bytes `conformance/currunt/exemplum-praecisio` blesses.
fn emissio_cum_monitis<'a>(r: &'a Rendered, monita: &str) -> &'a str {
    assert_eq!(r.code, 0, "expected a clean exit; stderr: {:?}", r.stderr);
    assert_eq!(r.stderr, monita, "the diagnostics are not the expected ones");
    &r.stdout
}

/// Exit 1, nothing on stdout.
fn erratum(r: &Rendered) -> &str {
    assert_eq!(r.code, 1, "expected a failure; stdout {:?}, stderr {:?}", r.stdout, r.stderr);
    assert_eq!(r.stdout, "", "a failing program printed something unexpected");
    &r.stderr
}

/// An error block whose location row names `locus` — the block grammar's replacement
/// for `starts_with("Error in linea N:")`, now that the location is its own row and
/// the line is a Roman numeral.
fn erratum_apud(stderr: &str, locus: &str) -> bool {
    stderr.starts_with("erratum: ") && stderr.contains(&format!("\n  in {locus}\n"))
}

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
    // 25 + 30507 = 30532 (not < 100, so unchanged)
    assert_eq!(emissio(&curre(src)), "X\u{0305}X\u{0305}X\u{0305}DXXXII\n");
}

#[test]
fn calc_example_still_runs() {
    let src = include_str!("../../../examples/calc.liss");
    assert_eq!(emissio(&curre(src)), "X\u{0305}X\u{0305}X\u{0305}DXXXII\n");
}

#[test]
fn praecisio_example_runs() {
    let src = include_str!("../../../examples/praecisio.liss");
    let r = curre(src);
    assert_eq!(
        emissio_cum_monitis(
            &r,
            // The gutter is four columns wide for `XI` and five for `XII`: the width
            // is the numeral's, and these two lines are where that shows.
            "monitum: nomen 'tertia_brevis' fortasse non Latinum est\n  in linea XI\n\
             \x20   |\n XI |     mnr tertia_brevis = °I:I/III;   // granum unciae\n\
             \x20   |         ^^^^^^^^^^^^^\n\
             monitum: nomen 'tertia_longa' fortasse non Latinum est\n  in linea XII\n\
             \x20    |\n XII |     nr tertia_longa = °I:I/III;     // granum scripuli\n\
             \x20    |        ^^^^^^^^^^^^\n",
        ),
        "I et triens\nI et triens\nI\nI et semuncia\nII et semis\nI et triens\n"
    );
}

#[test]
fn type_error_reports_latin_and_line() {
    let src = "munus initium reddit ni { ni x = \"salve\"; redde nullus; }";
    assert!(
        erratum_apud(erratum(&curre(src)), "linea I"),
        "got: {:?}",
        curre(src).stderr
    );
}

#[test]
fn char_and_text_comparison_end_to_end() {
    assert_eq!(
        emissio(&curre("munus initium reddit ni { lit c = 'z'; scribe(c); redde nullus; }")),
        "z\n"
    );
    assert_eq!(
        emissio(&curre(
            "munus initium reddit ni { scribe(\"apple\" minor_est \"banana\"); redde nullus; }"
        )),
        "verus\n"
    );
}

#[test]
fn non_latin_name_warns_but_program_still_runs() {
    let src = "munus initium reddit ni { ni result = °V; scribe(result); redde nullus; }";
    let r = curre(src);
    assert_eq!(r.stdout, "V\n");
    assert_eq!(r.code, 0);
    assert_eq!(
        r.stderr,
        "monitum: nomen 'result' fortasse non Latinum est\n  in linea I\n   |\n \
         I | munus initium reddit ni { ni result = °V; scribe(result); redde nullus; }\n   \
         |                              ^^^^^^\n"
    );
}

#[test]
fn latin_program_produces_no_warnings() {
    let src = "munus initium reddit ni { ni numerus = °V; scribe(numerus); redde nullus; }";
    assert_eq!(emissio(&curre(src)), "V\n");
}

#[test]
fn type_errors_halt_but_their_warnings_survive() {
    // D-09
    let r = curre("munus initium reddit ni { ni x = \"salve\"; redde nullus; }");
    assert_eq!(r.code, 1);

    let r = curre_fasciculos(&[(
        "principale.liss",
        "munus initium reddit ni { ni counter = \"salve\"; redde nullus; }",
    )]);
    assert_eq!(r.code, 1);
    // Ordering, not adjacency: each diagnostic is a block now, so the error's header
    // is two rows below the warning's rather than one.
    let monitum = r
        .stderr
        .find("monitum: nomen 'counter' fortasse non Latinum est")
        .unwrap_or_else(|| panic!("no warning in {:?}", r.stderr));
    let erratum = r
        .stderr
        .find("erratum: ")
        .unwrap_or_else(|| panic!("no error in {:?}", r.stderr));
    assert!(
        monitum < erratum,
        "the warning must come before the error it survived: {:?}",
        r.stderr,
    );
    assert!(
        r.stderr[erratum..].contains("\n  in 'principale.liss', linea I\n"),
        "the error must name the file and line: {:?}",
        r.stderr,
    );
}

#[test]
fn mnr_truncates_below_its_uncia_grain() {
    let r = curre("munus initium reddit ni { mnr x = °I:I/XXIV; scribe(x); redde nullus; }");
    assert_eq!(emissio(&r), "I\n");
}

#[test]
fn nr_keeps_the_sub_uncia_part() {
    let r = curre("munus initium reddit ni { nr y = °I:I/XXIV; scribe(y); redde nullus; }");
    assert_eq!(emissio(&r), "I et semuncia\n");
}

#[test]
fn exact_third_prints_same_under_both() {
    let m = curre("munus initium reddit ni { mnr x = °I:I/III; scribe(x); redde nullus; }");
    assert_eq!(emissio(&m), "I et triens\n");
    let n = curre("munus initium reddit ni { nr y = °I:I/III; scribe(y); redde nullus; }");
    assert_eq!(emissio(&n), "I et triens\n");
}

#[test]
fn representable_fraction_prints_same_under_both() {
    let m = curre("munus initium reddit ni { mnr x = °V:I/II; scribe(x); redde nullus; }");
    assert_eq!(emissio(&m), "V et semis\n");
    let n = curre("munus initium reddit ni { nr y = °V:I/II; scribe(y); redde nullus; }");
    assert_eq!(emissio(&n), "V et semis\n");
}

#[test]
fn mixing_float_types_is_a_type_error() {
    let r = curre(
        "munus initium reddit ni { mnr a = °I:I/II; nr b = °I:I/IV; scribe(a + b); redde nullus; }",
    );
    assert!(erratum_apud(erratum(&r), "linea I"), "got: {:?}", r.stderr);
}

#[test]
fn nr_arithmetic_and_comparison_run() {
    let r = curre("munus initium reddit ni { nr x = °I:I/IV + °I:I/IV; scribe(x); redde nullus; }");
    assert_eq!(emissio(&r), "II et semis\n");
    let r = curre("munus initium reddit ni { scribe(°I:I/II maior_est °I:I/IV); redde nullus; }");
    assert_eq!(emissio(&r), "verus\n");
}

#[test]
fn unconstrained_fraction_defaults_to_nr() {
    let r = curre("munus initium reddit ni { scribe(°I:I/III); redde nullus; }");
    assert_eq!(emissio(&r), "I et triens\n");
}

#[test]
fn fraction_divide_by_zero_errors() {
    let r = curre(
        "munus initium reddit ni { nr z = °I:I/II - °I:I/II; scribe(°I:I/II / z); redde nullus; }",
    );
    // Raised by the compiled image, not the compiler, so it keeps the one-line
    // grammar until `abi` adopts the block form.
    assert_eq!(erratum(&r), "erratum: divisio per nullum\n  in linea I\n");
}

#[test]
fn fraction_modulo_is_a_type_error() {
    let r = curre("munus initium reddit ni { nr x = °I:I/II % °I:I/IV; redde nullus; }");
    assert_eq!(r.code, 1);
}

#[test]
fn negative_fractions_print_signed() {
    let m = curre("munus initium reddit ni { mnr n = -°I:I/XXIV; scribe(n); redde nullus; }");
    assert_eq!(emissio(&m), "-I\n");
    let n = curre("munus initium reddit ni { nr n = -°I:I/XXIV; scribe(n); redde nullus; }");
    assert_eq!(emissio(&n), "-I et semuncia\n");
}

#[test]
fn series_example_runs() {
    let src = include_str!("../../../examples/series.liss");
    assert_eq!(emissio(&curre(src)), "[I, XX, XXX, XL]\nXCI\n");
}

#[test]
fn lists_have_value_semantics() {
    let src = "munus auctus capit (ni lista s) reddit ni { s = adde(s, °C); redde longitudo(s); } munus initium reddit ni { ni lista xs = [°I, °II]; ni n = auctus(xs); scribe(n); scribe(longitudo(xs)); redde nullus; }";
    let r = curre(src);
    assert_eq!(
        emissio_cum_monitis(
            &r,
            "monitum: nomen 'xs' fortasse non Latinum est\n  in linea I\n",
        ),
        "III\nII\n"
    );
}

#[test]
fn multi_file_program_with_package_runs() {
    let files = [
        ("principale.liss",
         "affer mathematica;\nmunus initium reddit ni { scribe(mathematica.summa(°XXV, °XVII)); redde nullus; }"),
        ("mathematica/summa.liss",
         "munus summa capit (ni a, ni b) reddit ni { redde a + b; }"),
    ];
    assert_eq!(emissio(&curre_fasciculos(&files)), "XLII\n");
}

#[test]
fn same_folder_files_need_no_import() {
    let files = [
        ("auxilia.liss", "munus quinque reddit ni { redde °V; }"),
        ("principale.liss", "munus initium reddit ni { scribe(quinque()); redde nullus; }"),
    ];
    assert_eq!(emissio(&curre_fasciculos(&files)), "V\n");
}

#[test]
fn program_without_initium_is_a_bare_error() {
    let files = [("a.liss", "munus f reddit ni { redde nullus; }")];
    assert_eq!(
        erratum(&curre_fasciculos(&files)),
        "erratum: munus 'initium' in programmate non inventum\n"
    );
}

#[test]
fn multi_file_error_names_file_and_line() {
    let files = [
        ("principale.liss",
         "affer mathematica;\nmunus initium reddit ni { redde mathematica.summa(°I, °II); }"),
        ("mathematica/summa.liss",
         "// prima linea\nmunus summa capit (ni a, ni b) reddit ni { redde \"salve\"; }"),
    ];
    let r = curre_fasciculos(&files);
    assert!(
        erratum_apud(erratum(&r), "'mathematica/summa.liss', linea II"),
        "got: {:?}",
        r.stderr,
    );
}

#[test]
fn exit_status_surfaces_through_the_pipeline() {
    let files = [("a.liss", "munus initium reddit ni { redde °V; }")];
    let r = curre_fasciculos(&files);
    assert_eq!(r.code, 5);
    assert_eq!(r.stderr, "");
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
    assert_eq!(emissio(&curre_fasciculos(&files)), "I\n");
}

#[test]
fn non_latin_package_name_warns_but_runs() {
    let files = [
        ("principale.liss",
         "affer foobar;\nmunus initium reddit ni { scribe(foobar.quinque()); redde nullus; }"),
        ("foobar/x.liss", "munus quinque reddit ni { redde °V; }"),
    ];
    let r = curre_fasciculos(&files);
    assert_eq!(r.stdout, "V\n");
    assert_eq!(r.code, 0);
    assert!(
        r.stderr.lines().any(|l| l.contains("fasciculi 'foobar'")),
        "got: {:?}",
        r.stderr,
    );
}

#[test]
fn top_level_statement_is_now_an_error() {
    let r = curre("scribe(°I);");
    let e = erratum(&r);
    assert!(e.contains("nihil nisi declarationes"), "got: {e:?}");
    assert!(erratum_apud(e, "linea I"), "got: {e:?}");
}

#[test]
fn single_file_program_still_needs_initium() {
    let r = curre("munus f reddit ni { redde °I; }");
    assert!(erratum(&r).contains("initium"), "got: {:?}", r.stderr);
}

#[test]
fn intra_package_unqualified_calls_run() {
    // Codegen must resolve a bare call against the package's table, not the root namespace.
    let files = [
        ("principale.liss",
         "affer mathematica;\nmunus initium reddit ni { scribe(mathematica.decem()); redde nullus; }"),
        ("mathematica/quinque.liss", "munus quinque reddit ni { redde °V; }"),
        ("mathematica/decem.liss", "munus decem reddit ni { redde quinque() + quinque(); }"),
    ];
    assert_eq!(emissio(&curre_fasciculos(&files)), "X\n");
}

#[test]
fn computatrum_example_runs() {
    let files = [
        ("principale.liss", include_str!("../../../examples/computatrum/principale.liss")),
        ("mathematica/summa.liss", include_str!("../../../examples/computatrum/mathematica/summa.liss")),
        ("mathematica/multiplicatio.liss", include_str!("../../../examples/computatrum/mathematica/multiplicatio.liss")),
    ];
    assert_eq!(emissio(&curre_fasciculos(&files)), "XLII\nXLII\n");
}
