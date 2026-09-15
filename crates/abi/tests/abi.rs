//! The C ABI's representation and ownership, read back off the heap rather than
//! through a formatter; `liss_panic` is observed in a subprocess. Rendering is
//! `emissio.rs`. See docs/internals/testing.md.

use std::process::{Command, Output};
use std::sync::Mutex;

use abi::defectus::{liss_exit, liss_panic};
use abi::memoria::liss_live_count;
use abi::numeri::{
    liss_frac_div, liss_frac_mul, liss_int_add, liss_int_cmp, liss_int_copy, liss_int_div,
    liss_int_free, liss_int_mod, liss_int_mul, liss_int_neg, liss_int_new, liss_int_sub,
};
use abi::scriptio::liss_print_int;
use abi::series::{
    liss_list_append, liss_list_copy, liss_list_eq, liss_list_free, liss_list_get, liss_list_len,
    liss_list_new, liss_list_put,
};
use abi::textus::{
    liss_text_cmp, liss_text_concat, liss_text_copy, liss_text_free, liss_text_new,
};
use abi::{
    GRADUS, KIND_FRAC_SCRIPULUM, KIND_FRAC_UNCIA, KIND_INT, KIND_TEXT, LISS_MAGIC, LissInt,
    LissList, LissText, ONUS,
};

/// The live count is one per process; everything that reads it serialises here.
static SOLUS: Mutex<()> = Mutex::new(());

fn solus<T>(f: impl FnOnce() -> T) -> T {
    let custos = SOLUS.lock().unwrap_or_else(|e| e.into_inner());
    let r = f();
    drop(custos);
    r
}

/// A `ni` from a host integer, built the way codegen will: little-endian digits.
unsafe fn ni(n: i64) -> *mut LissInt {
    unsafe { numerus(KIND_INT, n) }
}

unsafe fn numerus(kind: u32, n: i64) -> *mut LissInt {
    let mut magnitudo = n.unsigned_abs();
    let mut digiti = Vec::new();
    while magnitudo > 0 {
        digiti.push((magnitudo % 10) as u8);
        magnitudo /= 10;
    }
    unsafe { liss_int_new(kind, u64::from(n < 0), digiti.as_ptr(), digiti.len() as u64) }
}

/// A `LissInt`'s payload as `(negative, little-endian digits)`, read off the heap.
unsafe fn digiti(p: *const LissInt) -> (bool, Vec<u8>) {
    unsafe {
        (
            (*p).neg != 0,
            core::slice::from_raw_parts(p.cast::<u8>().add(ONUS), (*p).len as usize).to_vec(),
        )
    }
}

/// As [`digiti`], then free — the shape most assertions want.
unsafe fn consume(p: *mut LissInt) -> (bool, Vec<u8>) {
    let d = unsafe { digiti(p) };
    unsafe { liss_int_free(p) };
    d
}

unsafe fn verba(p: *const LissText) -> String {
    unsafe {
        String::from_utf8(
            core::slice::from_raw_parts(p.cast::<u8>().add(ONUS), (*p).len as usize).to_vec(),
        )
        .expect("a LissText payload is utf-8")
    }
}

/// A direct store into a list's item array at `ONUS + 8*i`, as codegen emits it.
unsafe fn pone(p: *mut LissList, i: usize, verbum: u64) {
    unsafe { *p.cast::<u8>().add(ONUS).cast::<u64>().add(i) = verbum };
}

unsafe fn lege(p: *const LissList, i: usize) -> u64 {
    unsafe { *p.cast::<u8>().cast_mut().add(ONUS).cast::<u64>().add(i) }
}

