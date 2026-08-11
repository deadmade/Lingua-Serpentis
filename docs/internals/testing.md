# Testing

How this repository proves itself: the one cargo invocation, the fixture corpus, the diagnostic
ratchet, the robustness invariant, the generated-program property, and the several tests that read
Rust source as text and therefore constrain how Rust source may be written. This document is a
reference for the machinery, not a list of what is covered — the corpus and the `catalogus`
snapshot are the authority on coverage. Everything measured here (deadlines, case counts, hit
rates, symbol names, exact Latin sentences) is carried over from the tests themselves; where a
number is stated, a test asserts it.

## The one command

```console
$ cargo test
```

There used to be a second, because `crates/abi` was `exclude`d from the root workspace and a run
that skipped it left `abi.rs`, `custos.rs` and `emissio.rs` unexecuted — most of what guards the
runtime. `abi` is a member as of 2026-08-09 and one command reaches everything;
`docs/internals/abi.md` has the reasoning and the one trap that came with it.

Work inside `nix develop`. Two environment facts are asserted rather than assumed, in
`lingua-serpentis/tests/instrumenta.rs`; see **Environment assertions** below.

## The test targets

| Target | Crate | What it is |
|---|---|---|
| `conformance.rs` | lingua-serpentis | The fixture corpus, one trial per case (`harness = false`) |
| `catalogus.rs` | lingua-serpentis | The Latin-diagnostic ratchet, snapshotted with `insta` |
| `robustness.rs` | lingua-serpentis | The robustness invariant: pathologies and a mutation fuzzer |
| `generative.rs` | lingua-serpentis | `proptest`-generated well-typed programs |
| `cli.rs` | lingua-serpentis | End-to-end behaviour of the binary through `support::proc` |
| `liss_cli.rs` | lingua-serpentis | End-to-end behaviour of `liss` driven directly |
| `programs.rs` | lingua-serpentis | Whole programs, compiled and run: the pipeline end to end |
| `ir.rs` | lingua-serpentis | Typed-IR snapshots and targeted assertions (spec §4) |
| `instrumenta.rs` | lingua-serpentis | Toolchain and environment assertions |
| `salve.rs`, `scalares.rs` | codegen | Compile-and-run: the pipeline, every type, every operator |
| `regimen.rs`, `series.rs`, `vocationes.rs` | codegen | Control flow, lists, calls — and object inspection |
| `aemulatio.rs` | codegen | aarch64 images run under `qemu-aarch64` |
| `pondus.rs` | codegen | What the embedded runtime was built *with*, read off the bytes |
| `golden.rs` | runtime | The captured `scribe` golden vectors |
| `abi.rs`, `custos.rs`, `emissio.rs` | abi | The C ABI: representation, ownership, guard page, output |

Unit tests live in `#[cfg(test)]` modules inside `crates/*/src`; the ratchet deliberately does not
see them (below).

## The conformance corpus

`conformance/` is 118 fixtures in four groups — `currunt/` 35, `errores/` 78, `monita/` 4,
`defectus/` 1. The vocabulary is the language's own: `currunt` "they run", `errores` programs that
fail, `monita` programs that warn but run, `defectus` a known defect pinning today's behaviour.

Each case directory holds:

- `programma/` — the program handed to the runner, or `VIA` — a file naming a program elsewhere in
  the repo, resolved relative to the repo root;
- `emissio` — expected stdout, `nuntii` — expected stderr, `exitus` — expected exit code (absent
  means 0). **An empty stream has no file at all**, and `bless` removes rather than empties one, so
  `git status` shows intent.
- `NOTA.md` — why the case exists, where one was written.

`nuntii` rather than the obvious `errores` because `errores` is already a *category* directory; a
file of that name inside it would read as a nested category.

**Running a fixture.** `support::discover` walks the four categories in the order
`["currunt", "errores", "monita", "defectus"]`, sorting the cases, so the report is stable; each
becomes a `libtest-mimic` trial named `<categoria>::<casus>`, which is what `cargo test --test
conformance errores::` filters on. `support::run_fixture` calls **`support::curre`**, which spawns
`liss run <target>` — a `.liss` file is a single-source program and a directory is a program
folder, and **`liss` chooses between them**, not the harness. That distinction is load-bearing: the
two modes render diagnostics differently (`Error in linea N:` against `Error in 'f.liss', linea
N:`), so a harness that decided would be able to hide the difference.

`liss run` rather than the bare image, because **Latinity warnings are a front-end product**: the
parent writes them and *then* spawns the child (spec §9.5), so what a caller sees is the parent's
warnings followed by the child's own diagnostics — exactly what the blessed `nuntii` files record,
the interpreter having had only one process to write from. A harness that ran the image would have
to re-bless three `nuntii` files empty, deleting the corpus's only coverage of Latinity warnings in
the name of a harness detail.

The two processes' streams stay in order because the parent's stderr and the child's are the *same
open file description* — `Stdio::from(File)` hands the child a dup of the handle — and the parent
flushes before it spawns. Captured to files rather than pipes because a program that outruns a pipe
buffer would deadlock a harness that waits before it reads, and some of these do so on purpose.
`curre` polls with `try_wait` every `PAUSA` = 2 ms against `MORA` = 120 s, a backstop rather than a
performance budget — through `support::proc::wait_or_kill`, which takes the interval as a parameter
because `proc.rs`'s own two callers poll at 5 ms and unifying them would change a documented number
for nothing; `liss run` on the slowest thing in the corpus is two orders of magnitude inside
it. A signalled *parent* is reported as a harness error, because `liss` maps a signalled **child**
to `Error: programma abrupte cessavit` and exit 1 (§9.5), so that arm means the compiler itself was
killed.

**Blessing.** `LISS_BLESS=1 cargo test --test conformance` rewrites every expectation file from
actual output. Read the diff before committing it: blessing a regression is the one way this suite
fails silently. A failing comparison prints a unified-ish diff of each stream that differs, with
the first differing line called out, and ends with the line
`re-bless with: LISS_BLESS=1 cargo test --test conformance`.

## The diagnostic catalogue: `catalogus.rs`

The catalogue of every diagnostic the source can emit, and whether a fixture produces it. A new
message added without a fixture changes the `insta` snapshot and fails review — which is what keeps
this suite from rotting.

### What it parses, and the consequence

**It parses, and does not scan.** `extract_sites` reads `crates/<c>/src` for six crates —
`lingua-serpentis`, `latinitas`, `runtime`, `ir`, `codegen`, `abi` — with a
**non-recursive `read_dir`**, keeping `.rs` files and sorting them, and hands each to
`syn::parse_file`. A file that does not parse is a panic naming it, deliberately: softening that
would silently drop every site in the file.

That list is complete: `runtime` joined when `roman.rs` and `exitus.rs` moved out of
`lingua-serpentis`; `abi` joined on creation, ahead of need, so that no message could slip in
unrecorded between its birth and its marker; `ir` and `codegen` joined with
`Defectus::novus(`.

**Nothing under `src/bin/` is visible to it.** That is why the `liss` driver lives in `cli.rs` and
`src/bin/liss.rs` is a shim that says nothing — its own header records the reason. New user-facing
sentences belong in module files.

**A unit test's Latin is not a diagnostic.** `visit_item_mod` and `visit_item_fn` stop at any item
carrying `#[cfg(test)]`. This replaced a `production_source` that truncated each file at the first
`"\n#[cfg(test)]\n"`, which was exact only because every file in this workspace happens to have at
most one such module and it always runs to EOF; the attribute check holds however many a file
grows and wherever they sit.

**This is why the source may carry block comments** everywhere except `crates/abi/src`. Until
2026-08-09 the ratchet searched source text for eleven marker strings, and could not tell a
statement from a sentence about one — so `CLAUDE.md` forbade `/* … */` under all of `crates/*/src`,
and a comment could not sit between a marker and its literal. Comments are not tokens. See
`docs/superpowers/specs/2026-08-09-test-harness-libraries-design.md`, and §6 there for why
`abi/tests/abi.rs`'s own line-oriented scanner keeps the rule.

