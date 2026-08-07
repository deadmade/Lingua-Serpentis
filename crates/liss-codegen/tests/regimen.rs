//! Control flow, and the ownership discipline that goes with it (spec §7).
//!
//! # What a passing fixture proves, and what it does not
//!
//! Every `memoria-*` case below ends "exit 0 with the blessed stdout", and by
//! spec §7.5 that is a real statement about memory: the runtime checks the
//! live-allocation count on the success path and exits **1** with
//! `memoria non liberata est` if anything is still alive, and `abolere` clears
//! a header's magic before freeing so that a double free is
//! `defectus internus: memoria violata est` rather than silent corruption.
//!
//! But a fixture proves the **sum** of §7.3's six rules, not any one of them:
//! two compensating bugs — a missing free and a double free of a different
//! object — would balance the count and pass. So each rule has a test below
//! that isolates it, and the doc comment on each says which rule it is for and
//! what the failure looks like. The fixtures are the integration; those are the
//! unit.

mod support;

use support::{casus_par_est, par_est};

// ---------------------------------------------------------------------------
// The checkpoint
// ---------------------------------------------------------------------------

/// `examples/calc.liss` — a call, an `si` with no `aliter`, and an `Assign`
/// inside the arm. The first program in the corpus that needs a branch.
#[test]
fn exemplum_calc_compilatum_currit() {
    casus_par_est("exemplum-calc");
}

/// `examples/computatrum` — a folder program with a package. Two calls, no
/// control flow of its own; it is here because the checkpoint says so and
/// because it is the only case that puts a mangled package symbol through the
/// linker before Task 14 owns them.
#[test]
fn exemplum_computatrum_compilatum_currit() {
    casus_par_est("exemplum-computatrum");
}

// ---------------------------------------------------------------------------
// The six `memoria-*` cases
// ---------------------------------------------------------------------------

/// §7.3.3 in a loop: `s = s + s` three times over.
#[test]
fn memoria_assignatio_in_ansa() {
    casus_par_est("memoria-assignatio-in-ansa");
}

/// §7.3.4 through five nested `Block::declares`, including a `dum` header and
/// a `voc` parameter.
#[test]
fn memoria_redditus_ex_ansis() {
    casus_par_est("memoria-redditus-ex-ansis");
}

/// §7.3.5 and §7.3.6 out of a nested `si`, past a `dum` header that owns a
/// `voc`.
#[test]
fn memoria_abrumpe_ex_si() {
    casus_par_est("memoria-abrumpe-ex-si");
}

/// §7.3.5's exception at its narrowest: a `dum` header holding a `voc` whose
/// loop is left only by `redde`, which never reaches the exit block where the
/// header teardown lives.
#[test]
fn memoria_caput_dum_redditum() {
    casus_par_est("memoria-caput-dum-redditum");
}

/// §7.3.2 across a back edge, twenty times, so a *drift* shows and not only an
/// outright omission.
#[test]
fn memoria_ansa_vocem_reponit() {
    casus_par_est("memoria-ansa-vocem-reponit");
}

/// The rule §7.3 was missing: a slot freed by a scope exit must be left
/// **null**, or the next iteration's `continua` frees the same pointer again.
#[test]
fn memoria_sedes_post_continua() {
    casus_par_est("memoria-sedes-post-continua");
}

// ---------------------------------------------------------------------------
// One test per §7.3 rule
// ---------------------------------------------------------------------------

/// **§7.3.1 — the statement root.** A call for effect is the only statement in
/// the language whose expression tree leaves a live temporary nothing consumes,
/// and `summare(°I, °II);` is the shape: an `ExprStmt` of type `ni`.
///
/// Task 12 implemented this arm and had nothing that reached it (its own report
/// §9.2 says so). This is the first program that does. Without the free it is
/// one leaked `LissInt` per call and `memoria non liberata est`, exit 1; the
/// two calls make it two, so a fix that freed one root per *function* rather
/// than per *statement* would still fail.
#[test]
fn radix_sententiae_semel_liberatur() {
    par_est(
        r#"munus summare capit (ni a, ni b) reddit ni {
    redde a + b;
}

munus initium reddit ni {
    summare(°I, °II);
    summare(°III, °IV);
    scribe("factum");
    redde nullus;
}
"#,
        "radix-sententiae",
    );
}