#[test]
fn a_nested_list_builds_reads_appends_and_frees_to_zero() {
    solus(|| unsafe {
        assert_eq!(liss_live_count(), 0, "a test left allocations behind");

        // `ni lista lista m = [[°I, °II], [°III]];`
        let intus_a = liss_list_new(u64::from(KIND_INT), 2);
        pone(intus_a, 0, ni(1) as u64);
        pone(intus_a, 1, ni(2) as u64);
        let intus_b = liss_list_new(u64::from(KIND_INT), 1);
        pone(intus_b, 0, ni(3) as u64);

        let ekind = u64::from(KIND_INT + GRADUS);
        let m = liss_list_new(ekind, 2);
        pone(m, 0, intus_a as u64);
        pone(m, 1, intus_b as u64);
        assert_eq!((*m).h.magic, LISS_MAGIC);
        assert_eq!((*m).h.kind, KIND_INT + GRADUS + GRADUS, "ni lista lista is 18");
        assert_eq!((*m).ekind, u64::from(KIND_INT + GRADUS), "its elements are 10");
        // Three headers plus three integers.
        assert_eq!(liss_live_count(), 6);

        // `m[°I]` — a deep copy, so the original stays whole and the read is not an alias.
        let interior = liss_list_get(liss_list_copy(m), ni(1), std::ptr::null(), 0) as *mut LissList;
        assert_ne!(interior, intus_a, "a deep copy is a different object");
        assert_eq!(consume(lege(interior, 1) as *mut LissInt), (false, vec![2]));
        pone(interior, 1, ni(2) as u64); // put back what `consume` freed
        liss_list_free(interior);
        assert_eq!(liss_live_count(), 6, "the copy is gone, the original is not");

        // `adde(m, [°IV])` — steals `m`'s element words, frees only its header.
        let intus_c = liss_list_new(u64::from(KIND_INT), 1);
        pone(intus_c, 0, ni(4) as u64);
        let auctus = liss_list_append(m, intus_c as u64);
        assert_eq!((*auctus).len, 3);
        assert_eq!(
            lege(auctus, 0),
            intus_a as u64,
            "adde stole the word rather than copying the sublist",
        );
        assert_eq!(lege(auctus, 2), intus_c as u64);

        liss_list_free(auctus);
        assert_eq!(liss_live_count(), 0, "the whole tree is gone");
    });
}

#[test]
fn freeing_a_null_is_a_no_op() {
    solus(|| unsafe {
        assert_eq!(liss_live_count(), 0);
        // §7.3.2 frees slots whose `Let` never ran, so the free arrives here as null.
        liss_int_free(std::ptr::null_mut());
        liss_text_free(std::ptr::null_mut());
        liss_list_free(std::ptr::null_mut());
        assert_eq!(liss_live_count(), 0, "a no-op allocates and frees nothing");

        // A list caught part-built holds zero words, and the free must walk past them.
        let xs = liss_list_new(u64::from(KIND_TEXT), 3);
        pone(xs, 1, liss_text_new(b"duo".as_ptr(), 3) as u64);
        assert_eq!(liss_live_count(), 2);
        liss_list_free(xs);
        assert_eq!(liss_live_count(), 0);
        // Reaching this line at all is the assertion: none of the above exited.
    });
}

/// Every Latin sentence this crate can say, read out of its own source: crate-local
/// and exhaustive, where `catalogus.rs`'s ratchet notices only what its markers find.
fn sententiae() -> Vec<String> {
    let dir = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src"));
    let mut inventa = Vec::new();
    for intrans in std::fs::read_dir(dir).expect("src/ is readable") {
        let via = intrans.expect("a directory entry").path();
        if via.extension().is_none_or(|e| e != "rs") {
            continue;
        }
        for linea in std::fs::read_to_string(&via)
            .expect("utf-8 source")
            .lines()
        {
            // Whole-line comments only, and no block comments exist: docs/internals/testing.md.
            if linea.trim_start().starts_with("//") {
                continue;
            }
            let mut reliquum = linea;
            while let Some(i) = reliquum.find("c\"") {
                let rest = &reliquum[i + 2..];
                let finis = rest
                    .find('"')
                    .expect("a diagnostic literal closes on its own line");
                inventa.push(rest[..finis].to_string());
                reliquum = &rest[finis + 1..];
            }
        }
    }
    inventa.sort();
    inventa.dedup();
    inventa
}