### The eleven markers

Eleven predicates over `syn` nodes. The right-hand column is not decoration: the span is taken from
the named sub-node, because `Expr::MethodCall::span()` starts at the *receiver*, which for a
multi-line chain is a different line from the call.

| Marker | AST node | Span from |
|---|---|---|
| `LinguaError {` | `Expr::Struct`, path ends `LinguaError` | `path` |
| `Warning {` | `Expr::Struct`, path ends `Warning` | `path` |
| `ProgramError::structural(` | `Expr::Call`, callee path ends `ProgramError::structural` | `func` |
| `.err(` | `Expr::MethodCall`, method `err` | `method` |
| `Err("` | `Expr::Call`, callee path `Err`, argument list **opening** with a string literal token | `func` |
| `.ok_or("` | `Expr::MethodCall`, method `ok_or`, argument list opening with a string literal token | `method` |
| `ok_or_else(\|\| format!(` | `Expr::MethodCall`, method `ok_or_else`, closure body an `Expr::Macro` with path `format` | `method` |
| `eprintln!(` | `Expr::Macro` **and `Stmt::Macro`**, macro path `eprintln` | `mac.path` |
| `liss_panic(` | `Expr::Call`, callee path ends `liss_panic` | `func` |
| `Defectus::novus(` | `Expr::Call`, callee path ends `Defectus::novus` | `func` |
| `NUNTIUS_` | `Item::Const` / `Item::Static`, ident begins `NUNTIUS_` | `ident` |

Two of those rows are written the way they are because the first attempt was wrong and the
differential test against the old scanner said so. **`eprintln!` needs both node kinds:** a macro
in statement position — `eprintln!("…");`, which is how all twelve of `cli.rs`'s are written — is a
`Stmt::Macro`, not an `Expr::Macro`. **`Err("` is about the first token, not the first argument:**
the marker always meant the quote follows the paren, and `Err("numerus Romanus vacuus".into())` in
`roman.rs` satisfies it while its first *argument* is a method call.

**Why each marker exists is unchanged by how it is found.** `ProgramError::in_file` is deliberately
absent: it re-wraps an existing error rather than producing one. The three `String`-shaped markers
exist because `roman.rs` is the one module whose errors are bare `String`s, wrapped into a
`LinguaError` only later in `lexer.rs`; its six Latin diagnostics carry none of the first five and
would otherwise be invisible.

The last three are the back end's, added in Task 19 with the crates that own them (spec §10.3).
`liss_panic(` finds the runtime's raises; `Defectus::novus(` finds the compiler's own failures;
`NUNTIUS_` finds the signal handler's messages, which are not call sites at all but `static` byte
constants. They were deferred to Task 19 on purpose: `abi` re-raises `divisio per nullum`,
`modulus per nullum` and `index extra fines: {} (longitudo {})`, and `interp.rs` raised all three
too; adding `liss_panic(` while both implementations stood would have made each pattern non-unique,
so six sites would have read `ambiguous` and the count would have recorded a *loss* of coverage
where nothing was lost. The interpreter went in the same commit.

**The runtime writes its sentences inline at the call site**, never as a reference to a shared
table, and §10.3 makes that a rule rather than a habit: a site written
`liss_panic(nuntii::DIVISIO_PER_NULLUM.as_ptr(), …)` carries no literal of its own and reports
`UNPARSED`. Parsing does not make an absent literal present.

### Finding the message literal

**`primum_literale`** takes the first string, byte-string or C-string literal in the site's own
token stream, descending into groups. `abi`'s messages are byte strings and C strings —
`b"Error: "`, `c"acervus vocationum exhaustus est"` — never plain `&str`, because the crate is
`no_std`-shaped and async-signal-safe; all three are `syn::Lit` arms and all three are read. A char
literal is not: `Lit::Char` falls through, which is what lets
`self.err(if c == '"' { "chorda non clausa" } else { … })` find its message.

That the three arms are load-bearing is why
`byte_and_c_string_literals_are_extracted_like_any_other` exists. An edit that dropped `Lit::CStr`
would silently stop reading every message the runtime can say — with the ratchet's own header
unmoved, because the sites would go `UNPARSED` rather than disappear.

The scanner this replaced took the first literal within a **300-byte forward window**, stopping at
the next marker, growing the window to the next `char_boundary` so it would not split a `°`, and
skipping a `"` written as the char literal `'"'`. All of that was a consequence of reading bytes,
and all of it is gone. A subtree search is also strictly narrower: the window could reach past a
site's own arguments into a sibling's literal.

### The one guard

There were four. Three are now structural:

- **A declaration is not a raise.** A return type is a `Type`, a function definition an `ItemFn`, a
  struct definition an `ItemStruct` and an impl header an `ItemImpl` — none is an `Expr::Struct` or
  an `Expr::Call`. The old guard tested whether the text before a marker ended in `->` or `fn`, and
  because it never considered `struct` or `impl … for`, it carried five phantom rows of its own;
  see §8 of the 2026-08-09 design for which.
- **Prose about a diagnostic is not a diagnostic.** The old guard looked left from the marker for
  `//` and assumed no block comment existed anywhere in a scanned tree. A token stream has no
  comments.
- **A `NUNTIUS_*` constant is a site where it is declared, and nowhere else.** A declaration is an
  `Item::Const` or `Item::Static`; a use is an `Expr::Path`, and only the first is visited.
  `custos.rs` uses each of its two message constants two or three times, and each use carries no
  literal, so without this one message became four rows, three of them `UNPARSED`.

The fourth is semantic, has no structural equivalent, and is kept verbatim:

**A pattern that is nothing but holes identifies nothing.** If removing every `{}` leaves an empty
string, the site is dropped: `eprintln!("{}", e)` prints an error built elsewhere, and matching it
against observed messages would match every one of them. Same reasoning that keeps
`ProgramError::in_file` out of the marker list — forwarding a diagnostic is not raising one.

**The three meta-tests are kept**, and two of them can no longer fail.
`a_diagnostic_entry_points_own_definition_is_not_a_site` and
`a_marker_in_a_comment_is_prose_and_a_marker_in_code_is_not` hold by construction; they are
statements of intent at negligible cost, and the second now asserts the positive case the old rule
forbade — a `/* … */` block quoting `NUNTIUS_`, `Defectus::novus(` and `Error: ` in prose, which
invents nothing. `only_a_nuntius_declaration_is_a_site_not_its_uses` still requires exactly two
`custos.rs` sites and no `UNPARSED` among them. Both fragment-level tests go through `sites_of`,
which wraps a fragment in `fn probatio() { … }` before parsing: at file level `eprintln!(…);` is an
`Item::Macro`, which no visitor here looks at, so a bare `parse_file` would find nothing and the
test would pass for the wrong reason.


### Matching, and the four statuses

`observed_messages` runs every fixture and collects **stderr only**, which is complete: every
diagnostic — errors and Latinity warnings alike — is rendered to stderr, and stdout carries only
what a program printed with `scribe`. Each line is reduced to what follows its first `": "`.

`matches(pattern, observed)` compares the literal segments between `{...}` holes, in order, anchored
at both ends unless the pattern begins or ends with a hole. The anchors matter: with an unanchored
`find` alone, any message that is a *prefix* of a longer one matches it — the interpreter's
`'longitudo' seriem expectat` would have been reported as fixtured by the checker's
`'longitudo' seriem expectat, non 'ni'`, and its `'{}' non est series` by the checker's
`'x' non est series (typus 'ni')`. Both are distinct diagnostics from distinct phases, and counting
them as covered would be padding.

Uniqueness is decided on the pattern **as written, byte for byte**: two patterns that differ only in
where a `{}` hole sits produce different sentences for a user and are therefore different messages.
Normalising the holes away would collapse genuinely distinct diagnostics and understate coverage in
the opposite direction.

