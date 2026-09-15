# Lingua Serpentis — `dum` Header Clause Design

- **Date:** 2026-09-15
- **Status:** Proposed
- **Author:** Manuel (with Claude)
- **Relationship:** Amends `2026-07-19-lingua-serpentis-rust-interpreter-design.md` §5.5 — the
  `dum (init; cond; step)` line gains a grammar for `init`, which it never had. Closes deferred
  defect D-26 (`defectus/2026-08-03-index.md`). Cites, and does not amend, the compiler-backend
  spec's §2 Non-Goal "No new language semantics" (:82-83), which was that milestone's scope and not
  a language rule, and its §7.3 (:630), whose `dum (voc s = "salve"; verus; s = s) { redde nullus; }`
  has its `redde` in the body and is unaffected. Adds **no** dependency: §11's table is untouched.

## 1. Vision

The parser reads a `dum` init clause with `self.statement()?` (`parser.rs:323`): any statement at
all. A statement can end control flow, or contain something that does, and nothing downstream says
no. Today, with the binary built from `cd516b3`:

```liss
// D-26
munus initium reddit ni {
    ni k = nullus;
    dum (redde nullus; verus; k = k + °I) { scribe("numquam"); }
}
```

```
$ liss run ./redde
$ echo $?
0
```

No output, no diagnostic, exit 0. The function returned from inside the loop's header. The same
happens one level out:

| init clause | stdout | stderr | exit |
|---|---|---|---|
| `redde nullus;` | — | — | 0 |
| `abrumpe;`, inside an enclosing `indem` that would print `extra` | — | — | 0 — the *outer* loop is left |
| `si (verus) { redde °I; }` | — | — | 1 — the function returned `°I` |
| `munus unum reddit ni { redde °I; }` | `nullus` `I` | — | 0 — a function nobody can call, checked and dropped |
| `scribe("salve");` | `salve` `nullus` `I` | — | 0 |
| *(empty)* | — | `erratum: expressio expectata` | 1 |