/// **§7.3.2 — scope exit, in reverse declaration order.**
///
/// Reverse order is not observable from a program: the frees are independent,
/// so a forward walk leaks nothing and every behavioural test passes. It is
/// asserted **in the object** instead, by the order of the relocations against
/// the two free entry points.
///
/// The function body declares `s` (a `voc`) and then `n` (a `ni`), so the
/// teardown must call `liss_int_free` and *then* `liss_text_free`. A forward
/// walk emits them the other way round. Cranelift does not reorder calls — they
/// have side effects — so the order in the `.text` relocations is the order
/// codegen emitted.
#[test]
fn ambitus_ordine_averso_liberatur() {
    let prog = support::ir_solum(
        r#"munus initium reddit ni {
    voc s = "salve";
    ni n = °I;
    redde nullus;
}
"#,
    );
    assert_eq!(
        support::vocationes(&prog, &["liss_int_free", "liss_text_free"]),
        ["liss_int_free", "liss_text_free"],
        "the teardown must walk `Block::declares` in reverse declaration order: \
         `n` was declared second and must die first",
    );
}

/// **§7.3.3 — `Assign` is evaluate → free old → store**, asserted **in the
/// object**, because no program can see it.
///
/// The obvious test is behavioural: `s = s + s` with the free first is a
/// use-after-free, and `liss_text_copy` checks the magic, so it should be
/// `defectus internus: memoria violata est`. It is not, and this is worth
/// stating plainly because it also bounds what every other test here proves:
/// **`abolere`'s `(*h).magic = 0` is dead-store-eliminated** in the release
/// build of `liss-abi` that actually ships. LLVM knows `free` ends the object's
/// lifetime and drops a store that precedes it, and the archive's
/// `liss_text_free` disassembles to a null test, a magic test, a kind test,
/// `call free`, `lock decq` — no write at all. So a read *immediately* after a
/// free sees intact bytes and the right answer comes out. Detection returns
/// only once something else has been allocated over the block, which is why the
/// double free in `memoria-sedes-post-continua` — a whole iteration apart — is
/// still caught.
///
/// Hence relocation order, which is not probabilistic. Both `liss_text_copy`s
/// and the `liss_text_concat` that consumes them must precede the
/// `liss_text_free` of the old value. The program is straight-line so that
/// `.text` layout order is execution order; see [`support::vocationes`].
#[test]
fn assignatio_vetus_post_lectionem_liberatur() {
    let prog = support::ir_solum(
        r#"munus initium reddit ni {
    voc s = "ab";
    s = s + s;
    scribe(s);
    redde nullus;
}
"#,
    );
    let ordo = support::vocationes(
        &prog,
        &["liss_text_copy", "liss_text_concat", "liss_text_free"],
    );
    assert_eq!(
        &ordo[..4],
        ["liss_text_copy", "liss_text_copy", "liss_text_concat", "liss_text_free"],
        "`Assign` is evaluate → free old → store: both reads of `s` and the \
         concatenation that consumes them must come before the old value dies \
         (got {ordo:?})",
    );
}

/// The same rule again, behaviourally and across a back edge: the answer is
/// still right when the slot is rewritten three times.
///
/// This one *cannot* fail on ordering — see
/// [`assignatio_vetus_post_lectionem_liberatur`] for why a use-after-free reads
/// intact memory — but it can and does fail on **balance**: an `Assign` that
/// frees nothing leaks three `LissText`s and exits 1 with
/// `memoria non liberata est`.
#[test]
fn assignatio_in_ansa_libratur() {
    par_est(
        r#"munus initium reddit ni {
    ni i = nullus;
    voc s = "ab";
    indem (i minor_est °II) {
        s = s + s;
        i = i + °I;
    }
    scribe(s);
    redde nullus;
}
"#,
        "assignatio-vetus",
    );
}