| Status | Meaning |
|---|---|
| `UNPARSED` | No literal within the window — the site's message is invisible to the ratchet |
| `UNCOVERED` | No observed message matched the pattern |
| `ambiguous` | The pattern was observed, but several sites share it |
| `fixtured` | The pattern was observed **and** is unique across the whole workspace |

`fixtured` means *uniquely attributable*, and nothing weaker. Nothing ties an observed message back
to the site that emitted it, so when several sites are worded identically, provoking any one makes
all of them look covered. So a shared pattern that was observed marks *every* site sharing it
`ambiguous`; a shared pattern nobody provoked is still plainly `UNCOVERED`. **Adding a fixture never
moves a site from `ambiguous` to `fixtured`** — the ambiguity is in the wording, not the coverage.
Only rewording one of the sites, or attributing messages some other way, could do that. Twelve rows
once sat in that trap: `emissio.rs`'s `munus declarari non potuit`, `munus vocatum non inventum` and
`datum declarari non potuit` (two sites each), `defectus internus: memoria violata est` (`custos.rs`
and `defectus.rs`), and `imago non coniuncta est` (four sites in `nexus.rs`). Each now carries a
tail naming which failure it is, and every pattern in the workspace is unique.

Each row prints as `{file}:{line}  {pattern}  → {status}`, under a header the snapshot pins:

```
118 sites · 70 fixtured · 0 ambiguous · 46 uncovered · 2 unparsed
```

`UNCOVERED` and `UNPARSED` are legal states, but they are *visible* states — the header is the
number the project is choosing to live with.

### The blessing net: `nuntii.rs`

`LISS_BLESS=1` rewrites all 86 `nuntii` files from actual output, and **blessing a regression is the
one way this corpus fails silently.** Nothing before could tell a diagnostic that changed *shape*
from one that changed *sentence*.

`nuntii.rs` closes that. It reads the committed files — not the compiler, which is what it is
guarding — and snapshots every diagnostic in the corpus reduced to `(genus, scida, linea,
sententia)`, with the framing discarded: 86 fixtures, 99 diagnostics. **A blessing that moves this
snapshot changed a sentence or a line number, and neither is a framing change.** Two companions keep
it honest: every non-empty line of every file must be accounted for as a diagnostic or as framing
belonging to one, and every rendered numeral must be the way the language writes that number — the
corpus being the widest sample of rendered numerals there is.

It reads through `support::diag::parse_stderr`, the same parser `judge` uses, which makes it a
second check on that too: a parser that quietly returned nothing would empty the snapshot rather
than pass.

Bless in **narrow commits, never one wide one**, and check the diff mechanically rather than by eye:
every added line must be framing, every removed header must pair with an added one carrying the
identical message, and no file may appear or vanish — `bless` deletes a `nuntii` whose stream came
back empty, so a fixture that silently stopped producing stderr would disappear rather than fail.

## The robustness invariant

> Within a timeout, the process terminates with either exit 0 and a stderr that is nothing but
> warning blocks, or exit 1 and a stderr whose last error block is well-formed. Any other outcome is
> a defect.

`support/diag.rs` is that specification; `robustness.rs` and `generative.rs` are its two callers.
Every input runs in its own process, because a stack overflow aborts and cannot be caught in
process.

### The grammar

`parse_stderr` reads the whole of stderr into blocks plus **residue** — the lines that belonged to
no block. Residue is a finding, not noise: under the block grammar the rows after a header are
framing the language wrote, so a line that is neither a header nor framing is something else
entirely (a panic, a leaked English sentence, a half-written message).

A block is a header at column 0 — `erratum: ` or `monitum: ` — then an optional `  in …` location
row, then optional gutter rows. Nothing separates two blocks but the next column-0 severity word,
which is what the parser keys on; a blank line would put a genuinely empty line into stderr, and
several consumers iterate it.

**It reads errors only, in the sense that matters.** A warning parses as a `Monitum` block and is
never the failure `judge` looks for — if one counted, a run that warned and never failed would be
accepted as a well-formed failure. `a_warning_is_not_a_failure_however_it_is_framed` pins that.

Two consequences worth stating, because both were faults in the one-line era:

- **The last *block*, not the last line.** The last physical line of a failing run is now a caret
  row. The tempting fix of reading the *first* line instead silently breaks the case where warnings
  precede the error.
- **The exit-0 branch broke just as hard, and less visibly.** It required every stderr line to begin
  with `Monitum`, which a warning's location, gutter and caret rows do not.

### `judge`

Quoted spans are stripped before the lexical rules run — they are user input echoed back, and the
rules are about text the *language* wrote. Without that, `signum ignotum: '5'` would trip the
no-Arabic-digits rule for quoting the user's own `5`, and `variabilis ignota: 'Semicolon'` would
trip the Rust-type-name rule for a user who named a variable `Semicolon`.

| Rule | Trips on |
|---|---|
| `terminates` | Still running at the deadline |
| `no abort` | Killed by a signal, **or** the message `programma abrupte cessavit` |
| `clean success` | Exit 0 with residue, or with an `erratum` block |
| `no panic` | Exit 101 |
| `diagnostic present` | Exit 1 with no error block |
| `diagnostic grammar` | Exit 1 with residue — a line belonging to no block |
| `no internal defect` | A message beginning `defectus internus: ` |
| `no leak` | The message `memoria non liberata est` |
| `file exists` | A location naming a file not in the program |
| `line in range` | `linea 0`, or past the end of the named file |
| `numeral round-trips` | A location whose numeral is not how the language writes that number |
| `caret aligns` | A gutter numeral disagreeing with the location's, or a caret past the line beneath it |
| `message non-empty` | An empty message |
| `Latin only` | An `ENGLISH_WORDS` token: `file`, `must`, `have`, `extension`, `reading`, `directory`, `invalid`, `expected` |
| `no Arabic digits` | An ASCII digit outside a quoted span |
| `no Rust type names` | One of the 24 `RUST_TYPE_NAMES` (`Semicolon`, `RParen`, `LBrace`, …) |

**`caret aligns` is the only automated check that a caret points where it claims.** Nothing else
notices a renderer counting columns in bytes, and `°` — two bytes, one column — begins every numeral
in the language. The caret may sit *at* the end of its line, which is the insertion point a missing
`;` is reported at, but never past it.

**`numeral round-trips` is what keeps an Arabic line number out of the location row**, and it is
only possible because `abi` stopped writing one (`abi.md` §13). Writing its test found the hole
in the first draft: `linea 3` parses as no numeral at all, which left the line unset and skipped the
check entirely — so the one shape the rule exists to ban was the one shape it passed.

`no Arabic digits` reaches further than the message now. The honest formulation is *the rule covers
every byte the language wrote and exempts only the bytes the user wrote*: the header outside quoted
spans, the location row except inside the quoted file path (`a1.liss` is a legal filename), and the
gutter numeral. **The echoed source and the caret row are exempt by construction** — the snippet is
user input reproduced verbatim, and the likeliest thing in it is the Arabic digit the diagnostic is
complaining about. Any stricter rule makes `conformance/errores/numerus-arabicus`
self-contradictory.

Any status outside {0, 1, 101} came from `initium` and is legitimate. English is checked *before*
digits, so `Error: file must have .liss extension` is reported as an untranslated string rather than
as a stray numeral.

Three of those rules exist because a well-formed Latin sentence can report something no input may
provoke, and would otherwise pass every lexical rule there is:

- **`ABRUPTE_CESSAVIT` = `programma abrupte cessavit`.** `BINARY` is `liss`, so the process the
  harness spawns is the compiling *parent*, which does not die by signal; the child's death reaches
  the harness only as this sentence. Spec §9.5's own justification for the mapping is wrong — a bare
  139 takes `judge`'s `code != 1` arm and is `Acceptable`, and the mapped form parses as a
  well-formed structural diagnostic — so the mapping closed no hole. This constant is what makes
  §8's guarantee that no compiled program dies by signal a fuzzed claim.