#[test]
fn every_latin_sentence_this_crate_can_say_is_listed_here() {
    let omnes = sententiae();
    let inventa: Vec<&str> = omnes.iter().map(String::as_str).collect();
    assert_eq!(
        inventa,
        [
            "acervus vocationum exhaustus est",
            // Two sentences, not one, deliberately: docs/internals/testing.md.
            "defectus internus: memoria violata est: signum",
            "defectus internus: memoria violata est: verificatio",
            "divisio per nullum",
            "index extra fines: {} (longitudo {})",
            // New here, and permanently UNCOVERED by design.
            "memoria exhausta est",
            "memoria non liberata est",
            "modulus per nullum",
            "numerus nimis magnus est",
        ],
        "a Latin sentence appeared or changed; justify it, then update this list",
    );
}

#[test]
fn every_copy_and_free_pair_balances() {
    solus(|| unsafe {
        assert_eq!(liss_live_count(), 0);

        let n = ni(-4021);
        let c = liss_int_copy(n);
        assert_eq!(liss_live_count(), 2);
        assert_eq!(digiti(c), digiti(n));
        assert_ne!(c, n);
        liss_int_free(c);
        liss_int_free(n);
        assert_eq!(liss_live_count(), 0);

        let t = liss_text_new(b"salve".as_ptr(), 5);
        let tc = liss_text_copy(t);
        assert_eq!(liss_live_count(), 2);
        assert_eq!(verba(tc), "salve");
        liss_text_free(tc);
        liss_text_free(t);
        assert_eq!(liss_live_count(), 0);

        let xs = liss_list_new(u64::from(KIND_TEXT), 2);
        pone(xs, 0, liss_text_new(b"a".as_ptr(), 1) as u64);
        pone(xs, 1, liss_text_new(b"b".as_ptr(), 1) as u64);
        let ys = liss_list_copy(xs);
        assert_eq!(liss_live_count(), 6, "a deep copy copies the elements too");
        assert_ne!(lege(ys, 0), lege(xs, 0), "elements are copied, not shared");
        assert_eq!(verba(lege(ys, 1) as *const LissText), "b");
        liss_list_free(ys);
        liss_list_free(xs);
        assert_eq!(liss_live_count(), 0);
    });
}

#[test]
fn append_in_a_loop_stays_balanced_and_copies_no_element() {
    solus(|| unsafe {
        assert_eq!(liss_live_count(), 0);
        let mut xs = liss_list_new(u64::from(KIND_INT), 0);
        let mut primum = 0u64;
        for i in 1..=100 {
            xs = liss_list_append(xs, ni(i) as u64);
            if i == 1 {
                primum = lege(xs, 0);
            }
            // One header plus i integers: a copying `adde` would show as 1 + i(i+1)/2.
            assert_eq!(liss_live_count(), 1 + i as u64, "after adde #{i}");
            assert_eq!(lege(xs, 0), primum, "the first element word never moved");
        }
        assert_eq!(consume(liss_list_len(liss_list_copy(xs))), (false, vec![0, 0, 1]));
        liss_list_free(xs);
        assert_eq!(liss_live_count(), 0);
    });
}

#[test]
fn index_assignment_frees_what_it_displaces_and_borrows_the_list() {
    solus(|| unsafe {
        assert_eq!(liss_live_count(), 0);
        let xs = liss_list_new(u64::from(KIND_TEXT), 2);
        pone(xs, 0, liss_text_new(b"unum".as_ptr(), 4) as u64);
        pone(xs, 1, liss_text_new(b"duo".as_ptr(), 3) as u64);
        assert_eq!(liss_live_count(), 3);

        // `xs[°II] = "tria";` — borrows the list, consumes index and value.
        liss_list_put(
            xs,
            ni(2),
            liss_text_new(b"tria".as_ptr(), 4) as u64,
            std::ptr::null(),
            0,
        );
        assert_eq!(liss_live_count(), 3, "the displaced element was freed");
        assert_eq!((*xs).h.magic, LISS_MAGIC, "the list itself was only borrowed");
        assert_eq!(verba(lege(xs, 0) as *const LissText), "unum");
        assert_eq!(verba(lege(xs, 1) as *const LissText), "tria");
        liss_list_free(xs);
        assert_eq!(liss_live_count(), 0);
    });
}

