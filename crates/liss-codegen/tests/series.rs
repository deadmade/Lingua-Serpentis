//! `lista`, compiled and run (Task 15) — the last part of codegen, and the one
//! with a shape none of the others have.
//!
//! # What makes lists different from everything Tasks 12–14 covered
//!
//! Three things, and each of them is a way to be wrong that the scalar tests
//! could not be:
//!
//! 1. **It is the only recursive type.** A `ni lista lista lista` is three
//!    headers deep, and freeing, copying, comparing and printing it are four
//!    separate walks that must all agree about where the recursion stops.
//! 2. **It is the only place a kind code is read at run time.** Codegen writes
//!    `ekind` once, at construction, as a compile-time immediate; every walk
//!    above reads it back. A wrong immediate is not a crash — it is a free that
//!    treats a pointer as a boolean, or a boolean as a pointer.
//! 3. **It is the only place a deep copy and a deep free can disagree.** A
//!    shallow copy plus a deep free is a double free of the shared children; a
//!    deep copy plus a shallow free is a leak. Both are silent in the printed
//!    output and both are visible in the exit status, because spec §7.5 checks
//!    the live-allocation count on the success path.
//!
//! # The oracle, and the one thing it cannot see
//!
//! Almost everything here is [`support::par_est`]: one source, run interpreted
//! and compiled, the two renderings required equal. The interpreter is what the
//! corpus is blessed against, so it is the only oracle worth having at this
//! size.
//!
//! What it cannot see is **`liss_list_append`'s steal**. `adde` allocates a
//! header one longer, `memcpy`s the existing element words, stores the new one
//! and frees only the *old header* — zero element copies, because the caller
//! already handed over a private copy of each. A `_append` that deep-copied
//! every element instead would print the same thing, exit 0 the same way, and be
//! quadratic. No behavioural test can tell them apart. Two things can, and one
//! of them is here: the **live-allocation count** after each of a hundred
//! appends is `liss-abi`'s `append_in_a_loop_stays_balanced_and_copies_no_element`
//! (it finds `1 + i`, and a copying append would find `1 + i(i+1)/2`); and the
//! **object**, which is [`series_addita_nihil_praeter_appendicem_vocat`] below —
//! codegen emits one `liss_list_append` and adds no element copy of its own, so
//! whatever linearity the ABI has survives compilation.
//!
//! # Why there are no over-freeing tests, rather than an oversight
//!
//! Spec §7.3.2: every free stores null into the slot it emptied, so the free
//! discipline is idempotent and **over-freeing a slot is systematically
//! unobservable**. A mutation that adds a redundant `Corpus::libera` and leaves
//! the suite green proves nothing. What remains detectable is under-freeing (the
//! count), a redundant free of a **temporary** (the header magic, and only since
//! D-27 made `abolere`'s clear volatile), a slot read after its scope died, and
//! a wrong jump target. Every mutation this file was tested against is one of
//! those, and the relocation-order tests at the bottom exist because they are
//! the only thing that can see the ownership rules a program's output cannot.

mod support;

use support::{casus_par_est, par_est, par_est_fasciculorum, vocationes};

// ---------------------------------------------------------------------------
// The checkpoint: `exemplum-series` and every list fixture, compiled
// ---------------------------------------------------------------------------

/// `examples/series.liss` — the checkpoint. A literal, an `adde`, an
/// `IndexAssign`, a `dum` that indexes with the loop variable, and `longitudo`
/// in the loop condition, which is a fresh list copy and a fresh `ni` every
/// iteration.
#[test]
fn exemplum_series_compilatum_currit() {
    casus_par_est("exemplum-series");
}

/// A literal with elements, and the empty literal — which is one allocation
/// with a length and no payload, not a null pointer.
#[test]
fn lista_litteralis_compilata_currit() {
    casus_par_est("lista-litteralis");
}

/// `xs[i]` at both ends of the range.
#[test]
fn lista_index_lectio_compilata_currit() {
    casus_par_est("lista-index-lectio");
}

