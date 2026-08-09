//! `lista`, compiled and run: the only recursive type, the only kind code read at
//! run time. See docs/internals/testing.md.

mod support;

use support::{
    casus_par_est, par_est, par_est_erroris, par_est_fasciculorum, vocationes,
};

/// `examples/series.liss`: a literal, an `adde`, an `IndexAssign`, and `longitudo`
/// in a loop condition — a fresh list copy and a fresh `ni` every iteration.
#[test]
fn exemplum_series_compilatum_currit() {
    casus_par_est("exemplum-series");
}

/// A literal with elements, and the empty literal — one allocation with a length
/// and no payload, not a null pointer.
#[test]
fn lista_litteralis_compilata_currit() {
    casus_par_est("lista-litteralis");
}

/// `xs[i]` at both ends of the range.
#[test]
fn lista_index_lectio_compilata_currit() {
    casus_par_est("lista-index-lectio");
}

/// Two headers, and the first case where `ekind` names a container, not a scalar.
#[test]
fn lista_nidificata_compilata_currit() {
    casus_par_est("lista-nidificata");
}

/// `liss_list_eq` returns 1 for equal, so routing it through the `icmp_imm_s(cc, ordo, 0)`
/// every other type uses reports exactly the wrong answer.
#[test]
fn lista_aequalitas_compilata_currit() {
    casus_par_est("lista-aequalitas");
}

/// `longitudo(xs)` — consumes the list and returns a `ni`.
#[test]
fn longitudo_numerat_compilatum_currit() {
    casus_par_est("longitudo-numerat");
}

/// Nesting three deep. The printed lines are the weak half; the exit status is the
/// assertion.
#[test]
fn memoria_series_nidificata() {
    casus_par_est("memoria-series-nidificata");
}

/// `adde` in a loop over `ni`, `voc` and `ni lista` — the appended word is a list
/// header in the last case, and the append path still never looks at it.
#[test]
fn memoria_adde_in_ansa() {
    casus_par_est("memoria-adde-in-ansa");
}

/// `xs[i] = v` displacing an owned element, including `xs[i] = xs[i] + "!"`, which
/// reads and writes the same slot in one statement.
#[test]
fn memoria_elementum_repositum() {
    casus_par_est("memoria-elementum-repositum");
}

/// All six element kinds (spec §5.3). The two fractions need declared types to be told
/// apart: an undeclared fraction literal falls back to `nr`, so kind 3 would go untried.
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
        "[verus, falsus, verus]\n[a, b]\n[I et semis, II et uncia]\n[I et scripulum, II et deunx et semuncia et sicilicus et sextula et scripulum]\n[I, II, III]\n[unum, duo]\n",
    );
}

/// `veritas` (`i8`) and `lit` (`i32`) survive the round trip through a `u64` element
/// word; neither the zero-extension nor the truncation is checked by the compiler.
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
        "verus\nfalsus\nverus\nverus\nramus verus\nω\nfalsus\nverus\n",
    );
}

/// Nesting needs no interning table: the element kind of `k` is `k - GRADUS` in closed
/// form, and every walk recurses on the child's own header.
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
        "[[[[I, II]]], [[[III]], [[IV], [V]]]]\nIV\nII\nverus\nverus\n",
    );
}

/// The empty list at every depth: not a null pointer, which every `_free` treats as
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
        "[]\nnullus\n[]\n[[]]\nnullus\n[[], [unum], []]\nIII\n",
    );
}

/// `xs[i]` is 1-based, and out of range at either end is `index extra fines` with both
/// numbers as Roman numerals. The fixtures are `conformance/errores/index-extra-fines`,
/// `index-infra-fines` and `index-scriptio-extra-fines`; the empty list is D-12.
#[test]
fn index_extra_fines_utrimque_clamat() {
    par_est_erroris(
        r#"munus initium reddit ni {
    ni lista numeri = [°I, °II];
    scribe(numeri[°X]);
    redde nullus;
}
"#,
        "index-supra",
        "erratum: index extra fines: X (longitudo II)\n  in linea III\n",
    );
    par_est_erroris(
        r#"munus initium reddit ni {
    ni lista numeri = [°I, °II];
    scribe(numeri[nullus]);
    redde nullus;
}
"#,
        "index-infra",
        "erratum: index extra fines: nullus (longitudo II)\n  in linea III\n",
    );
    par_est_erroris(
        r#"munus initium reddit ni {
    ni lista numeri = [°I];
    numeri[°II] = °X;
    redde nullus;
}
"#,
        "index-scriptio-supra",
        "erratum: index extra fines: II (longitudo I)\n  in linea III\n",
    );
    par_est_erroris(
        r#"munus initium reddit ni {
    ni lista numeri = [];
    scribe(numeri[°I]);
    redde nullus;
}
"#,
        "index-in-vacuum",
        "erratum: index extra fines: I (longitudo nullus)\n  in linea III\n",
    );
}