#[test]
fn a_liss_int_carries_romans_canonical_representation() {
    solus(|| unsafe {
        assert_eq!(liss_live_count(), 0);
        // Byte for byte what `Roman::digits()` returns; the empty payload is the zero.
        assert_eq!(consume(ni(0)), (false, vec![]));
        assert_eq!(consume(ni(1)), (false, vec![1]));
        assert_eq!(consume(ni(-25)), (true, vec![5, 2]));
        assert_eq!(consume(ni(1024)), (false, vec![4, 2, 0, 1]));
        assert_eq!(consume(ni(30532)), (false, vec![2, 3, 5, 0, 3]));

        // A blob written non-canonically still lands canonical.
        let p = liss_int_new(KIND_INT, 1, [0u8, 0, 0].as_ptr(), 3);
        assert_eq!((*p).len, 0);
        assert_eq!((*p).neg, 0);
        assert_eq!((*p).h.magic, LISS_MAGIC);
        assert_eq!((*p).h.kind, KIND_INT);
        liss_int_free(p);
        assert_eq!(
            consume(liss_int_new(KIND_INT, 0, [4u8, 2, 1, 0, 0].as_ptr(), 5)),
            (false, vec![4, 2, 1]),
        );
        assert_eq!(liss_live_count(), 0);
    });
}

#[test]
fn integer_arithmetic_consumes_and_returns_one_value() {
    solus(|| unsafe {
        assert_eq!(liss_live_count(), 0);
        assert_eq!(consume(liss_int_add(ni(5), ni(3))), (false, vec![8]));
        assert_eq!(consume(liss_int_sub(ni(5), ni(13))), (true, vec![8]));
        assert_eq!(consume(liss_int_mul(ni(7), ni(6))), (false, vec![2, 4]));
        assert_eq!(consume(liss_int_neg(ni(7))), (true, vec![7]));
        // Rust's `i64` `/` and `%`, which is what the corpus is blessed against.
        assert_eq!(
            consume(liss_int_div(ni(-7), ni(2), std::ptr::null(), 0)),
            (true, vec![3]),
        );
        assert_eq!(
            consume(liss_int_mod(ni(-7), ni(2), std::ptr::null(), 0)),
            (true, vec![1]),
        );
        assert_eq!(liss_int_cmp(ni(3), ni(5)), -1);
        assert_eq!(liss_int_cmp(ni(5), ni(5)), 0);
        assert_eq!(liss_int_cmp(ni(5), ni(3)), 1);
        assert_eq!(liss_int_cmp(ni(-5), ni(3)), -1);
        assert_eq!(liss_int_cmp(ni(-5), ni(-3)), -1);
        assert_eq!(liss_live_count(), 0, "every operand was consumed");
    });
}

#[test]
fn arithmetic_passes_a_machine_word() {
    solus(|| unsafe {
        assert_eq!(liss_live_count(), 0);
        // 3999^6 is about 4.1e21, far past i64::MAX; the payload simply grows.
        let mut acc = ni(1);
        for _ in 0..6 {
            acc = liss_int_mul(acc, ni(3999));
        }
        let (neg, d) = consume(acc);
        assert!(!neg);
        assert_eq!(d.len(), 22, "twenty-two digits does not fit a machine word");
        assert_eq!(liss_live_count(), 0);
    });
}

#[test]
fn a_number_of_xxxviii_digits_is_stored() {
    // D-20
    solus(|| unsafe {
        assert_eq!(liss_live_count(), 0);
        let novem = [9u8; 38];
        assert_eq!(
            consume(liss_int_new(KIND_INT, 0, novem.as_ptr(), 38)),
            (false, vec![9; 38]),
        );
        assert_eq!(
            consume(liss_int_new(KIND_INT, 1, novem.as_ptr(), 38)),
            (true, vec![9; 38]),
        );
        let maximus = liss_int_new(KIND_INT, 0, novem.as_ptr(), 38);
        assert_eq!(consume(liss_int_sub(ni(0), maximus)), (true, vec![9; 38]));
        assert_eq!(liss_live_count(), 0);
    });
}

