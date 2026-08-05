# Defect Register Remediation — Implementation Plan

**Goal:** Fix 22 of the 25 defects in `docs/defectus/2026-08-03-index.md`, deferring the three that are language changes rather than bugs.

**Spec:** `docs/superpowers/specs/2026-08-03-conformance-and-robustness-suite-design.md` (§5 the robustness invariant, §7 the catalogue ratchet, §8.1 the defect table).

**Register:** `docs/defectus/2026-08-03-index.md` — authoritative for severity, repro, observed, expected and `file:line` cause.

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:subagent-driven-development` or `superpowers:executing-plans`. Follow the house step convention from `docs/superpowers/plans/2026-08-03-conformance-and-robustness-suite.md`: every code task opens with **Step 1: Write the failing tests**, **Step 2: Run them and verify they fail** (with the expected failure text stated), then implement, then verify, then commit.
>
> **First action:** copy this file to `docs/superpowers/plans/2026-08-05-defect-register-remediation.md` and commit it (`docs(plan): remediate the defect register's twenty-two fixable defects`). The plan belongs in the repo alongside its siblings.

---

## Context

The conformance and robustness suite (completed 2026-08-05, all 13 tasks) was built under a strict **no behaviour changes** rule. Its output is a 25-entry defect register plus a corpus that *pins today's wrong behaviour* so `cargo test` stays green. Verified: `git diff 9644ffd --stat -- crates/*/src/` shows only the Task 1 `render()` extraction — the lexer, parser, checker, interpreter, `roman.rs`, `program.rs` and `latinitas` are byte-untouched since the register opened. **Nothing is fixed.**

That was the correct call for the suite. It is not a stable resting place: the language currently has an unsound type system (D-01), diagnostics that point at the wrong line (D-03), two ways to abort the process (D-04, D-05), a runtime error that throws away output the program provably produced (D-08), and English text in a language whose one rule is that everything the user sees is Latin (D-11, D-22).

Each defect's acceptance criterion is already written and executable: its `conformance/defectus/` fixture flips red. This plan turns 22 of those red flips into landed fixes.

**Deferred, and why** — these are language changes the suite design's §2 lists as Non-Goals; each needs its own brainstorm and spec first:

| ID | why deferred |
|---|---|
| **D-13** | Validating Roman numeral form (`°IIX` currently means 10) is a breaking change to every non-canonical literal. Needs a spec deciding canonical form, subtractive-pair rules and repetition limits. |
| **D-19** | 62 unit tests drive the laxer legacy `parser::parse` / `checker::check` / `interp::execute` entry points. Removing them is a test rewrite, not a fix. |
| **D-20** | `x = x * x` in a loop never terminates because `Roman` is arbitrary-precision. A magnitude cap is a new language limit with a new diagnostic. |

Consequence: `RECORDED_RULES`' `"terminates"` entry **cannot be deleted** even after D-23 lands, because D-20 still needs it.

---

## Global Constraints

- **Build:** `nix develop -c cargo test --workspace`. Every task leaves it green.
- **Zero runtime dependencies.** No `stacker`, no `libc`, no `tempfile`. Dev-dependencies stay at `insta`, `libtest-mimic`, `proptest`.
- **Every user-facing string is Latin.** Any newly coined word is validated with `latinitas::is_latin` before landing. No Arabic digits outside quoted spans (`roman-native-integers` §1).
- **Diagnostics are asserted verbatim, never `contains()`.** The two existing carve-outs (OS errno tails; assertions inside `#[ignore]`d tests) both **expire** during this plan — see B8 and B7.
- **`RECORDED_RULES` (`tests/support/diag.rs:72`) may only shrink.** Never weaken a rule to get green. The array length is part of the type, so each deletion edits it.
- **Commit safety:** the user has ten unrelated entries staged (`.envrc`, `.gitignore`, `.idea/*` deletions, `flake.nix`, `flake.lock`). Use `git add <path>` only for untracked paths, then `git commit --only <paths> -m "…"`. Never `git add -A` / `git add .`. All ten staged entries must still show afterwards.
- **Commit style:** conventional commits, Latin-flavoured scopes (`fix(lexer)`, `fix(interp)`, `test(conformance)`, `docs(defectus)`). Subject is a lowercase prose claim, no trailing period. Trailer `Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>`.

### Fixing a defect — the standing procedure

1. Write the test that fails (usually: watch the existing `defectus/` fixture go red).
2. Implement.
3. **Move the fixture:** `git mv conformance/defectus/D-NN-<slug> conformance/errores/<slug>` (or `currunt/`), rewrite its expectation files, and rewrite `NOTA.md` — the "pins the wrong behaviour" paragraph becomes a "this is now correct, here is what it used to do" paragraph.
4. If the fix closes the last violation of a `RECORDED_RULES` entry, delete it and shrink `[&str; N]`.
5. If a `robustness.rs` pathology no longer breaks its recorded rule, flip its `Option<&str>` to `None`. `every_curated_pathology_matches_its_recorded_verdict` is **bidirectional** and will fail with *"expected to break the rule, but it now passes"* if you forget.
6. Re-bless `catalogus__catalogus.snap` (`cargo insta review`) and **read the diff line by line** — `LISS_BLESS=1` and `insta accept` will happily absorb a real regression.
7. Update `docs/defectus/2026-08-03-index.md`: mark the entry fixed, with the commit.

### Two cross-cutting effects on every task

- `catalogus__catalogus.snap` is keyed by `file:line`. **Any** line shift in `crates/lingua-serpentis/src/*.rs` re-blesses it. This makes the snapshot a guaranteed conflict between parallel agents — see Ordering.
- `tests/support/mutate.rs::seed_corpus` seeds the deterministic fuzzer from every `.liss` under `examples/` and `conformance/`. Adding, renaming or editing a fixture `.liss` shifts the whole mutation stream. Blessing a `nuntii` file does not.

---

## Ordering

