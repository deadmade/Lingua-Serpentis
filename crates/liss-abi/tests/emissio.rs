//! What `scribe` writes: the output buffer, the flush, and the six
//! `liss_print_*` entry points — checked by reading descriptor 1 back through a
//! pipe, so the assertion covers the real path (buffer, release-store, flush,
//! `write(2)`) rather than only the formatters.
//!
//! **This binary holds exactly one `#[test]`, and that is load-bearing.**
//! Redirecting fd 1 redirects it for the whole process, and `libtest` writes
//! its own progress lines (`test foo ... ok`) to that descriptor from the main
//! thread as other tests finish. With more than one test here, those lines land
//! in the middle of a captured value; the failure is intermittent and reads
//! like a formatter bug. One test means `libtest` prints only before it starts
//! and after it ends. A separate binary is what keeps that constraint from
//! infecting the ownership tests in `abi.rs`.

use liss_abi::memoria::liss_live_count;
use liss_abi::numeri::liss_int_new;
use liss_abi::scriptio::{
    liss_flush, liss_print_bool, liss_print_char, liss_print_frac, liss_print_int, liss_print_list,
    liss_print_text,
};
use liss_abi::series::liss_list_new;
use liss_abi::textus::liss_text_new;
use liss_abi::{
    GRADUS, KIND_BOOL, KIND_CHAR, KIND_FRAC_SCRIPULUM, KIND_FRAC_UNCIA, KIND_INT, KIND_TEXT,
    LissInt, LissList, LissText, ONUS,
};

/// Run `f`, flush, and return everything that reached descriptor 1.
fn capta(f: impl FnOnce()) -> String {
    unsafe {
        let mut fistula = [0 as libc::c_int; 2];
        assert_eq!(libc::pipe(fistula.as_mut_ptr()), 0, "pipe");
        let servatus = libc::dup(1);
        assert!(servatus >= 0, "dup");
        assert!(libc::dup2(fistula[1], 1) >= 0, "dup2");
        libc::close(fistula[1]);

        f();
        liss_flush();

        assert!(libc::dup2(servatus, 1) >= 0, "restore");
        libc::close(servatus);

        let mut out = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = libc::read(fistula[0], buf.as_mut_ptr().cast(), buf.len());
            if n <= 0 {
                break;
            }
            out.extend_from_slice(&buf[..n as usize]);
        }
        libc::close(fistula[0]);
        String::from_utf8(out).expect("the formatters only ever write utf-8")
    }
}

/// Render one value and return what it printed, minus the trailing newline
/// `scribe` always appends.
fn pinge(f: impl FnOnce()) -> String {
    capta(f)
        .strip_suffix('\n')
        .expect("every liss_print_* ends its line")
        .to_string()
}

unsafe fn pone(p: *mut LissList, i: usize, verbum: u64) {
    unsafe { *p.cast::<u8>().add(ONUS).cast::<u64>().add(i) = verbum };
}

unsafe fn ni(n: i64) -> *mut LissInt {
    let mut magnitudo = n.unsigned_abs();
    let mut digiti = Vec::new();
    while magnitudo > 0 {
        digiti.push((magnitudo % 10) as u8);
        magnitudo /= 10;
    }
    unsafe { liss_int_new(KIND_INT, u64::from(n < 0), digiti.as_ptr(), digiti.len() as u64) }
}

// ------------------------------------------------------------ golden vectors

/// `tests/data/scriptio-aurea.txt`, captured from the interpreter while both
/// implementations still existed (spec §10.4). `liss-runtime`'s own
/// `golden.rs` skips the `List` rows because `scriptio` has no list formatter;
/// those rows are precisely what this crate adds, so they are checked here.
const AUREA: &str = include_str!("../../liss-runtime/tests/data/scriptio-aurea.txt");

/// The `Debug` shape of the interpreter's old `Value`, parsed back into
/// something the ABI can build.
enum Nodus {
    Bool(bool),
    Char(char),
    Numerus {
        kind: u32,
        neg: bool,
        digiti: Vec<u8>,
    },
    Textus(String),
    Series(Vec<Nodus>),
}

fn genus(n: &Nodus) -> u32 {
    match n {
        Nodus::Bool(_) => KIND_BOOL,
        Nodus::Char(_) => KIND_CHAR,
        Nodus::Numerus { kind, .. } => *kind,
        Nodus::Textus(_) => KIND_TEXT,
        // An empty list has no element to ask; every element kind prints `[]`.
        Nodus::Series(items) => items.first().map_or(KIND_INT, genus) + GRADUS,
    }
}