/// **§7.3.4 — `redde` frees every enclosing block, innermost first, and the
/// parameter frame is one of them and only one of them.**
///
/// `nomen` is a `voc` **parameter**: slot 0, and also the first entry of the
/// body block's `declares`. The `redde` is two scopes deep, so the walk must
/// reach the body block from inside an `si` inside an `si` — and must arrive
/// there exactly once.
///
/// - A walk that stopped at the innermost block leaks `intus` and `medium`.
/// - A walk that also swept `0..params` off `Func::params` double-frees
///   `nomen`, which `abolere`'s cleared magic reports as
///   `defectus internus: memoria violata est`.
///
/// The returned value is `nomen + medium`, a fresh `LissText` built from copies,
/// so it survives a teardown that frees the slots it was built from — which is
/// §7.3's "the returned temporary is never one of them".
#[test]
fn redditus_omnem_ambitum_semel_liberat() {
    par_est(
        r#"munus orna capit (voc nomen) reddit voc {
    si (verus) {
        voc medium = "-";
        si (verus) {
            voc intus = "!";
            redde nomen + medium + intus;
        }
        redde medium;
    }
    redde nomen;
}

munus initium reddit ni {
    scribe(orna("radix"));
    redde nullus;
}
"#,
        "redditus-ambitus",
    );
}

/// **§7.3.5 — `abrumpe` and `continua` free up to and including the loop body,
/// and *not* the `dum` header.**
///
/// The header owns `caput`, a `voc`, and the teardown §7.3.5 emits once after
/// the exit block is the only thing that frees it. So:
///
/// - if the post-exit teardown were dropped, `caput` leaks —
///   `memoria non liberata est`, exit 1;
/// - if the jumps also freed `header.declares`, `caput` is freed by the
///   **`continua`** and the *next* iteration reads a null slot —
///   `verifica`'s null leg, `defectus internus: memoria violata est`, exit 1.
///
/// **The `continua` is what makes the second half testable, and that is a
/// finding rather than a detail.** With `Corpus::libera` leaving each freed slot
/// null, an `abrumpe` that over-freed the header would be *harmless*: the loop
/// is over, so the exit block's teardown frees null and nothing observes it. It
/// is `continua` — which returns to a body that still reads the header
/// variable — that turns the same mistake into a fault. Written with `abrumpe`
/// alone, this test passes over a codegen that gets §7.3.5 wrong.
///
/// Neither failure is a wrong answer, so the differential's *status* is the
/// assertion and its stdout is the corroboration.
#[test]
fn abrumptio_et_continuatio_caput_dum_non_liberant() {
    par_est(
        r#"munus initium reddit ni {
    ni i = nullus;
    dum (voc caput = "c"; verus; i = i + °I) {
        voc intus = "i";
        si (i maior_est °IV) { abrumpe; }
        si (i minor_est °II) { continua; }
        scribe(caput + intus);
    }
    redde nullus;
}
"#,
        "abrumptio-caput",
    );
}

/// **§7.3.6 — `continua` frees the same set as `abrumpe`, then jumps to the
/// *step* block of a `dum` and the *condition* block of an `indem`.**
///
/// The target is what this isolates, and it is the difference between a program
/// and a hang: a `dum`'s `continua` that jumped to the condition would skip
/// `i = i + °I` and loop forever, and an `indem`'s that jumped to a step it does
/// not have has nowhere to go. Both loops here `continua` on their *first*
/// iteration, so a wrong target does not terminate — the test would time out
/// rather than fail, which is worth knowing before reading the diff.
///
/// The `voc` in each body is what makes it a memory test as well: the jump
/// leaves a scope, and leaving a scope frees it.
#[test]
fn continuatio_ad_gradum_et_ad_condicionem_salit() {
    par_est(
        r#"munus initium reddit ni {
    ni summa = nullus;
    dum (ni i = nullus; i minor_est °V; i = i + °I) {
        voc nota = "n";
        si (i minor_est °II) { continua; }
        summa = summa + i;
    }
    ni j = nullus;
    indem (j minor_est °V) {
        voc alia = "a";
        j = j + °I;
        si (j minor_est °III) { continua; }
        summa = summa + j;
    }
    scribe(summa);
    redde nullus;
}
"#,
        "continuatio-saltus",
    );
}

// ---------------------------------------------------------------------------
// The shapes that break a CFG builder rather than a program
// ---------------------------------------------------------------------------

/// Both arms of an `si` return, so the join block has **no predecessors**.
///
/// This is the shape that fails inside `cranelift-frontend` rather than in the
/// program: whatever codegen emits after the `si` goes into a block nothing
/// reaches, and `use_var` there has no definition to find. It is also the shape
/// where a `switch_to_block` guarded by "the current block is filled" alone is
/// wrong — the arm may legitimately be *pristine*.
#[test]
fn si_utroque_bracchio_reddente_compilatur() {
    par_est(
        r#"munus elige capit (ni n) reddit voc {
    si (n minor_est °I) {
        voc parvum = "parvum";
        redde parvum + "!";
    } aut {
        voc magnum = "magnum";
        redde magnum + "!";
    }
}

munus initium reddit ni {
    scribe(elige(nullus));
    scribe(elige(°V));
    redde nullus;
}
"#,
        "utrumque-bracchium",
    );
}

