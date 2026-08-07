//! Every scalar type and every operator, compiled and **run** (Task 12).
//!
//! # The oracle is the interpreter, not a hand-written expectation
//!
//! Almost every test here is [`support::par_est`]: one source, run both ways,
//! the two renderings required equal. That is spec §10.1's property applied to
//! a program instead of a fixture, and it is the only oracle that can be trusted
//! at this size — thirty-odd operator results transcribed by hand would be
//! thirty-odd chances to bless a miscompile. `tests/compilata.rs` (Task 17)
//! generalises it to the whole corpus.
//!
//! The interpreter's **warnings are cleared before the comparison**, because a
//! compiled program never prints them: the CLI writes Latinity warnings before
//! spawning the child (spec §9.5), which is Task 16. Everything else — stdout
//! byte for byte, stderr byte for byte, the exit status — must agree.
//!
//! # The one thing here that is not a differential
//!
//! `et` and `vel` **evaluate both operands** (spec §3.5). The interpreter does,
//! the corpus is blessed against it, and so the differential above would already
//! catch a short-circuiting compiler. It is nonetheless asserted separately and
//! explicitly, with the reason spelled out, because it is the single easiest
//! silent divergence in the project and a test that only fails as a by-product
//! is a test whose failure message says the wrong thing.

mod support;

use support::{curre, par_est, radix};

/// Every one of the six scalar types, printed.
#[test]
fn omnes_typi_scribuntur() {
    par_est(
        r#"munus initium reddit ni {
    scribe(verus);
    scribe(falsus);
    scribe('x');
    scribe(°XLII);
    scribe(°I:I/III);
    scribe("salve, munde");
    redde nullus;
}
"#,
        "typi",
    );
}

/// `ni`: every arithmetic operator, both unary operators, all four comparisons.
#[test]
fn numeri_omnem_operatorem_ferunt() {
    par_est(
        r#"munus initium reddit ni {
    ni maior = °XVII;
    ni minor = °V;
    scribe(maior + minor);
    scribe(maior - minor);
    scribe(maior * minor);
    scribe(maior / minor);
    scribe(maior % minor);
    scribe(-maior);
    scribe(minor - maior);
    scribe(maior par_est minor);
    scribe(maior par_non_est minor);
    scribe(maior maior_est minor);
    scribe(maior minor_est minor);
    redde nullus;
}
"#,
        "numeri",
    );
}

/// `mnr` and `nr`: `add`, `sub`, `neg` and `cmp` are shared with `ni` — a grain
/// count adds like an integer — while `mul` and `div` rescale by XII or
/// CCLXXXVIII. Both grains are exercised, because taking the wrong one is a
/// wrong *answer* rather than a crash.
#[test]
fn fractiones_utroque_grano_computantur() {
    par_est(
        r#"munus initium reddit ni {
    mnr uncialis = °II:I/II;
    mnr altera = °I:I/IV;
    scribe(uncialis + altera);
    scribe(uncialis - altera);
    scribe(uncialis * altera);
    scribe(uncialis / altera);
    scribe(-uncialis);
    scribe(uncialis maior_est altera);
    scribe(uncialis par_est altera);
    nr scripularis = °II:I/II;
    nr tenuis = °I:I/XXIV;
    scribe(scripularis + tenuis);
    scribe(scripularis - tenuis);
    scribe(scripularis * tenuis);
    scribe(scripularis / tenuis);
    scribe(-scripularis);
    scribe(scripularis minor_est tenuis);
    scribe(scripularis par_non_est tenuis);
    redde nullus;
}
"#,
        "fractiones",
    );
}

/// `voc`: concatenation is the only arithmetic text has, and the ordering is
/// its UTF-8 bytes.
#[test]
fn voces_iunguntur_et_comparantur() {
    par_est(
        r#"munus initium reddit ni {
    voc prior = "salve";
    voc posterior = "munde";
    scribe(prior + posterior);
    scribe(prior + "");
    scribe(prior par_est posterior);
    scribe(prior par_non_est posterior);
    scribe(prior maior_est posterior);
    scribe(prior minor_est posterior);
    scribe(prior par_est prior);
    redde nullus;
}
"#,
        "voces",
    );
}

