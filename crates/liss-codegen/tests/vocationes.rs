//! Calls, across a package boundary and all the way down (spec §4.3, §5.4, §8).

mod support;

use liss_codegen::target::Target;
use object::{Object, ObjectSymbol};
use support::{
    curre, curre_fasciculos, imago, ir_fasciculorum, par_est, par_est_fasciculorum,
    par_est_fasciculorum_erroris,
};

/// The whole of what a stack-exhausted image puts on stderr (spec §8.2).
const ACERVUS: &str = "Error: acervus vocationum exhaustus est\n";

/// Three namespaces, every call crossing one, and `alpha` declaring an ordinary
/// `initium` of its own (spec §4.3). `beta.triplum` is deliberately never called.
const TRES: [(&str, &str); 3] = [
    (
        "principale.liss",
        r#"affer alpha;

munus initium reddit ni {
    scribe(alpha.summa(°XXV, °XVII));
    scribe(alpha.initium());
    redde nullus;
}
"#,
    ),
    (
        "alpha/porta.liss",
        r#"affer beta;

munus summa capit (ni a, ni b) reddit ni {
    redde beta.duplum(a + b);
}

munus initium reddit ni {
    redde °VII;
}
"#,
    ),
    (
        "beta/opus.liss",
        r#"munus duplum capit (ni n) reddit ni {
    redde n * °II;
}

munus triplum capit (ni n) reddit ni {
    redde n * °III;
}
"#,
    ),
];

/// A root function shadowed by a package function of the same name, reached through
/// the package's own unqualified call — the one shape that catches `checker.rs:435`.
const UMBRA: [(&str, &str); 2] = [
    (
        "principale.liss",
        r#"affer alpha;

munus quadratum capit (ni n) reddit ni {
    redde n + °C;
}

munus initium reddit ni {
    scribe(alpha.porta(°III));
    redde nullus;
}
"#,
    ),
    (
        "alpha/a.liss",
        r#"munus quadratum capit (ni n) reddit ni {
    redde n * n;
}

munus porta capit (ni n) reddit ni {
    redde quadratum(n);
}
"#,
    ),
];

/// A raise inside a package file, so the diagnostic names a path with a folder in it.
const DIVISIO: [(&str, &str); 2] = [
    (
        "principale.liss",
        r#"affer alpha;

munus initium reddit ni {
    scribe(alpha.dimidium(°X, nullus));
    redde nullus;
}
"#,
    ),
    (
        "alpha/porta.liss",
        r#"munus dimidium capit (ni a, ni b) reddit ni {
    redde a / b;
}
"#,
    ),
];

/// Identifiers no ASCII escape would round-trip.
const MACRA: [(&str, &str); 2] = [
    (
        "principale.liss",
        r#"affer mathēmatica;

munus initium reddit ni {
    scribe(mathēmatica.quadrātum(°VII));
    redde nullus;
}
"#,
    ),
    (
        "mathēmatica/potestas.liss",
        r#"munus quadrātum capit (ni n) reddit ni {
    redde n * n;
}
"#,
    ),
];

/// Two packages that import each other and alternate, twenty levels; the depth is
/// asserted, not merely chosen — [`fasciculi_se_invicem_vocant`] demands `"XX\n"`.
const ALTERNA: [(&str, &str); 3] = [
    (
        "principale.liss",
        r#"affer alpha;

munus initium reddit ni {
    scribe(alpha.pare(°XX));
    redde nullus;
}
"#,
    ),
    (
        "alpha/a.liss",
        r#"affer beta;

munus pare capit (ni n) reddit ni {
    si (n par_est nullus) {
        redde nullus;
    }
    redde beta.impar(n - °I) + °I;
}
"#,
    ),
    (
        "beta/b.liss",
        r#"affer alpha;

munus impar capit (ni n) reddit ni {
    si (n par_est nullus) {
        redde nullus;
    }
    redde alpha.pare(n - °I) + °I;
}
"#,
    ),
];

/// The same two packages, unbounded.
const ALTERNA_SINE_FINE: [(&str, &str); 3] = [
    (
        "principale.liss",
        r#"affer alpha;

munus initium reddit ni {
    scribe("ante acervum");
    redde alpha.ultro(°I);
}
"#,
    ),
    (
        "alpha/a.liss",
        r#"affer beta;

munus ultro capit (ni n) reddit ni {
    redde beta.citro(n + °I);
}
"#,
    ),
    (
        "beta/b.liss",
        r#"affer alpha;

munus citro capit (ni n) reddit ni {
    redde alpha.ultro(n + °I);
}
"#,
    ),
];

/// Two heap parameters over a package boundary, five times round a loop.
const VERBA: [(&str, &str); 2] = [
    (
        "principale.liss",
        r#"affer verba;

munus initium reddit ni {
    voc s = "a";
    dum (ni i = nullus; i minor_est °V; i = i + °I) {
        s = verba.iunge(s, "-b");
    }
    scribe(s);
    redde nullus;
}
"#,
    ),
    (
        "verba/iunctio.liss",
        r#"munus iunge capit (voc a, voc b) reddit voc {
    redde a + b;
}
"#,
    ),
];