/// `ni lista lista` — two headers, and the first case where `ekind` names a
/// container rather than a scalar.
#[test]
fn lista_nidificata_compilata_currit() {
    casus_par_est("lista-nidificata");
}

/// `par_est` on two lists: structural, recursive, and **not** a three-way
/// compare — `liss_list_eq` returns 1 for equal, so routing it through the
/// `icmp_imm_s(cc, ordo, 0)` every other type uses reports exactly the wrong
/// answer.
#[test]
fn lista_aequalitas_compilata_currit() {
    casus_par_est("lista-aequalitas");
}

/// `longitudo(xs)` — consumes the list and returns a `ni`.
#[test]
fn longitudo_numerat_compilatum_currit() {
    casus_par_est("longitudo-numerat");
}

// ---------------------------------------------------------------------------
// The three `memoria-*` cases Task 13 deferred to this one
// ---------------------------------------------------------------------------

/// Nesting three deep, built, read through, deep-copied, deep-compared and
/// freed. The printed lines are the weak half of this; the exit status is the
/// assertion.
#[test]
fn memoria_series_nidificata() {
    casus_par_est("memoria-series-nidificata");
}

/// `adde` in a loop, over `ni`, over `voc`, and over `ni lista` — so the word
/// being appended is a list header in the last case and the append path still
/// never looks at it.
#[test]
fn memoria_adde_in_ansa() {
    casus_par_est("memoria-adde-in-ansa");
}

/// `xs[i] = v` displacing an owned element, including a whole nested list, and
/// including `xs[i] = xs[i] + "!"` — which reads and writes the same slot in
/// one statement.
#[test]
fn memoria_elementum_repositum() {
    casus_par_est("memoria-elementum-repositum");
}

// ---------------------------------------------------------------------------
// Every element kind, and the two that are not pointers
// ---------------------------------------------------------------------------

/// **A list of every one of the six element kinds** (spec §5.3): `veritas` 0,
/// `lit` 1, `ni` 2, `mnr` 3, `nr` 4, `voc` 5, each `| (1 << 3)` on the
/// container.
///
/// The kind code is the one number codegen writes and the runtime reads, and a
/// wrong one is never a compile error: `liss_list_free` would call
/// `liss_int_free` on a `voc`'s header, or skip a `ni` because its base is one
/// of the two immediate kinds. Printing exercises the read; scope exit
/// exercises the free; and the two together are what the exit status covers.
///
/// **The two fractions need declared types to be told apart.** An undeclared
/// fraction literal falls back to `nr` (`examples/praecisio.liss`), so a list
/// literal on its own would exercise kind 4 twice and kind 3 never — and kind 3
/// is the one whose grain is XII, which is a wrong *answer* rather than a crash.
#[test]
fn omnis_generis_elementa_in_seriebus_vivunt() {
    par_est(
        r#"munus initium reddit ni {
    veritas lista vera = [verus, falsus, verus];
    lit lista litterae = ['a', 'b'];
    mnr lista unciae = [°I:VI/XII, °II:I/XII];
    nr lista scripula = [°I:I/CCLXXXVIII, °II:CCLXXXVII/CCLXXXVIII];
    ni lista numeri = [°I, °II, °III];
    voc lista verba = ["unum", "duo"];
    scribe(vera);
    scribe(litterae);
    scribe(unciae);
    scribe(scripula);
    scribe(numeri);
    scribe(verba);
    redde nullus;
}
"#,
        "genera-elementorum",
    );
}

