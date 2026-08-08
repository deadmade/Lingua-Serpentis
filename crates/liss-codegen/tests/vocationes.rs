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
//! # The oracle was only 64 frames deep, and that shaped this file
//!
//! `interp.rs`'s `VOCATIONES_MAXIMAE` was 64 and its refusal was
//! `vocatio nimis alte implicata`, so while it stood, only the **bounded** cases
//! here could be differentials — the deep ones ran the compiled image against a
//! written expectation, because the two backends were meant to disagree about
//! depth until spec §3.3 removed the cap with the interpreter.
//!
//! Task 19 deleted it, and the split survives in a weaker form: every case below
//! is now an image run against a recorded answer, and the bounded ones' answers
//! are the interpreter's, captured on 2026-08-08 (see `support`'s header).
//!
//! **The corpus has the deep cases now**, which it could not while it was
//! blessed against a 64-frame ceiling: `currunt/recursio-profunda` (5,000
//! levels), `currunt/scribe-ante-acervum` and `errores/acervus-exhaustus`. What
//! stays here is what a fixture cannot say — the *object*, and the ten-thousand
//! frame depth that would make a corpus run needlessly slow.
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
use support::{
    curre, curre_fasciculos, imago, ir_fasciculorum, par_est, par_est_fasciculorum,
    par_est_fasciculorum_erroris,
};

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

/// **A root function shadowed by a package function of the same name, reached
/// through the package's own *unqualified* call.**
///
/// This is the one program shape that catches `checker.rs:435` — the sole
/// namespace-sensitive `FuncId` binding in the compiler. Line 371's qualified
/// arm is forced by the syntax (`alpha.porta` can only mean `alpha`'s), but a
/// bare `quadratum(n)` inside `alpha/a.liss` has to resolve in the *checker's
/// own* namespace, exactly as the interpreter resolves in the executing frame's.
/// Bind it to `None` instead and the package silently calls the **root's**
/// `quadratum`.
///
/// It has to be a decoy of the same name *on the call path*. [`TRES`] already
/// carries a root/package `initium` collision, but only ever reaches the
/// package's through a qualified call, so it exercises the symbol table and
/// never the resolution — the mangled names differ and everything links.
///
/// The two bodies are deliberately far apart in value, not merely different:
/// `alpha.porta(°III)` is `IX` through the package's `n * n` and `CIII` through
/// the root's `n + °C`, so the wrong binding is a wrong *number*, not a
/// coincidence that could agree on some input. And it is the dangerous shape —
/// links, runs, exits 0, prints the wrong answer.
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

/// A runtime raise from inside a **package** file, so the diagnostic has to
/// name a path with a folder in it.
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