/// Every name the object defines globally, sorted.
fn definita(prog: &[(&str, &str)], machina: &Target) -> Vec<String> {
    let octeti =
        liss_codegen::compila(&ir_fasciculorum(prog), machina).expect("the program compiles");
    let fasciculus = object::File::parse(&octeti[..]).expect("a relocatable object");
    let mut nomina: Vec<String> = fasciculus
        .symbols()
        .filter(|s| s.is_global() && s.is_definition())
        .filter_map(|s| s.name().ok().map(str::to_owned))
        .collect();
    nomina.sort_unstable();
    nomina
}

/// Written as literals rather than `liss_ir::symbol(..)` calls: a mangling that
/// changed would still agree with itself.
#[test]
fn omne_munus_symbolum_fasciculi_sui_gerit() {
    for machina in Target::OMNES {
        assert_eq!(
            definita(&TRES, &machina),
            [
                "liss_0__7_initium",
                "liss_4_beta_6_duplum",
                "liss_4_beta_7_triplum",
                "liss_5_alpha_5_summa",
                "liss_5_alpha_7_initium",
                "liss_initium",
            ],
            "{}",
            machina.triple(),
        );
    }
}

/// `liss_initium` must be a second name for the *root* `initium`, never for a
/// package's (spec §4.3).
#[test]
fn initium_fasciculi_munus_ordinarium_est() {
    for machina in Target::OMNES {
        let octeti =
            liss_codegen::compila(&ir_fasciculorum(&TRES), &machina).expect("the program compiles");
        let fasciculus = object::File::parse(&octeti[..]).expect("a relocatable object");
        let signum = |nomen: &str| {
            fasciculus
                .symbols()
                .filter(|s| s.is_global() && s.is_definition())
                .find(|s| s.name() == Ok(nomen))
                .map(|s| (s.section_index(), s.address(), s.size()))
                .expect("the symbol is defined")
        };
        assert_eq!(
            signum("liss_initium"),
            signum("liss_0__7_initium"),
            "{}: the entry alias must name the root `initium`",
            machina.triple(),
        );
        assert_ne!(
            signum("liss_initium"),
            signum("liss_5_alpha_7_initium"),
            "{}: `alpha.initium` is an ordinary function and must not be the \
             program's entry point",
            machina.triple(),
        );
    }
}

/// The escape is per character, not per byte (spec §4.3), and only a written-out
/// expectation catches a per-byte one.
#[test]
fn nomina_non_ascii_per_fasciculos_transeunt() {
    for machina in Target::OMNES {
        assert_eq!(
            definita(&MACRA, &machina),
            [
                "liss_0__7_initium",
                "liss_16_math_u113_matica_14_quadr_u101_tum",
                "liss_initium",
            ],
            "{}",
            machina.triple(),
        );
    }
}

/// The alias survives the link at one address, and `--gc-sections` did not take
/// `beta.triplum` with it. Host target only.
#[test]
fn imago_coniuncta_symbola_servat() {
    let machina = Target::hospes().expect("the host is x86_64 or aarch64");
    let octeti = imago(&ir_fasciculorum(&TRES), machina);
    let fasciculus = object::File::parse(&octeti[..]).expect("a linked image");
    // `expect`, not an `Option` comparison: two absent symbols compare equal.
    let locus = |nomen: &str| {
        fasciculus
            .symbols()
            .find(|s| s.name() == Ok(nomen))
            .unwrap_or_else(|| panic!("the linked image defines {nomen}"))
            .address()
    };
    assert_eq!(
        locus("liss_initium"),
        locus("liss_0__7_initium"),
        "two names, one address, after the linker has had its say",
    );
    locus("liss_4_beta_7_triplum");
}

/// Root → `alpha` → `beta`, plus a call to the package's own `initium`.
#[test]
fn programma_trium_fasciculorum_currit() {
    par_est_fasciculorum(&TRES, "tres-fasciculi", "LXXXIV\nVII\n");
}

/// The same with macrons in the identifiers, through the linker and out the other side.
#[test]
fn nomina_non_ascii_coniunguntur_et_currunt() {
    par_est_fasciculorum(&MACRA, "nomina-macra", "XLIX\n");
}

/// A bare call inside a package resolves in that package even when the root declares
/// the same name — the only test in the workspace that observes `checker.rs:435`.
#[test]
fn vocatio_nuda_in_fasciculo_in_fasciculo_suo_solvitur() {
    par_est_fasciculorum(&UMBRA, "umbra", "IX\n");
}