/// **The two narrow scalars survive the round trip through a `u64` element
/// word.**
///
/// `veritas` is an `i8` and `lit` an `i32`; a list element is a machine word
/// whatever its kind, so both are zero-extended on the way in and truncated on
/// the way out. Neither direction is checked by the compiler, and the failure is
/// a wrong *answer*: the runtime compares boolean and character elements with a
/// bare `==` on the whole word, so an element carrying garbage above its own
/// bits compares unequal to an identical one built elsewhere — which is what the
/// two `par_est` lines are for. The `si` is the other direction: a `veritas`
/// read back 64 bits wide and handed to `brif` without the truncation is a
/// branch on the wrong register width.
#[test]
fn elementa_angusta_per_verbum_integrum_redeunt() {
    par_est(
        r#"munus initium reddit ni {
    veritas lista vera = [verus, falsus];
    scribe(vera[°I]);
    scribe(vera[°II]);
    scribe(vera[°I] et non vera[°II]);
    scribe(vera par_est [verus, falsus]);
    si (vera[°I]) { scribe("ramus verus"); }

    lit lista litterae = ['a', 'Z', 'ω'];
    scribe(litterae[°III]);
    scribe(litterae[°I] minor_est litterae[°II]);
    scribe(litterae par_est ['a', 'Z', 'ω']);
    redde nullus;
}
"#,
        "elementa-angusta",
    );
}

/// Nesting is unbounded and needs no interning table, because the element kind
/// of `k` is `k - GRADUS` in closed form. Four levels, built, indexed through,
/// compared and printed — and every one of those walks recurses on the child's
/// own header rather than on arithmetic it did itself.
#[test]
fn nidificatio_quattuor_gradibus_constat() {
    par_est(
        r#"munus initium reddit ni {
    ni lista lista lista lista alta = [[[[°I, °II]]], [[[°III]], [[°IV], [°V]]]];
    scribe(alta);
    scribe(alta[°II][°II][°I][°I]);
    scribe(longitudo(alta[°II]));
    scribe(alta par_est [[[[°I, °II]]], [[[°III]], [[°IV], [°V]]]]);
    scribe(alta par_non_est [[[[°I]]]]);
    redde nullus;
}
"#,
        "nidificatio-alta",
    );
}

/// The empty list at every depth. `liss_list_new(ekind, 0)` is one allocation
/// with a length and no payload — not a null pointer, which is what a
/// `len == 0` shortcut would have made it, and which every `_free` treats as
/// "nothing to do", so the leak would be silent.
#[test]
fn series_vacuae_sunt_res_verae() {
    par_est(
        r#"munus initium reddit ni {
    ni lista nulla = [];
    scribe(nulla);
    scribe(longitudo(nulla));

    ni lista lista vacua = [];
    scribe(vacua);
    vacua = adde(vacua, []);
    scribe(vacua);
    scribe(longitudo(vacua[°I]));

    voc lista lista tabula = [[], ["unum"], []];
    scribe(tabula);
    scribe(longitudo(tabula));
    redde nullus;
}
"#,
        "series-vacuae",
    );
}

// ---------------------------------------------------------------------------
// Reading, writing, growing
// ---------------------------------------------------------------------------

/// `xs[i]` is **1-based**, and out of range at either end is
/// `index extra fines` with **both numbers as Roman numerals**.
///
/// Three fixtures assert that exact sentence, so it is checked here against the
/// interpreter rather than transcribed: `errores/index-extra-fines`,
/// `errores/index-infra-fines` and `errores/index-scriptio-extra-fines`. The
/// zero subscript is the one that used to compute `0usize - 1` before the guard
/// could reject it — an overflow panic across an `extern "C"` boundary, which is
/// `SIGABRT` and exit 134 rather than a diagnostic. The empty list is D-12: it
/// reads `longitudo nullus`, not `longitudo 0`.
#[test]
fn index_extra_fines_utrimque_clamat() {
    par_est(
        r#"munus initium reddit ni {
    ni lista numeri = [°I, °II];
    scribe(numeri[°X]);
    redde nullus;
}
"#,
        "index-supra",
    );
    par_est(
        r#"munus initium reddit ni {
    ni lista numeri = [°I, °II];
    scribe(numeri[nullus]);
    redde nullus;
}
"#,
        "index-infra",
    );
    par_est(
        r#"munus initium reddit ni {
    ni lista numeri = [°I];
    numeri[°II] = °X;
    redde nullus;
}
"#,
        "index-scriptio-supra",
    );
    par_est(
        r#"munus initium reddit ni {
    ni lista numeri = [];
    scribe(numeri[°I]);
    redde nullus;
}
"#,
        "index-in-vacuum",
    );
}