/// **The alias survives the link, and so does an export nothing calls.**
///
/// [`definita`] reads the *relocatable object*, which is this compiler's own
/// output and therefore the easy half. Two claims can only be made about the
/// finished image: that `liss_initium` and `liss_0__7_initium` came out of the
/// linker at **one address** rather than as two definitions it was free to place
/// apart, and that `--gc-sections` — which `nexus.rs` passes because it is worth
/// about 0.92 MB on every image — did not take `beta.triplum` with it on its way
/// through.
///
/// The host target only: this reads a linked file, and the other target's cannot
/// be produced here any more usefully than it can be run.
#[test]
fn imago_coniuncta_symbola_servat() {
    let machina = Target::hospes().expect("the host is x86_64 or aarch64");
    let octeti = imago(&ir_fasciculorum(&TRES), machina);
    let fasciculus = object::File::parse(&octeti[..]).expect("a linked image");
    // `expect`, not an `Option` comparison: two absent symbols compare equal,
    // and a vacuous pass is exactly what this test exists to rule out.
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

// ---------------------------------------------------------------------------
// Programs that cross a boundary and run
// ---------------------------------------------------------------------------

/// Three namespaces, two boundary crossings deep: root → `alpha` → `beta`, plus
/// a call to the package's own `initium`.
#[test]
fn programma_trium_fasciculorum_currit() {
    par_est_fasciculorum(&TRES, "tres-fasciculi", "LXXXIV\nVII\n");
}

/// The same, with the identifiers spelled with macrons — through the linker and
/// out the other side.
#[test]
fn nomina_non_ascii_coniunguntur_et_currunt() {
    par_est_fasciculorum(&MACRA, "nomina-macra", "XLIX\n");
}

/// **A bare call inside a package resolves in that package**, even when the
/// root declares the same name.
///
/// The counterpart to the fact that a package *cannot* call a root function
/// (the root has no name to `affer`, and `check_file` gives the checker only its
/// own namespace's signatures): the language forbids the binding, and the flat
/// `FuncId` table is the one place it can still happen by accident.
/// `checker.rs:435` is that place, and this is the only test in the workspace
/// that observes it.
///
/// Nothing else catches the mutation. Binding line 435 to `None` leaves
/// `vocationes.rs` without this, `regimen.rs` and `scalares.rs` green, and kills
/// exactly one workspace test — `programs.rs`, and by an `expect` in
/// `Checker::func_id` firing because the root happens not to declare the name,
/// which is a total-function assertion tripping rather than an observation of
/// wrong behaviour. Give the root a declaration of the same name and even that
/// goes quiet: the program links, runs and prints the other function's answer.
#[test]
fn vocatio_nuda_in_fasciculo_in_fasciculo_suo_solvitur() {
    par_est_fasciculorum(&UMBRA, "umbra", "IX\n");
}

/// **A raise inside a package names the package's file.**
///
/// `Corpus::scida` is per-function — it is `Func::file`, emitted as one C string
/// per function — and `locus()` passes it to every fallible entry point rather
/// than leaving an ambient one, precisely so a raise in a callee names the
/// callee's file and line and not whatever the caller last set. Across a package
/// boundary is where that has teeth: the path has a folder in it, and the
/// caller's file is a different string in a different function's `.rodata`.
///
/// `scalares.rs`'s division raise covers a folder program's **root** file, which
/// cannot tell an ambient locus from a travelling one, because there is only the
/// one file to name.
#[test]
fn error_in_fasciculo_scidam_fasciculi_nominat() {
    par_est_fasciculorum_erroris(
        &DIVISIO,
        "divisio-in-fasciculo",
        "Error in 'alpha/porta.liss', linea 2: divisio per nullum\n",
    );
}

/// Two packages calling each other, twenty levels of alternation. A circular
/// `affer` is legal — nothing in the checker forbids one — and it is the only
/// program shape in which a `FuncId` resolved in one namespace has to name a
/// function declared in another that has not been lowered yet.
#[test]
fn fasciculi_se_invicem_vocant() {
    par_est_fasciculorum(&ALTERNA, "alterna", "XX\n");
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
    par_est_fasciculorum(&VERBA, "verba-iuncta", "a-b-b-b-b-b\n");
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
/// evidence: the interpreter refused at 64 and this returns an answer.
///
/// It never had a recorded interpreter answer and could not have one — the
/// tree-walker's reply at this depth was `vocatio nimis alte implicata`, its own
/// limit and not the language's. Ten thousand is comfortably inside an ordinary
/// 8 MiB `RLIMIT_STACK`, so this test says "deep recursion completes", while the
/// three above say "and when it cannot, it says so in Latin".
///
/// `currunt/recursio-profunda` makes the same point in the corpus at 5,000, and
/// `cli.rs::deep_recursion_runs_because_there_is_no_limit` at 10,000 through the
/// CLI. Three levels of the suite, because a guard that fired early would pass
/// every test that only checks the *failure* path.
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

/// Shallow recursion — twenty frames — where the recorded answer really is the
/// interpreter's, digit for digit.
///
/// This case was chosen to sit inside the old 64-frame ceiling precisely so that
/// one recursive program in this file could be checked against the tree-walker
/// rather than against a transcription. That is why its expectation is a
/// forty-glyph multi-tier vinculum nobody would have typed correctly by hand:
/// `factorial(°XX)` is 2,432,902,008,176,640,000, and the string below came out
/// of the interpreter on 2026-08-08.
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