#[test]
fn fractions_share_the_integer_engine_and_differ_only_where_they_must() {
    solus(|| unsafe {
        assert_eq!(liss_live_count(), 0);
        // add/sub/neg/cmp are the integer operations outright.
        let sum = liss_int_add(numerus(KIND_FRAC_UNCIA, 12), numerus(KIND_FRAC_UNCIA, 4));
        assert_eq!(
            (*sum).h.kind,
            KIND_FRAC_UNCIA,
            "arithmetic carries the operand's kind rather than stamping `ni`",
        );
        assert_eq!(consume(sum), (false, vec![6, 1]));

        // mul rescales: (16/12)*(6/12) back in grain units is trunc(96/12) = 8.
        assert_eq!(
            consume(liss_frac_mul(
                numerus(KIND_FRAC_UNCIA, 16),
                numerus(KIND_FRAC_UNCIA, 6),
                12,
            )),
            (false, vec![8]),
        );
        // div rescales the other way: trunc(288*288/2016) = 41 scripula.
        assert_eq!(
            consume(liss_frac_div(
                numerus(KIND_FRAC_SCRIPULUM, 288),
                numerus(KIND_FRAC_SCRIPULUM, 2016),
                288,
                std::ptr::null(),
                0,
            )),
            (false, vec![1, 4]),
        );
        assert_eq!(liss_live_count(), 0);
    });
}

#[test]
fn text_concatenates_compares_and_frees() {
    solus(|| unsafe {
        assert_eq!(liss_live_count(), 0);
        let joined = liss_text_concat(
            liss_text_new(b"sal".as_ptr(), 3),
            liss_text_new(b"ve".as_ptr(), 2),
        );
        assert_eq!(verba(joined), "salve");
        assert_eq!((*joined).h.kind, KIND_TEXT);
        liss_text_free(joined);

        // Byte order over the UTF-8 payload, which is `str`'s own `Ord`.
        assert_eq!(
            liss_text_cmp(
                liss_text_new("apple".as_ptr(), 5),
                liss_text_new("banana".as_ptr(), 6)
            ),
            -1,
        );
        assert_eq!(
            liss_text_cmp(
                liss_text_new("ave".as_ptr(), 3),
                liss_text_new("ave".as_ptr(), 3)
            ),
            0,
        );

        // Multi-byte text survives the round trip byte for byte.
        let serpens = "Lingua Serpentis \u{1F40D}";
        let s = liss_text_new(serpens.as_ptr(), serpens.len() as u64);
        assert_eq!((*s).len, serpens.len() as u64);
        assert_eq!(verba(s), serpens);
        liss_text_free(s);

        // The empty string is one allocation like any other.
        let vacuum = liss_text_new(std::ptr::null(), 0);
        assert_eq!((*vacuum).len, 0);
        liss_text_free(vacuum);
        assert_eq!(liss_live_count(), 0);
    });
}

#[test]
fn list_indexing_is_one_based() {
    solus(|| unsafe {
        assert_eq!(liss_live_count(), 0);
        let build = || {
            let xs = liss_list_new(u64::from(KIND_INT), 3);
            pone(xs, 0, ni(10) as u64);
            pone(xs, 1, ni(20) as u64);
            pone(xs, 2, ni(30) as u64);
            xs
        };
        for (i, expectatum) in [(1i64, vec![0u8, 1]), (2, vec![0, 2]), (3, vec![0, 3])] {
            let got = liss_list_get(build(), ni(i), std::ptr::null(), 0) as *mut LissInt;
            assert_eq!(consume(got), (false, expectatum), "xs[{i}]");
        }
        assert_eq!(liss_live_count(), 0);
    });
}