/// **Evaluation order, made observable by making two subexpressions raise
/// different sentences.**
///
/// Order is otherwise invisible in this language — it has no side-effecting
/// expression except a raise, and a raise ends the process — so the only way to
/// ask which operand went first is to make both of them fail and read which
/// diagnostic came out. `divisio per nullum` and `modulus per nullum` are two
/// sentences from the same shape of expression, which is exactly the pair
/// needed.
///
/// Four orders are pinned, and each is the interpreter's:
///
/// - a literal's elements, left to right (`eval` walks the `Vec`);
/// - `xs[i]`: **base then index** (`interp.rs`'s `Index` arm);
/// - `adde(xs, v)`: **list then item**, which is argument order;
/// - `xs[i] = v`: **index then value** (`interp.rs:267-268`) — and this is the
///   one a codegen author is most likely to get backwards, because
///   `liss_list_put`'s bounds check happens after both, so swapping them is
///   invisible in every program where neither raises.
#[test]
fn ordo_evaluationis_a_fonte_legitur() {
    par_est(
        r#"munus initium reddit ni {
    ni lista numeri = [°I / nullus, °II % nullus];
    scribe(numeri);
    redde nullus;
}
"#,
        "ordo-litteralis",
    );
    par_est(
        r#"munus initium reddit ni {
    scribe([°I / nullus][°I % nullus]);
    redde nullus;
}
"#,
        "ordo-lectionis",
    );
    par_est(
        r#"munus initium reddit ni {
    scribe(adde([°I / nullus], °II % nullus));
    redde nullus;
}
"#,
        "ordo-additionis",
    );
    par_est(
        r#"munus initium reddit ni {
    ni lista numeri = [°I];
    numeri[°I % nullus] = °II / nullus;
    redde nullus;
}
"#,
        "ordo-scriptionis",
    );
}

/// A raise from **inside** a list literal, which is the case that decides
/// whether the header is allocated before or after its elements.
///
/// A raise does not unwind — `liss_panic` flushes and exits — so a header
/// allocated first would simply be live at the moment the process ends, and the
/// user would still see one diagnostic and exit 1. What this asserts is the
/// diagnostic and the line it names, which is the part they can see; the
/// ordering is why `Corpus::series` evaluates every element before it allocates
/// anything.
#[test]
fn elementum_quod_clamat_seriem_non_aedificat() {
    par_est(
        r#"munus initium reddit ni {
    ni lista numeri = [°I, °II / nullus, °III];
    scribe(numeri);
    redde nullus;
}
"#,
        "elementum-clamans",
    );
}

/// `adde` returns a **new** list and consumes the old one; the element is
/// consumed too. Growing from a literal, growing inside an expression rather
/// than an assignment, and growing a list whose elements are themselves lists.
#[test]
fn adde_seriem_novam_reddit() {
    par_est(
        r#"munus initium reddit ni {
    ni lista numeri = [°I];
    scribe(adde(numeri, °II));
    scribe(longitudo(adde(adde([°I], °II), °III)));

    ni lista lista tabula = [[°I]];
    tabula = adde(tabula, [°II, °III]);
    scribe(tabula);
    scribe(tabula[°II][°II]);
    redde nullus;
}
"#,
        "adde-nova",
    );
}

/// `xs[i] = v` is a **mutation of the slot's own list**, not a rebinding.
///
/// The silent failure is a `Load` in place of the bare `use_var`: the write
/// would land on a private copy, the copy would leak, and the list in the slot
/// would print unchanged. Every line below would then be wrong, and the exit
/// status too. The second statement writes at a subscript that is itself a
/// `longitudo` of the same list, and from a value that reads two of its
/// elements — so the borrow has to survive three expressions that each take a
/// copy of it.
#[test]
fn scriptio_indicis_seriem_ipsam_mutat() {
    par_est(
        r#"munus initium reddit ni {
    voc lista nomina = ["a", "b", "c"];
    nomina[°II] = "B";
    scribe(nomina);
    nomina[longitudo(nomina)] = nomina[°I] + nomina[°II];
    scribe(nomina);

    ni lista lista tabula = [[°I], [°II]];
    tabula[°I] = [°IX, °X];
    scribe(tabula);
    scribe(tabula[°I][°II]);
    redde nullus;
}
"#,
        "scriptio-indicis",
    );
}

