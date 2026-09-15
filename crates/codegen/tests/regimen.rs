//! Control flow and the ownership discipline that goes with it (spec §7): one test
//! per §7.3 rule beside the `memoria-*` fixtures. See docs/internals/testing.md.

mod support;

use support::{casus_par_est, par_est};

/// `examples/calc.liss`: the first program in the corpus that needs a branch.
#[test]
fn exemplum_calc_compilatum_currit() {
    casus_par_est("exemplum-calc");
}

/// `examples/computatrum`: a folder program, and the only case that puts a mangled
/// package symbol through the linker.
#[test]
fn exemplum_computatrum_compilatum_currit() {
    casus_par_est("exemplum-computatrum");
}

/// §7.3.3 in a loop: `s = s + s` three times over.
#[test]
fn memoria_assignatio_in_ansa() {
    casus_par_est("memoria-assignatio-in-ansa");
}

/// §7.3.4 through five nested `Block::declares`, a `dum` header and a `voc` parameter.
#[test]
fn memoria_redditus_ex_ansis() {
    casus_par_est("memoria-redditus-ex-ansis");
}

/// §7.3.5 and §7.3.6 out of a nested `si`, past a `dum` header that owns a `voc`.
#[test]
fn memoria_abrumpe_ex_si() {
    casus_par_est("memoria-abrumpe-ex-si");
}

/// §7.3.5's exception at its narrowest: a `dum` header whose loop is left only by `redde`.
#[test]
fn memoria_caput_dum_redditum() {
    casus_par_est("memoria-caput-dum-redditum");
}

/// §7.3.2 across a back edge, twenty times, so a drift shows and not only an omission.
#[test]
fn memoria_ansa_vocem_reponit() {
    casus_par_est("memoria-ansa-vocem-reponit");
}

/// A slot freed by a scope exit must be left null, or the next `continua` frees it again.
#[test]
fn memoria_sedes_post_continua() {
    casus_par_est("memoria-sedes-post-continua");
}

/// §7.3.1, the statement root: a call for effect leaves a live temporary nothing
/// consumes. Two calls, so a free per *function* rather than per *statement* still fails.
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
        "factum\n",
    );
}

/// §7.3.2, scope exit in reverse declaration order — asserted in the object, because
/// a forward walk leaks nothing and every behavioural test passes.
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

/// §7.3.3, `Assign` is evaluate → free old → store, asserted in the object: relocation
/// order is not probabilistic. See [`support::vocationes`].
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
    // `starts_with` rather than `&ordo[..4]`: fewer calls must fail with the message below.
    assert!(
        ordo.starts_with(&[
            "liss_text_copy".to_owned(),
            "liss_text_copy".to_owned(),
            "liss_text_concat".to_owned(),
            "liss_text_free".to_owned(),
        ]),
        "`Assign` is evaluate → free old → store: both reads of `s` and the \
         concatenation that consumes them must come before the old value dies \
         (got {ordo:?})",
    );
}

/// §7.3.3 again, behaviourally and across a back edge (D-27).
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
        "abababab\n",
    );
}

/// §7.3.4: `redde` frees every enclosing block, innermost first, and the parameter
/// frame is one of them and only one of them.
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
        "radix-!\n",
    );
}

/// §7.3.5: `abrumpe` and `continua` free up to and including the loop body, and not
/// the `dum` header. The `continua` is what makes the over-free half testable.
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
        "ci\nci\nci\n",
    );
}

/// §7.3.6: `continua` jumps to a `dum`'s step block and an `indem`'s condition block.
/// A wrong target does not terminate, so this times out rather than failing.
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
        "XXI\n",
    );
}

/// Both arms of an `si` return, so the join block has no predecessors — the shape that
/// fails inside `cranelift-frontend` rather than in the program.
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
        "parvum!\nmagnum!\n",
    );
}

/// An empty `aliter` arm is a *pristine* Cranelift block, not a filled one.
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
        "parvum\n",
    );
}

/// `abrumpe` and `continua` fill a Cranelift block exactly as `redde` does, and the
/// block's teardown must not run a second time after a jump that already performed it.
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
        "factum\n",
    );
}

/// Without this the second target's CFG is never built at all.
#[test]
fn regimen_utrique_machinae_componitur() {
    use codegen::target::Target;
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
        let octeti = codegen::compila(&prog, &machina).expect("it compiles");
        let fasciculus = object::File::parse(&octeti[..]).expect("a relocatable object");
        let expectata = match machina {
            Target::X86_64 => Architecture::X86_64,
            Target::Aarch64 => Architecture::Aarch64,
        };
        assert_eq!(fasciculus.architecture(), expectata, "{}", machina.triple());
    }
}

/// `abrumpe` and `continua` bind to the innermost enclosing loop; a stack that recorded
/// one target compiles fine and gives a wrong answer.
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
        "CCCLXVI\n",
    );
}