- **`DEFECTUS_INTERNUS` = the prefix `defectus internus: `.** The compiler admitting to a bug must
  never be `Acceptable`, since finding compiler bugs is what these fuzzers are for. **Task 12's
  Critical had exactly this shape**: the legal program `redde nullus; scribe("numquam");` made
  release codegen emit `defectus internus: munus componi non potuit`. A prefix rather than a list,
  because the list is §10.3's to grow.
- **`MEMORIA_NON_LIBERATA` = `memoria non liberata est`.** §7.5's argument against a distinctive
  exit code such as 70 is right but incomplete: exit 1 with a well-formed Latin sentence was
  accepted too. This constant is what makes the argument true.

All three are asserted directly rather than left to the fuzzer, because the input that provokes any
of them is a compiler defect nobody can write on purpose —
`a_child_killed_by_a_signal_is_an_abort_however_politely_it_is_reported` and
`the_compilers_own_failures_are_violations_and_not_diagnostics`. Both also check a neighbouring
sentence of the same shape (`acervus vocationum exhaustus est`, `memoria exhausta est`) is still
`Acceptable`, so the rules are keyed on the message and not on "exit 1 with a structural error",
which would condemn most of the corpus.

`RECORDED_RULES` is violations the suite *tolerates*, currently one: `"terminates"` (D-20).
Shrinking it is how progress is measured; never add to it without adding a register entry too. Five
entries have been retired, and none by weakening its rule — `no panic` (D-06), `Latin only` (D-11,
D-22), `no abort` (D-04, D-05), `no Rust type names` (D-02), `no Arabic digits` (D-12). Both
underlying checks stay in `judge`; they are what keep these fixed.

### Deadlines, and why there are two

`T` = 60 s is the wide deadline: the fuzzer's, and the one curated case whose cost is genuinely
codegen's. It was 10 s until `BINARY` became `liss`, and had to rise because each case now pays a
Cranelift pass and a link before it runs at all. Measured on this machine with `liss run`:

| case | debug | release |
|---|---|---|
| `series longissima` (5,001-element literal) | **17.1 s** | 0.67 s |
| `nomen longissimum` (100,000-char identifier) | 0.39 s | — |
| `numerus longissimus` (5,000 `M`s) | 0.4 s | — |
| `parentheses nimium implicatae` | 0.05 s | — |
| `recursio infinita` | 0.6 s | — |

One case dominates, and the 25× gap is the debug build of Cranelift rather than anything about the
program: it answers `V̅I` — 5,001 — correctly in both builds. Sixty seconds is roughly 3.5× the
measured debug worst case.

`MORA_ARTA` = 5 s is the tight deadline, and it exists because raising `T` globally silently retired
a different guard. `nomen longissimum` timed out under the old 10 s deadline until D-23 was fixed:
`latinitas::analyze`'s `for i in 0..=w.len()` loop hashed a `&w[..i]` slice on every iteration —
O(n) per iteration, O(n²) in the identifier's length. Measured against the binary on 2026-08-03:
20,000 chars 1.26 s, 50,000 ~6.2 s, 100,000 ~24 s. Re-measured 2026-08-05 after the fix: 318 ms,
317 ms, 322 ms — flat, the remainder being the process's own dictionary build. **Nothing else in the
workspace times `analyze` at all**, so under a 60 s deadline a full D-23 regression passes
unnoticed. The deadline is therefore **data, like the expected rule beside it**.

`pathologies()` returns `(name, source, deadline, expected_rule)`; `None` means the case must
satisfy the invariant, and today every one does.
`every_curated_pathology_matches_its_recorded_verdict` reports a case that *starts* passing as
loudly as one that starts failing, with the instruction to update `pathologies()` and the
register. `chorda multilinea` is kept because D-03 is **not** detectable by this invariant: a wrong
line that is still inside the file passes `line in range`. Its source is a string spanning three
lines and then a missing semicolon on source line 5, immediately before `redde` on line 6; while the
drift lasted the reported line was 4, matching the string's two embedded newlines. **D-03 was fixed
in `efa0f57`, and its fixture is `conformance/errores/linea-post-chordam`** — not
`conformance/defectus/D-03-linea-post-chordam`, which is what `robustness.rs`'s comment beside this
case still says and has said since before the fix. Every fixed defect's fixture moved out of
`conformance/defectus/`, which now holds D-13's directory and nothing else. The case stays as proof
that the input remains inside the error model.

### The mutation fuzzer

`no_mutation_of_any_known_program_escapes_the_error_model` draws from `seed_corpus`, every `.liss`
file under `examples/` and `conformance/`, and mutates one at a time. `support/mutate.rs` is a
SplitMix64 `Rng` and seven operations: delete a char, duplicate a char, swap two, substitute an
`INTERESTING` char, truncate, insert an `INTERESTING` char, and duplicate a span of up to 16 (good
at producing unbalanced brackets). `INTERESTING` is the sixteen characters the lexer treats
specially, including `°` and `5`. Mutation operates on `char`s rather than bytes, so the output
stays valid UTF-8 and the test is of the language rather than of `read_to_string`. The default
budget is 300 cases at seed `0x5EED`; `LISS_FUZZ_CASES` and `LISS_FUZZ_SEED` override both for a
soak. It stops after five repros, because five is plenty and the rest is burnt time.

## Generated programs: `generative.rs`

Mutation rarely gets past the parser, so this is what exercises the checker, codegen and the runtime
in depth; `proptest` shrinks a failure to a minimal repro, which a hand-rolled generator cannot.

**The generator is narrow, and a later reader should not mistake it for coverage of the language.**
It produces `ni`-typed arithmetic and nothing else: the five binary operators `+ - * / %`, unary
negation, three variables, expressions at most four levels deep. No `si`, no `dum`, no `lista`, no
type other than `ni`, no recursion, no multi-file programs. Every program it emits is meant to
type-check, so generation exercises the checker's *accept* path only; every reject path is still
reached solely by the hand-written `conformance/errores/` fixtures. Each expression is wrapped in a
two-function program whose `initium` calls `computa(°VII, °III, °II)`.

**Every case is a process spawn**, and spec §10.2 names this file as the milestone's real cost risk.
`liss run` on a program of this shape costs **384 ms** against the tree-walker's **308 ms**, so the
counts are the whole budget and they are written down:

| Constant | Value | What it buys |
|---|---|---|
| `CASUS` | 64 | Property draws; ~25 s under `liss`, where 200 would have been ~77 s |
| `SAMPLES` | 50 | Draws for the reach test, each a spawn |
| `SPECULATIONES` | 2000 | Draws for the statistical guard; no spawns at all |
| `RETRACTATIONES` | 256 | Shrink cap; proptest's default 2000 would be ~13 minutes per failure |
| `PONDUS_NULLIUS` | 3 | Weight of an explicit zero among the leaves, against 4 each for a number and a variable |

The two tests run on separate libtest threads, so wall time tracks the **larger** of `CASUS` and
`SAMPLES` rather than their sum. `PONDUS_NULLIUS` is what keeps `divisio per nullum` reachable at 64
draws: a uniform `0..200` leaf put the error path at **1.75% per draw** (measured over 400 samples),
which is 97% over 200 cases and 67% over 64 — so roughly one `cargo test` in three would have run
the property without ever producing a diagnostic, and its error half only exists when one is
produced. Weight 3 puts the error path at **13.75% per draw**, or 99.99% over sixty-four, while 86%
of programs still run to completion. Weighting costs no process; a case costs one.

Three tests:

- **`generated_programs_never_escape_the_error_model`** — the property. Each program is written to a
  temp dir, run through `run_binary` with a 10 s timeout, and judged. `Acceptable` passes; a
  violation whose rule is in `RECORDED_RULES` passes; anything else fails with the rule, the detail
  and the source.