/// `longitudo` consumes its argument, so calling it on an expression rather
/// than a variable must not leave the intermediate alive — and calling it twice
/// in one expression must not consume the same list twice.
#[test]
fn longitudo_argumentum_consumit() {
    par_est(
        r#"munus initium reddit ni {
    scribe(longitudo([°I, °II, °III]));
    scribe(longitudo(adde([°I], °II)));
    ni lista lista tabula = [[°I, °II], [°III]];
    scribe(longitudo(tabula));
    scribe(longitudo(tabula[°I]));
    scribe(longitudo(tabula) + longitudo(tabula[°II]));
    redde nullus;
}
"#,
        "longitudo-consumens",
    );
}

/// Deep equality, at depth and across element kinds, including the three ways
/// to be unequal that a shallow comparison would get wrong: same length and
/// different contents, different lengths at depth, and equal at the top level
/// only.
#[test]
fn aequalitas_alte_recurrit() {
    par_est(
        r#"munus initium reddit ni {
    scribe([[°I, °II], [°III]] par_est [[°I, °II], [°III]]);
    scribe([[°I, °II], [°III]] par_est [[°I, °II], [°IV]]);
    scribe([[°I, °II], [°III]] par_non_est [[°I, °II], [°III]]);
    scribe([["a"], ["b"]] par_est [["a"], ["b"]]);
    scribe([[verus]] par_est [[falsus]]);
    scribe([[°I]] par_est [[°I], [°II]]);
    redde nullus;
}
"#,
        "aequalitas-alta",
    );
}

// ---------------------------------------------------------------------------
// Lists and the rest of the language
// ---------------------------------------------------------------------------

/// A list slot in every scope the ownership rules distinguish (spec §7.3): a
/// `dum` header, a loop body, a `si` arm, and an `indem` inside all three, with
/// a `continua` and an `abrumpe` leaving them. Every one of those has its own
/// teardown path, and a list is the costliest thing to get wrong in any of
/// them — §7.3.5's rule that a `dum` header is freed once in the exit block,
/// and not on the back edge, is a double free of a whole nested structure if
/// broken.
#[test]
fn series_in_omni_ambitu_moriuntur() {
    par_est(
        r#"munus initium reddit ni {
    ni summa = nullus;
    dum (ni lista caput = [°I, °II]; longitudo(caput) minor_est °V; caput = adde(caput, °I)) {
        ni lista corpus = [°III];
        si (longitudo(caput) par_est °III) {
            ni lista ramus = adde(corpus, °IV);
            summa = summa + longitudo(ramus);
            continua;
        }
        indem (longitudo(corpus) minor_est °II) {
            ni lista intus = [];
            corpus = adde(corpus, longitudo(intus));
        }
        summa = summa + longitudo(corpus);
        si (summa maior_est °X) { abrumpe; }
    }
    scribe(summa);
    redde nullus;
}
"#,
        "series-in-ambitibus",
    );
}

/// `redde` of a list from inside a loop inside a `dum` header's scope: the
/// returned temporary is the one value that is *not* freed, and every enclosing
/// scope — the body, the header, the parameter frame — is (§7.3.4).
#[test]
fn redditus_seriei_ambitus_omnes_liberat() {
    par_est(
        r#"munus series reddit ni lista {
    dum (ni lista caput = [°I]; verus; caput = adde(caput, °II)) {
        ni lista corpus = [°III, °IV];
        si (longitudo(caput) par_est °II) {
            redde adde(corpus, longitudo(caput));
        }
    }
    redde [];
}

munus initium reddit ni {
    ni lista reditus = series();
    scribe(reditus);
    scribe(longitudo(reditus));
    redde nullus;
}
"#,
        "redditus-seriei",
    );
}