/// `lit` and `veritas`: the two unowned scalars. Both compare as immediates —
/// there is no `liss_bool_*` or `liss_char_*` entry point and there must not be
/// one — and `veritas` additionally carries `et`, `vel` and `non`.
#[test]
fn litterae_et_veritates_immediatae_sunt() {
    par_est(
        r#"munus initium reddit ni {
    lit prima = 'a';
    lit secunda = 'b';
    scribe(prima par_est secunda);
    scribe(prima par_non_est secunda);
    scribe(prima maior_est secunda);
    scribe(prima minor_est secunda);
    veritas verum = verus;
    veritas dubium = falsus;
    scribe(verum et dubium);
    scribe(verum vel dubium);
    scribe(verum et verum);
    scribe(dubium vel dubium);
    scribe(non verum);
    scribe(non non verum);
    scribe(verum par_est dubium);
    scribe(verum par_non_est dubium);
    scribe(verum maior_est dubium);
    scribe(verum minor_est dubium);
    redde nullus;
}
"#,
        "immediata",
    );
}

/// Slots: `Let` binds, `Assign` rebinds, and `x = x + °I` must read the old
/// value **before** it dies (spec §7.3.3). A `voc` doubling itself is the same
/// rule where getting it wrong is a use-after-free rather than a wrong number.
///
/// The non-zero `redde` also proves the frees happened: a leaked slot would
/// make the runtime say `memoria non liberata est` and exit 1 instead.
#[test]
fn sedes_reguntur_et_liberantur() {
    par_est(
        r#"munus initium reddit ni {
    ni numerus = °I;
    numerus = numerus + °I;
    numerus = numerus * °X;
    scribe(numerus);
    voc nomen = "ab";
    nomen = nomen + nomen;
    nomen = nomen + nomen;
    scribe(nomen);
    mnr pars = °I:I/II;
    pars = pars + pars;
    scribe(pars);
    veritas signum = verus;
    signum = non signum;
    scribe(signum);
    redde numerus;
}
"#,
        "sedes",
    );
}

/// `divisio per nullum`, raised by `liss_int_div` itself with the statement's
/// line — and D-08 with it: the line printed *before* the failure still reaches
/// stdout, because `liss_panic` flushes before it writes its sentence.
#[test]
fn divisio_per_nullum_lineam_suam_nominat() {
    let (code, stdout, stderr) = curre(
        r#"munus initium reddit ni {
    scribe("ante");
    scribe(°X / nullus);
    redde nullus;
}
"#,
        "divisio",
    );
    assert_eq!(stdout, "ante\n", "D-08: output before the failure still gets out");
    assert_eq!(stderr, "Error in linea 3: divisio per nullum\n");
    assert_eq!(code, Some(1));
}

/// `modulus per nullum` is a **different sentence at a different site**, and
/// the catalogue must be able to tell them apart (D-24).
#[test]
fn modulus_per_nullum_suam_sententiam_habet() {
    let (code, stdout, stderr) = curre(
        r#"munus initium reddit ni {
    scribe(°X % nullus);
    redde nullus;
}
"#,
        "modulus",
    );
    assert_eq!(stdout, "");
    assert_eq!(stderr, "Error in linea 2: modulus per nullum\n");
    assert_eq!(code, Some(1));
}

/// A fraction divided by a zero *count* raises the same sentence from
/// `liss_frac_div`, which is a third site again.
#[test]
fn fractio_per_nullum_quoque_prohibetur() {
    let (code, _, stderr) = curre(
        r#"munus initium reddit ni {
    nr pars = °I:I/II;
    nr nihil = pars - pars;
    scribe(pars / nihil);
    redde nullus;
}
"#,
        "fractio-nulla",
    );
    assert_eq!(stderr, "Error in linea 4: divisio per nullum\n");
    assert_eq!(code, Some(1));
}

