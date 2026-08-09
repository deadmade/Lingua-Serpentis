//! Every scalar type and every operator, compiled and run. The recorded answers
//! came from the interpreter (spec §10.4); see `support`'s header.

mod support;

use support::{curre, par_est, par_est_plene, radix};

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
        "verus\nfalsus\nx\nXLII\nI et triens\nsalve, munde\n",
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
        "XXII\nXII\nLXXXV\nIII\nII\n-XVII\n-XII\nfalsus\nverus\nverus\nfalsus\n",
    );
}

/// `mul` and `div` rescale by XII or CCLXXXVIII; both grains are exercised, because
/// taking the wrong one is a wrong answer rather than a crash.
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
        "III et dodrans\nI et quadrans\nIII et uncia\nII\n-II et semis\nverus\nfalsus\nIII et semis et semuncia\nI et quincunx et semuncia\nII et septunx et sicilicus\nII et triens et semuncia et sicilicus et scripulum\n-II et semis\nfalsus\nverus\n",
    );
}

/// `voc`: concatenation is its only arithmetic, and the ordering is its UTF-8 bytes.
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
        "salvemunde\nsalve\nfalsus\nverus\nverus\nfalsus\nverus\n",
    );
}

/// The two unowned scalars: there is no `liss_bool_*` or `liss_char_*` entry point,
/// and there must not be one.
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
        "falsus\nverus\nfalsus\nverus\nfalsus\nverus\nverus\nfalsus\nfalsus\nverus\nfalsus\nverus\nverus\nfalsus\n",
    );
}

/// `x = x + °I` must read the old value before it dies (spec §7.3.3).
#[test]
fn sedes_reguntur_et_liberantur() {
    par_est_plene(
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
        "XX\nabababab\nIII\nfalsus\n",
        "",
        20,
    );
}

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
    assert_eq!(stderr, "erratum: divisio per nullum\n  in linea III\n");
    assert_eq!(code, Some(1));
}

/// D-24: a different sentence at a different site.
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
    assert_eq!(stderr, "erratum: modulus per nullum\n  in linea II\n");
    assert_eq!(code, Some(1));
}

/// The same sentence from `liss_frac_div`, a third site again.
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
    assert_eq!(stderr, "erratum: divisio per nullum\n  in linea IV\n");
    assert_eq!(code, Some(1));
}

/// Spec §3.5; `conformance/errores/vel-utrumque-latus` is this exact program.
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
    assert_eq!(stderr, "erratum: divisio per nullum\n  in linea II\n");
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
    assert_eq!(stderr, "erratum: divisio per nullum\n  in linea II\n");
    assert_eq!(code, Some(1));
}

/// `-nullus` is the case a fold gets wrong: negating zero must stay canonical zero.
#[test]
fn numeri_negativi_in_constantes_conflantur() {
    par_est_plene(
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
        "-V\nnullus\nnullus\n-I et semis\nV\n",
        "",
        123,
    );
}

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

/// The fixture's `nuntii` are the checker's warnings, which a compiled program never
/// prints (spec §9.5), so this binary's own stderr must be empty.
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

/// A folder program names its file — the other half of spec §4.1's `Option`.
#[test]
fn error_in_programmate_scidam_nominat() {
    let machina = liss_codegen::target::Target::hospes().expect("a supported host");
    let prog = support::ir(
        std::path::Path::new("principale.liss"),
        "munus initium reddit ni {\n    scribe(°X / nullus);\n    redde nullus;\n}\n",
    );
    let (code, _, stderr) = support::exsequere(&support::imago(&prog, machina), "scida");
    assert_eq!(stderr, "erratum: divisio per nullum\n  in 'principale.liss', linea II\n");
    assert_eq!(code, Some(1));
}

/// A statement after `redde` is legal; `conformance/currunt/sententia-post-redditum`
/// is the corpus's copy.
#[test]
fn sententia_post_redditum_neglegitur() {
    par_est(
        r#"munus initium reddit ni {
    redde nullus;
    scribe("numquam");
}
"#,
        "post-redditum",
        "",
    );
}

/// The same shape with owned slots either side of the `redde`, where a mistake is a
/// leak rather than a crash (spec §5.4: freeing null is a no-op).
#[test]
fn sedes_post_redditum_semel_liberantur() {
    par_est_plene(
        r#"munus initium reddit ni {
    voc nomen = "salve";
    redde °VII;
    voc numquam = nomen;
}
"#,
        "post-redditum-sedes",
        "",
        "",
        7,
    );
}