/// A list as a parameter and as a return value — moved in, moved out, and
/// never copied at the boundary. The caller's own list must survive the call
/// unchanged, which is what the last line checks.
#[test]
fn series_per_munera_transeunt() {
    par_est(
        r#"munus duplica capit (ni lista numeri) reddit ni lista {
    redde adde(numeri, longitudo(numeri));
}

munus profunda capit (ni lista lista tabula) reddit ni {
    redde longitudo(tabula) + longitudo(tabula[°I]);
}

munus initium reddit ni {
    ni lista numeri = [°I, °II];
    scribe(duplica(numeri));
    scribe(duplica([°V]));
    scribe(profunda([[°I, °II, °III], [°IV]]));
    scribe(numeri);
    redde nullus;
}
"#,
        "series-per-munera",
    );
}

/// **A `series` across a package boundary**, which Task 14 recorded as
/// uncovered: only `voc` crossed a namespace before this.
///
/// A package cannot call a *root* function, so the crossing is root→package.
/// Nothing in `emissio.rs` can tell a cross-package call from a same-namespace
/// one — a package's whole identity lives in `Func::symbol`, mangled before
/// codegen sees the function — so this tests that claim rather than new code,
/// and a list is the argument most likely to expose an ABI difference if one
/// had crept in: it is a pointer the callee must free and the caller must not,
/// and `profunda` returns an element of its own argument, which is a value that
/// has to outlive the list it came out of.
#[test]
fn series_limitem_fasciculi_transeunt() {
    par_est_fasciculorum(
        &[
            (
                "principale.liss",
                r#"affer mathematica;

munus initium reddit ni {
    ni lista numeri = [°I, °II, °III];
    scribe(mathematica.summa(numeri));
    scribe(mathematica.cresce([°IX]));
    scribe(mathematica.profunda([[°I], [°II, °III]]));
    scribe(numeri);
    redde nullus;
}
"#,
            ),
            (
                "mathematica/series.liss",
                r#"munus summa capit (ni lista numeri) reddit ni {
    ni acervus = nullus;
    dum (ni i = °I; i minor_est longitudo(numeri) + °I; i = i + °I) {
        acervus = acervus + numeri[i];
    }
    redde acervus;
}

munus cresce capit (ni lista numeri) reddit ni lista {
    redde adde(numeri, °X);
}

munus profunda capit (ni lista lista tabula) reddit ni lista {
    redde tabula[°II];
}
"#,
            ),
        ],
        "series-trans-fasciculum",
    );
}

/// Three hundred elements, built by `adde`, read back at both ends and summed.
///
/// The offset of the last element is 2416 bytes from its header, which is the
/// only test here that would notice a store folded to the wrong displacement;
/// and if the steal ever regressed to a copy, this is the shape that would go
/// quadratic first.
#[test]
fn series_longa_ad_ultimum_elementum_pertingit() {
    par_est(
        r#"munus initium reddit ni {
    ni lista numeri = [];
    dum (ni i = °I; i minor_est °CCCI; i = i + °I) {
        numeri = adde(numeri, i * i);
    }
    scribe(longitudo(numeri));
    scribe(numeri[°I]);
    scribe(numeri[°CCC]);
    ni summa = nullus;
    dum (ni j = °I; j minor_est longitudo(numeri) + °I; j = j + °I) {
        summa = summa + numeri[j];
    }
    scribe(summa);
    redde nullus;
}
"#,
        "series-longa",
    );
}

// ---------------------------------------------------------------------------
// The object: what the printed output cannot show
// ---------------------------------------------------------------------------