/// **`vel` evaluates its right operand even when its left is `verus`.**
///
/// This is spec §3.5, and it is the one rule in this task that fails *silently*:
/// a compiler that emitted the branch its author's reflexes reach for would
/// print `verus` and exit 0 here, which looks like a better language and is a
/// different one. `interp.rs:131-135` evaluates both sides and then dispatches
/// on the operator, and the 103-fixture corpus is blessed against that.
///
/// The program is chosen so that short-circuiting and evaluating both are
/// **observably** different, which a fault on the *left* operand would not be:
/// `A vel B` evaluates `A` first either way, so only a fault in `B` can tell
/// the two apart.
#[test]
fn vel_dextrum_latus_quoque_aestimat() {
    let (code, stdout, stderr) = curre(
        r#"munus initium reddit ni {
    scribe(verus vel (°V / nullus par_est °I));
    redde nullus;
}
"#,
        "vel",
    );
    assert_ne!(
        stdout, "verus\n",
        "`vel` short-circuited: the right operand was not evaluated (spec §3.5)",
    );
    assert_eq!(stderr, "Error in linea 2: divisio per nullum\n");
    assert_eq!(code, Some(1));
}

/// The same for `et` with a `falsus` left operand.
#[test]
fn et_dextrum_latus_quoque_aestimat() {
    let (code, stdout, stderr) = curre(
        r#"munus initium reddit ni {
    scribe(falsus et (°V / nullus par_est °I));
    redde nullus;
}
"#,
        "et",
    );
    assert_ne!(
        stdout, "falsus\n",
        "`et` short-circuited: the right operand was not evaluated (spec §3.5)",
    );
    assert_eq!(stderr, "Error in linea 2: divisio per nullum\n");
    assert_eq!(code, Some(1));
}

/// A negated literal, which is where `Roman::negative()` finally matters: `-°V`
/// parses to `Unary{Neg}` over a `Const`, and codegen folds it, so the sign
/// travels in the emitted digit blob rather than through a call.
///
/// `-nullus` is the case a fold gets wrong: negating zero must stay the
/// canonical, non-negative zero.
#[test]
fn numeri_negativi_in_constantes_conflantur() {
    par_est(
        r#"munus initium reddit ni {
    scribe(-°V);
    scribe(-nullus);
    scribe(-°V + °V);
    scribe(-°I:I/II);
    scribe(- -°V);
    redde -°V + °CXXVIII;
}
"#,
        "negativi",
    );
}

/// The fold, seen in the **object**: a program whose only negation is of a
/// literal must not ask the linker for `liss_int_neg` at all.
#[test]
fn constans_negata_nullam_vocationem_relinquit() {
    use object::{Object, ObjectSymbol};

    let machina = liss_codegen::target::Target::hospes().expect("a supported host");
    let prog = support::ir_solum("munus initium reddit ni {\n    redde -°V;\n}\n");
    let octeti = liss_codegen::compila(&prog, &machina).expect("the program compiles");
    let fasciculus = object::File::parse(&octeti[..]).expect("the object parses");
    let petita: Vec<_> = fasciculus
        .symbols()
        .filter(|s| s.is_undefined())
        .filter_map(|s| s.name().ok())
        .filter(|n| !n.is_empty())
        .collect();
    assert!(
        !petita.contains(&"liss_int_neg"),
        "`-°V` was not folded; the object still calls liss_int_neg: {petita:?}",
    );
    assert!(petita.contains(&"liss_int_new"), "the constant itself is still built: {petita:?}");
}

/// **The checkpoint.** `conformance/currunt/exemplum-praecisio`, compiled and
/// run, against the same `emissio` the interpreted path is held to.
///
/// The fixture's `nuntii` are the *checker's* Latinity warnings, which a
/// compiled program never prints — the CLI writes them before spawning the
/// child (spec §9.5). So the compiled binary's own stderr must be **empty**,
/// and that is what is asserted.
#[test]
fn exemplum_praecisio_compilatum_currit() {
    let radix = radix();
    let fixtura = radix.join("conformance/currunt/exemplum-praecisio");
    let relativa = std::fs::read_to_string(fixtura.join("VIA")).expect("VIA").trim().to_owned();
    let fons = std::fs::read_to_string(radix.join(&relativa)).expect("the source");
    let emissio = std::fs::read_to_string(fixtura.join("emissio")).expect("emissio");

    let (code, stdout, stderr) = curre(&fons, "praecisio");
    assert_eq!(stdout, emissio, "stdout does not match the fixture's `emissio`");
    assert_eq!(stderr, "", "the warnings belong to the CLI, not to the program");
    assert_eq!(code, Some(0));
}