/// Evaluation order, made observable by making two subexpressions raise different
/// sentences. Four orders are pinned, each the interpreter's.
#[test]
fn ordo_evaluationis_a_fonte_legitur() {
    par_est_erroris(
        r#"munus initium reddit ni {
    ni lista numeri = [°I / nullus, °II % nullus];
    scribe(numeri);
    redde nullus;
}
"#,
        "ordo-litteralis",
        "erratum: divisio per nullum\n  in linea II\n",
    );
    par_est_erroris(
        r#"munus initium reddit ni {
    scribe([°I / nullus][°I % nullus]);
    redde nullus;
}
"#,
        "ordo-lectionis",
        "erratum: divisio per nullum\n  in linea II\n",
    );
    par_est_erroris(
        r#"munus initium reddit ni {
    scribe(adde([°I / nullus], °II % nullus));
    redde nullus;
}
"#,
        "ordo-additionis",
        "erratum: divisio per nullum\n  in linea II\n",
    );
    par_est_erroris(
        r#"munus initium reddit ni {
    ni lista numeri = [°I];
    numeri[°I % nullus] = °II / nullus;
    redde nullus;
}
"#,
        "ordo-scriptionis",
        "erratum: modulus per nullum\n  in linea III\n",
    );
}

/// A raise from inside a list literal: `Corpus::series` evaluates every element
/// before it allocates anything.
#[test]
fn elementum_quod_clamat_seriem_non_aedificat() {
    par_est_erroris(
        r#"munus initium reddit ni {
    ni lista numeri = [°I, °II / nullus, °III];
    scribe(numeri);
    redde nullus;
}
"#,
        "elementum-clamans",
        "erratum: divisio per nullum\n  in linea II\n",
    );
}

/// `adde` returns a new list and consumes the old one, and the element too.
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
        "[I, II]\nIII\n[[I], [II, III]]\nIII\n",
    );
}

/// `xs[i] = v` mutates the slot's own list rather than rebinding; a `Load` in place of
/// the bare `use_var` would write into a private copy and print unchanged.
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
        "[a, B, c]\n[a, B, aB]\n[[IX, X], [II]]\nX\n",
    );
}

/// `longitudo` consumes its argument: no intermediate left alive, and no list
/// consumed twice in one expression.
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
        "III\nII\nII\nII\nIII\n",
    );
}

/// Deep equality, including the three ways to be unequal a shallow comparison gets wrong.
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
        "verus\nfalsus\nfalsus\nverus\nfalsus\nfalsus\n",
    );
}

/// A list slot in every scope the ownership rules distinguish (spec §7.3), each with
/// its own teardown path.
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
        "VI\n",
    );
}

/// §7.3.4: the returned temporary is the one value not freed, and every enclosing
/// scope is.
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
        "[III, IV, II]\nIII\n",
    );
}

/// A list moved in and out of a call, never copied at the boundary; the caller's own
/// list must survive unchanged.
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
        "[I, II, II]\n[V, I]\nV\n[I, II]\n",
    );
}

/// A `series` across a package boundary: a pointer the callee must free and the caller
/// must not, and `profunda` returns an element that has to outlive its list.
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
        "VI\n[IX, X]\n[II, III]\n[I, II, III]\n",
    );
}

/// Three hundred elements: the last sits 2416 bytes from its header, the only test here
/// that would notice a store folded to the wrong displacement.
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
        "CCC\nI\nX\u{305}C\u{305}\nM\u{305}X\u{305}\u{305}X\u{305}L\u{305}V\u{305}L\n",
    );
}

/// Only sound within one basic block, so every program below is straight-line —
/// see `support::vocationes`.
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

/// `adde` lowers to one `liss_list_append` and nothing else; an `adde` that rebuilt
/// the list would show up here as an extra `liss_list_new`.
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
            // `[°I, °II, °III]` — the elements are stores.
            "liss_list_new",
            // `adde(numeri, °IV)`: the `Load`'s copy, then the append.
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

/// `IndexAssign` borrows: no copy before the `put`, no free after it.
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

/// `Index` reads through a copy of the whole list and yields an owned element. The
/// absence to notice is a second `liss_list_free`: the `get` already consumed the copy.
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
            // Scope exit; `liss_print_text` is not among `SPECTANDA`.
            "liss_list_free",
        ],
        "`xs[i]` must consume exactly the copy the `Load` made",
    );
}

/// A list literal is one `liss_list_new` per list and no setter at all (spec §5.2);
/// "nothing between them" is what shows no correct path ever has a part-built list.
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

/// One `liss_list_eq` consuming both, and no free after it: the result is a `veritas`.
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

/// Without this the second backend never sees a `store` at a computed displacement,
/// a `uextend` of an `i8` into an element word, or an `ireduce` back out of one.
#[test]
fn series_utrique_machinae_componuntur() {
    use codegen::target::Target;
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
        let octeti = codegen::compila(&prog, &machina).expect("it compiles");
        let fasciculus = object::File::parse(&octeti[..]).expect("a relocatable object");
        let expectata = match machina {
            Target::X86_64 => Architecture::X86_64,
            Target::Aarch64 => Architecture::Aarch64,
        };
        assert_eq!(fasciculus.architecture(), expectata, "{}", machina.triple());
    }
}