/// `Corpus::scida` is per-function, so a raise in a callee names the callee's file
/// rather than whatever the caller last set. `scalares.rs` covers only the root file.
#[test]
fn error_in_fasciculo_scidam_fasciculi_nominat() {
    par_est_fasciculorum_erroris(
        &DIVISIO,
        "divisio-in-fasciculo",
        "Error in 'alpha/porta.liss', linea 2: divisio per nullum\n",
    );
}

/// A circular `affer` is legal, and the only shape where a `FuncId` resolved in one
/// namespace names a function in another that has not been lowered yet.
#[test]
fn fasciculi_se_invicem_vocant() {
    par_est_fasciculorum(&ALTERNA, "alterna", "XX\n");
}

/// Ownership across the boundary (spec §5.4, §7.3); exit 0 is the assertion (§7.5).
#[test]
fn argumenta_per_fasciculum_mota_semel_liberantur() {
    par_est_fasciculorum(&VERBA, "verba-iuncta", "a-b-b-b-b-b\n");
}

/// Unbounded self-recursion: exit 1 (not D-04's signal), the Latin sentence, and
/// stdout still holding what `scribe` wrote before the fault.
#[test]
fn recursio_profunda_acervum_exhaurit() {
    let (code, stdout, stderr) = curre(
        r#"munus abyssus capit (ni n) reddit ni {
    redde abyssus(n + °I);
}

munus initium reddit ni {
    scribe("ante acervum");
    redde abyssus(°I);
}
"#,
        "abyssus",
    );
    assert_eq!(code, Some(1), "a signalled image is D-04's shape, not a diagnostic");
    assert_eq!(stderr, ACERVUS);
    assert_eq!(stdout, "ante acervum\n", "the handler flushes what was buffered");
}

/// Recursion into `initium` binds to the mangled name; otherwise this would not link.
#[test]
fn initium_se_ipsum_vocans_acervum_exhaurit() {
    let (code, stdout, stderr) = curre(
        r#"munus initium reddit ni {
    redde initium();
}
"#,
        "initium-recursivum",
    );
    assert_eq!(code, Some(1));
    assert_eq!(stderr, ACERVUS);
    assert_eq!(stdout, "");
}

/// The same exhaustion, reached only through mangled cross-package symbols.
#[test]
fn recursio_per_fasciculos_acervum_exhaurit() {
    let (code, stdout, stderr) = curre_fasciculos(&ALTERNA_SINE_FINE, "alterna-sine-fine");
    assert_eq!(code, Some(1));
    assert_eq!(stderr, ACERVUS);
    assert_eq!(stdout, "ante acervum\n");
}

/// There is no recursion limit (spec §3.3). `conformance/currunt/recursio-profunda`
/// makes the same point at 5,000, and `cli.rs::deep_recursion_runs_because_there_is_no_limit`
/// at 10,000 through the CLI.
#[test]
fn recursio_decem_milium_graduum_perficitur() {
    let (code, stdout, stderr) = curre(
        r#"munus profunditas capit (ni n) reddit ni {
    si (n par_est nullus) {
        redde nullus;
    }
    redde °I + profunditas(n - °I);
}

munus initium reddit ni {
    scribe(profunditas(°X.M));
    redde nullus;
}
"#,
        "profunditas",
    );
    assert_eq!(stderr, "", "ten thousand frames is not an exhausted stack");
    assert_eq!(stdout, "X\u{305}\n");
    assert_eq!(code, Some(0), "and the live-allocation count is still zero");
}

/// Twenty frames, and the recorded answer really is the interpreter's, digit for
/// digit: `factorial(°XX)` is 2,432,902,008,176,640,000.
#[test]
fn recursio_brevis_responsum_interpretis_reddit() {
    par_est(
        r#"munus factorial capit (ni n) reddit ni {
    si (n par_est nullus) {
        redde °I;
    }
    redde n * factorial(n - °I);
}

munus initium reddit ni {
    scribe(factorial(°XX));
    redde nullus;
}
"#,
        "factorial",
        "M\u{305}\u{305}\u{305}\u{305}\u{305}M\u{305}\u{305}\u{305}\u{305}\u{305}C\u{305}\u{305}\u{305}\u{305}\u{305}D\u{305}\u{305}\u{305}\u{305}\u{305}X\u{305}\u{305}\u{305}\u{305}\u{305}X\u{305}\u{305}\u{305}\u{305}\u{305}X\u{305}\u{305}\u{305}\u{305}\u{305}M\u{305}\u{305}\u{305}\u{305}M\u{305}\u{305}\u{305}\u{305}C\u{305}\u{305}\u{305}\u{305}M\u{305}\u{305}\u{305}\u{305}M\u{305}\u{305}\u{305}M\u{305}\u{305}\u{305}V\u{305}\u{305}\u{305}M\u{305}\u{305}M\u{305}\u{305}M\u{305}\u{305}C\u{305}\u{305}L\u{305}\u{305}X\u{305}\u{305}X\u{305}\u{305}V\u{305}\u{305}M\u{305}D\u{305}C\u{305}X\u{305}L\u{305}\n",
    );
}