- **`generation_reaches_the_runtime`** — pins what the property cannot. The property accepts any
  recorded-rule violation, and `no Rust type names` is the rule a *parse error* trips, so a
  generator that regressed until every program was a syntax error would satisfy it in silence. An
  all-`ni` arithmetic program has exactly two legitimate failure modes, both raised at run time by
  the compiled image: the last stderr line's message must be exactly `divisio per nullum` or
  `modulus per nullum` — verbatim, because `contains` would let a parse error that merely mentioned
  division count as a run. The reach was measured independently over 400 samples: **388 exited 0, 12
  exited 1, and all twelve were those two sentences. Zero parse errors, zero checker errors, zero
  signals, zero timeouts.** A final `assert_eq!(reached, SAMPLES)` closes the vacuous-loop hole.
- **`the_generator_still_draws_a_zero_divisor`** — the ratchet on `PONDUS_NULLIUS`. It spawns
  nothing: 2000 trees are drawn and rendered, and sources containing an *immediate* `/ nullus` or
  `% nullus` are counted. That is a deliberate under-count — `(x / (nullus + nullus))`,
  `(x / (-nullus))` and `(x / (a - a))` all divide by zero and none is matched — and the measured
  syntactic rate is 8.0% against a true error rate of 13.75%, so it is a strict lower bound. The
  floor asserted is 4%, half the measured rate and fifty times the **0.15%** the unweighted leaf
  produces: deleting the weighted arm drops the count to 3 of 2000, and the failure message names
  `PONDUS_NULLIUS` by name.

**D-26 is excluded by a check rather than by luck.** A terminator in a `dum` **init clause** is the
one program shape whose meaning is an open language question — the two backends answered it
differently while there were two — so `caput_dum_terminatum` rejects `dum (redde …; …)` and its
`abrumpe` and `continua` spellings, and both generating tests check every program they emit against
it. The generator cannot draw the shape today, which is precisely why the guard is a function and
not a sentence: a widening that adds loops trips it on the first draw and is sent to D-26 instead of
quietly compiling a program nobody has decided the meaning of. The guard is textual, reading the
source the generator *renders*, so no change to the `Expr` tree routes around it.