/// The ABI entry points a list program calls, in the order codegen emitted
/// them.
///
/// **Only sound within one basic block**, so every program below is
/// straight-line — see `support::vocationes`.
const SPECTANDA: &[&str] = &[
    "liss_list_new",
    "liss_list_get",
    "liss_list_put",
    "liss_list_append",
    "liss_list_len",
    "liss_list_eq",
    "liss_list_copy",
    "liss_list_free",
    "liss_print_list",
    "liss_text_copy",
    "liss_text_free",
];

/// **`adde` lowers to one `liss_list_append` and nothing else.**
///
/// This is the half of the steal that codegen owns. The ABI's own test measures
/// the live count after each of a hundred appends and finds `1 + i`, which is
/// what proves no element was copied; what this proves is that the compiler
/// reaches that entry point directly — one call, no per-element copy, no
/// rebuild — so the linearity is not thrown away on the way in. A codegen that
/// implemented `adde` as "a new list one longer, then copy each element across"
/// would produce identical output and would show up here as **an extra
/// `liss_list_new`** — and that is the whole of what catches it. The
/// per-element copies would *not* show: this program is `ni`-typed and
/// `SPECTANDA` carries `liss_text_copy` but no `liss_int_copy`, deliberately,
/// because a list of `voc` is the only element kind whose copies would survive
/// the filter and adding every scalar `_copy` would drown the sequence in the
/// `Load`s. The extra `new` is sufficient, and it is the assertion doing the
/// work here.
///
/// The `liss_list_copy` is the `Load` of `numeri`, which is the language's only
/// unconditional copy site (§7.2); the `liss_list_free` is `Assign`'s free of
/// the slot's previous value, and it comes **after** the append, because §7.3.3
/// is evaluate → free old → store. Freeing first would hand `adde` a header
/// whose magic had already been cleared.
#[test]
fn series_addita_nihil_praeter_appendicem_vocat() {
    let prog = support::ir_solum(
        r#"munus initium reddit ni {
    ni lista numeri = [°I, °II, °III];
    numeri = adde(numeri, °IV);
    scribe(numeri);
    redde nullus;
}
"#,
    );
    assert_eq!(
        vocationes(&prog, SPECTANDA),
        [
            // `[°I, °II, °III]` — one call, and the three elements are stores.
            "liss_list_new",
            // `adde(numeri, °IV)`: the `Load`'s copy, then the append itself.
            "liss_list_copy",
            "liss_list_append",
            // `Assign`: free the old value, after the new one exists.
            "liss_list_free",
            // `scribe(numeri)`: another `Load`, and a printer that consumes.
            "liss_list_copy",
            "liss_print_list",
            // Scope exit.
            "liss_list_free",
        ],
        "`adde` must be one call, and the only copy must be the `Load`'s",
    );
}

/// **`IndexAssign` borrows.** No copy before the `put`, no free after it.
///
/// `liss_list_put` is one of the ABI's two non-consuming entry points, and the
/// mistake it exists to prevent is invisible in the printed output only if you
/// are unlucky enough for it to be: a `liss_list_copy` before the call writes
/// the element into a private copy and leaks it, and a `liss_list_free` after it
/// leaves the slot holding a cleared header. Neither is a compile error and the
/// first is not even a crash. It is visible here, and — for the free — in the
/// exit status of `memoria-elementum-repositum`.
#[test]
fn scriptio_indicis_seriem_mutuatur_non_consumit() {
    let prog = support::ir_solum(
        r#"munus initium reddit ni {
    ni lista numeri = [°I, °II];
    numeri[°I] = °IX;
    redde nullus;
}
"#,
    );
    assert_eq!(
        vocationes(&prog, SPECTANDA),
        [
            "liss_list_new",
            // The `put`, reached with the slot's own pointer.
            "liss_list_put",
            // Scope exit, and this is the *only* free: the list was borrowed.
            "liss_list_free",
        ],
        "`xs[i] = v` must neither copy the list before the put nor free it after",
    );
}