Each row except the last is a reading the implementation took rather than one the language chose.
The register filed it as deferred precisely so that someone would choose (D-26, "Revisited
2026-08-08"). This document chooses: **the init clause is a declaration or an assignment, and
nothing else parses.**

```
erratum: clausula prima capitis 'dum' declaratio vel assignatio esse debet
  in 'principale.liss', linea IV
    |
 IV |     dum (redde nullus; verus; k = k + °I) { scribe("numquam"); }
    |          ^^^^^
```

What it buys, beyond the diagnostic: the checker stops needing an arm for a `munus` it cannot
reach, codegen stops needing a terminator guard for a header that cannot terminate, and two tests
whose only job was to pin or dodge an undecided shape go away.

## 2. Goals and Non-Goals

### Goals

- **One grammar rule for `init`** — `TYPE ident = expr ;` or `ident = expr ;` (§3).
- **One new sentence**, inline at its call site, unique in the workspace, every word passing
  `latinitas::is_latin`, no Arabic digits (§4).
- **Every shape that leaves the init clause other than by falling through is unparseable**,
  including one nested at any depth under `si`, `indem`, `dum` or `munus` (§5).
- **The `munus`-in-header hole closes with it** (§5, §6).
- **Delete what the rule makes dead** (§6).
- **Four `errores/` fixtures**, the first D-26 has ever had (§8).

### Non-Goals

- **Not the compiled reading made normative.** The register's other candidate (`via.md:76`) is
  rejected: it would freeze into 1.0 a meaning a reader has to reason out, for a shape no program in
  the repository writes.
- **Not a checker rule.** A checker variant (`take(loop_depth)` around `init` plus an `in_caput`
  flag for `redde`) was considered. It needs at least one sentence per terminator to read well, it
  borrows D-07's "extra iterationem" wording where the user can see an enclosing loop, and it leaves
  the `munus` hole open. The parser rule is smaller and closes both.
- **Not the step or the condition.** The step is `assign_no_semi` (`parser.rs:326`, `:332-338`) and
  the condition an `expr` (`:324`); neither can hold a statement. `indem` and `si` have
  expression-only headers (`parser.rs:293`, `:313`). The `dum` init clause is the only
  statement-valued header in the language.
- **Not the dead code D-26 did not create.** `checker.rs`'s `nop()` (`:114-122`) and its `:609`
  fallback, and `emissio.rs`'s step guard (`:776-778`), are unreachable today because the step is
  already an `Assign`. Deleting `nop()` moves every `checker.rs` row of the catalogue; it is its own
  change, not this one.
- **Not a new statement form.** `ni i = °I, j = °II` and similar multi-declarations stay out.

## 3. The grammar

Before (`2026-07-19` §5.5, and `parser.rs:319-330`):

```
dum_stmt := 'dum' '(' statement expr ';' assign_no_semi ')' block
```

After:

```
dum_stmt    := 'dum' '(' dum_init expr ';' assign_no_semi ')' block
dum_init    := TYPE ident '=' expr ';'        -- StmtKind::Let
             | ident '=' expr ';'             -- StmtKind::Assign
```

`TYPE` is what `type_keyword` reads (`parser.rs:249-265`), `lista` suffixes included, so
`dum (ni lista l = [°I]; …)` stays legal.

**The decision is made on lookahead, before anything is consumed.** The next token must be a type
keyword, or an identifier whose following token is `=` — the lookahead `statement()` already uses to
tell an assignment from a call (`parser.rs:434-435`). If it is, `statement()` parses the clause
exactly as it does today, so the AST, the spans and every downstream stage see the same `Let` or
`Assign` they see now. If it is not, the parser errors **at that token**, having consumed nothing
of the clause.

A `Let` whose right-hand side is malformed still reports what it reports today (`expressio
expectata`, `expectatum: ';'`): the rule decides which *production* the clause is, not whether its
contents are well-formed.

**The header stays a scope.** `conformance/currunt/memoria-caput-dum-redditum` declares a `voc` in
the init clause and returns from the body; it is the positive pin that the header remains a `Block`
(`checker.rs:602-603`) and is unaffected. `crates/lingua-serpentis/tests/cli.rs:266` is the one
in-repository `Assign` init clause and is likewise unaffected.

## 4. The sentence

```
clausula prima capitis 'dum' declaratio vel assignatio esse debet
```

*The first clause of the `dum` header must be a declaration or an assignment.*

| word | `latinitas::is_latin` |
|---|---|
| `clausula` | true |
| `prima` | true |
| `capitis` | true |
| `declaratio` | true |
| `vel` | true |
| `assignatio` | true |
| `esse` | true |
| `debet` | true |

Measured on 2026-09-15 against `crates/latinitas` at `cd516b3`. `'dum'` is a quoted keyword, as
`'abrumpe'` and `'reddit'` are elsewhere.

- **No hole.** It quotes nothing the user wrote; the caret shows what was written.
- **Unique.** No other literal in `crates/*/src` contains `clausula`, so the catalogue reads it
  `fixtured`, never `ambiguous`.
- **Inline.** Written as the literal argument of `self.err(…)` at the call site, per spec §10.3; a
  forwarded constant would read `UNPARSED`.
- **`esse debet`** follows `condicio 'veritas' esse debet` (`checker.rs:747`) and
  `index seriei 'ni' esse debet` (`:391`); **`vel`** follows `'mnr' vel 'nr'` (`:455`). It says what
  is required rather than naming what was found, because what was found ranges over eleven
  statement kinds (§5).

## 5. Every shape rejected

The error lands on the first token of the init clause in every case.

| # | init clause | caret on | today |
|---|---|---|---|
| 1 | `redde e;` | `redde` | returns from the function |
| 2 | `abrumpe;` | `abrumpe` | D-07 error at loop depth 0; inside a loop, leaves the enclosing loop |
| 3 | `continua;` | `continua` | as 2, to the enclosing loop's step |
| 4 | `si (c) { … } [aut …]` — with or without a terminator inside | `si` | runs; returns if both arms do |
| 5 | `indem (c) { … }` — with or without a terminator inside | `indem` | runs; a `redde` inside escapes the header |
| 6 | `dum (…) { … }` — including a nested `dum` whose own init is any row of this table | the outer init's `dum` | runs; the same question, recursively |
| 7 | `munus f … { … }` | `munus` | checked, then dropped (`checker.rs:613-618`) |
| 8 | `scribe(e);` | `scribe` | runs |
| 9 | `f(e);`, `p.f(e);` or any other expression statement | the expression's first token | runs |
| 10 | `x[i] = e;` | `x` | runs — `x` is followed by `[`, not `=` |
| 11 | *(empty)*: `dum (; c; s)` | `;` | `expressio expectata` |

Rows 4–9 include shapes that were harmless. That is the cost of a grammar rule over a flow rule,
and it is paid knowingly: no program in the repository writes any of them (every init clause is a
`Let` — 38 of them — or the one `Assign` at `cli.rs:266`), and a `scribe` or a call belongs on the
line above the `dum`, where it already means the same thing.

Row 11 changes an existing message for a malformed program. No fixture holds it; the robustness
fuzz may reach it, and either sentence satisfies `judge`.

Rows 5, 6, 9 and 10 have no fixture (§8); the parser unit test covers them.

## 6. What is deleted

With the init clause a `Let` or an `Assign`, three pieces of code and two tests lose the only input
that reached them.

**`crates/lingua-serpentis/src/checker.rs`**

- **The `StmtKind::Func` arm of `check_stmt`** (`:613-618`) existed for a `munus` in a header. The
  only other caller of `check_stmt` on a block statement is `check_block_inner` (`:735`), which
  rejects a `Func` before calling it (`:729-733`). So the rejection **moves into the arm**: the arm
  returns the nested-function error that `check_block_inner` returns today, and
  `check_block_inner`'s own `if let` goes. `check_stmt` still matches `StmtKind` exhaustively; the
  sentence, its status and `errores/munus-nidificatum` are unchanged; the error still carries the
  statement's `line` and `spatium`.
- **The comment at `:594-595`** becomes a bare `// D-26`, per `CLAUDE.md`. Its reasoning is in §3 and
  in `docs/internals/rationale.md`.

**`crates/codegen/src/emissio.rs`**

- **The header walk's guard** (`:736-749`): the comment, the `if self.terminatus() { break; }` inside
  the loop and the `if self.terminatus() { return Ok(()); }` after it. A `Let` or an `Assign` cannot
  fill a block. What remains is `for stmt in &header.stmts { self.sententia(stmt)?; }`.

**`crates/codegen/tests/regimen.rs`**

- **`caput_dum_terminatum_componitur`** (`:298-344`). Its three programs no longer parse, so
  `support::curre` would not see exit 0; its pin passes to the four fixtures of §8.

**`crates/lingua-serpentis/tests/generative.rs`**

- **`caput_dum_terminatum`, `clausula_initialis` and `verbum_est`** (`:91-142`), the assertion in
  `generated_programs_never_escape_the_error_model` (`:154-159`) and the one in
  `generation_reaches_the_runtime` (`:200`), and the self-test
  `the_guard_recognises_the_shape_it_excludes_and_nothing_else` (`:257-280`). The guard marked a
  program whose meaning was undecided; there is no such program now. It was also inexact — it cut the
  init clause at the first `;` whatever the braces said — and `program()` (`:83-89`) never emits a
  `dum`, so it has never fired.

## 7. What this does to the catalogue

Measured from `crates/lingua-serpentis/tests/snapshots/catalogus__catalogus.snap` at `cd516b3`:

```
before:  118 sites · 70 fixtured · 0 ambiguous · 46 uncovered · 2 unparsed
after:   119 sites · 71 fixtured · 0 ambiguous · 46 uncovered · 2 unparsed
```

Every row difference, with its cause:

| row | change | cause |
|---|---|---|
| `parser.rs:<for_stmt>  clausula prima capitis 'dum' declaratio vel assignatio esse debet  → fixtured` | **added** | §4; fixtured by all four fixtures of §8 |
| `parser.rs:350` … `parser.rs:494` (16 rows) | `:line` only | the lines `for_stmt` gains |
| `checker.rs:730  functio '{}' intra corpus definiri non potest; functiones nidificatae non sustinentur  → fixtured` | `:line` only, and reorders to sit above the `:623` row | §6 — the rejection moves into the `Func` arm |
| `checker.rs:623` … `checker.rs:932` (the other 13 rows: `623 635 643 650 661 664 694 747 821 838 882 924 932`) | `:line` only | the comment loses a line; the arm and the `if let` change size |
| `emissio.rs:813  defectus internus: munus vocatum non inventum: vocatio  → UNCOVERED` | `:line` only | the header-walk guard's lines go |

No row in any other file moves. `ambiguous`, `uncovered` and `unparsed` do not move. The check,
since `:line` shifts and one row reorders: **the sorted set of `(pattern, status)` pairs after equals
the set before plus exactly one pair**, `(clausula prima capitis 'dum' declaratio vel assignatio esse
debet, fixtured)`.

## 8. Fixtures, and what the corpus counts become

Four new `conformance/errores/` cases, each with `programma/principale.liss` opening on a comment
line (an `errores/` program must not report on line 1), `exitus` = `1`, a blessed `nuntii` holding
exactly one diagnostic, and a `NOTA.md` that opens "D-26 is fixed" and records what the program did
before. No `emissio`: an empty stream has no file. Identifiers are Latin, or a `monitum` joins the
stream.

| fixture | init clause | before |
|---|---|---|
| `redde-in-capite-dum` | `redde nullus;` | exit 0, silent |
| `abrumpe-in-capite-dum` | `abrumpe;`, inside an `indem` that prints after the inner loop | exit 0, silent — the outer loop left |
| `si-in-capite-dum` | `si (verus) { redde °I; }` | exit 1, silent |
| `munus-in-capite-dum` | `munus unum reddit ni { redde °I; }` | ran; the `munus` dropped |

`continua` is not given its own fixture: it takes the same branch as `abrumpe`, and the parser test
covers it.

| count | before | after |
|---|---|---|
| `conformance/` fixtures | 118 | 122 |
| `errores/` | 78 | 82 |
| `nuntii__nuntii.snap` header | `86 fixtures · 99 diagnostics` | `90 fixtures · 103 diagnostics` |
| catalogue header | §7 | §7 |

The four new `nuntii` rows are the **only** difference in `nuntii__nuntii.snap`. No existing fixture
moves: `errores/abrumpe-extra-iterationem` and `errores/continua-extra-iterationem` put their
terminator in a function body, not a header.

The restated counts that follow — `CLAUDE.md:52,54,68,79`, `README.md:80`, `docs/README.md:220`,
`docs/internals/testing.md:55,284,292,298`, `crates/lingua-serpentis/tests/nuntii.rs:3` — are updated
where milestone 2 closes, not in the fix commit, so that D-13 and D-20 do not each re-edit them.

## 9. Blessing

Four `nuntii` files are *new*; none is rewritten. That makes this the narrow case, and it is kept
narrow mechanically:

1. **Before blessing,** `cargo nextest run --test conformance` fails on exactly the four new trials,
   each missing its `nuntii`, and nothing else.
2. **Bless with a filter**, not the whole corpus: `LISS_BLESS=1 cargo test --test conformance --
   in-capite-dum`. `libtest-mimic` matches the substring against `errores::<casus>`.
3. **`git status --porcelain conformance/`** lists the four new directories and nothing else.
4. **Read each `nuntii`**: one `erratum:` block, the §4 sentence, `linea` on the `dum` line, the
   caret under the init clause's first token.
5. **`cargo nextest run --test nuntii`** fails with exactly four added rows and no removed or
   changed row; accept with `INSTA_UPDATE=always`, re-run green.

## 10. Couplings

- **A `dum` header written anywhere in the repository — a `.liss` example, a test string in any
  crate — must open with a declaration or an assignment.** `examples/` are reached from
  `conformance/` through `VIA` files, and several crates' tests compile programs from strings, so a
  stray `dum (scribe(…); …)` now fails a test far from the parser.
- **The four fixtures shift the fuzz seed stream.** `support/mutate.rs:67-74` sorts the corpus by
  path; any new `.liss` under `conformance/` changes which mutations run. Say so in the fix commit.
- **D-26's register entry, status row and deferred-table row close with this change**, and
  `docs/via.md`'s milestone 2 row is marked when all three of milestone 2's decisions are in.

## 11. Success criteria

1. Every shape in §5's table fails to parse with the §4 sentence, caret on the token named there —
   the parser unit test enumerates all eleven and asserts both the message and that `Let`, `lista`
   `Let` and `Assign` init clauses still parse.
2. `crates/lingua-serpentis/tests/snapshots/catalogus__catalogus.snap` opens with
   `119 sites · 71 fixtured · 0 ambiguous · 46 uncovered · 2 unparsed`, and its sorted
   `(pattern, status)` set is §7's before-set plus one pair.
3. `crates/lingua-serpentis/tests/snapshots/nuntii__nuntii.snap` opens with
   `90 fixtures · 103 diagnostics`, and its diff against `cd516b3` is four added rows.
4. `ls conformance/errores | wc -l` prints `82`; `git diff --stat cd516b3 -- conformance/` touches only
   the four new directories.
5. `grep -rn 'caput_dum_terminatum\|clausula_initialis' crates/` prints nothing.
6. `grep -n 'terminatus' crates/codegen/src/emissio.rs` no longer matches inside `dum_intus`'s header
   loop; `grep -n 'StmtKind::Func' crates/lingua-serpentis/src/checker.rs` matches no
   `check_block_inner` line.
7. `cargo nextest run` green; the test count is Task 0's baseline plus the new parser test and the four
   conformance trials, minus `caput_dum_terminatum_componitur` and the generative self-test.
8. `cargo tree -e normal` shows no new package; §11's table is unchanged.
9. `docs/defectus/2026-08-03-index.md` shows D-26 **fixed** in its status table and no longer lists it
   as deferred.