**The init clause is the whole of D-26, and that is provable rather than hopeful.** A `dum` header
has three clauses, and only two of them could in principle carry a terminator. The *step* clause
cannot: `parser.rs`'s `assign_no_semi` admits only `<ident> = <expr>` there, so `dum (…; …; redde
x)` is a parse error — it never reaches lowering at all, and never reached either backend's reading
while there were two. The init clause is therefore the only route, and matching on it alone is
complete, not merely narrow.

`clausula_initialis` finds the init clause by tracking parenthesis depth and string quotes, and
returns `None` where the header closes without a `;` — `dum (verus)` is not a three-clause header,
and reading past its `)` for a `;` would find the one ending the first statement of the **body**,
reporting the ordinary program `dum (verus) { redde nullus; }` as D-26's shape. That was a real bug
in the first draft, caught by the negative. `verbum_est` rejects `dummodo`, `nondum`, `reddendum`
and a `continua` inside a string literal; substring matching over-excludes, which is the safe
direction but a silent one. `the_guard_recognises_the_shape_it_excludes_and_nothing_else` is what
stops the guard degenerating into a function that always answers `false` once per drawn program and
protecting nothing. Its positives are D-26's own three spellings in the init clause; the first
of its negatives is a terminator in the loop **body**, which is an ordinary program both backends
agree about and which the corpus holds as `conformance/currunt/memoria-caput-dum-redditum`. The
behaviour is pinned instead by `caput_dum_terminatum_componitur` in
`codegen/tests/regimen.rs`, deliberately **not** by a `conformance/` fixture: with one backend
left, a fixture would settle the question by implication rather than by decision.

## The CLI suites

**`cli.rs`** covers what the fixture format cannot express, one named test each. Twenty-two of
its twenty-four spawn the binary through `support::proc`; the other two are
`wait_or_kill_returns_the_status_of_a_child_that_finishes` and
`wait_or_kill_reaps_a_child_that_outlives_its_deadline`, which drive that module's shared wait
helper directly — its deadline arm had no coverage of its own while it was three copied loops.
`BINARY` is `liss`; eighteen of the twenty-two were untouched by that flip, because the front end
and `program::load_dir` are shared and the runtime reproduces the rest. It covers argv arity, `.liss` extension enforcement and its interaction with
`is_dir()` (a *directory* named `x.liss` runs as a program folder), missing paths, unreadable files
and directories, non-UTF-8 content, the stdout/stderr split, real process exit status including the
truncated and negative cases, hidden-directory skipping (D-17), broken pipe (D-06), unbounded
recursion (D-04) and deep expression nesting (D-05).

Its `USUS` constant — `liss`'s three invocation lines and the sentence disambiguating a folder named
`run` — is **duplicated from `liss_cli.rs` deliberately rather than shared**. The two files drive
the same binary by different routes, and a shared constant would let a reworded usage string stay
green in both because both read the new wording. Two independent transcriptions is the point.

`deep_recursion_runs_because_there_is_no_limit` asserts ten thousand frames complete — two orders of
magnitude past the interpreter's `VOCATIONES_MAXIMAE = 64`, and past the ~2,000 it managed even
before the cap, while staying inside the 40,000–80,000 a default 8 MiB stack allows (§3.3).
`currunt/recursio-profunda` makes the same point in the corpus at 5,000; a handler that fired early
would fail both, and only these two would notice.

**`liss_cli.rs`** drives `liss` directly and owns the compiling surface: the `./run` rule, `build`'s
output naming, `--exi`, `--ad` cross-compilation (checked by reading `e_machine` at ELF offset 18 —
`EM_AARCH64` is 183), the parent's Latinity warnings arriving before the child's output (via
`liss_merged`, which gives stdout and stderr **one** open file description so the write order is
recoverable), and the destructive-write cases: `/dev/full`, a symlink to `/dev/full` that must
survive, and `--exi /dev/null` which must come back out with its mode unchanged.

**Of that pair the symlink is the reproducible one, and the device case cannot be.** `build` removes
a destination it truncated, and `O_TRUNC` truncates nothing on a device node, so after
`a_build_that_cannot_write_its_output_says_so` there is nothing to clean up and the removal it
guards against never actually fires: as an ordinary user the unlink is refused silently, and only
run as root in a container would it delete `/dev/full` from a devtmpfs that will not put it back.
`a_failed_write_through_a_symlink_removes_neither_end` puts the same hazard on an object the test
owns. `open` follows the link to `/dev/full`, so nothing is truncated and every write fails; but
`remove_file` acts on the **link**, which is ours. Classify the failure as destructive and the link
is gone — caught without root and without touching anything outside a temporary directory.

`run_leaves_nothing_behind_when_the_parent_is_killed` `SIGKILL`s the parent while the child still
runs and asserts `TMPDIR` is empty, which no `Drop` guard or signal handler could cover — the image
has had no name since before the exec. `a_child_killed_by_a_signal_is_reported_in_latin` finds the
child through `/proc/<pid>/task/*/children`, kills it, and requires
`Error: programma abrupte cessavit` and exit 1.

## Whole programs: `programs.rs`

The pipeline end to end. Every case here used to call `lingua_serpentis::run`,
`run_with_warnings` or `run_program_with_warnings`; Task 19 removed those entry points with the
interpreter, so "run this program, check what it printed" now has exactly one spelling: `liss run`.
**Not one expectation changed in the move.** Where a test asserted something structural about a
`ProgramError` or a `Vec<Warning>` — `err.line == 1`, `warnings.len() == 1` — the assertion is now
made against the *rendered* form of the same fact, because a process has two streams and no return
type.

Three helpers set the discipline: `emissio` requires exit 0 **and** an empty stderr, so a program
that printed the right thing and then exited 3, or drew an unexpected Latinity warning, no longer
passes; `emissio_cum_monitis` is for the two sources that legitimately warn, and its expected lines
are the same bytes `conformance/currunt/exemplum-praecisio`'s blessed `nuntii` records; `erratum`
requires exit 1 and an empty stdout.

## The typed IR: `ir.rs`

Snapshots of `check_program_typed`'s output for six fixture programs, rendered by a hand-written
`dump` rather than by `Debug`, because a `Roman` prints as its little-endian digit bag and a
reviewer needs to see `V` — a snapshot nobody can read is a snapshot nobody checks. The snapshots
are the contract codegen is written against: every expression node's `Ty`, every block's
`declares`, and every place lowering does or deliberately does not desugar.

Beside them are targeted assertions: every expression node carries a type and comparisons yield
`veritas` while their operands do not; a block declares exactly its own slots; a shadowing
declaration gets a fresh slot and the outer name comes back; parameters are the first slots;
`aut si` arrives already nested from the parser; `dum` is **not** desugared into `indem`, because
`continua` must still run the step; a `dum` header declares its own slot, and every owned slot is
in exactly one `declares` — the invariant a codegen walking blocks relies on, and the one a header
outside the `Block` tree used to break. `a_redde_out_of_a_dum_still_reaches_the_header_slot` is that
regression in the shape it was found: a heap `voc` in a `dum` header whose only exit is a `redde`,
which never reaches the post-exit teardown, so §7.5's live-allocation counter turned a legal program
into `Error: memoria non liberata est`.

## Codegen: recorded answers and the object

`codegen/tests/support/mod.rs` is the pipeline spelled once. Until Task 19 `par_est` was a live
**differential**: it ran each source through both backends and required the two `Rendered` values
equal, so the oracle was the interpreter the corpus is blessed against rather than a hand-copied
string. That is what made one test per operator cheap enough to write, and why forty-seven of them
exist. On **2026-08-08**, with both backends still standing, every one of those forty-seven answers
was captured from the interpreter and written into its call site — spec §10.4's discipline,
*"everything that uses the interpreter as a source of truth happens while it still exists"*. **So no
expectation there was ever produced by the compiler**; a later reader may treat every string as the
tree-walker's own answer.

The `par_est` family compares all three streams at once so none can be forgotten separately:
`par_est` (stdout only, empty stderr, exit 0), `par_est_erroris` (empty stdout, a diagnostic, exit
1), `par_est_plene` (all three given), and `_fasciculorum` variants for multi-file programs.
`casus_par_est` runs a `conformance/currunt` case against its blessed expectation, with **one
subtraction, by identity**: the checker's Latinity warnings, obtained from the same
`latinity::lint_program` call on the same `ParsedProgram` and rendered the way
`lingua_serpentis::render` renders them, are `strip_prefix`ed off the front of the blessed stderr —
because the CLI writes those and *then* spawns the child (§9.5). It used to be a `Monitum` prefix
filter, and that was wrong: **the runtime writes `Monitum` lines too**, and `abi`'s
`ingressus.rs` writes the D-10 truncated-exit-status note at run time, which is exactly
`conformance/monita/status-exitus-redactus`. A filter keyed on the word would have deleted a
diagnostic the compiled program is *required* to emit.

Exit status is not an afterthought in a `memoria-*` case, it is the whole assertion: §7.5 has the
runtime check the live-allocation count on the success path and exit **1** with `memoria non
liberata est` if it is nonzero, so "exit 0 with the blessed stdout" *is* "every allocation this
program made was freed exactly once". `abolere` clears a header's magic **volatilely** before
freeing — volatilely as of D-27; a plain store there was dead-store-eliminated in every shipped
build — so a double free of a temporary reports `defectus internus: memoria violata est:
verificatio` rather than corrupting silently.

**One thing no behavioural test can prove.** `Corpus::libera` leaves every slot it frees null, so a
*redundant* free of a slot is a no-op and over-freeing is systematically unobservable. "I added a
redundant free and the suite stayed green" proves nothing. What remains detectable is under-freeing
(the count), a redundant free of a **temporary** (the magic), a slot read after its scope died, and
a wrong jump target.

**`support::vocationes`** is the oracle for what output cannot show: the ABI entry points one
program calls, in the order codegen emitted them, read off the object's relocations and filtered to
a named set. Two frees in the wrong order leak nothing and print the same thing (§7.3.2); a copy
taken after the free it should precede usually returns the right bytes (§7.3.3). It is **only sound
within one basic block** — relocation offsets are `.text` *layout* order, and Cranelift orders
blocks for the branch predictor, so a loop body routinely lands after the code that follows it —
which is why every caller uses a straight-line program. Within a block, layout order is emission
order, because a call has side effects and nothing reorders one.

`exsequere` writes the image to a scratch directory, `chmod 0700`s it and runs it, retrying up to 64
times at 10 ms intervals on **`ETXTBSY` (errno 26)**: libtest runs these in parallel, and while one
thread holds a write descriptor on its own image another thread's `Command` forks and the child
inherits it for the moment between `fork` and `exec`. It is transient by construction, so the answer
is to try again rather than serialise the file.

### One test per §7.3 rule: `regimen.rs`

A `memoria-*` fixture proves the **sum** of §7.3's six rules and not any one of them: two
compensating bugs — a missing free and a double free of a different object — would balance the count
and pass. So each rule also has a test that isolates it. The fixtures are the integration; these are
the unit, and each one records what its own failure looks like.

- **§7.3.1, the statement root** — `radix_sententiae_semel_liberatur`. A call for effect is the only
  statement in the language whose expression tree leaves a live temporary nothing consumes, and
  `summare(°I, °II);` is the shape. Task 12 implemented this arm and had nothing that reached it
  (its own report §9.2 says so); this is the first program that does. Without the free it is one
  leaked `LissInt` per call and `memoria non liberata est`, exit 1 — and there are *two* calls, so a
  fix that freed one root per **function** rather than per **statement** would still fail.
- **§7.3.2, scope exit in reverse declaration order** — `ambitus_ordine_averso_liberatur`. The order
  is not observable from a program: the frees are independent, so a forward walk leaks nothing and
  every behavioural test passes. It is asserted **in the object** instead, by the order of the
  relocations against the two free entry points. The body declares `s` (a `voc`) and then `n` (a
  `ni`), so the teardown must call `liss_int_free` and *then* `liss_text_free`; a forward walk emits
  them the other way round.
- **§7.3.3, `Assign` is evaluate → free old → store** — `assignatio_vetus_post_lectionem_liberatur`
  in the object, where both `liss_text_copy`s and the `liss_text_concat` that consumes them must
  precede the `liss_text_free` of the old value, and `assignatio_in_ansa_libratur` behaviourally
  across a back edge. The object form is the stronger of the two: relocation order is not
  probabilistic and does not depend on the runtime noticing anything.
- **§7.3.4, `redde` frees every enclosing block, innermost first, and the parameter frame is one of
  them and only one of them** — `redditus_omnem_ambitum_semel_liberat`. `nomen` is a `voc`
  **parameter**: slot 0, and also the first entry of the body block's `declares`, so the walk must
  reach the body block from inside an `si` inside an `si` and arrive there exactly once. A walk that
  stopped at the innermost block leaks `intus` and `medium`; a walk that also swept `0..params` off
  `Func::params` double-frees `nomen`, which `abolere`'s cleared magic reports as `defectus
  internus: memoria violata est: verificatio`.
- **§7.3.5, `abrumpe` and `continua` free up to and including the loop body and *not* the `dum`
  header** — `abrumptio_et_continuatio_caput_dum_non_liberant`. Drop the post-exit teardown and the
  header's `voc` leaks (`memoria non liberata est`, exit 1); let the jumps free `header.declares`
  and the **`continua`** frees it, so the next iteration reads a null slot and `verifica`'s null leg
  reports `defectus internus: memoria violata est: verificatio`. **The `continua` is what makes that
  second half testable, and that is a finding rather than a detail.** With `Corpus::libera` leaving
  each freed slot null, an `abrumpe` that over-freed the header would be *harmless*: the loop is
  over, so the exit block's teardown frees null and nothing observes it. Written with `abrumpe`
  alone, this test passes over a codegen that gets §7.3.5 wrong.
- **§7.3.6, where `continua` jumps** — `continuatio_ad_gradum_et_ad_condicionem_salit`: the *step*
  block of a `dum`, the *condition* block of an `indem`. Both loops here `continua` on their
  **first** iteration, so a wrong target does not terminate — the test would time out rather than
  fail, which is worth knowing before reading the diff.

### Lists and calls, where the failure is a wrong answer

`series.rs`'s `omnis_generis_elementa_in_seriebus_vivunt` builds a list of every one of the six
element kinds (spec §5.3) — `veritas` 0, `lit` 1, `ni` 2, `mnr` 3, `nr` 4, `voc` 5, each
`| (1 << 3)` on the container. The kind code is the one number codegen writes and the runtime reads,
and a wrong one is never a compile error: `liss_list_free` would call `liss_int_free` on a `voc`'s
header, or skip a `ni` because its base is one of the two immediate kinds. **The two fractions need
declared types to be told apart.** An undeclared fraction literal falls back to `nr`
(`examples/praecisio.liss`), so a list literal on its own would exercise kind 4 twice and kind 3
never — and kind 3 is the one whose grain is XII, which is a wrong *answer* rather than a crash.

`vocationes.rs`'s `ALTERNA` is two packages that import **each other** and alternate: the shape a
single namespace cannot produce, and the one that needs both directions of the `FuncId` table to be
right at once. Twenty levels, and the depth is **asserted, not merely chosen** —
`fasciculi_se_invicem_vocant` demands the written string `"XX\n"`, so one alternation too few or too
many is a wrong answer rather than a shorter run.

### Compiling for the target nothing runs

Only the host's image is executed by these files — aarch64 under emulation came later and is
`aemulatio.rs`'s — so three tests exist to make the second backend build at all. Each asserts no
more than `compila` returning an object of the right architecture, which is what Cranelift's
verifier and register allocator running clean look like from here.

- **`regimen.rs`'s `regimen_utrique_machinae_componitur`.** Without it the second target's CFG is
  never built at all — and `brif` on `i8`, the block ordering and the probestack prologue are all
  backend code that a `si` inside a `dum` is the first thing to reach.
- **`series.rs`'s `series_utrique_machinae_componuntur`.** Without it the second backend never sees
  a `store` at a computed displacement, a `uextend` of an `i8` into an element word, or an `ireduce`
  back out of one.
- **`salve.rs`'s `both_targets_emit_an_object_of_their_own_architecture`.** It is the guard on spec
  §9.3's one codegen rule — `Signature::call_conv` always comes from
  `module.isa().default_call_conv()`. A literal call convention compiles and passes the host test
  beside it, and fails here only if it is wrong for the *other* target; a wrong-but-valid literal
  would still slip through, which is why the rule is also stated at the one place signatures are
  built.

### aarch64 under emulation: `aemulatio.rs`

It runs aarch64 images under `qemu-aarch64` — the second target's only execution anywhere. Earlier
tasks ran aarch64 images by hand and wrote the result into a report; this file is that evidence
turned into assertions the suite re-derives on every run. Nothing skips: a cross-architecture test
that quietly does not run is worse than none, because the suite reports green either way and only
one of those greens means the second
architecture works. Where `qemu-user` differs from a kernel is recorded exactly: `si_addr` is
faithful, so both branches of §8.2's discriminator are reachable; the guest stack does *not* grow on
demand, so `abi`'s `the_bounds_are_the_mapping_top_minus_the_limit` would fail under emulation
and stays on the host; `SIGBUS` cannot be provoked and is checked by inspecting the installer in
`target.rs`'s `the_guard_traps_both_sigsegv_and_sigbus`; the page size is the host's 4 KiB.
`current_dir` is set to the scratch directory not for tidiness but for **containment** — on an
uncaught guest signal `qemu-user` writes a 36 MB `qemu_<image>_<timestamp>_<pid>.core` into its
working directory, which for a cargo-spawned test is the crate root, and the one run that finds the
regression is the run that would litter the repository with it.

Four programs run there, and three of them carry a claim nothing else in the workspace holds.
`exemplum_salve_currit_sub_aemulatore` is the checkpoint — `exemplum-salve`, cross-compiled, linked
and run against the same `emissio` the host-compiled path is held to.
`nota_de_statu_redacto_transit_aemulationem` takes its expectation from
`conformance/monita/status-exitus-redactus` rather than restating it, so it cannot drift from the
corpus: `redde °CCC` exits 44 and the runtime says so on stderr in Latin with the truncated value
rendered as a Roman numeral. **Three things have to be right at once** for that to pass on aarch64 —
the three-byte `Roman` digit blob reaching `liss_exit` intact, the comparison that notices the
truncation, and the `Monitum: ` line the runtime writes at *run* time — and Task 10 checked them by
hand on a stand-in `liss_initium`, never on a compiled program.
`acervus_exhaustus_sub_aemulatore` restates `conformance/errores/acervus-exhaustus`'s message rather
than sharing it, and **`compilata` and the corpus run host images only**, so this assertion is the
*only* thing in the workspace holding that sentence on aarch64. A corpus-wide rename does not touch
it, which is the point and also the hazard: if the wording ever changes, this is the second place to
change. `memoria_liberatur_sub_aemulatore` runs `conformance/currunt/memoria-adde-in-ansa`, the
corpus's densest allocation case — a list grown in a loop, a list of text, and a list of lists —
because ownership is where a second architecture could plausibly go wrong without the verifier
noticing: `VERBUM`-scaled stores, argument registers, the spilled temporaries a nested list forces.
Nothing in this suite had ever checked it anywhere but on the host.

`acervus_displacitus_est_defectus_internus` is the negative control that makes the guard-page test
mean something: `QEMU_STACK_SIZE=1048576` moves the guest stack out from under the derived bounds,
the fault lands outside the band, and the classification must move with it to `defectus internus:
memoria violata est: signum`. Three things fall out — the band is *derived* rather than a constant
that happens to match; that sentence gets its first coverage on aarch64; and a `RLIMIT_STACK`
misconfiguration becomes self-diagnosing, because it makes this test pass and
`acervus_exhaustus_sub_aemulatore` fail, the opposite pattern from a real §8 regression where both
fail. The first costs ~1.4 s (16 MiB of guest stack to fill), the second ~0.10 s.

## The runtime ABI: `abi`

**`abi.rs`** reads the layout back directly — sign, digits, bytes, element words — rather than going
through a formatter, because a `liss_int_add` that produced the right *string* from the wrong digits
would pass a rendering test. The live-allocation count is one per process, so every test that reads
it runs under one mutex. `liss_panic` is observed in a **subprocess**: it does not return, so the
test binary re-executes *itself* with `LISS_ABI_SCAENA` naming the scenario, and the assertion is on
the exact bytes and status a real program would produce. A test-only hook that made `liss_panic`
return would have tested a `liss_panic` no compiled program ever calls.

**`custos.rs` has no `libtest` harness, and that is load-bearing.** Every bound §8.2 derives
describes the *main* thread's stack, and `libtest` spawns a thread per test even at
`--test-threads=1`; a recursion on a worker would fault in an ordinary `mmap` megabytes from the
band and take the handler's other branch, so the test would pass or fail for reasons unrelated to
what it claims. Owning `main` is the only way to put the scenario on the thread the runtime runs on.
It hand-rolls a six-test runner and its own `ok`/`FAILED` reporting. Its scenarios likewise
re-execute the binary, under `LISS_CUSTOS_SCAENA`.

**`emissio.rs` holds exactly one `#[test]`, and that is load-bearing.** Redirecting fd 1 redirects
it for the whole process, and `libtest` writes its own progress lines to that descriptor from the
main thread as other tests finish; with more than one test here, those lines land in the middle of a
captured value and the failure is intermittent and reads like a formatter bug. One test means
`libtest` prints only before it starts and after it ends. A separate binary is what keeps that
constraint from infecting the ownership tests in `abi.rs`.

**`runtime/tests/golden.rs`** pins `scriptio` against `tests/data/scriptio-aurea.txt`, captured
from the interpreter's `Value::display()` (§10.4). **The generator is gone and the data cannot be
regenerated, which is the point** — `crates/lingua-serpentis/examples/scriptio-aurea.rs` was deleted
in Task 19 with `value.rs`. A golden vector that can be regenerated by the implementation under test
is not a golden vector; if a row disagrees with `scriptio`, the row is right. `List` rows are
skipped here because `scriptio` has no list formatter — those rows are precisely what `abi`'s
`emissio.rs` adds, and it reads the same file.

## Environment assertions: `instrumenta.rs`

Not unit tests: they fail loudly in a shell that cannot build the runtime. Both musl targets must
have a `libstd`, and their `self-contained` directories must hold `crt1.o`, `crti.o` and `crtn.o`.
`qemu-aarch64` must be on `PATH`, asserted here beside the toolchain rather than skipped in
`aemulatio.rs`.

`the_stack_limit_is_finite_and_large_enough_for_the_guard_test` reads `/proc/self/limits` — not
`libc`, which is not among this crate's dependencies — and requires `RLIMIT_STACK` to be finite and
at least 8 MiB, because the §8 test under emulation is otherwise not testing what it says. The
dependency is silent in **both** directions: with `ulimit -s unlimited`, `modus_acervi()` returns
`None`, the handler takes its bounds-unknown branch and reports exhaustion *unconditionally*, so
`acervus_exhaustus_sub_aemulatore` passes **without the discriminator ever running** — a green that
means nothing. With `ulimit -s 2048`, `qemu-user` floors the guest stack at 8 MiB whatever the limit
says, the derived floor lands about 6 MiB above the real one, and a correct build reports `defectus
internus: memoria violata est: signum` — a failure any reader takes for a §8 runtime regression.
Measured: 8192 is the boundary, 2048 fails, 8192 passes.
## Source-reading tests, and the rules they impose

Four tests read Rust source. Two of them **parse** it — `catalogus.rs` since 2026-08-09 — and
impose nothing; the rules below are what the two remaining *text* scanners still need. They are
cheap and they catch things no type system can. **These are the rules a future maintainer must
keep.**

### Never introduce a block comment into `crates/abi/src`

`abi/tests/abi.rs`'s `sententiae()` skips a line only when its trimmed start is `//`. A
`/* … */` containing `c"…"` becomes a phantom sentence, and one hiding a real `c"…"` removes it.
The affected tree is `crates/abi/src` only.

**This rule used to cover all six trees `catalogus.rs` reads.** Its comment guard tested whether the
text left of a marker on its own line contained `//`, so a block comment mentioning
`Defectus::novus(` or `NUNTIUS_` invented a phantom site. `catalogus.rs` parses now and a token
stream has no comments, so `crates/{lingua-serpentis, latinitas, runtime, ir,
codegen}/src` are free of it —
`docs/superpowers/specs/2026-08-09-test-harness-libraries-design.md` §6, and
`a_marker_in_a_comment_is_prose_and_a_marker_in_code_is_not` asserts the freedom rather than
assuming it.