/// **`Index` reads through a copy of the whole list, and the element it yields
/// is owned.**
///
/// `liss_list_get` consumes its list and returns a deep copy of the element —
/// `interp.rs:182`'s `items[i - 1].clone()` — so `scribe(xs[°I])` copies the
/// list (the `Load`), gets an owned `voc` out of it, and the get frees the copy
/// itself. The element then belongs to the expression, and `scribe` consumes it.
///
/// The absence to notice is a second `liss_list_free`: nothing here frees the
/// list the `get` already consumed, and a codegen that did would be freeing a
/// header whose magic the first free had cleared.
#[test]
fn lectio_indicis_elementum_possidet() {
    let prog = support::ir_solum(
        r#"munus initium reddit ni {
    voc lista verba = ["unum"];
    scribe(verba[°I]);
    redde nullus;
}
"#,
    );
    assert_eq!(
        vocationes(&prog, SPECTANDA),
        [
            "liss_list_new",
            // `verba[°I]`: the `Load`'s copy, then the get, which consumes it.
            "liss_list_copy",
            "liss_list_get",
            // Scope exit. The `voc` the get returned was consumed by
            // `liss_print_text`, which is not among `SPECTANDA`.
            "liss_list_free",
        ],
        "`xs[i]` must consume exactly the copy the `Load` made",
    );
}

/// **A list literal is one `liss_list_new` per list and no setter at all.**
///
/// Spec §5.2: the constructor hands back a header whose element words are
/// codegen's to fill, and codegen writes each one itself at `ONUS + VERBUM*i`,
/// because it already knows the layout. There is deliberately no per-element
/// entry point, and this is what says so — a nested literal is two `new`s with
/// **nothing between them**, and the inner one comes first because elements are
/// evaluated before the header they go into is allocated.
///
/// "Nothing between them" is worth more than it looks. `liss_list_new` allocates
/// with `calloc`, so a part-built list holds null words rather than heap
/// garbage — but this assertion is what shows no correct path ever *has* a
/// part-built list: the constructor is followed by `items.len()` stores and no
/// call, so the zeroing is a net for the raising path only, and a reviewer
/// should not read it as a step the construction depends on.
#[test]
fn litteralis_unam_constructionem_per_seriem_vocat() {
    let prog = support::ir_solum(
        r#"munus initium reddit ni {
    ni lista lista tabula = [[°I, °II]];
    redde nullus;
}
"#,
    );
    assert_eq!(
        vocationes(&prog, SPECTANDA),
        ["liss_list_new", "liss_list_new", "liss_list_free"],
        "the inner literal, then the outer; the elements are stores, not calls",
    );
}

/// **`par_est` on two lists is one `liss_list_eq` that consumes both**, and the
/// statement leaves nothing behind: the result is a `veritas`, which owns
/// nothing, so there is no free after it.
#[test]
fn aequalitas_seriei_utramque_consumit() {
    let prog = support::ir_solum(
        r#"munus initium reddit ni {
    scribe([°I] par_est [°II]);
    redde nullus;
}
"#,
    );
    assert_eq!(
        vocationes(&prog, SPECTANDA),
        ["liss_list_new", "liss_list_new", "liss_list_eq"],
        "both operands are built and both are consumed by the comparison",
    );
}

/// The whole of `lista` compiles for **both** targets.
///
/// Only the host's image is executed anywhere in this suite (Task 18 owns
/// aarch64 under emulation), so without this the second backend never sees a
/// `store` at a computed displacement, a `uextend` of an `i8` into an element
/// word, or an `ireduce` back out of one.
#[test]
fn series_utrique_machinae_componuntur() {
    use liss_codegen::target::Target;
    use object::{Architecture, Object};

    let prog = support::ir_solum(
        r#"munus initium reddit ni {
    veritas lista vera = [verus, falsus];
    lit lista litterae = ['a', 'b'];
    ni lista lista tabula = [[°I], [°II, °III]];
    tabula[°I] = adde(tabula[°II], longitudo(vera));
    scribe(tabula);
    scribe(vera[°I] vel litterae[°I] par_est 'b');
    dum (ni i = °I; i minor_est longitudo(tabula) + °I; i = i + °I) {
        scribe(longitudo(tabula[i]));
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