#[test]
fn list_equality_is_structural_and_recursive() {
    solus(|| unsafe {
        assert_eq!(liss_live_count(), 0);
        let pair = |a: i64, b: i64| {
            let xs = liss_list_new(u64::from(KIND_INT), 2);
            pone(xs, 0, ni(a) as u64);
            pone(xs, 1, ni(b) as u64);
            xs
        };
        assert_eq!(liss_list_eq(pair(1, 2), pair(1, 2)), 1);
        assert_eq!(liss_list_eq(pair(1, 2), pair(1, 3)), 0);
        assert_eq!(
            liss_list_eq(pair(1, 2), liss_list_new(u64::from(KIND_INT), 0)),
            0,
            "different lengths are never equal",
        );

        let nest = |a: i64, b: i64| {
            let m = liss_list_new(u64::from(KIND_INT + GRADUS), 1);
            pone(m, 0, pair(a, b) as u64);
            m
        };
        assert_eq!(liss_list_eq(nest(1, 2), nest(1, 2)), 1);
        assert_eq!(liss_list_eq(nest(1, 2), nest(2, 1)), 0);
        assert_eq!(liss_live_count(), 0);
    });
}

#[test]
fn an_empty_list_is_one_allocation_with_a_length() {
    solus(|| unsafe {
        assert_eq!(liss_live_count(), 0);
        let xs = liss_list_new(u64::from(KIND_INT), 0);
        assert_eq!(liss_live_count(), 1);
        assert_eq!((*xs).h.kind, KIND_INT + GRADUS);
        assert_eq!(consume(liss_list_len(xs)), (false, vec![]));
        assert_eq!(liss_live_count(), 0);
    });
}

fn in_subprocesso(scaena: &str) -> Output {
    Command::new(std::env::current_exe().expect("the test binary knows its own path"))
        .args(["--exact", "scaena", "--nocapture", "--test-threads=1"])
        .env("LISS_ABI_SCAENA", scaena)
        .output()
        .expect("the test binary re-executes itself")
}