### Never write a `c"…"` — or an unpaired `c"` — in a trailing comment in `abi/src`

`sententiae()` greps every `c"…"` out of `crates/abi/src/*.rs`, skipping **whole-line**
comments only, and `every_latin_sentence_this_crate_can_say_is_listed_here` asserts that the
resulting set equals a hardcoded eight-element list:

```
acervus vocationum exhaustus est
defectus internus: memoria violata est: signum
defectus internus: memoria violata est: verificatio
divisio per nullum
index extra fines: {} (longitudo {})
memoria exhausta est
memoria non liberata est
modulus per nullum
```

Two hazards follow. A **trailing** comment mentioning `c"…"` is a phantom sentence and fails the
assertion. An **unpaired** `c"` in a trailing comment panics the test outright, because the
extractor does `rest.find('"').expect("a diagnostic literal closes on its own line")`.

The test is kept alongside the catalogue for what the ratchet cannot do: it is **crate-local and
exhaustive**, so it notices a sentence added anywhere in `abi` even where no marker matches.
It counts no sites and touches no snapshot; it makes "a new Latin sentence appeared" a diff someone
has to justify. It reads `c"…"` only, so a `b"…"` message would slip past it — the catalogue covers
that gap, and `catalogus.rs::byte_and_c_string_literals_are_extracted_like_any_other` pins that it
does.

