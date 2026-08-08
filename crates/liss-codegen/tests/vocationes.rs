//! Calls — across a package boundary, and all the way down (spec §4.3, §5.4,
//! §8).
//!
//! # What is actually new here, and what merely gets proved
//!
//! Nothing in `emissio.rs` distinguishes a call that crosses a namespace from
//! one that does not, and that is the design rather than an omission: spec §5.4
//! abolished the shadow call stack, so a call is `declare_func_in_func` and an
//! instruction. A package's identity lives entirely in `Func::symbol`, which
//! `liss_ir::symbol` mangled before codegen ever saw the function, and in
//! `Emissor::ordo`, which is indexed by `FuncId` and therefore flat across every
//! namespace in the program. So this file is where that claim is **checked**:
//! that the symbols in the object are the ones §4.3 specifies, for three
//! namespaces and for an identifier no ASCII escape would round-trip; and that
//! programs built out of those calls run.
//!
//! # The interpreter is an oracle only 64 frames deep
//!
//! `interp.rs`'s `VOCATIONES_MAXIMAE` is 64 and its refusal is
//! `vocatio nimis alte implicata`. Every *bounded* case below therefore goes
//! through `par_est_fasciculorum` or `par_est` and is compared against it; every
//! *deep* one runs the compiled image alone against a written expectation,
//! because the two backends are meant to disagree about depth until the
//! interpreter is deleted (Task 19). That is also why none of this is a
//! `conformance/` fixture: the corpus asserts interpreter behaviour, and would
//! turn red.
//!
//! # What "exit 0" is still worth
//!
//! Spec §7.5: the runtime checks the live-allocation count on the success path
//! and exits 1 with `memoria non liberata est` if anything survives. So a
//! cross-package call that passes two `voc`s and exits 0 has proved §5.4's
//! ownership rule across the boundary — arguments moved, freed by the callee
//! through `body.declares`, the return value the caller's. What it cannot prove
//! is the absence of a *redundant* free of a slot: `Corpus::libera` leaves every
//! slot it empties null, so over-freeing is systematically unobservable (Task
//! 13, spec §7.3.2).

mod support;

use liss_codegen::target::Target;
use object::{Object, ObjectSymbol};
use support::{curre, curre_fasciculos, ir_fasciculorum, par_est, par_est_fasciculorum};

/// The exact sentence spec §8.2 has the guard handler write, and the whole of
/// what a stack-exhausted image puts on stderr.
const ACERVUS: &str = "Error: acervus vocationum exhaustus est\n";

// ---------------------------------------------------------------------------
// The programs
// ---------------------------------------------------------------------------

/// Three namespaces, and every call in it crosses one: the root calls `alpha`,
/// `alpha` calls `beta`, and `alpha` declares its own `initium` — an **ordinary
/// function**, because only the root's is the entry point (spec §4.3).
///
/// `beta.triplum` is deliberately **never called**. A package that exports more
/// than its users take is the ordinary shape of a library, and a `FuncId` that
/// no `Call` names still has to be declared and defined at its own index or
/// every later index in `Emissor::ordo` is one function out.
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

/// A package and a function whose names are Latin with the vowel lengths
/// written — which is to say, identifiers no ASCII escape would round-trip.
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

/// Two packages that import **each other** and alternate — the shape a single
/// namespace cannot produce, and the one that needs both directions of the
/// `FuncId` table to be right at once. Twenty levels, so the interpreter can
/// still answer for it.
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

// ---------------------------------------------------------------------------
// Symbols (spec §4.3)
// ---------------------------------------------------------------------------

/// Every name the object defines globally, sorted — which is to say, everything
/// the linker can be asked for by name.
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

/// The global definitions in the object are exactly the §4.3 names — one per
/// declaration, in its own namespace, plus the entry point's second name, and
/// `beta.triplum` among them although nothing calls it.
///
/// Written as literals rather than as `liss_ir::symbol(..)` calls on purpose: a
/// mangling that changed would still agree with itself, and this is the
/// assertion that the *linker* sees what §4.3 wrote down. The one to read
/// closely is `liss_5_alpha_7_initium` — the length prefixes are of the
/// **escaped** components, and `alpha` is five characters and `initium` seven.
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

/// **A package function named `initium` is an ordinary function.** Only the
/// root's is the entry point (spec §4.3), so `liss_initium` — the one symbol
/// the prelinked runtime leaves undefined — must be a second name for the
/// *root* `initium` and not for `alpha`'s.
///
/// Both halves matter. Aliasing the wrong one links and runs the wrong program;
/// giving the alias its own definition breaks recursion into `initium`, which
/// binds to the mangled name.
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