/// An `aliter` with an empty body, and an `si` with none.
///
/// The empty `aliter` arm is the *pristine block* the carried warning names: the
/// builder switches to it and emits nothing at all before switching away, which
/// `FunctionBuilder::is_filled` alone would reject.
#[test]
fn bracchium_vacuum_compilatur() {
    par_est(
        r#"munus initium reddit ni {
    ni n = °I;
    si (n minor_est °II) {
        scribe("parvum");
    } aut {
    }
    si (n maior_est °II) {
        scribe("numquam");
    }
    redde nullus;
}
"#,
        "bracchium-vacuum",
    );
}

/// `abrumpe` and `continua` fill a Cranelift block exactly as `redde` does, so
/// the guard Task 12 added for a statement after `redde` must cover them —
/// including the block's own scope teardown, which must **not** run a second
/// time after a jump that already performed it.
///
/// The checker admits the dead statement for the same reason it admits one
/// after `redde`: its all-paths analysis asks whether any statement returns, and
/// it has no dead-code lint. Without the guard this is
/// `you cannot add an instruction to a block already filled`; with the guard but
/// without skipping the teardown it is a double free of `nota`.
#[test]
fn sententia_post_abrumpe_neglegitur() {
    par_est(
        r#"munus initium reddit ni {
    dum (ni i = nullus; i minor_est °III; i = i + °I) {
        voc nota = "n";
        abrumpe;
        scribe(nota);
    }
    ni k = nullus;
    indem (k minor_est °II) {
        voc alia = "a";
        k = k + °I;
        continua;
        scribe(alia);
    }
    scribe("factum");
}
"#,
        "post-abrumpe",
    );
}

/// The whole of control flow compiles for **both** targets.
///
/// Only the host's image is executed anywhere in this suite (Task 18 owns
/// aarch64 under emulation), so without this the second target's CFG is never
/// built at all — and `brif` on `i8`, the block ordering and the probestack
/// prologue are all backend code that a `si` inside a `dum` is the first thing
/// to reach. The assertion is `compila` returning an object with the right
/// `e_machine`, which is what Cranelift's verifier and register allocator
/// running clean look like from here.
#[test]
fn regimen_utrique_machinae_componitur() {
    use liss_codegen::target::Target;
    use object::{Architecture, Object};

    let prog = support::ir_solum(
        r#"munus initium reddit ni {
    ni summa = nullus;
    dum (voc marca = "m"; verus; summa = summa + °I) {
        voc nota = "n";
        si (summa maior_est °III) { abrumpe; }
        si (summa par_est °I) { continua; }
        indem (summa minor_est nullus) { continua; }
        scribe(marca + nota);
    }
    redde nullus;
}
"#,
    );
    for machina in Target::OMNES {
        let octeti = liss_codegen::compila(&prog, &machina).expect("it compiles");
        let fasciculus = object::File::parse(&octeti[..]).expect("a relocatable object");
        let expectata = match machina {
            Target::X86_64 => Architecture::X86_64,
            Target::Aarch64 => Architecture::Aarch64,
        };
        assert_eq!(fasciculus.architecture(), expectata, "{}", machina.triple());
    }
}

/// Loops nested three deep, with `abrumpe` and `continua` binding to the
/// **innermost** enclosing loop.
///
/// A loop stack that recorded one target rather than a stack of them compiles
/// fine and jumps to the wrong block, which is a wrong answer rather than a
/// crash — so the differential against the interpreter is the whole assertion.
#[test]
fn ansae_nidificatae_intimam_petunt() {
    par_est(
        r#"munus initium reddit ni {
    ni summa = nullus;
    dum (ni i = nullus; i minor_est °III; i = i + °I) {
        dum (ni j = nullus; j minor_est °III; j = j + °I) {
            indem (verus) {
                si (j par_est i) { abrumpe; }
                summa = summa + °I;
                abrumpe;
            }
            si (j maior_est i) { continua; }
            summa = summa + °X;
        }
        summa = summa + °C;
    }
    scribe(summa);
    redde nullus;
}
"#,
        "ansae-nidificatae",
    );
}