**Track A runs in parallel with everything.** It touches `crates/latinitas/` only, which contributes zero catalogue sites (none of `catalogus.rs`'s eight `MARKERS` appears in that crate). **Track B is strictly serial** — every task re-blesses the same snapshot.

| # | task | defects | rule deleted | snapshot |
|---|---|---|---|---|
| **A1** | latinitas: bounded analysis + supplement | D-23, D-18 | — | none |
| **B1** | de-duplicate diagnostics; delete dead `roman.rs` arms | D-24 (part), D-25 | — | re-bless |
| **B2** | Latin token names | D-02 | `no Rust type names` | re-bless |
| **B3** | Roman `longitudo` | D-12 | `no Arabic digits` | re-bless |
| **B4** | lexer: line counting, separator trim, digit hint | D-03, D-14, D-15 | — | re-bless |
| **B5** | duplicate params; `abrumpe` outside a loop | D-16, D-07 | — | re-bless |
| **B6** | partial output, surviving warnings, exit-status warning | D-08, D-09, D-10 | — | re-bless |
| **B7** | parser and interpreter depth limits | D-04, D-05 | `no abort` | re-bless |
| **B8** | Latin I/O errors; hidden directories | D-11, D-22, D-17 | `Latin only` | re-bless |
| **B9** | broken pipe | D-06 | `no panic` | re-bless |
| **B10** | `div_rem` returns `Option` | D-21 | — | re-bless |
| **B11** | reword the duplicated diagnostics; new fixtures | D-24 (rest) | — | re-bless |
| **B12** | definite-return analysis | D-01 | — | re-bless |
| **B13** | reconcile register and spec; final verification | — | — | — |

**Must land together** (a rule covers more than one defect, and `RECORDED_RULES` is keyed by rule, not by ID): {D-04, D-05}; {D-11, D-22}. Also {D-08, D-09, D-10} — they share one plumbing change — and {D-23, D-18}, which both edit `latinitas::tables()`'s initializer.

**Why B1 first:** every later task edits the same functions. De-duplicating first means B3 changes one literal instead of two, B6's `interp.rs` edits land on already-collapsed arms, and the line churn happens once.

**Why B12 last:** fixing D-01 makes four `interp.rs` diagnostics provably dead (`:136` `subscriptio invalida`, `:288` `condicio non est veritas`, `:295` `operatio binaria invalida`, `:358` `comparatio invalida` — D-01's own Note says it *promotes* them to reachable). Those register entries should be written after B11 has finalised all wording.

**Final `RECORDED_RULES`:** `["terminates"]` — `[&str; 1]`, held open by the deferred D-20.

---

## Track A

### A1 — D-23 (quadratic identifier checker) + D-18 (`summare`)

**Files:** `crates/latinitas/src/{model.rs,analyze.rs,whitaker.rs,lib.rs}`, new `crates/latinitas/data/supplement.txt`, `crates/latinitas/data/NOTICE`, `conformance/currunt/exemplum-calc/nuntii` (delete).

**D-23.** `analyze.rs:31`'s `for i in 0..=w.len()` hashes a `&w[..i]` slice per iteration — O(n²). Bound the loop on **both** ends: a split `stem = w[..i]` / `ending = w[i..]` can only match when `stem.len() <= max_stem` **and** `ending.len() <= max_ending`.

```rust
let lo = w.len().saturating_sub(tables.max_ending);
let hi = w.len().min(tables.max_stem);
for i in lo..=hi { /* body unchanged */ }
```

Measured against the shipped data: `max_stem = 18` (`comprehensibilitas`), `max_ending = 7` (`abamini`). For a 100,000-char name `lo = 99_993 > hi = 18` → **zero iterations**; what remains is one `normalize` allocation and one `whole` hash. Register says 24s today; this is O(1) in name length, not merely "near-linear" as the register's Expected understates.

Soundness: for `i < lo` no ending is that long, so `ends.contains` was already false; for `i > hi` no stem is that long, so `stems.get` was already `None`. For `n <= 7` `lo = 0` and for `n <= 18` `hi = n`, so short words take the *identical* range — which is what keeps `analyze.rs:115::matches_a_bare_stem_via_the_empty_ending` (the test pinning the inclusive `0..=len`) passing.

**The trap:** `Tables` gains `max_stem`/`max_ending`, but **drop `#[derive(Default)]` and hand-write `Default` with `usize::MAX`**, not `0`. `analyze.rs:72::fixture()` hand-builds a `Tables`; a `0` default would silently break it and every future hand-built table. With `usize::MAX` an un-narrowed table is merely slow, never wrong. Add `Tables::narrow_bounds(&mut self)`; call it from `latinitas/src/lib.rs`'s `tables()` initializer, **not** inside `build_tables` — 20 `whitaker.rs` tests call `build_tables` directly and must keep compiling.

**D-18.** Add `crates/latinitas/data/supplement.txt` containing **`summare` and nothing else**, under a header stating the policy: *add a word only when a shipped example or program actually fails*. Speculative additions weaken the checker for no measured benefit. Record in `data/NOTICE` that this one file is ours, not Whitaker's.

Do **not** change `build_tables`'s arity. Add `pub fn add_supplement(t: &mut Tables, text: &str)` filling `Tables::whole`, and compose:

```rust
TABLES.get_or_init(|| {
    let mut t = whitaker::build_tables(DICTLINE, INFLECTS, UNIQUES);
    whitaker::add_supplement(&mut t, SUPPLEMENTUM);  // D-18
    t.narrow_bounds();                               // D-23, last
    t
})
```

**Tests:** a differential test in `analyze.rs` (build the fixture twice, `narrow_bounds()` one copy, assert `analyze` agrees on every existing positive and negative plus a 10,000-char word); a ratchet in `whitaker.rs` asserting `max_stem == 18 && max_ending == 7` against the real data — that assertion is what catches a future dictionary or supplement change silently widening the bound.

**What breaks:** `robustness.rs::pathologies()` — `nomen longissimum` flips `Some("terminates")` → `None`, and its long root-cause comment is rewritten to record the fix. `conformance/currunt/exemplum-calc/nuntii` is **deleted** (the file's absence is how an empty stderr is expressed). `"terminates"` **stays** in `RECORDED_RULES` (D-20). No catalogue re-bless.

---

## Track B

### B1 — D-24 (de-duplication) + D-25 (dead arms)

**Files:** `crates/lingua-serpentis/src/{checker.rs,interp.rs,roman.rs}`.

Pure refactor, **no wording changes** — every existing `nuntii` file stays byte-identical. This is the task that makes the rest cheap.

| group | change | result |
|---|---|---|
| `checker.rs:90`, `:366`, `:427` `variabilis ignota` | `fn lookup_or_err(&self, name, line) -> LResult<Type>` | 3 sites → 1, `fixtured` |
| `interp.rs:51`, `:67`, `:215` `variabilis ignota` | one `Interp` helper | 3 → 1 |
| `interp.rs:130`, `:206` `index extra fines` | `fn index_error(idx, len, line) -> LinguaError` | 2 → 1; **D-12 then has one site, not two** |
| `interp.rs:317`, `:321`, `:325` `divisio per nullum` | a `let by_zero = \|\| …` closure, mirroring the existing `let bad` at `:295` | 3 → 1; all three `divisio-per-nullum*` fixtures keep their `nuntii` verbatim |
| `checker.rs:458` vs `:475` nested-function guard | `check_block` delegates to `check_block_inner` (`push; let r = …; pop; r`) — behaviour-preserving, today's `check_block` also pops before returning the error | 2 → 1; the cleanest win in the sweep |
| `checker.rs:524` vs `:548` `nomen reservatum` | extract `fn reserved_name_error(name, line) -> Option<LinguaError>` used by both `check` and `collect_sigs` | 2 → 1; kills D-24's "two checkers can drift apart" half without needing D-19 |

**D-25** (`roman.rs`): delete `:248` `fractio sine numeratore` — `str::splitn` always yields a first item, so the arm is *impossible*, not merely unreached. Delete `:253` `denominator est nullus` — `convert_single` adds its final symbol unconditionally and only ever subtracts a strictly smaller one, so its value is ≥ 1 and `convert_int_roman` is a sum of products of such; no accepted numeral evaluates to zero. **Keep `:238` `numerus sine '°'`** with a doc comment marking it an API-only guard (`parse_roman` is `pub`; the lexer guarantees the prefix by construction) and a unit test calling `parse_roman("V")` directly.

**Correction to make in the register:** D-24 claims `checker.rs:414` `'redde' extra functionem` is unreachable. It is unreachable *from the program pipeline* — but `checker::check` (`:519`) walks top-level statements with `current_ret == None`, so a top-level `redde` reaches it through the legacy API. Since **D-19 is deferred and that API stays**, `:414` stays too. Rewrite its register entry to say so.

**What breaks:** catalogue re-bless. Sites moving `ambiguous` → `fixtured`: `checker.rs:90`, `:175`, `:195`, `:355`, `:475`, `:548`, `:574`; `interp.rs:130`, `:317`. Roughly ten duplicate sites disappear and three `roman.rs` sites go. Re-bless and read the header rather than predicting it.

---

### B2 — D-02: Latin token names

**Files:** `crates/lingua-serpentis/src/token.rs`, `parser.rs:28`.

Add `impl TokenKind { pub fn name(&self) -> &'static str }` — **exhaustive match, no catch-all**. Exhaustiveness is the point: the register's complaint is that renaming a variant silently changes user-facing text, and a catch-all reintroduces exactly that for every variant added later. With an exhaustive match, adding a variant is a compile error.

Punctuation renders as its **glyph** (`";"`, `"{"`, `")"`) — the user's mistake is a missing character, so showing them `';'` is actionable where `punctum et virgula` is a riddle. Keywords render as the keyword the user would have written (`si`, `aut`, `munus`, `redde`, `ni`, `voc`…). Value-carrying kinds get a Latin category noun (`Int(_) => "numerus"`, `Text(_) => "chorda"`, `Ident(_) => "nomen"`) — never passed to `eat`, but named to keep the match honest.

Not a `Display` impl: `Display` on a token is ambiguous (source text? description?) and invites misuse.

`parser.rs:28` becomes `Err(self.err(&format!("expectatum: '{}'", kind.name())))`. **The single quotes are load-bearing** — `diag.rs::without_quoted` strips quoted spans, so what the rules see is just `expectatum: `.

**What breaks:** `errores/expectatum-semicolon/nuntii` → `expectatum: ';'`; `errores/expectatum-rparen/nuntii` → `expectatum: ')'`. `robustness.rs::pathologies()` — `chorda multilinea` flips `Some("no Rust type names")` → `None`. Delete `"no Rust type names"` from `RECORDED_RULES` — `parser.rs:28` is the **only** `{:?}` in production sources (the other eight are `panic!`s inside `#[cfg(test)]`). Run one fuzz soak (`LISS_FUZZ_CASES=5000`, two seeds) before deleting, since mutation is a parse-error factory and this is precisely the rule it trips. `token.rs` adds **no** catalogue sites (no `MARKERS` appear in it).

---

### B3 — D-12: the Arabic `longitudo`

**Files:** `crates/lingua-serpentis/src/interp.rs` (one site, thanks to B1).

`format!("index extra fines: {} (longitudo {})", idx.display(), items.len())` → render the length through `Roman::from_usize(items.len()).display()`.

**What breaks:** `errores/index-extra-fines/nuntii` → `… index extra fines: X (longitudo II)`, and its `NOTA.md` ("`longitudo 2` is Arabic … pinned as-is; see D-12") is rewritten. `robustness.rs::pathologies()` — `index extra fines` flips `Some("no Arabic digits")` → `None`. Delete `"no Arabic digits"` from `RECORDED_RULES`.

---

### B4 — D-03, D-14, D-15: the lexer

**Files:** `crates/lingua-serpentis/src/lexer.rs`.

**D-03 (line numbers after a multi-line string).** `lexer.rs:43-53` walks the string body with `while i < chars.len() && chars[i] != '"' { i += 1; }`, never incrementing `line`, so every token afterwards is understated by one per embedded newline — unbounded drift. Count newlines in the loop, but **capture the opening line first** and push the token with that, so the literal is still attributed to where it starts:

```rust
'"' => {
    let str_line = line;
    i += 1;
    let start = i;
    while i < chars.len() && chars[i] != '"' {
        if chars[i] == '\n' { line += 1; }
        i += 1;
    }
    …
    tokens.push(Token { kind: TokenKind::Text(text), line: str_line });
}
```

**D-14 (`°X/°II` lexes as one numeral).** The register's implied fix — retreat one character at a time until a prefix parses — **is wrong and silently destroys two fixtures**: `errores/fractio-sine-denominatore` is `nr x = °I:I;`, which would retreat `°I:I` → `°I:` → `°I` (parses as 1), leaving `I` to lex as `Ident("I")`; `roman.rs:249` `fractio sine denominatore` would flip to UNCOVERED.

Correct policy: **trim trailing separators only, unconditionally, with no retry loop.**

```rust
while i > start + 1 && matches!(chars[i - 1], '.' | ':' | '/') { i -= 1; }
```

Proof this is exactly "try, fail, retreat over separators": `parse_roman` splits on `:`, then `/`, then `.`, and every leaf goes to `convert_single`, which rejects the empty string (`roman.rs:193` `numerus Romanus vacuus`). A lexeme ending in a separator always produces an empty final leaf, so **no such lexeme can ever parse**. O(k) for k trailing separators; amortized O(1).

| input | greedy lexeme | after trim | result |
|---|---|---|---|
| `°V:I/II` | `°V:I/II` | unchanged | `Frac{11,2}` ✓ |
| `°XXX.M` | `°XXX.M` | unchanged | `Int(30000)` ✓ |
| `°X/°II` | `°X/` | `°X` | `Int(10)`, `Slash`, `Int(2)` → prints `V` ✓ |
| `°V.summa` | `°V.` | `°V` | `Int(5)`, `Dot`, `Ident("summa")` |
| `°I:I` | `°I:I` | unchanged | `fractio sine denominatore` ✓ preserved |
| `°` | `°` | `i > start + 1` blocks the trim | `numerus Romanus vacuus` ✓ preserved |

The `i > start + 1` guard is the whole reason `errores/numerus-romanus-vacuus` survives.

**D-15 (Arabic digits get the catch-all).** Special-case `'0'..='9'` ahead of `lexer.rs:73`'s `_ =>` arm: `numerus Arabicus '{c}' non licet; numeri Romani '°' praeponunt`. The digit sits inside quotes, which `without_quoted` strips, so `no Arabic digits` is satisfied.

**What breaks:** `defectus/D-03-linea-post-chordam` → `errores/linea-post-chordam` with `linea 5`, `NOTA.md` rewritten. `defectus/D-14-numerus-sine-spatio` → `currunt/numerus-sine-spatio`, `emissio` = `V`, `nuntii`/`exitus` deleted. **`errores/numerus-romanus-invalidus`'s program must be rewritten** — it is `ni x = °X/;`, which now lexes as `°X` `/` `;` → `expressio expectata`, a different fixture's message; replace with `ni x = °I:I/II/III;` on line 2, giving `numerus Romanus invalidus: 'II/III'`. Without this, `roman.rs:205` flips to UNCOVERED. `errores/signum-ignotum/nuntii` gets the new digit message. New lexer unit tests beside `dot_inside_roman_literal_still_belongs_to_the_numeral` (`lexer.rs:258`) pinning `°X/°II` → `[Int(10), Slash, Int(2)]` and `°V:I/II` unchanged. These are `.liss` edits, so the fuzz stream shifts.

---

### B5 — D-16 (duplicate parameters) + D-07 (`abrumpe` outside a loop)

**Files:** `crates/lingua-serpentis/src/{parser.rs,checker.rs}`.

**D-16.** The register names three last-wins sites (`parser.rs:305`, `checker.rs:39`, `interp.rs:154`). Reject at the **parser** only: after `params.push((ty, pname))`, error if the name is already present — `parametrum '{}' bis nominatum`. The checker's `declare` and the interpreter's `frame.insert` then never see a duplicate, and adding guards there would manufacture two more unreachable diagnostics, which is the exact disease D-24 and D-25 record.

**D-07.** `checker.rs:449`'s `StmtKind::Break | StmtKind::Continue => Ok(())` accepts unconditionally. Add `loop_depth: usize` to `Checker`; increment around the `While` and `For` body walks; in the Break/Continue arm error when it is zero: `'abrumpe' extra iterationem` / `'continua' extra iterationem`. Leave `interp.rs:166`'s `Flow::Break` drop-on-the-floor alone — after the checker rejects it, it is a defensive fallback for the legacy `execute()` path that D-19 defers removing.

**What breaks:** `defectus/D-16-parametri-gemini` → `errores/parametri-gemini`; `defectus/D-07-abrumpe-extra-iterationem` → `errores/abrumpe-extra-iterationem`. Both need their error off line 1 per the Global Constraints — prepend a comment line, as `errores/fasciculus-non-allatus` already does. Two new catalogue sites, both `fixtured`.

---

### B6 — D-08 + D-09 + D-10: partial output, surviving warnings, exit-status warning

**Files:** `crates/lingua-serpentis/src/{error.rs,lib.rs,interp.rs,main.rs}`, `tests/programs.rs`.

The heaviest task. D-08 and D-09 are one thing: `lib.rs::run_loaded`'s `?` operators throw away data that has already been computed.

**Grow `ProgramError`; do not introduce a new result type.** `ProgramError` has five construction sites; `LinguaError` has 66 struct literals across the front end and is untouchable.

```rust
pub struct ProgramError {
    pub file: Option<String>,
    pub line: Option<usize>,
    pub message: String,
    /// Everything `scribe` printed before the run stopped. Empty for any error
    /// raised before execution began. (D-08)
    pub stdout: String,
    /// Latinity warnings computed before the run stopped. (D-09)
    pub warnings: Vec<Warning>,
}
```

`Display for ProgramError` (`error.rs:56-65`) **does not change** — that is what keeps all 70+ `errores/` `nuntii` files valid.

**Change `run_file_with_warnings`'s error type from `LinguaError` to `ProgramError`.** Free: `ProgramError`'s `(None, Some(l))` Display arm is byte-identical to `LinguaError`'s. Keep `run_with_warnings` and `run` on `LResult` by converting at the boundary, so `programs.rs` needs no signature edits.

`render` drops its generic and becomes concrete, emitting **warnings first, then the error**, matching the `Ok` arm's ordering and D-09's stated Expected:

```rust
Err(e) => {
    let mut stderr = String::new();
    for w in &e.warnings { stderr.push_str(&w.to_string()); stderr.push('\n'); }
    stderr.push_str(&format!("{e}\n"));
    Rendered { stdout: e.stdout, stderr, code: 1 }
}
```

`interp.rs:416` fills `stdout: std::mem::take(&mut it.out)`.

**Move linting before the checker.** This is what stops one problem hiding another:

```rust
let warnings = latinity::lint_program(&parsed);
if let Err(mut e) = checker::check_program(&mut parsed) { e.warnings = warnings; return Err(e); }
match interp::execute_program(&parsed) {
    Ok((output, code, mut ws)) => { ws.splice(..0, warnings); Ok((output, code, ws)) }
    Err(mut e) => { e.warnings = warnings; Err(e) }
}
```

Provably warning-set-preserving: `lint_stmt` matches only on declaration *names*, and the only thing `check_program` mutates is `Expr::FloatLit` nodes via `resolve_floatlit`.

**Warnings survive a checker error too, not just a runtime one.** Cost, measured: 7 `errores/` fixtures gain one `Monitum … nomen 'xs' fortasse non Latinum est` line. Pay it — D-09's own text is "a program with two problems only ever reports one", the register names the `lib.rs:42` checker `?` as part of the cause, and a lint that only appears once the program type-checks is useless. **Do not "fix" the churn by renaming `xs` in those fixtures**: blessing `nuntii` leaves the `.liss` files alone, so the fuzz stream is undisturbed; renaming would perturb it.

**Parse, lex and structural errors carry no warnings** — there is no `ParsedProgram` to lint. Document that in `run_loaded`'s doc comment; it is the one place the "warnings always survive" claim is not true.

**D-10 rides the same channel.** `interp.rs:381`'s `exit_status` reduces mod 256 silently. Change `execute_program` to return `(String, u8, Vec<Warning>)` and emit `Warning { file: None, line: None, message }` when the returned `ni` is outside `0..=255` — for example `reditus 'initium' extra fines status exitus est; ad {roman} redactus`, with the truncated value rendered through `Roman::display()` so no Arabic digit appears. The semantics stay as the packages design §4.3 specifies; only the silence is fixed, which is exactly what the register's Expected asks for.

**What breaks:**

| artefact | change |
|---|---|
| `defectus/D-08-emissio-perdita` | → `errores/emissio-ante-erratum`; `emissio` = `ante`, `nuntii` unchanged |
| `defectus/D-09-monita-perdita` | → `errores/monitum-cum-errato`; `nuntii` gains the `Monitum` line first |
| 7 `errores/*` (`adde-argumenta`, `adde-elementum-alienum`, `elementa-discrepantia`, `elementum-typi-falsi`, `index-assignationis-non-ni`, `index-extra-fines`, `index-seriei-non-ni`) | `nuntii` gains one `Monitum` line for `xs` |
| `cli.rs` D-10 tests (`…116`, `…255`) | now also assert the `Monitum` line |
| `programs.rs:78` | rename `type_errors_still_halt_before_any_linting` → `type_errors_halt_but_their_warnings_survive`; rewrite the comment; strengthen the body |
| `error.rs:85` test literal, `main.rs:5`'s `finish<E: Display>` | add fields / drop the generic |

**The `programs.rs` collision the register warns about is illusory.** `type_errors_still_halt_before_any_linting` asserts only `.is_err()` — it *passes unchanged*. Only its name and its comment ("no warnings are produced at all") become false. Correct the register on this point.

**Risk:** `PartialEq` on `ProgramError` now compares `stdout`/`warnings`. Confirm no test does `assert_eq!` on a whole `ProgramError` (the `interp.rs` and `program.rs` tests use field access).

---

### B7 — D-04 + D-05: depth limits

**Files:** `crates/lingua-serpentis/src/{parser.rs,interp.rs}`, `tests/cli.rs`, `tests/robustness.rs`, `tests/support/diag.rs`.

**The register under-scopes D-05.** `parser.rs:143`'s `LParen` arm is not the only unbounded recursion — two more bypass the expression ladder entirely: `unary()` (`:101-107`) self-recurses on `Not`/`Minus`, so `-`×20000 aborts; and `statement() → if_stmt() → block() → statement()` (`:328`, `:221`, `:208`) aborts on 20,000 nested `si`, with `if_stmt`'s `aut si` at `:231` self-recursing too. A counter placed only where the register points leaves two live SIGABRTs that `mutate.rs`'s span duplication will eventually find. **Guard all three.**

Parser: add `depth: usize` to `Parser` and a `descend` wrapper erroring at `ALTITUDO_MAXIMA` with `expressio nimis alte implicata`. Wrap exactly three bodies — `expr()` (every re-entry into the ladder from `primary`'s LParen, `postfix`'s `[`, `call_args`, list literals and all three statement conditions goes through it), `unary()`'s two recursive arms, and `block()`. One shared counter is correct: they all consume the same Rust stack and their costs are additive.

Interpreter: add `depth: usize` to `Interp`; split `call` (`interp.rs:142`) into a guard plus `call_inner` (today's body verbatim) so the counter cannot leak on the two early returns. Error `vocatio nimis alte implicata`.

**Constants must be measured, not guessed.** The register measured the parser at clean/1000, abort/2000 in a debug build. Binary-search both limits against the built debug binary during implementation; provisional starting points are 256 (parser) and 512 (interpreter), assuming the interpreter's per-level Rust frame is ≲4× the parser's — verify. **Not configurable:** there is no config surface in the language, and `robustness.rs`/`generative.rs` spawn the binary with the inherited environment, so a tunable limit would make the invariant environment-dependent.

**The wording is constrained by an existing test.** `cli.rs:215`'s aspirational `assert!(err.contains("alte"))` is the ignored test's one substantive claim — pick messages containing `alte`, do not rewrite the assertion to fit a freely chosen message. Both messages carry no number, so nothing trips `no Arabic digits`; do **not** interpolate the limit.

**What breaks:** `robustness.rs::pathologies()` — `parentheses nimium implicatae` and `recursio infinita` both flip `Some("no abort")` → `None`; the bidirectional test fails loudly if you fix one and not the other. Delete `"no abort"` from `RECORDED_RULES`. `cli.rs:205` `unbounded_recursion_is_a_clean_latin_error` — drop `#[ignore]` and tighten `contains("alte")` to a verbatim `assert_eq!`, since Global-Constraint exception #2 expires the moment the message exists. **Two ex-`cli.rs`-only defects become corpus fixtures:** `errores/expressio-nimis-alte-implicata` (300 nested parens on line 2) and `errores/vocatio-nimis-alte-implicata` (a self-recursive `munus`) — a genuine coverage win that keeps both new catalogue sites `fixtured`. Verify the fuzz target's runtime does not blow up with ~600 extra chars of mutation surface.

---

### B8 — D-11 + D-22 + D-17: Latin I/O errors and hidden directories

**Files:** `crates/lingua-serpentis/src/{main.rs,program.rs}`, `tests/cli.rs`.

**"Not valid UTF-8" cannot be named** — `UTF-8` contains a digit and would trip the suite's own `no Arabic digits` rule. Describe the symptom instead. The classification:

| condition | `ErrorKind` | message |
|---|---|---|
| file missing | `NotFound` | `scida '{}' non inventa est` |
| file unreadable | `PermissionDenied` | `scida '{}' legi non potest` |
| file not text | `InvalidData` | `scida '{}' signis incognitis scripta est` |
| otherwise | `_` | `scida '{}' aperiri non potest` |
| directory unreadable | any | `directorium {} legi non potest` |
| directory not walkable | any | `directorium {} percurri non potest` |
| argv path missing | — | `via '{}' non inventa est` |
| argv path not `.liss` | — | `nomen scidae in '.liss' desinere debet` |

`signis incognitis scripta est` chimes with the lexer's existing `signum ignotum`. Validate `desinere`, `percurri`, `scida`, `aperiri` with `latinitas::is_latin` before landing.

**Write one full literal per cause, not a shared frame plus an interpolated tail.** A `format!("scida '{}' {}", rel, causa(&e))` gives the catalogue a single pattern matching *any* file message — precision loss, and uniquely-attributable sites is D-24's whole theme.

**D-22 / `program.rs`.** `collect_liss` already threads `prefix` (`:72`), so the absolute-path leak at `:73` is fixable with data in hand: `directorium programmatis legi non potest` for the root, `directorium 'mathematica' legi non potest` for a package. Never `dir.display()`. `:76`'s per-entry failure gets the distinct `percurri non potest`; `:84` uses the four-way classification with the already-relative `rel`.

**D-17 edits the same loop** (`program.rs:81`) — skip directory entries whose name starts with `.`. The packages design §4.1 already says folders with no `.liss` beneath them are ignored; this extends it to the case where one *is* there, which is what a `.git` checkout looks like.

**D-11 is not "translating two string literals."** The register's own Note proves the *control flow* is wrong: a nonexistent plain directory fails `is_dir()`, falls through the extension check, and gets `Error: file must have .liss extension`; a nonexistent `x.liss/` gets the file message. Translating the literals leaves both misfires in place, in Latin. Add a `!path.exists()` branch after the `is_dir()` check and before the extension check. `is_dir()` still runs first, so `cli.rs:78::a_directory_named_dot_liss_runs_as_a_program_folder` stays green.

**Accepted edge, to document:** interpolating `args[1]` can inject Arabic digits (`prog2/a.liss`), tolerated only because the path sits inside `'…'` and `without_quoted` strips it. **The quotes are load-bearing.** A path containing a literal `'` defeats the stripper.

**What breaks:** delete `"Latin only"` from `RECORDED_RULES` — **only once both D-11 and D-22 are done**, since the allowlist is keyed by rule. Verified against `ENGLISH_WORDS`: no remaining Latin message contains any of the eight as a whole token (`invalidus` ≠ `invalid`, `directorium` ≠ `directory`; the check splits on non-alphabetic and compares exactly). **Both `starts_with` carve-outs in `cli.rs` expire** — with the errno tail gone, every message is ours and can be asserted byte-exact (computed from the temp path, still verbatim, just not a literal):

| test | new assertion |
|---|---|
| `a_missing_file_reports_in_english_defect_d11` → rename | `Error: via '{}' non inventa est` |
| `a_non_liss_extension_is_refused_in_english_defect_d11` → rename | `Error: nomen scidae in '.liss' desinere debet` |
| `a_nonexistent_directory_gets_the_extension_error_defect_d11` → rename | now the *missing-path* error — **the assertion inverts** |
| `an_unreadable_file_is_reported_not_panicked` | `Error: scida '{}' legi non potest` |
| `a_non_utf8_source_file_is_reported_not_panicked` | `Error: scida '{}' signis incognitis scripta est` |
| `a_hidden_directory_containing_liss_kills_the_program_defect_d17` → rename | now runs cleanly, `.git/` ignored |

Plus new `cli.rs` tests for D-22's three library sites (mode-000 `.liss` inside a program folder, mode-000 subdirectory, non-UTF-8 `.liss` inside a folder) — that is where the relative-path claim gets pinned. The new `main.rs`/`program.rs` catalogue sites all stay UNCOVERED (`observed_messages()` runs only the conformance corpus, which cannot carry a file mode); record that in the register so a later reader does not read it as a regression.

---

### B9 — D-06: the broken pipe

**Files:** `crates/lingua-serpentis/src/main.rs`, `tests/cli.rs`, `tests/support/diag.rs`.

Rust's runtime sets `SIGPIPE` to `SIG_IGN`, so `main.rs:9`'s `print!` panics with exit 101 on `prog | head`. Zero runtime dependencies rules out `libc::signal`, so handle the write error instead: replace `eprint!`/`print!` with explicit `io::Write` calls against locked handles, and on `ErrorKind::BrokenPipe` exit **0** silently, the way every Unix tool behaves. Flush explicitly before `exit()` — `std::process::exit` does not run destructors, and today's code relies on that only by luck.

**What breaks:** `cli.rs:181` `a_closed_pipe_is_not_a_panic` — drop `#[ignore]`. Delete `"no panic"` from `RECORDED_RULES`, which after B2/B3/B7/B8 leaves `["terminates"]`, `[&str; 1]`.

---

### B10 — D-21: `mag_divmod`'s zero guard

**Files:** `crates/lingua-serpentis/src/{roman.rs,fraction.rs,interp.rs}`.

`roman.rs:110`'s `debug_assert!` compiles out in release; with `d` empty the doubling loop at `:118` runs forever until memory runs out. `Roman::div_rem` (`:336`) is `pub`, so this is a latent release-only hang for any future caller.

Change `div_rem` to return `Option<(Roman, Roman)>`, `None` on a zero divisor. Update the callers, all of which already guard and so can take the value directly with a comment naming the guard: `interp.rs:317`/`:330` (guarded by `divisio per nullum` / `modulus per nullum`), `fraction.rs:31` (already returns `None`), `fraction::from_rational` and `fraction::mul` (constant grain), `exit_status` (constant 256). Not `assert!` — an unreachable panic is still a panic, and `no panic` is a live fuzzer rule.

No new diagnostic, no fixture change. Correct the register's D-21 entry, which records the fraction-literal reasoning against the wrong guard: `°I:I/N`, `°I:I/` and `°I:I/O` all die in `convert_single` (`roman.rs:193`/`:205`), never reaching `:252`.

---

### B11 — D-24: the rewording sweep

**Files:** `crates/lingua-serpentis/src/{checker.rs,interp.rs}`, several fixtures.

B1 already collapsed the sites that were genuinely one condition. What remains are sites that say the same thing about *different* conditions, and should not.

| site | today | new | why |
|---|---|---|---|
| `interp.rs` `variabilis ignota` (post-B1) | `variabilis ignota: '{}'` | `variabilis '{}' nondum instituta est` | the checker's is a *static* fact, the interpreter's a *temporal* one; `nondum` says exactly that. The checker keeps the original, so `errores/variabilis-ignota` is untouched |
| `interp.rs:145` | `functio ignota: '{}'` | `munus ignotum: '{}'` | uses the language's own keyword; `errores/functio-ignota` hits the checker → unchanged |
| `interp.rs:147` | `functio '{}' numerum argumentorum falsum accepit` | `munus '{}' numerum …` | same rule; `errores/numerus-argumentorum` hits the checker → unchanged |
| `checker.rs:369` (Assign) | `'{}' typum '{}' expectat, sed '{}' datum est` | `'{}' iam typum '{}' habet, sed '{}' datum est` | `iam` ("already") is exactly what distinguishes assignment from declaration; `:355` (Let) keeps the original |
| `checker.rs:100` (`Neg`) | `operatio unaria typo non convenit` | `negatio numerum requirit` | `:100` is unambiguously negation of a non-number |
| `checker.rs:102` (`Not`) | same string | `'non' veritatem requirit` | quoting the keyword matches the `'longitudo' seriem expectat` house style |
| `checker.rs:437` (IndexAssign) | `index seriei 'ni' esse debet` | `index assignationis 'ni' esse debet` | matches the fixture's own name; `:195` keeps the original |

**`checker.rs:117` is mis-classified in the register** — it is dead *by construction*, not merely never provoked, on the same argument as `:414` and `:524`: `check_file` (`:572-578`) rejects every import whose package is absent from `pkg_sigs` before any body is walked, and `:111` requires the name be in `self.imports`. Do **not** replace it with `.expect(...)`. Keep the guard, reword to `fasciculus '{}' inter allatos non inventus` — honest, uniquely attributable, and it tells a maintainer which invariant tripped if it ever does. It reads UNCOVERED, correctly. Promote it to a full D-24-style register entry with the call-graph argument written out.

**What breaks:** `errores/operatio-unaria-typo/nuntii` → `negatio numerum requirit`; `errores/index-assignationis-non-ni/nuntii` → `index assignationis 'ni' esse debet`. Two new fixtures: `errores/non-veritatis` (`veritas b = non °V;` on line 2) for `checker.rs:102`, and `errores/typus-assignationis` (`ni x = °I;` then `x = "a";` on line 3) for `checker.rs:369`.

**Correct the register's stale rows** while editing: `checker.rs:437`, `interp.rs:321` and `:325` are described as "plainly reachable and simply never written", but the register's own closing note says fixtures for all three now exist. They are pure wording ambiguities, not coverage gaps.

---

### B12 — D-01: definite-return analysis

**Files:** `crates/lingua-serpentis/src/checker.rs`.

**Surveyed empirically: this has zero collateral fixture churn.** Every `munus` in all 79 fixtures, both `examples/` trees, `tests/programs.rs`, `generative.rs::program()` and every `check_src`/`pprog` string in `checker.rs`'s test module was checked. **Exactly one program fails the analysis: `conformance/defectus/D-01-reditus-omissus`'s own.** Every other function body's last statement is a `redde`. `interp.rs`'s tests never invoke the checker, so `falling_off_initium_exits_zero` is untouched regardless.

```rust
/// Does every path through this statement leave the function?
/// Deliberately conservative: `true` only when it provably returns.
fn always_returns(s: &Stmt) -> bool {
    match &s.kind {
        StmtKind::Return(_) => true,
        // Both arms, and an `aut` must exist. `aut si` parses as a nested
        // `If` inside a one-statement else block (parser.rs:231), so the
        // else-if chain falls out of the recursion with no extra case.
        StmtKind::If { then, else_: Some(eb), .. } => block_returns(then) && block_returns(eb),
        _ => false,
    }
}
fn block_returns(body: &[Stmt]) -> bool { body.iter().any(always_returns) }
```

`any`, not "the last statement": a `redde` followed by dead code still returns, and the language has no unreachable-code lint to contradict.

**Do not special-case `indem (verus)`.** The language can express it, and Rust/Java would treat it as diverging — but only *without* a reachable `abrumpe`, and B5's loop-depth tracking is not break-*target* tracking. Treating a `verus` loop as non-returning is sound (it over-rejects), no corpus program uses the idiom, and the workaround is a trailing `redde`. Document the omission in the diagnostic's doc comment as a known conservatism.

**Hook point:** `checker.rs:402`'s `StmtKind::Func(f)` arm, **after** `check_block_inner` succeeds, so a body error still wins and no existing `errores/` message is displaced. Message: **`munus '{}' non in omni via reddit`** — `via` is already the language's word for a path (`main.rs:16`), and `reddit` is its own keyword, so it reads as "does not *reddit* on every way".

**The `initium` exemption is root-only.** Packages design §4.3 licenses the root `initium` falling off its end and `interp.rs:609` pins it; a package function that happens to be called `initium` is ordinary (and `checker.rs:964` pins *that*). Plumbing: `Checker` gains `root_namespace: bool`; `Checker::new()` sets it **`true`** so the legacy `check()` stays permissive (D-19 deferred; none of its 29 tests trip the analysis anyway); `check_file` takes a fourth `is_root: bool`; `check_program` passes `true` at `:623` and `false` at `:628`. Nested functions are already rejected, so `block_returns` never sees a `StmtKind::Func`.

**Keep `interp.rs:168`'s `Ok(Value::Int(Roman::zero()))` fallback**, with an updated comment. After D-01 the only body reaching it is a root `initium`, which the checker forces to be `reddit ni` — so the fallback is now correct *by construction* rather than merely defensive; the legacy `execute()` path still needs it; and turning it into an error would manufacture another unreachable diagnostic.

**What breaks:** `defectus/D-01-reditus-omissus` → `errores/reditus-omissus`; `nuntii` = `Error in 'principale.liss', linea 2: munus 'nomen' non in omni via reddit`, `exitus` 1, `emissio` deleted. `munus nomen` currently sits on line 1 and the Global Constraints forbid a line-1 `errores` fixture — prepend a comment line.

**Knock-on for the register:** D-01's Note says fixing it *promotes* four defensive interpreter messages to reachable. Fixing it **demotes them again** — `interp.rs:136` `subscriptio invalida`, `:288` `condicio non est veritas`, `:295` `operatio binaria invalida`, `:358` `comparatio invalida` become provably dead. All four already read UNCOVERED, so the header does not move, but they earn D-25-style "proved unreachable" entries.

---

### B13 — Reconcile the documentation

**Files:** `docs/defectus/2026-08-03-index.md`, `docs/superpowers/specs/2026-08-03-conformance-and-robustness-suite-design.md`.

- Rewrite the register header: it currently opens **"Nothing here is fixed."** Replace with a status table — 22 fixed with commits, 3 deferred with the spec each needs.
- Fold in the corrections this plan establishes, each as a dated note rather than a silent edit (house rule: the register is the corrected reading, and §8.1 is brought into line, with the delta recorded in **Appendix A**):
  1. **D-05 has three recursion sites**, not one — `unary()` and `statement→if_stmt→block→statement` abort independently of the expression ladder.
  2. **D-11 is a control-flow bug**, not two string literals; both messages misfire on nonexistent paths.
  3. **D-14's implied per-character retreat breaks two `errores/` fixtures**; separator-only trimming is the correct algorithm.
  4. **D-22's Expected is unsatisfiable as written** — "not valid UTF-8" cannot be named without an Arabic digit.
  5. **D-09 has no collision with `programs.rs`** — that test asserts only `.is_err()`; its *name and comment* were wrong, not its assertion.
  6. **D-24's `checker.rs:414` is reachable** through the legacy `checker::check` API that D-19 defers removing; only `:524` and `:117` are dead.
  7. **D-24's table is stale on three rows** (`checker.rs:437`, `interp.rs:321`, `:325` are wording ambiguities with fixtures, not coverage gaps).
  8. **D-12's "a fix must change both sites"** stops being true after B1's de-duplication.
  9. **D-21's fraction-literal reasoning cites the wrong guard** — `convert_single` (`roman.rs:193`/`:205`) is what stops those probes, not `roman.rs:252`.
  10. **D-23's "near-linear" understates the achievable** — the two-sided bound is O(1) in identifier length.
  11. **`error.rs:20`'s UNCOVERED status is a catalogue artefact**, not a gap: `observed_messages()` splits each stderr line on the first `": "` and keeps the tail, so a `Monitum` frame can never be observed. Note it; do not "fix" it.
- Amend the suite design **in place** with a dated `Superseded` block on §2's Non-Goals, recording that the behaviour freeze was scoped to that suite and has now been lifted for these 22.

---

## Verification

Run from a `nix develop` shell.

- [ ] `cargo test --workspace` — green. Baseline was 347 tests (345 passed, 2 ignored); expect a higher count and **zero ignored**, since B7 and B9 un-ignore both `cli.rs` tests.
- [ ] `cargo test --test cli -- --ignored` — **reports no tests**. This is the headline: the two aspirational tests are now ordinary passing tests.
- [ ] `LISS_FUZZ_CASES=5000 cargo test --test robustness` — passes, and again with a second seed (`LISS_FUZZ_SEED=0xC0FFEE`). Baseline runtime was ~76s.
- [ ] `cargo test --test conformance` — every case passes and **`conformance/defectus/` is empty**; `git log --diff-filter=R` shows each fixture's move.
- [ ] `grep -c . <<< RECORDED_RULES` — the array is `["terminates"]`, `[&str; 1]`, with a comment naming D-20 as the sole remaining holder.
- [ ] `robustness.rs::pathologies()` — no entry carries `Some(rule)` except those tied to D-20.
- [ ] `catalogus__catalogus.snap` — re-blessed and **read line by line**. `roman.rs` loses two sites (B1); `checker.rs`/`interp.rs` lose ~10 duplicates; `parser.rs`, `interp.rs`, `main.rs`, `program.rs` gain sites. The `ambiguous` count should drop sharply. Expect `main.rs`/`program.rs` sites to stay UNCOVERED — that is correct, not a regression.
- [ ] Manual end-to-end spot checks against `target/debug/lingua-serpentis`, each of which fails today:
  - `°X/°II` prints `V` (D-14)
  - a multi-line string then an undefined variable reports the *true* line (D-03)
  - `scribe("ante")` then a division by zero prints `ante` **and** the error (D-08)
  - a non-Latin name plus a type error prints **both** the `Monitum` and the `Error` (D-09)
  - unbounded recursion exits 1 with a Latin message, not 134 (D-04)
  - `./lingua-serpentis big.liss | head -1` exits 0 silently (D-06)
  - a program folder containing `.git/x.liss` runs (D-17)
  - `lingua-serpentis absens.liss` and `lingua-serpentis notae.txt` are Latin (D-11)
  - `time` a 100,000-character identifier — milliseconds, not 24s (D-23)
  - `examples/calc.liss` lints clean (D-18)
- [ ] `git status --short` — the user's ten pre-existing staged entries are all still present and unmodified.

---

## Critical files

`crates/lingua-serpentis/src/{lib.rs,error.rs,lexer.rs,parser.rs,checker.rs,interp.rs,roman.rs,program.rs,main.rs,token.rs}` · `crates/latinitas/src/{analyze.rs,model.rs,whitaker.rs,lib.rs}` · `crates/lingua-serpentis/tests/support/diag.rs` (`RECORDED_RULES`) · `crates/lingua-serpentis/tests/{robustness.rs,cli.rs,programs.rs}` · `crates/lingua-serpentis/tests/snapshots/catalogus__catalogus.snap` · `conformance/**` · `docs/defectus/2026-08-03-index.md`