/// **The escape is per character, not per byte** (spec §4.3), and a package
/// boundary is where it has to survive the linker rather than only a unit test.
///
/// `mathēmatica` escapes to `math_u113_matica` — sixteen characters — and
/// `quadrātum` to `quadr_u101_tum`, fourteen. A per-*byte* escape would have
/// written `_uc4__u93_` for the one `ē` and the length prefixes would still have
/// been self-consistent, so nothing but a written-out expectation catches it.
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

// ---------------------------------------------------------------------------
// Programs that cross a boundary and run
// ---------------------------------------------------------------------------

/// Three namespaces, two boundary crossings deep: root → `alpha` → `beta`, plus
/// a call to the package's own `initium`.
#[test]
fn programma_trium_fasciculorum_currit() {
    par_est_fasciculorum(&TRES, "tres-fasciculi");
}

/// The same, with the identifiers spelled with macrons — through the linker and
/// out the other side.
#[test]
fn nomina_non_ascii_coniunguntur_et_currunt() {
    par_est_fasciculorum(&MACRA, "nomina-macra");
}

/// Two packages calling each other, twenty levels of alternation. A circular
/// `affer` is legal — nothing in the checker forbids one — and it is the only
/// program shape in which a `FuncId` resolved in one namespace has to name a
/// function declared in another that has not been lowered yet.
#[test]
fn fasciculi_se_invicem_vocant() {
    par_est_fasciculorum(&ALTERNA, "alterna");
}

/// **Ownership across the boundary** (spec §5.4, §7.3). Two `voc` arguments are
/// **moved** into a function in another package, five times round a loop; the
/// callee frees them exactly once through `body.declares`, whose first entries
/// are the parameter slots, and the caller owns the returned `voc`.
///
/// Exit 0 is the assertion: by spec §7.5 the runtime exits 1 with
/// `memoria non liberata est` if anything is still alive at the end, so a
/// callee that failed to free its parameters — or a `redde` that swept
/// `0..params` a second time on top of the block teardown, which is the
/// documented way to get this wrong — is a failing test rather than a passing
/// one that leaks.
#[test]
fn argumenta_per_fasciculum_mota_semel_liberantur() {
    par_est_fasciculorum(&VERBA, "verba-iuncta");
}

// ---------------------------------------------------------------------------
// Deep recursion (spec §8)
// ---------------------------------------------------------------------------

/// **The checkpoint.** Unbounded self-recursion is
/// `Error: acervus vocationum exhaustus est` and exit 1 — not a signal, not
/// `vocatio nimis alte implicata`, and not English.
///
/// Three assertions, and the third is the one that is easy to lose. Exit 1
/// rules out D-04's shape, a signalled process, which `judge` maps straight to
/// an invariant violation. The sentence rules out the discriminator's other
/// branch, `defectus internus: memoria violata est` — which is what every
/// obvious way of computing the stack bounds produces instead, because they all
/// describe the pages already touched rather than the ones the stack may grow
/// into (spec §8.2's measured table). And **stdout still holds what `scribe`
/// wrote before the fault**: the runtime's chunk-list buffer had not been
/// flushed, and the handler flushes it with a raw `write(2)` before exiting.
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

/// **Recursion into `initium` binds to the mangled name.** The entry point
/// carries two symbols at one address precisely so that a user can call it: if
/// `liss_initium` were a trampoline, or if `initium` were emitted only under the
/// runtime's name, this program would not link.
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

/// Unbounded recursion that alternates between two packages on every frame —
/// the same exhaustion, reached only through mangled cross-package symbols.
#[test]
fn recursio_per_fasciculos_acervum_exhaurit() {
    let (code, stdout, stderr) = curre_fasciculos(&ALTERNA_SINE_FINE, "alterna-sine-fine");
    assert_eq!(code, Some(1));
    assert_eq!(stderr, ACERVUS);
    assert_eq!(stdout, "ante acervum\n");
}

/// **There is no recursion limit** (spec §3.3), and ten thousand frames is the
/// evidence: the interpreter refuses at 64 and this returns an answer.
///
/// It is deliberately not a differential — `par_est` would compare against
/// `vocatio nimis alte implicata`, which is the interpreter's own limit and not
/// the language's. Ten thousand is also comfortably inside an ordinary 8 MiB
/// `RLIMIT_STACK`, so this test says "deep recursion completes", while the three
/// above say "and when it cannot, it says so in Latin".
///
/// `°X.M` is ten thousand: a numeral's `.` multiplies (`convert_int_roman`),
/// and `X̄` is how ten thousand is written back out — an `X` under a combining
/// overline, the bar meaning ×1000.
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

/// Shallow recursion inside the interpreter's own limit, so that one case in
/// this file is still differential: the compiled answer is the interpreter's,
/// digit for digit.
#[test]
fn recursio_intra_terminum_interpretis_consentit() {
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
    );
}