unsafe fn aedifica(n: &Nodus) -> u64 {
    match n {
        Nodus::Bool(b) => u64::from(*b),
        Nodus::Char(c) => *c as u64,
        Nodus::Numerus { kind, neg, digiti } => unsafe {
            liss_int_new(*kind, u64::from(*neg), digiti.as_ptr(), digiti.len() as u64) as u64
        },
        Nodus::Textus(s) => unsafe { liss_text_new(s.as_ptr(), s.len() as u64) as u64 },
        Nodus::Series(items) => unsafe {
            let p = liss_list_new(u64::from(genus(n) - GRADUS), items.len() as u64);
            for (i, item) in items.iter().enumerate() {
                pone(p, i, aedifica(item));
            }
            p as u64
        },
    }
}

fn lege_roman(s: &str) -> (bool, Vec<u8>, &str) {
    let r = s.strip_prefix("Roman { negative: ").expect("Roman opens");
    let (neg, r) = match r.strip_prefix("true") {
        Some(x) => (true, x),
        None => (false, r.strip_prefix("false").expect("a bool")),
    };
    let r = r.strip_prefix(", tiers: [").expect("tiers");
    let finis = r.find(']').expect("tiers close");
    let digiti = r[..finis]
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.parse().expect("a digit"))
        .collect();
    (
        neg,
        digiti,
        r[finis + 1..].strip_prefix(" }").expect("Roman closes"),
    )
}

fn lege_nodum(s: &str) -> (Nodus, &str) {
    if let Some(mut r) = s.strip_prefix("List([") {
        let mut items = Vec::new();
        if let Some(x) = r.strip_prefix("])") {
            return (Nodus::Series(items), x);
        }
        loop {
            let (n, reliquum) = lege_nodum(r);
            items.push(n);
            r = reliquum;
            if let Some(x) = r.strip_prefix(", ") {
                r = x;
                continue;
            }
            return (
                Nodus::Series(items),
                r.strip_prefix("])").expect("list closes"),
            );
        }
    }
    for (praefixum, kind) in [
        ("Int(", KIND_INT),
        ("Float(", KIND_FRAC_UNCIA),
        ("Double(", KIND_FRAC_SCRIPULUM),
    ] {
        if let Some(r) = s.strip_prefix(praefixum) {
            let (neg, digiti, r) = lege_roman(r);
            return (
                Nodus::Numerus { kind, neg, digiti },
                r.strip_prefix(')').expect("number closes"),
            );
        }
    }
    if let Some(r) = s.strip_prefix("Text(\"") {
        let finis = r.find('"').expect("text closes");
        return (
            Nodus::Textus(r[..finis].to_string()),
            r[finis + 1..].strip_prefix(')').expect("Text closes"),
        );
    }
    if let Some(r) = s.strip_prefix("Char('") {
        let c = r.chars().next().expect("one char");
        return (
            Nodus::Char(c),
            r[c.len_utf8()..].strip_prefix("')").expect("Char closes"),
        );
    }
    if let Some(r) = s.strip_prefix("Bool(true)") {
        return (Nodus::Bool(true), r);
    }
    if let Some(r) = s.strip_prefix("Bool(false)") {
        return (Nodus::Bool(false), r);
    }
    panic!("unparsed golden payload: {s}");
}

/// Render one built value through whichever entry point its kind names.
unsafe fn pinge_verbum(verbum: u64, kind: u32) -> String {
    pinge(|| unsafe {
        match kind {
            KIND_BOOL => liss_print_bool(verbum as i8),
            KIND_CHAR => liss_print_char(verbum as i32),
            KIND_INT => liss_print_int(verbum as *mut LissInt),
            KIND_FRAC_UNCIA => liss_print_frac(verbum as *mut LissInt, 12),
            KIND_FRAC_SCRIPULUM => liss_print_frac(verbum as *mut LissInt, 288),
            KIND_TEXT => liss_print_text(verbum as *mut LissText),
            _ => liss_print_list(verbum as *mut LissList),
        }
    })
}

