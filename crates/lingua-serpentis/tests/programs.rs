//! Whole programs, compiled and run: the pipeline end to end.
//!
//! # These are the same tests, relocated rather than rewritten
//!
//! Every case here used to call `lingua_serpentis::run`, `run_with_warnings` or
//! `run_program_with_warnings` — the library entry points the interpreter sat
//! behind. Spec §3.1 removes the interpreter and Task 19 removes those entry
//! points with it, so what these tests said ("run this program, check what it
//! printed") now has exactly one spelling: `liss run`.
//!
//! **Not one expectation changed in the move.** Every string below is the one
//! the interpreted pipeline was asserted against, and each reproduces byte for
//! byte through the compiler. Where a test asserted something structural about
//! a `ProgramError` or a `Vec<Warning>` — `err.line == 1`, `warnings.len() == 1`
//! — the assertion is now made against the *rendered* form of the same fact,
//! because a process has two streams and no return type. `lingua_serpentis::
//! render` is what draws it, and it is unchanged: an error is
//! `Error in linea N: …` on stderr with exit 1, a warning is `Monitum …` on
//! stderr before it.
//!
//! That rendering is not this file's opinion. It is what `cli.rs` and the whole
//! `conformance/` corpus already assert, and Task 17's differential proved the
//! compiled path reproduces it for all 113 fixtures before any of this moved.

mod support;

use lingua_serpentis::Rendered;
use support::proc::temp_dir;

/// Run one **single-source** program: written to a `.liss` file and handed to
/// `liss run`, which is the degenerate case of §4.1 — same rules, but errors
/// and warnings keep the file-less phrasing (`Error in linea N:`).
///
/// The file is named `fons.liss` because that is the synthetic name the
/// single-source path has always used, and because the name is then stripped
/// from every diagnostic anyway.
fn curre(fons: &str) -> Rendered {
    let d = temp_dir("liss-programs");
    let p = d.write("fons.liss", fons);
    support::curre(&p).unwrap_or_else(|e| panic!("{e}"))
}

/// Run a **multi-file** program given as (program-relative path, source) pairs
/// — the shape `run_program_with_warnings` took, written to disk so that
/// `liss run` can be handed the folder.
///
/// The folder rule is unchanged and is what makes the two spellings equivalent:
/// `"principale.liss"` is a root file and `"mathematica/summa.liss"` is in the
/// `mathematica` package, exactly as `program::assemble` read them.
fn curre_fasciculos(scidae: &[(&str, &str)]) -> Rendered {
    let d = temp_dir("liss-programs");
    for (via, fons) in scidae {
        d.write(via, fons);
    }
    support::curre(d.path()).unwrap_or_else(|e| panic!("{e}"))
}

/// What a program printed, having required it to succeed in silence.
///
/// Most of this file asserted `run(src).unwrap()`, which discarded the exit
/// status *and* the warnings. Both are checked here rather than dropped: a
/// program that printed the right thing and then exited 3, or drew a Latinity
/// warning nobody expected, was passing before and should not be.
///
/// Two sources here do legitimately warn, and they say so through
/// [`emissio_cum_monitis`] rather than by weakening this.
fn emissio(r: &Rendered) -> &str {
    emissio_cum_monitis(r, "")
}

/// [`emissio`] for a program that succeeds but draws Latinity warnings.
///
/// **These are not new expectations and not a divergence.** `latinity::
/// lint_program` runs in the front end, which both backends shared, so the
/// interpreted pipeline produced exactly these lines too — the old assertions
/// simply threw them away, because `run` returned only the output. They are
/// written down here because the move made them visible, and because the corpus
/// already records the same bytes: `conformance/currunt/exemplum-praecisio`'s
/// blessed `nuntii`, recorded from the interpreter, is the two lines below.
fn emissio_cum_monitis<'a>(r: &'a Rendered, monita: &str) -> &'a str {
    assert_eq!(r.code, 0, "expected a clean exit; stderr: {:?}", r.stderr);
    assert_eq!(r.stderr, monita, "the diagnostics are not the expected ones");
    &r.stdout
}

/// The diagnostic of a program that must fail: exit 1, nothing on stdout.
fn erratum(r: &Rendered) -> &str {
    assert_eq!(r.code, 1, "expected a failure; stdout {:?}, stderr {:?}", r.stdout, r.stderr);
    assert_eq!(r.stdout, "", "a failing program printed something unexpected");
    &r.stderr
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
    // mnr grains at the uncia (1/12), nr at the scripulum (1/288); an
    // unconstrained fractional literal defaults to nr.
    let src = include_str!("../../../examples/praecisio.liss");
    let r = curre(src);
    assert_eq!(
        emissio_cum_monitis(
            &r,
            "Monitum in linea 11: nomen 'tertia_brevis' fortasse non Latinum est\n\
             Monitum in linea 12: nomen 'tertia_longa' fortasse non Latinum est\n",
        ),
        "I et triens\nI et triens\nI\nI et semuncia\nII et semis\nI et triens\n"
    );
}