/// **The checkpoint.** `conformance/currunt/numerus-sine-spatio` — `°X/°II`,
/// the fixture that says whitespace is not load-bearing — compiled and run.
///
/// A **folder** program, so `Func::file` keeps its name; nothing here raises,
/// so the name is never printed, but the two fixtures between them cover both
/// modes of spec §4.1's `Option`.
#[test]
fn numerus_sine_spatio_compilatus_currit() {
    let radix = radix();
    let fixtura = radix.join("conformance/currunt/numerus-sine-spatio");
    let fons =
        std::fs::read_to_string(fixtura.join("programma/principale.liss")).expect("the source");
    let emissio = std::fs::read_to_string(fixtura.join("emissio")).expect("emissio");

    let machina = liss_codegen::target::Target::hospes().expect("a supported host");
    let prog = support::ir(std::path::Path::new("principale.liss"), &fons);
    let (code, stdout, stderr) = support::exsequere(&support::imago(&prog, machina), "sine-spatio");
    assert_eq!(stdout, emissio);
    assert_eq!(stderr, "");
    assert_eq!(code, Some(0));
}

/// A raise from a **folder** program names its file, exactly as
/// `ProgramError`'s `Display` does — the other half of spec §4.1's `Option`.
#[test]
fn error_in_programmate_scidam_nominat() {
    let machina = liss_codegen::target::Target::hospes().expect("a supported host");
    let prog = support::ir(
        std::path::Path::new("principale.liss"),
        "munus initium reddit ni {\n    scribe(°X / nullus);\n    redde nullus;\n}\n",
    );
    let (code, _, stderr) = support::exsequere(&support::imago(&prog, machina), "scida");
    assert_eq!(stderr, "Error in 'principale.liss', linea 2: divisio per nullum\n");
    assert_eq!(code, Some(1));
}

/// **A statement after `redde` is legal and must compile.**
///
/// The checker's all-paths analysis asks whether *any* statement returns, not
/// whether the last one does — `checker.rs:128-131` says so outright: "`any`,
/// not 'the last statement': a `redde` followed by unreachable code still
/// returns, and this language has no dead-code lint to disagree with." So this
/// program type-checks, and the interpreter runs it to an empty stdout and
/// exit 0.
///
/// Codegen used to walk every statement in a block unconditionally, so `redde`
/// filled the Cranelift block and the next statement tried to add an
/// instruction to a filled one: a panic in debug, and in release a `Defectus`
/// telling the user their valid program had triggered an internal compiler
/// failure. Nothing in `conformance/` or `examples/` had this shape, so neither
/// this task's checkpoint nor Task 17's whole-corpus differential would have
/// caught it.
///
/// `conformance/currunt/sententia-post-redditum` is the corpus's copy, which is
/// what matters after Task 19 leaves the corpus as the only oracle.
#[test]
fn sententia_post_redditum_neglegitur() {
    par_est(
        r#"munus initium reddit ni {
    redde nullus;
    scribe("numquam");
}
"#,
        "post-redditum",
    );
}

/// The same shape with **owned** slots either side of the `redde`, which is
/// where getting it wrong would be a leak rather than a crash: the teardown
/// after a terminator must not run, and the `redde`'s own walk must already
/// have freed everything — including the slot whose `Let` never executed, which
/// is exactly what spec §5.4's "freeing null is a no-op" buys.
#[test]
fn sedes_post_redditum_semel_liberantur() {
    par_est(
        r#"munus initium reddit ni {
    voc nomen = "salve";
    redde °VII;
    voc numquam = nomen;
}
"#,
        "post-redditum-sedes",
    );
}