#[test]
fn scribe_writes_what_the_interpreter_wrote() {
    unsafe {
        assert_eq!(liss_live_count(), 0);

        // ---- the golden vectors, including the list rows `golden.rs` skips.
        let mut probata = 0usize;
        let mut seriei = 0usize;
        for (i, linea) in AUREA.lines().enumerate() {
            if linea.is_empty() {
                continue;
            }
            let mut campi = linea.splitn(3, '\t');
            let _kind = campi.next().expect("kind");
            let payload = campi.next().expect("payload");
            let expectatum = campi.next().expect("expected");

            let (nodus, reliquum) = lege_nodum(payload);
            assert!(reliquum.is_empty(), "line {}: trailing {reliquum:?}", i + 1);

            let kind = genus(&nodus);
            let got = pinge_verbum(aedifica(&nodus), kind);
            assert_eq!(got, expectatum, "line {} ({payload})", i + 1);
            assert_eq!(liss_live_count(), 0, "line {} leaked", i + 1);
            probata += 1;
            if kind >= GRADUS {
                seriei += 1;
            }
        }
        assert!(probata >= 30, "expected at least 30 golden rows, got {probata}");
        assert!(
            seriei >= 3,
            "the list rows are what this crate adds over `golden.rs`; got {seriei}",
        );

        // ---- scalars, which own nothing and free nothing.
        assert_eq!(pinge(|| liss_print_bool(1)), "verus");
        assert_eq!(pinge(|| liss_print_bool(0)), "falsus");
        assert_eq!(pinge(|| liss_print_char('x' as i32)), "x");
        assert_eq!(pinge(|| liss_print_char('\u{1F40D}' as i32)), "\u{1F40D}");
        assert_eq!(liss_live_count(), 0);

        // ---- a list nested three deep, which no golden vector reaches.
        let intima = liss_list_new(u64::from(KIND_INT), 2);
        pone(intima, 0, ni(1) as u64);
        pone(intima, 1, ni(2) as u64);
        let media = liss_list_new(u64::from(KIND_INT + GRADUS), 1);
        pone(media, 0, intima as u64);
        let summa = liss_list_new(u64::from(KIND_INT + 2 * GRADUS), 2);
        pone(summa, 0, media as u64);
        pone(summa, 1, liss_list_new(u64::from(KIND_INT + GRADUS), 0) as u64);
        assert_eq!(pinge(|| liss_print_list(summa)), "[[[I, II]], []]");
        assert_eq!(liss_live_count(), 0);

        // ---- a list of each remaining element kind.
        let veri = liss_list_new(u64::from(KIND_BOOL), 2);
        pone(veri, 0, 1);
        pone(veri, 1, 0);
        assert_eq!(pinge(|| liss_print_list(veri)), "[verus, falsus]");

        let litterae = liss_list_new(u64::from(KIND_CHAR), 2);
        pone(litterae, 0, 'a' as u64);
        pone(litterae, 1, '\u{1F40D}' as u64);
        assert_eq!(pinge(|| liss_print_list(litterae)), "[a, \u{1F40D}]");

        let voces = liss_list_new(u64::from(KIND_TEXT), 2);
        pone(voces, 0, liss_text_new(b"salve".as_ptr(), 5) as u64);
        pone(voces, 1, liss_text_new(std::ptr::null(), 0) as u64);
        assert_eq!(pinge(|| liss_print_list(voces)), "[salve, ]");

        let scripula = liss_list_new(u64::from(KIND_FRAC_SCRIPULUM), 2);
        pone(scripula, 0, liss_int_new(KIND_FRAC_SCRIPULUM, 0, [2u8, 1].as_ptr(), 2) as u64);
        pone(scripula, 1, liss_int_new(KIND_FRAC_SCRIPULUM, 1, [2u8].as_ptr(), 1) as u64);
        assert_eq!(
            pinge(|| liss_print_list(scripula)),
            "[semuncia, -dimidia sextula]",
            "a list reads its grain from `ekind`, not from a tag on each element",
        );
        assert_eq!(liss_live_count(), 0);

        // ---- more output than the 8192-byte buffer holds. Each line is four
        // bytes, so 4000 of them force the overflow flush several times over,
        // and nothing may be lost, duplicated or reordered.
        let scriptum = capta(|| {
            for _ in 0..4000 {
                liss_print_int(ni(3000));
            }
        });
        assert_eq!(scriptum.len(), 4 * 4000);
        assert_eq!(scriptum.matches("MMM\n").count(), 4000);
        assert!(scriptum.chars().all(|c| c == 'M' || c == '\n'));

        // ---- a single value larger than the whole buffer, which `scribe`
        // must split across flushes rather than truncate.
        let longum = "\u{1F40D}".repeat(5000);
        let scriptum = pinge(|| liss_print_text(liss_text_new(longum.as_ptr(), longum.len() as u64)));
        assert_eq!(scriptum, longum);

        assert_eq!(liss_live_count(), 0);
    }
}