`defectus internus: memoria violata est: signum` and `… : verificatio` are **two sentences, not
one**, deliberately. The handler's non-stack branch and `defectus::memoria_violata` report the same
fact about the same program and cannot share a literal — a handler must not call `liss_panic`, and
§10.3 wants the literal inline at the site the catalogue attributes it to. Worded identically they
were a pair `catalogus.rs` could only ever downgrade, since `fixtured` requires a workspace-unique
pattern and `ambiguous` is a one-way door. The tails name how the fault was found: the kernel's
signal, or a `verifica`'s rejection.

### A `fn main(` in `abi/src` must also contain `liss_custos_institue(`

`custos.rs`'s `a_main_in_this_crate_must_install_the_guard` is a bare `contains` over whole file
text, **comments included**, requiring any `crates/abi/src/*.rs` file containing `fn main(` to
also contain `liss_custos_institue(`. Crude on purpose: it fires the moment an entry point appears
without the call, and asserts nothing about a crate that has no entry point. The hole it closes is
that every other check in that file installs the guard explicitly, so a `main` that called
`liss_initium` and forgot the installer would leave them all passing while D-04 stayed quietly open
until somebody ran a recursive program. The comments-included reading means a file that merely
*discusses* `fn main(` must also mention `liss_custos_institue(` or fail.

### New user-facing sentences belong in module files, not `src/bin/`

`catalogus.rs`'s `read_dir` does not recurse, so `crates/*/src/bin/**` is invisible to it. That is
why the `liss` driver lives in `cli.rs` and `src/bin/liss.rs` is a nine-line shim that says nothing.
The same applies to any future subdirectory of a scanned `src`.

### Every message literal must sit inline at its own site

Spec §10.3, and the ratchet is what enforces it. A site written
`liss_panic(nuntii::DIVISIO_PER_NULLUM.as_ptr(), …)` carries no literal, reports `UNPARSED`, and
hides its message from the catalogue. The one exception is the `NUNTIUS_*` constants, which are
sites *at their declaration* and nowhere else — because an async-signal-safe handler cannot format.

### Keep every message pattern unique across the workspace

Two sites worded identically can never be better than `ambiguous`, and no fixture can rescue them.
When a new site would duplicate an existing sentence, give it a distinguishing tail rather than
reusing the wording.

### `#[cfg(test)]` no longer has to end the file — except in `abi/src`

**This rule is retired for `catalogus.rs`.** It read: keep unit-test modules last and the attribute
at column 0 on its own line, because `production_source` truncated each file at the first
`"\n#[cfg(test)]\n"` and a test module that did not run to EOF would hide every diagnostic below it.
The ratchet now skips items by their `#[cfg(test)]` attribute, which holds however many such modules
a file has and wherever they sit.

It still binds `crates/abi/src`, for a different reason and by a different test:
`sententiae()` reads whole files with no notion of a test module, and
`every_latin_sentence_this_crate_can_say_is_listed_here` asserts an exact eight-element set, so a
`c"…"` written inside a unit test in that crate is a ninth sentence and a failure.