#[test]
fn type_error_reports_latin_and_line() {
    let src = "munus initium reddit ni { ni x = \"salve\"; redde nullus; }";
    // Was `err.line == 1` plus a `starts_with` on the rendered error. The line
    // number is now visible only in the rendering, which is where a user reads
    // it, so the two assertions have become one.
    assert!(
        erratum(&curre(src)).starts_with("Error in linea 1:"),
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
    // The warning must not change the program's behaviour.
    assert_eq!(r.stdout, "V\n");
    assert_eq!(r.code, 0);
    // Was: one warning, at line 1, naming `result`. Rendered, that is one line,
    // and asserting it whole is strictly more than the three parts were.
    assert_eq!(r.stderr, "Monitum in linea 1: nomen 'result' fortasse non Latinum est\n");
}

#[test]
fn latin_program_produces_no_warnings() {
    let src = "munus initium reddit ni { ni numerus = °V; scribe(numerus); redde nullus; }";
    assert_eq!(emissio(&curre(src)), "V\n");
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
    let r = curre("munus initium reddit ni { ni x = \"salve\"; redde nullus; }");
    assert_eq!(r.code, 1);

    let r = curre_fasciculos(&[(
        "principale.liss",
        "munus initium reddit ni { ni counter = \"salve\"; redde nullus; }",
    )]);
    assert_eq!(r.code, 1);
    let mut lineae = r.stderr.lines();
    assert_eq!(
        lineae.next(),
        Some("Monitum in 'principale.liss', linea 1: nomen 'counter' fortasse non Latinum est"),
    );
    assert!(
        lineae.next().is_some_and(|l| l.starts_with("Error in 'principale.liss', linea 1:")),
        "the warning must be followed by the error it survived: {:?}",
        r.stderr,
    );
}

#[test]
fn mnr_truncates_below_its_uncia_grain() {
    // 1/24 is below mnr's uncia grid, so it is dropped — only the whole prints.
    let r = curre("munus initium reddit ni { mnr x = °I:I/XXIV; scribe(x); redde nullus; }");
    assert_eq!(emissio(&r), "I\n");
}

#[test]
fn nr_keeps_the_sub_uncia_part() {
    // nr's scripulum grid holds 1/24 exactly: it is a semuncia.
    let r = curre("munus initium reddit ni { nr y = °I:I/XXIV; scribe(y); redde nullus; }");
    assert_eq!(emissio(&r), "I et semuncia\n");
}

#[test]
fn exact_third_prints_same_under_both() {
    // 1/3 = 4/12 is exact on both grids, so mnr and nr agree here.
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
    assert!(erratum(&r).starts_with("Error in linea 1:"), "got: {:?}", r.stderr);
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
    // A bare fractional literal in an untyped sink defaults to nr.
    let r = curre("munus initium reddit ni { scribe(°I:I/III); redde nullus; }");
    assert_eq!(emissio(&r), "I et triens\n");
}

#[test]
fn fraction_divide_by_zero_errors() {
    // (½) / (½ − ½): a runtime zero divisor is an error, not inf.
    let r = curre(
        "munus initium reddit ni { nr z = °I:I/II - °I:I/II; scribe(°I:I/II / z); redde nullus; }",
    );
    assert!(erratum(&r).starts_with("Error in linea 1:"), "got: {:?}", r.stderr);
}

#[test]
fn fraction_modulo_is_a_type_error() {
    let r = curre("munus initium reddit ni { nr x = °I:I/II % °I:I/IV; redde nullus; }");
    assert_eq!(r.code, 1);
}

#[test]
fn negative_fractions_print_signed() {
    // Unary minus on a fraction: mnr drops the 1/24 (truncation toward zero) and
    // keeps the sign; nr keeps it as a semuncia. -(1 + 1/24).
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
    // Passing a list to a function copies it: the caller's list is untouched
    // even though the callee appends to its own copy.
    let src = "munus auctus capit (ni lista s) reddit ni { s = adde(s, °C); redde longitudo(s); } munus initium reddit ni { ni lista xs = [°I, °II]; ni n = auctus(xs); scribe(n); scribe(longitudo(xs)); redde nullus; }";
    let r = curre(src);
    assert_eq!(
        emissio_cum_monitis(
            &r,
            "Monitum in linea 1: nomen 'xs' fortasse non Latinum est\n",
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
        "Error: munus 'initium' in programmate non inventum\n"
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
        erratum(&r).starts_with("Error in 'mathematica/summa.liss', linea 2:"),
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
    assert!(e.starts_with("Error in linea 1:"), "got: {e:?}");
}

#[test]
fn single_file_program_still_needs_initium() {
    let r = curre("munus f reddit ni { redde °I; }");
    assert!(erratum(&r).contains("initium"), "got: {:?}", r.stderr);
}

#[test]
fn intra_package_unqualified_calls_run() {
    // Two files in the same package call each other with bare (unqualified)
    // names — the checker validates this against the package's own function
    // table, and codegen must resolve it there too, not against the root
    // namespace (regression test for the package bare-call bug).
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