fn erratum(scaena: &str) -> (Option<i32>, String) {
    let out = in_subprocesso(scaena);
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Ordinary runs find no `LISS_ABI_SCAENA` and pass immediately.
#[test]
fn scaena() {
    let Ok(nomen) = std::env::var("LISS_ABI_SCAENA") else {
        return;
    };
    let scida = c"principale.liss".as_ptr();
    unsafe {
        match nomen.as_str() {
            "divisio" => {
                liss_int_div(ni(1), ni(0), scida, 3);
            }
            "divisio-sine-scida" => {
                liss_int_div(ni(1), ni(0), std::ptr::null(), 7);
            }
            "modulus" => {
                liss_int_mod(ni(1), ni(0), scida, 3);
            }
            "frac-divisio" => {
                liss_frac_div(
                    numerus(KIND_FRAC_UNCIA, 12),
                    numerus(KIND_FRAC_UNCIA, 0),
                    12,
                    scida,
                    3,
                );
            }
            "index-supra" => {
                let xs = liss_list_new(u64::from(KIND_INT), 2);
                pone(xs, 0, ni(1) as u64);
                pone(xs, 1, ni(2) as u64);
                liss_list_get(xs, ni(10), scida, 3);
            }
            "index-infra" => {
                let xs = liss_list_new(u64::from(KIND_INT), 2);
                pone(xs, 0, ni(1) as u64);
                pone(xs, 1, ni(2) as u64);
                liss_list_get(xs, ni(0), scida, 3);
            }
            "index-vacuus" => {
                liss_list_get(liss_list_new(u64::from(KIND_INT), 0), ni(1), scida, 3);
            }
            "index-immanis" => {
                // A subscript far past `usize`, which this language can spell.
                let xs = liss_list_new(u64::from(KIND_INT), 0);
                let magnus = liss_int_new(KIND_INT, 0, [9u8; 38].as_ptr(), 38);
                liss_list_get(xs, magnus, scida, 3);
            }
            "index-scriptio" => {
                let xs = liss_list_new(u64::from(KIND_INT), 1);
                pone(xs, 0, ni(1) as u64);
                liss_list_put(xs, ni(2), ni(9) as u64, scida, 3);
            }
            "digitus-corruptus" => {
                // A digit above 9: `Roman::from_digits`' `None` arm.
                let p = ni(1);
                *p.cast::<u8>().add(ONUS) = 12;
                liss_print_int(p);
            }
            "duplex-liberatio" => {
                let p = ni(1);
                liss_int_free(p);
                liss_int_free(p);
            }
            "usus-post-liberationem" => {
                // Clearing the magic before the free is what makes this deterministic.
                let p = ni(1);
                liss_int_free(p);
                liss_int_add(p, ni(2));
            }
            "numerus-pro-serie" => {
                // A `LissInt` handed to a list entry point: only the depth of the kind objects.
                liss_list_free(ni(1).cast());
            }
            "genus-alienum" => {
                // A `LissText` handed to an integer entry point: only the kind objects.
                let t = liss_text_new(b"salve".as_ptr(), 5);
                liss_print_int(t.cast());
            }
            "granum-alienum" => {
                liss_frac_mul(numerus(KIND_FRAC_UNCIA, 1), numerus(KIND_FRAC_UNCIA, 1), 7);
            }
            "memoria-non-liberata" => {
                let _fugit = ni(1);
                liss_exit(0);
            }
            "exitus-mundus" => {
                let t = liss_text_new(b"salve".as_ptr(), 5);
                abi::scriptio::liss_print_text(t);
                liss_exit(0);
            }
            "fistula-erratum" => {
                // Both descriptors gone, output pending so the flush reaches a `write`.
                // `liss_custos_institue` sets `SIG_IGN` in a compiled program; say it here
                // rather than lean on `std::rt::init`.
                libc::signal(libc::SIGPIPE, libc::SIG_IGN);
                let mut fistula = [0 as libc::c_int; 2];
                assert_eq!(libc::pipe(fistula.as_mut_ptr()), 0, "pipe");
                libc::close(fistula[0]);
                assert!(libc::dup2(fistula[1], 1) >= 0, "dup2");
                assert!(libc::dup2(fistula[1], 2) >= 0, "dup2");
                liss_print_int(ni(1));
                liss_panic(
                    std::ptr::null(),
                    0,
                    c"res mala".as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                );
            }
            "nuntius-nudus" => {
                liss_panic(
                    std::ptr::null(),
                    0,
                    c"res mala".as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                );
            }
            "memoria-exhausta" => {
                // Rust's own heap, not `allocare`'s: more than any address space holds.
                let forma = std::alloc::Layout::from_size_align(1 << 62, 1).expect("a layout");
                let _ = std::alloc::alloc(forma);
            }
            "numerus-xxxix" => {
                let mut decem = [0u8; 39];
                decem[38] = 1;
                liss_int_new(KIND_INT, 0, decem.as_ptr(), 39);
            }
            "summa-xxxix" => {
                let novem = [9u8; 38];
                liss_int_add(liss_int_new(KIND_INT, 0, novem.as_ptr(), 38), ni(1));
            }
            other => panic!("unknown scenario {other}"),
        }
    }
    unreachable!("every scenario ends the process");
}

#[test]
fn division_and_modulus_by_zero_raise() {
    assert_eq!(
        erratum("divisio"),
        (
            Some(1),
            "erratum: divisio per nullum\n  in 'principale.liss', linea III\n".to_string()
        )
    );
    assert_eq!(
        erratum("modulus"),
        (
            Some(1),
            "erratum: modulus per nullum\n  in 'principale.liss', linea III\n".to_string()
        )
    );
    // The same sentence, one literal at one site, so the catalogue can attribute it.
    assert_eq!(
        erratum("frac-divisio"),
        (
            Some(1),
            "erratum: divisio per nullum\n  in 'principale.liss', linea III\n".to_string()
        )
    );
}

#[test]
fn a_diagnostic_without_a_file_names_only_its_line() {
    // Spec §4.1 lowers `Func::file: None` to a zero-length name at the ABI.
    assert_eq!(
        erratum("divisio-sine-scida"),
        (Some(1), "erratum: divisio per nullum\n  in linea VII\n".to_string())
    );
    assert_eq!(
        erratum("nuntius-nudus"),
        (Some(1), "erratum: res mala\n".to_string())
    );
}

#[test]
fn an_index_outside_the_bounds_raises_with_roman_numerals() {
    // Byte for byte what `conformance/errores/index-{extra,infra,scriptio-extra}-fines`
    // are blessed with.
    assert_eq!(
        erratum("index-supra"),
        (
            Some(1),
            "erratum: index extra fines: X (longitudo II)\n  in 'principale.liss', linea III\n".to_string()
        )
    );
    assert_eq!(
        erratum("index-infra"),
        (
            Some(1),
            "erratum: index extra fines: nullus (longitudo II)\n  in 'principale.liss', linea III\n"
                .to_string()
        )
    );
    assert_eq!(
        erratum("index-scriptio"),
        (
            Some(1),
            "erratum: index extra fines: II (longitudo I)\n  in 'principale.liss', linea III\n".to_string()
        )
    );
    // D-12: an empty list reads `longitudo nullus`, never an Arabic number.
    assert_eq!(
        erratum("index-vacuus"),
        (
            Some(1),
            "erratum: index extra fines: I (longitudo nullus)\n  in 'principale.liss', linea III\n"
                .to_string()
        )
    );
    // A subscript with no `usize` at all takes the same branch as one of zero.
    let (code, stderr) = erratum("index-immanis");
    assert_eq!(code, Some(1));
    assert!(
        stderr.starts_with("erratum: index extra fines: ")
            && stderr.ends_with(" (longitudo nullus)\n  in 'principale.liss', linea III\n"),
        "got {stderr:?}",
    );
}

#[test]
fn corrupt_input_is_a_latin_exit_and_never_a_panic() {
    for scaena in [
        "digitus-corruptus",
        "duplex-liberatio",
        "usus-post-liberationem",
        "genus-alienum",
        "numerus-pro-serie",
        "granum-alienum",
    ] {
        let (code, stderr) = erratum(scaena);
        assert_eq!(code, Some(1), "{scaena} status");
        assert_eq!(
            stderr, "erratum: defectus internus: memoria violata est: verificatio\n",
            "{scaena} stderr",
        );
        // A Rust panic across an `extern "C"` boundary aborts: SIGABRT, exit 134, D-04's shape.
        assert!(!stderr.contains("panicked"), "{scaena} must not be a Rust panic");
    }
}

#[test]
fn an_exhausted_heap_is_a_latin_exit_and_never_an_abort() {
    // D-28: Rust's own allocations, which `allocare` never sees.
    assert_eq!(
        erratum("memoria-exhausta"),
        (Some(1), "erratum: memoria exhausta est\n".to_string())
    );
}

#[test]
fn a_number_of_xxxix_digits_raises() {
    // D-20
    for scaena in ["numerus-xxxix", "summa-xxxix"] {
        assert_eq!(
            erratum(scaena),
            (Some(1), "erratum: numerus nimis magnus est\n".to_string()),
            "{scaena}",
        );
    }
}

#[test]
fn a_closed_pipe_never_turns_a_failure_into_a_success() {
    // The counterpart to `custos.rs`'s `a_closed_pipe_never_decides_the_exit_status`,
    // on the other failing path; nothing can be read back, so the status is the assertion.
    let out = in_subprocesso("fistula-erratum");
    assert_eq!(
        out.status.code(),
        Some(1),
        "`liss_panic` must exit 1 even when neither descriptor accepts a byte",
    );
}

#[test]
fn a_leak_is_exit_one_and_a_clean_exit_is_exit_zero() {
    // Exit 1 and not 70: `judge` treats any status outside {0, 1, 101} as legitimate.
    assert_eq!(
        erratum("memoria-non-liberata"),
        (Some(1), "erratum: memoria non liberata est\n".to_string())
    );

    let out = in_subprocesso("exitus-mundus");
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&out.stderr), "");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("salve\n"), "liss_exit flushes: {stdout:?}");
    assert!(
        !stdout.contains("test result"),
        "the scenario ended the process itself, so libtest never finished: {stdout:?}",
    );
}
