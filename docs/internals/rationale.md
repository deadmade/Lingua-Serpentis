# The front end, and the counterfactual record

This document holds two things that lived in the source as comments. **Part 1** is a reference for
the front end — the lexer, the parser, the checker, the Latinity lint, package resolution and the
`liss` driver — written so it can be read start to finish by someone new to
`crates/lingua-serpentis`. **Part 2** is the workspace-wide record of what was tried and what it
broke: every "this used to be Z", "without this line, *measured failure*", "we did X and Y went
silently wrong" the source carried, from all six crates, with the file and symbol each belongs to.
The full narrative of every numbered defect lives in `docs/defectus/2026-08-03-index.md`; this
document keeps only the one-line statement of what would break, and points there.

---

# Part 1 — The front end

## The pipeline, and where it stops

**There is no interpreter, and the library does not run programs** (`lib.rs`). Until Task 19 the
crate exposed five `run*` entry points that lexed, parsed, checked and then executed a program by
walking its tree; `interp.rs` was that tree-walker. Spec §3.1 removed it: the corpus is the oracle,
a second implementation is a second thing to keep in step forever, and differential testing is a way
of detecting the drift that having two implementations causes.

So the pipeline stops at the typed IR:

```
lexer::tokenize → parser::parse_file → latinity::lint_program → checker::check_program_typed
                                                                        ↓
                                                             liss_ir::Programma
                                                                        ↓
                                                    liss_codegen::compila → nexus → image
```

**Running a program is a thing the `liss` binary does, not a thing this library does.** Spec §9.6
said the opposite — that `run_program_with_warnings`, `run_dir_with_warnings` and
`run_file_with_warnings` would keep their exact signatures with "the `String` … now the child's
captured stdout" — and it has been amended. Its argument was that the corpus tooling should keep
calling what it called; that premise died when the fixture harness moved to `liss run`, which it had
to, because Latinity warnings are written by the parent before it spawns and a raw image has none.
What §9.6 would have preserved is a library function that forks a compiler: a small `liss run` with
a worse interface.

What survived the swap is **`render`** (`lib.rs`), and that is what the corpus tooling actually
depends on — the shape of the two streams, not a signature. `parse_loaded` was split out of the
private `run_loaded` the five deleted entry points shared, so the compiled and interpreted paths
could not drift into different notions of what loading is. One path remains and the split is now
merely where the code lives.

`Roman` and its arithmetic, and fraction arithmetic, live in `liss-runtime` (spec §11) because they
are compiled twice — once for the host inside this compiler, once for musl inside `liss-abi`.
`lib.rs` re-exports them so every `lingua_serpentis::roman::…` call site keeps working.
`BinOp`/`UnOp` likewise moved down to `liss-runtime` (spec §5.5, §10.4): they carry no AST data of
their own, just the operator's identity, so they belong with the arithmetic they drive rather than
the syntax tree that names them.

## Loading a program: files, folders and packages

**`program.rs`** turns a path into a `Program` — the root folder's files plus one entry per package
(§4.1–4.2 of the packages design). Files are sorted by path and packages by name, so everything
downstream is deterministic.

- **`assemble`** enforces the flat rule: a one-component path is a root file, a two-component path
  is a package file, anything deeper is `fasciculus in fasciculo non licet: 'pkg/sub'`.
- **`is_valid_package_name`** requires the folder name to lex as exactly one `Ident`. That rules out
  spaces, hyphens, leading digits — and keywords. A package name has to be writable in code.
- **`load_dir`** walks the folder. Non-`.liss` files are ignored; folders with no `.liss` beneath
  them contribute nothing and are thereby ignored.
- **`collect_liss`** skips hidden directories (D-17), and never names a path with `dir.display()`,
  which would be an absolute filesystem path where every other diagnostic names a path relative to
  the program root; `prefix` already carries the relative name (D-22).
- **`scida_legi_non_potest`** classifies an `std::io::Error` into one of four whole Latin sentences
  rather than forwarding it (D-22). "not valid UTF-8" cannot be named as such — `UTF-8` carries a
  digit, and no message in this language may — so `signis incognitis scripta est`, "written in
  unknown signs", says the same thing and chimes with the lexer's own `signum ignotum`.

A **bare `.liss` file** is a one-file program, loaded under the synthetic path `fons.liss`; see *The
single-source strip* below.

## The lexer

**`lexer::tokenize`** (`lexer.rs`) is a single character-index scan producing `Token { kind, line
}`. Keywords are recognised by whole word in **`word_to_kind`**, so `nimis`, `capitulum`,
`redditus`, `listae` and `afferre` stay single identifiers.

Three parts of it are not obvious:

- **The `°` branch gives back a trailing separator.** `.`, `:` and `/` join the parts of a numeral
  (`°XXX.M`, `°V:I/II`) but are also the language's `Dot` and `Slash`, and the scan is greedy
  (D-14). Trimming unconditionally is exactly "try, fail, retreat over separators" without the
  retry: `parse_roman` splits on `:`, then `/`, then `.`, and every leaf goes to `convert_single`,
  which rejects the empty string — so a lexeme ending in a separator can never parse. Retreating one
  *character* at a time would be wrong: `°I:I` would fall back to `°I` and leave a stray
  `Ident("I")`, silently killing `fractio sine denominatore`. The guard `i > start + 1` keeps a bare
  `°` intact so it still reaches `parse_roman` and reports `numerus Romanus vacuus`.
- **A string literal is attributed to the line it opens on**, while `line` itself advances past the
  newlines inside it, so every token after a multi-line literal is numbered correctly (D-03).
- **An Arabic digit is caught ahead of the catch-all** (D-15): `numerus Arabicus '5' non licet;
  scribe '°V' vel simile`. The digit is quoted, so the robustness suite's no-Arabic-digits rule —
  which strips quoted spans — is satisfied.

**`TokenKind::name`** (`token.rs`) is the name shown when a token is the one that was missing:
punctuation as its glyph, keywords as the Latin word that produces them, value-carrying kinds as a
Latin category noun (`numerus`, `numerus fractus`, `chorda`, `littera`, `nomen`). It is **exhaustive
on purpose, with no catch-all arm** (D-02): a catch-all would quietly recouple user-facing text to a
private Rust enum for every variant added later. As written, adding one is a compile error until it
is named. `every_keyword_is_named_by_the_word_that_produces_it` closes the round trip; `nullus` is
excluded deliberately, because it lexes to a *value*, `Int(zero)`, not to a keyword kind.

## The parser

**`parser::parse_file`** (`parser.rs`) is the only entry point. A file is `affer` lines first, then
`munus` declarations, and nothing else — `extra munera nihil nisi declarationes licet`. That
guarantee is what the checker leans on; see *`check_file` and what it no longer needs* below.

Expression parsing is an ordinary precedence ladder, lowest first: `or_expr` → `and_expr` →
`cmp_expr` → `add_expr` → `mul_expr` → `unary` → `postfix` → `primary`. `aut si` is flattened at
parse time into a nested `If` inside a one-statement else block, so nothing downstream has an
else-if case of its own.

### The depth cap

**`ALTITUDO_MAXIMA = 64`**, and **`Parser::descend`** is what enforces it (D-05). Recursive descent
re-enters itself once per nesting level with nothing counting, so a deeply nested source aborted the
process — SIGABRT, exit 134, entirely outside the error model, and not something an embedder can
survive.

The number that matters is **not** how much stack the binary has. On this build's main thread (8
MiB) each of the three recursions aborts somewhere past a thousand levels; on a spawned thread with
Rust's default 2 MiB — which is what the in-process fixture runner uses, and what any embedder
calling this library off the main thread gets — **a limit of 128 still parses and 192 overflows**.
The library has to be safe on the smaller one, so the measured ceiling is 128 and the constant is
half of it. That is far more nesting than written code uses: the counter covers block nesting and
expression nesting together, and a deeply indented function with a complicated condition reaches
perhaps fifteen.

**It is not configurable.** There is no config surface in the language, and the robustness suite
spawns this binary with whatever environment it inherits — a tunable limit would make the invariant
depend on the shell that ran it.

The defect register cites only `primary`'s `LParen` arm. There are **three independent routes**, and
guarding one leaves the other two live:

| route | reached from |
|---|---|
| `expr` | parentheses, subscripts, call arguments, list literals, every statement condition |
| `unary` | self-recursion on `non` and `-`, reached without going through `expr` |
| `block` | `statement` → `if_stmt` → `block` → `statement` |

One shared counter is right for all three: they consume the same Rust stack, and in a real program
their costs add up rather than competing.

### Function headers, and the syntaxes that were removed

**`func_stmt`** requires `munus <nomen> [capit (…)] reddit <typus>`. Several shapes are caught early
so they get real advice instead of `nomen expectatum`:

| written | diagnostic |
|---|---|
| `munus ni adde(…)` (old inline form) | `typus reditus post 'reddit' ponendus, non ante nomen` |
| `munus f capit () …` | `munus sine parametris 'capit' omittat` |
| `munus f(ni a) …` | `'capit' ante parametra expectatum` |
| `munus f capit (ni a) { … }` | `'reddit' et typus reditus expectati` |
| `adde(a, b) { … }` | `munus ante definitionem expectatum` |

`old_inline_form_is_rejected` is load-bearing: without it a future change could quietly restore a
dual-syntax parser.

**Duplicate parameter names are rejected here and nowhere else** (D-16): `parametrum 'a' bis
nominatum`. Rejecting at the parser is enough — neither the checker nor the lowering can then see a
duplicate — and guarding them as well would manufacture two more unreachable diagnostics.

`FuncDecl::def_line` records the line of the header's parameter clause, which for a header split
across lines is below the `munus` line. Anything pointing at a parameter must use it rather than the
enclosing `Stmt.line`.

### Quoting in parser diagnostics

**`Parser::eat`** writes `expectatum: '{}'` with the quotes deliberately: the robustness suite
strips quoted spans before its lexical checks, so a glyph like `{` cannot be mistaken for stray
punctuation in the message.

## The checker

**`checker.rs`** does two jobs in one walk: it checks, and it lowers to `liss_ir`. `check_program`
is the checking-only form (it discards the IR) and exists so a checker test need not care about IR
it is not asserting; `check_program_typed` is what the driver calls.

### Types, and the abstract fractional literal

`InferTy` is `Known(Type)` or `FloatLit` — the not-yet-resolved type of a fractional literal, and of
arithmetic that combines only such literals. Everything else is `Known`.

- **`ty_of`** maps the checker's `Type` onto `ir::Ty`. `mnr` and `nr` collapse into one `Frac`
  variant carrying its grain: every operation on them is identical modulo one constant, and only the
  checker's *diagnostics* need the two names apart (spec §4.1).
- A bare `FloatLit` **defaults to `nr`** — in `infer_ir_ty`, in `infer_name` when a diagnostic has
  to name one, and in `resolve_expr` with no float context.
- **`resolve_floatlit`** (over the AST) and **`resolve_floatlit_ir`** (over the IR) are deliberately
  adjacent, with the same three arms in the same order, so the two cannot drift apart.
  **`resolve_pair`** is the only caller of either, so neither rewrite can be written without the
  other.
- The IR node was built while the AST node was still abstract, so it carries `nr`'s grain.
  Re-narrowing onto `mnr`'s is **exact rather than lossy, because truncation composes**:
  `trunc(trunc(n·CCLXXXVIII/d)·XII/CCLXXXVIII) = trunc(n·XII/d)`, which is what `resolve_floatlit`
  computes directly from the rational. The composition is exact in **one direction only**; going the
  other way violates it for the great majority of rationals. Nothing textual protects this —
  `resolve_floatlit` recomputes from the original numerator and denominator and so is
  direction-free, which means the two can stay perfectly parallel while `resolve_floatlit_ir`
  silently goes wrong. Hence its `debug_assert!`. **Conflating an abstract literal with an
  already-resolved `mnr`/`nr` one is safe for the same reason `resolve_floatlit` may**: it is only
  ever called on a subtree whose synthesized type was `FloatLit`, and every leaf of such a subtree
  is a literal.

### Scopes and slots

`Checker` keeps `scopes` (names → types), `slots` (names → `SlotId`) and `scope_slots` (the slots
each open scope introduced, in declaration order) as three parallel stacks, pushed and popped
together by `push_scope`/`pop_scope`.

**Slots are allocated per function and never reused.** Leaving a block pops the name binding and
keeps the number — which is what makes shadowing need no special case, and what makes a function's
locals a flat `Vec<Ty>` (spec §4.2). `pop_scope` yields the slots the scope introduced; that list
becomes the block's `Block::declares`, which is the ownership machinery of spec §7.3 and which
codegen must no more re-derive than it re-derives types.

**`lookup`** reads the two stacks in lockstep rather than searching them separately, so a frame can
only answer for a name it has in both; `declare` is the one writer and always writes both.

### Return analysis

**`always_returns`** / **`block_returns`** are deliberately conservative: `true` only where a return
is provable. Over-rejecting costs a user one extra `redde`; under-rejecting is D-01.

- `Return` returns. An `If` returns when **both** arms return and there **is** an else. `aut si`
  parses as a nested `If` inside a one-statement else block, so else-if chains fall out of the
  recursion with no case of their own.
- **A loop never guarantees a return**, because it may run zero times. `indem (verus)` is not
  special-cased even though it diverges, which is what Rust and Java would do — their version of the
  rule only holds for a loop with no reachable break, and the checker tracks loop depth but not
  break targets. Treating it as non-returning is sound (it only over-rejects), no program in the
  corpus writes it, and the workaround is a trailing `redde`.
- `block_returns` is **`any`, not "the last statement"**: a `redde` followed by unreachable code
  still returns, and this language has no dead-code lint to disagree with. Codegen owes a terminator
  guard because of exactly this; see Part 2.

**Only the root `initium` is exempt** from the all-paths rule. The packages design §4.3 licenses its
fall-through, and `conformance/currunt/initium-sine-reditu` pins it end to end — the program runs,
prints and exits 0. A package function that happens to share the name is ordinary
(`initium_in_a_package_is_an_ordinary_function`). Where the root `initium` falls off its end,
lowering appends `Return(Const::Int(zero))`, supplying the exit status the interpreter used to
supply at run time (§4.4).

### `current_ret` is a `Type`, not an `Option`

**That is a structural claim rather than tidying.** The field was `Option<Type>`, `None` meaning "no
enclosing function", and the `Return` arm raised `'redde' extra functionem` when it found one. That
diagnostic was reachable only through `checker::check`, the legacy entry point that walked a bare
`&mut [Stmt]` and so could be handed a top-level `redde`. `check` is gone (D-19), and with it the
only caller that could produce the state. Every remaining path into `check_stmt` runs through
`check_func_decl`, which assigns the field from `f.ret` before walking a single statement. Rather
than leave an `unreachable!` standing in for a state the type system can rule out, the type rules it
out; the initial value in `Checker::new` is never read.

### `namespace` is a parameter, not a default

**`Checker::new(namespace)`.** It used to be initialised to `None` here, with a comment explaining
that the legacy single-block entry point "stays permissive: it has no program around it". `None`
means *the program's root*, and the root is the one namespace whose `initium` is the entry point and
may fall off its end (§4.3). **A default that quietly says "root" is a default that hands the entry
point's exemption to whoever forgot to set the field.** With `checker::check` gone there is no
caller without a namespace to declare, so every construction states one and the permissive default
cannot be written.

### Packages, signatures and function numbering

`check_program_typed` runs in this order:

1. **`collect_sigs`** per folder — one map of name → `(param types, return type)` across all a
   folder's files, rejecting reserved names and duplicates (§4.2). Two `initium` in the root land
   here as the ordinary `functio '…' bis definita`.
2. The **entry-point check**: `munus 'initium' in programmate non inventum` if absent (structural,
   no file and no line), `munus 'initium' nihil capere et 'ni' reddere debet` if its signature is
   wrong — attributed to the declaration by a search, since the signature map has no location.
3. **`assign_func_ids`** numbers every declaration in the order `check_program_typed` will lower
   them: the root's files first, then each package in `BTreeMap` order, each file in order, each
   declaration in order. `Programma::funcs[id]` is therefore the function `id` names, and
   `check_file` `debug_assert!`s it as it goes.
4. **`check_file`** per file, with the folder's namespace, the program-wide package signatures and
   the file's own `affer` set. Imports naming an unknown package are rejected before any body is
   walked.

**`Checker::func_id` is total by construction**: a name only reaches it once `funcs` or `packages`
has answered for it, and both tables are built by `collect_sigs` from exactly the declarations
`assign_func_ids` numbered, in the same walk.

Call resolution is by qualifier, not by accident. A bare call resolves in the checker's own
namespace; a qualified call requires the package to be in this file's `affer` set (`fasciculus '…'
non allatus est`), then in the program (`fasciculus '…' inter allatos non inventus` — dead by
construction, kept rather than `.expect`ed because an unreachable panic is still a panic, and it
reads UNCOVERED, correctly), then the function within it. Import **cycles are allowed**
(`import_cycles_are_allowed`), and a local and an imported function may share a name
(`local_and_imported_functions_may_share_a_name`).

**One binding carries the bare-call half of that rule, and exactly one test observes it.** In
`check_expr`'s bare `Call` arm the `FuncId` comes from `self.func_id(self.namespace.as_deref(),
name)` — the namespace of the file the call is written in, never the program's root. The qualified
arm's `self.func_id(Some(p), name)` is forced by the syntax, since `alpha.porta` can only mean
`alpha`'s; so the bare arm is the **sole namespace-sensitive `FuncId` binding in the compiler**, and
the flat `FuncId` table is the one place a cross-namespace binding can still happen by accident. The
language forbids it in the other direction outright — a package cannot call a root function, because
the root has no name to `affer` and `check_file` hands the checker only its own namespace's
signatures. `liss-codegen/tests/vocationes.rs::vocatio_nuda_in_fasciculo_in_fasciculo_suo_solvitur`,
over that file's `UMBRA` program, is the only test in the workspace that would notice; Part 2
records what the mutation looks like.

`ir::symbol` (liss-ir) mangles namespace and name to `liss_<n>_<ns>_<m>_<name>`, where `n` and `m`
are the byte lengths of the **escaped** components. Length-prefixing the escaped form is what keeps
the mapping injective — without it, `symbol(None, "aê")` and `symbol(None, "a_u00ea_")` would
collide, since the first's escape is a literal prefix of the second's spelling. The character
immediately after `liss_` is always a digit, which is the collision proof against runtime entry
points such as `liss_int_add` and `liss_panic`, whose next character is always a letter.

### `check_file` and what it no longer needs

`check_file` skips any non-`Func` statement with `continue`. **Only `munus` declarations reach it,
and `parse_file` is what guarantees it**: its loop pushes `func_stmt()` and refuses anything else
with `extra munera nihil nisi declarationes licet`. There used to be a fallback that walked a
non-`Func` statement through `check_stmt` anyway. It was unreachable, and it was the last path by
which a bare `redde` could have been checked outside a function — which is what `current_ret` had to
be an `Option` for. Dropping it is what lets that field be a `Type`.

### Statements worth naming

- **`For`** pushes a scope for the header *before* the init statement, so the header is a `Block`
  and the `dum` variable belongs in a `declares` like every other owned slot. §7.3.5 still emits its
  teardown once after the exit block rather than on the back edge — but `redde` never reaches that
  block, and §7.3.4 frees every enclosing *block*, so a header outside the `Block` tree would leak.
  **`init`, `cond` and `step` are the loop's header, not its body: an `abrumpe` there is as loose as
  one outside.** (That reading is the subject of D-26 — see Part 2.)
- **`Break`/`Continue` outside a loop are rejected** (D-07): `'abrumpe' extra iterationem`,
  `'continua' extra iterationem`. `check_loop_body` is the only thing that raises `loop_depth`, and
  `check_func_decl` takes it to zero for a body — a function body is not inside any loop, whatever
  encloses the declaration.
- **`et` and `vel` do NOT short-circuit and must not become a branch** (spec §3.5). An ordinary
  `Binary` node is exactly what says so, and emitting an `And`/`Or` control-flow form is the easiest
  silent semantic change in the project. It is pinned by
  `conformance/errores/{vel,et}-utrumque-latus`, whose right operand divides by zero: evaluating
  both raises, and short-circuiting prints the left operand and exits 0 — a difference on all three
  streams.
- **`operand_ty` travels on the `Binary` node separately from the node's own type**, because a
  comparison yields `veritas` from operands that are not (§4.1, property 2).
- **Nested functions are rejected** in `check_block_inner`, one guard rather than two identical ones
  (D-24). A `munus` reaching `check_stmt` means a `munus` in a `dum` header, which the parser
  accepts and nothing can call: it is checked, then dropped.
- **`nop`** exists only because `StmtKind::For` needs a `Stmt` for its step and an unreachable panic
  is still a panic; `assign_no_semi` makes the step an `Assign` and nothing else, so it is
  unreachable.

## The Latinity lint

**`latinity.rs`** collects one warning per **declaration site**, so each name is reported at most
once per declaration; assignments and expressions only *use* names declared elsewhere.
Single-character names (`a`, `b`, `i`) are conventional counters and are exempt. Parameters are
reported at `FuncDecl::def_line`, not the enclosing `Stmt.line` — a header split across lines would
otherwise blame the `munus` line.

**`lint_program`** additionally warns on package **folder** names (§4.2), with no file and no line;
file names are never checked because they never appear in code. Package warnings come first; within
a file, warnings stay sorted by line.

Latinity is **warning-only**, and it runs **before the checker** and is attached to a failure rather
than dropped with it (D-09) — a lint that only appears once the program compiles is one nobody
reads.

### `latinitas`: the Whitaker's WORDS wrapper

**`is_latin` is the entire public surface** (`latinitas/src/lib.rs`); the morphology engine —
normalisation, the stem/ending split, the table builder and the `Tables`/`Group`/`Pos` types — is an
implementation detail free to change, so those modules stay private (identifier-checker design §6.1,
§6.5). `DICTLINE.GEN`, `INFLECTS.LAT` and `UNIQUES.LAT` are embedded so the crate is self-contained
at runtime; `data/NOTICE` carries provenance and licensing.

- **`analyze::normalize`** folds to the dictionary's orthography: lowercase, `v`→`u`, `j`→`i`.
  Classical Latin has no distinct `v`/`j`. Underscores and digits survive normalisation so they can
  fail lookup and correctly produce a warning.
- **`analyze::analyze`** answers whole-word first, then tries every `stem + ending` split in
  `lo..=hi`, where `lo = len - max_ending` and `hi = min(len, max_stem)`. The range is **inclusive**
  at both ends: `lux` is a stem whose group allows a zero-length ending, so its only match is at `i
  == w.len()`.
- **`whitaker::merge_wildcard_endings`** distributes declension/conjugation `0` endings onto every
  concrete group of the same part of speech. Taken literally, the `Group { pos, decl }` join key
  files the **31 `V 0 0`** lines (the conjugation-independent perfect-system personal endings) and
  the **52 `ADJ 0 0`** lines (comparatives and superlatives) under a group essentially no dictionary
  stem references, so both entire inflectional systems become unreachable and `amavit`, `vidi`,
  `melior` and `minimum` are wrongly reported as non-Latin — including the notorious `maximum`
  accepted but `minimum` rejected split. Wildcards must not cross parts of speech.
- **`whitaker::parse_inflect_line`** strips trailing scholarly citation comments. Some markers stand
  alone (`-- G+L 130 1`) and some are glued onto the following word (`--JFW,`, `--PRON`, `--quis,`),
  so it matches on a `--` **prefix** rather than exact equality; matching only exact `--` misses
  real INFLECTS line 409 entirely, injecting a bogus empty ending and discarding the real ending
  `is`.
- **`whitaker::add_supplement`** is deliberately separate from `build_tables`, which stays a pure
  function of the three Whitaker files. Supplement entries land in `whole`, so they are accepted as
  written and inflect no further — a supplemented infinitive should not silently license every
  ending of a conjugation it was never checked against. D-18 is why the supplement exists.
- **Known, accepted limitation** (`whitaker::inflecting_pos`): `PRON` and `NUM` are not mapped, so
  only their truncated dictionary stems (`qu`, `h`, `ill`, `du`, `tr`) land in `whole` as bare
  words, and inflected forms like `quae`, `quem`, `duo`, `tres`, `ille` are reported as non-Latin.
  Routing these truncated 1–3 character stems permissively would accept enormous amounts of junk and
  gut the checker's precision, so this is left as-is.

## Errors, warnings and rendering

**`error.rs`** carries three types. `LinguaError` is a message and a line. `Warning` is a message,
an optional file and an optional line, and never stops a program. `ProgramError` is a program-level
error, optionally attributed to a file and a line; **structural** errors — about the program's shape
rather than any source text — carry neither.

A failed run is not only its message. Two things the program had already produced travel on the
`ProgramError` rather than being thrown away with it:

- **`stdout`**, everything `scribe` printed before the error (D-08). Output the program provably
  produced is not the error's to discard.
- **`warnings`**, the Latinity notes computed before it (D-09). A program with two problems should
  report both, not whichever one halts it first.

Both are empty for an error raised before there was anything to collect.

Both also carry a **`fragmentum`**: the one source line the diagnostic points at, already
extracted. `Display` renders it and never reads a file, which is the whole reason it is taken early
— see below.

`Display` writes the three-level grammar of
`superpowers/specs/2026-08-09-caret-diagnostics-design.md` §3, through `pictura::depinge`:

```
erratum: <sententia>          ← always
  in '<scida>', linea <N>     ← when the diagnostic has a location
     |
 <N> | <the source line>      ← when the source was in hand too
     |        ^^^
```

**Each level is independent of the one below**, which is what lets the compiled image — which
knows its file and line but has no source text at run time — emit the first two and stop, rather
than keeping a grammar of its own (`abi.md` §13). Six shapes result, on severity × location:

```
erratum: …                          monitum: …
  in 'mathematica/summa.liss', linea V   in 'a.liss', linea II
erratum: …                          monitum: …
  in 'x.liss'                         in 'a.liss'
erratum: …                          monitum: …
  in linea V                          in linea III
erratum: …                          monitum: …
```

The bare fourth must survive: a package-folder Latinity warning has no file and no line by
construction, because a directory name is not written on a line of anything.

**`Error` became `erratum`** — it was the one English word the language said — and the line number
became a Roman numeral, in the location row and in the gutter alike. That was D-12's last survivor,
and it survived only because nothing had made it awkward enough to fix; `liss-abi`'s `decimales`
went with it.

**Gutter width is the numeral's**, not two: line 88 is `LXXXVIII`. The `|` column is shared by all
four rows, and the location row keeps a fixed two-space indent regardless, so the header is a
two-line unit whose shape does not depend on whether a snippet follows. Nothing ends in whitespace:
this repository has no `.gitattributes`, and a blessed fixture with a trailing space is one
editor-on-save from a diff nobody meant to make.

### Why the fragment is taken early

`strip` clears `e.file = None` *after* the error exists, so a single-source program reads
`in linea V` rather than naming the synthetic `fons.liss`. A renderer that looked the source up by
filename would therefore have to guess "the first root file" for every single-source program —
reviving exactly the coupling that makes `fons.liss` delicate. Extracting where the text is still
in hand costs nothing: `strip` goes on clearing the *header's* filename, and the fragment beside it
stays correct. It also keeps `render`'s signature, and keeps `cli::finish` callable on the `load`
failure path, where no source exists at all.

`fragmentum::extract` is pure and does no I/O, which is what lets every edge case be a unit test
rather than a fixture: a span crossing lines shows its first line only, a span at EOF or on an empty
file or past 200 characters draws nothing, a tab becomes one space, a `\r` is stripped, and every
offset is floored onto a `char` boundary before anything slices with it — `extract` must not be a
way to panic the compiler.

**Columns are counted in `char`s, not bytes.** The renderer prints the line and counts the caret
from the same character iteration, so alignment is exact for all ASCII, for `°` — which begins every
numeral in the language and is two bytes wide — and for every accented letter `is_alphabetic()`
admits into an identifier. A wide character misaligns by one column each; spec §11 forbids
`unicode-width`, and that is the accepted cost.

**`render`** (`lib.rs`) puts warnings on stderr — so they never contaminate piped program output —
program output on stdout, and the program's own exit status on success, 1 on any error. On the error
path the failure's own warnings are written **first** and its partial output still goes to stdout,
so a run that printed something and then failed reports all three (D-08, D-09). Warnings before the
error matches the success path's ordering and reads chronologically.

## The `liss` driver

### Why the driver is a library module and not `src/bin/liss.rs`

`tests/catalogus.rs::extract_sites` reads `crates/*/src` with a **non-recursive** `read_dir`, so
anything under `src/bin/` is invisible to the diagnostic ratchet: every sentence written there could
be added, reworded or lost with the snapshot header still reading the same number of sites and
nobody the wiser. Keeping the driver in `cli.rs` keeps all of it inside the catalogue with no change
to the scanner — the cheaper half of the choice, since the scanner is shared with three other
crates. `src/bin/liss.rs` is a shim that collects argv and calls `cli::liss`, and says nothing.

### Unix only, and deliberately

`run` stages an image through `O_EXCL`, mode bits, `unlink` and `/proc/self/fd`, none of which have
a portable spelling. Spec §9.4 draws the Windows seam at the linker and does not pretend the answer
exists yet, and `liss-codegen` embeds musl blobs for two Linux targets, so this crate is already a
Linux compiler. Nothing in `cli.rs` is `#[cfg]`-guarded to suggest otherwise.

### Argument grammar

**The first argument is a subcommand iff it is exactly `run` or `build`.** Anything else — including
a folder that happens to be named `run` — is the path, which is why such a folder must be given as
`./run`, exactly as `git` and `cargo` require. Matching on the whole word rather than probing the
filesystem is what makes that rule statable in one sentence.

`--exi` and `--ad` belong to `build`. Under `run` they fall through to the unknown-option arm,
because an option that is silently accepted and then ignored is worse than one refused. A repeated
option, an option with no value, and a second positional are each a usage failure.

**`usus`** writes **two sentences and is therefore two sites**, which is also what keeps each
literal inside the 300-byte window `catalogus.rs` reads a pattern from.

### Loading, and the single-source strip

**`load`** asks whether the path exists at all *before* asking what kind of path it is. That is
`main.rs`'s control flow, defect for defect avoided: it is what keeps a missing directory from being
told it needs a `.liss` extension (D-11).

**`strip`** applies the single-source rule to a failure: a one-file program is checked under a
synthetic `fons.liss` and the name is removed afterwards, so the message reads `Error in linea N:`.
Structural errors, which have no line of their own, surface at line 1. The deleted
`run_file_with_warnings` did exactly this; `strip` is now the only implementation of it. A **load**
failure is structural and carries no line, so it must **not** be given one by `strip`, or `scida '…'
non inventa est` would acquire a phantom `in linea 1`.

**Spec §4.1's debt, and `liss` is the only place that can pay it.** The checker has no single-file
signal — it is handed an ordinary one-file program under a synthetic path — so whoever builds the
`Programma` must clear `Func::file`, exactly as the driver already clears the error's. It cannot be
repaired downstream either: the name is baked into the image at codegen time and `run` inherits the
child's stderr, so the parent never sees the sentence it would have to rewrite.

Latinity warnings are written to stderr **before the child exists**. The parent writes no stdout at
all — the child's runtime owns it — so the warnings-then-output order the corpus records only holds
if these are on stderr before the spawn (spec §9.5).

The back end's own failures are forwarded through a **bare hole**: `eprintln!("{}", d)`, already
`Error: `-prefixed and already Latin, so the catalogue does not read it as a site of its own — the
literal lives in `liss-codegen`.

### Broken pipes

**`write_or_exit`** treats a closed pipe as an ordinary end of output (D-06). Rust's runtime sets
SIGPIPE to `SIG_IGN`, so the `print!` this replaces panicked on `prog | head` — a Rust panic
message, a backtrace note and exit 101, for the most ordinary idiom a shell has. Every Unix tool
terminates quietly instead, and `head` closing its end of the pipe is not an error the user needs
telling about.

Zero runtime dependencies rules out resetting the signal handler through libc, so the write error is
handled where it surfaces. **`exit` does not run destructors**, which means nothing else would flush
these handles: the flush in `write_or_exit` is load-bearing rather than tidy. Anything that is not a
broken pipe is a real failure to write, and saying so on a stream that may itself be broken is not
worth a second panic.

**`finish`** renders a finished pipeline result to the two streams and exits. `liss run` never
reaches it on the success path — the child owns stdout there — but every front-end failure does.

### `build`: where the image goes, and what it may destroy

**`destination`** takes `--exi` if given, else the `.liss` stem or the program folder's name, in the
current directory. A folder keeps its whole name (`computatrum`, not `computa` for a folder called
`computa.trum`); a file loses only its `.liss`. `canonicalize` is what makes `liss build .` from
inside a program folder name the folder rather than the dot. A destination that **is a directory**
is refused here rather than at the write, which is several seconds of code generation and linking
later — the one shape that cannot work and is easy to reach is running from the parent of a program
folder, where `liss build ./computatrum` resolves its default output to the input directory itself.

**`write_image`** distinguishes a failure that **destroyed the destination's contents** from one
that did not: `Err(true)` means bytes were already lost. **The rule, and it is the whole reason this
is a function: remove only what we truncated, and we truncated only if it was a regular file.** The
two ways to get it wrong are opposite.

- Removing on `Err(false)` would delete a file this process never managed to open — an existing
  program in a writable directory whose own mode refuses us — turning a refusal to write into a
  deletion.
- Answering `true` merely because the open succeeded is the other, and is worse. **`/dev/full` is
  exactly the shape that provokes it**: the open succeeds, every write fails `ENOSPC`, and nothing
  was ever truncated — so a caller told `true` would unlink the **device node**. Run as root in a
  container, that removes `/dev/full` from a devtmpfs which will not repopulate it, and every later
  process that touches it breaks.

**`truncatur`** answers the question, and is asked of the **open handle** rather than by a second
`stat` of the path, so it answers about the object actually opened and cannot be raced. **Only a
regular file has content for `O_TRUNC` to discard**; on a device, FIFO or socket the flag is a
no-op. The same test decides the `chmod`: a device the user named at `--exi` is left alone in every
respect, because 0755 means nothing on something that is not a program.

Mode 0755 is set **twice**, and the second is the one that holds: `OpenOptions::mode()` applies only
when the file is created and is masked by the umask, and `--exi` may well name a file that already
exists with some other mode entirely. A built artefact is 0755 because it is a program someone will
hand to someone else, unlike `run`'s temporary, which is 0700 and nameless. On a destructive failure
`build` unlinks: a truncated 0755 file where a working program used to be is worse than no file at
all, because it still looks like a program and fails to load.

### `run`: staging, unlinking, exec

**`create_exclusive`** creates the temporary `O_EXCL` with mode 0700 and writes it. The returned
path names a **closed** file: no write descriptor survives the function, and that is required rather
than tidy — `execve` refuses with `ETXTBSY` any file that is open for writing anywhere in the
system, so reusing the write handle instead of reopening read-only turns every `liss run` into a
failure to start. Permissions are set again through the handle (not by path, which would be a second
lookup in a world-writable directory) because `mode()` is masked by the umask, and the one bit this
file cannot do without is owner-execute.

**`create_new` is not about collisions.** The name carries this process's own pid, so no second
`liss run` can propose it; what `O_CREAT|O_EXCL` buys is that the open refuses an existing file *and
refuses to follow a symlink*, which is the whole defence against a name pre-planted in a
world-writable `TMPDIR`. Weakening it to `create` would silently reintroduce the clobber. The 64-way
retry is for the residue of a previous run of this same pid.

**`stage_in`** reopens the written file read-only, **unlinks the path**, and execs
`/proc/self/fd/<n>`.

- **The unlink is the entire cleanup story.** After it the file has no name at all, and the kernel
  frees the inode when the last descriptor closes — so cleanup covers normal exit, error exit,
  panic, `SIGINT`, `SIGTERM` and **`SIGKILL`** alike, none of which a `Drop` guard or a signal
  handler could. Concurrent runs cannot collide either, because after the unlink there is no name to
  collide on.
- **`FD_CLOEXEC` is not cleared, and does not need to be.** Spec §9.5 says to clear it. std has no
  such API and spec §11 keeps this crate free of `libc`, so the step is skipped — and it is skipped
  safely: `execve` resolves its path argument in `do_open_execat`, at the very top of
  `do_execveat_common`, long before `begin_new_exec` runs `do_close_on_exec`. A close-on-exec
  descriptor is therefore still open when `/proc/self/fd/<n>` is looked up. **This was measured
  before it was relied on** — a real ELF, created `O_EXCL` 0700, written, closed, reopened read-only
  (std forces `O_CLOEXEC`), unlinked, then exec'd through `/proc/self/fd/<n>`: exit 0, correct
  output, no name left behind. It is also the mechanism behind `fexecve(3)`, which glibc and musl
  both implement as exactly this path and which exists to be used with `memfd_create(MFD_CLOEXEC)`.
  **Do not "fix" this back by adding a dependency.**
- **A failed unlink degrades to the fallback's contract rather than to a failure.** The descriptor
  is open and the image is perfectly runnable, so refusing to start the program because a directory
  entry could not be removed would be the worst of the three outcomes — and returning `Err` would
  leave the name behind, which is the one thing the two error paths on either side both take care
  not to do. It is spelled as a `match` rather than `Result::err` composed with `map`, because a
  dotted call to `err` is one of `catalogus.rs`'s diagnostic markers and the shorter form would add
  a phantom UNPARSED row to the ratchet for a call that raises nothing.
- **The fallback, and what it honestly does not cover.** Where `/proc` is not mounted there is no
  way to name an unlinked file, so the name is kept and removed after `wait` returns. Spec §9.5
  calls that path best-effort — a `SIGKILL`ed parent leaves a file behind — and here `SIGINT` and
  `SIGTERM` join it, because the parent-side handlers the spec asks for need `sigaction` and this
  crate has no `libc` (§11). That is a widening of an already-documented gap on a path that requires
  `/proc` to be absent to be reached at all, not a new one.

The `proc_fd` parameter of `stage_in` exists **only** so the fallback has automated coverage: both
branches are otherwise unreachable in a test suite, since every host that runs these tests mounts
`/proc`, and a refactor that stopped keeping the name or stopped removing it would leak one 0700
image per `liss run` on every host without `/proc` — containers, chroots, some sandboxes — with the
whole suite green. `Statio::purga` is a method rather than three lines inside `run` so that the
fallback's cleanup is the code a test can call, instead of a second spelling of it that could agree
with a broken original.

**`run`** spawns with **inherited** stdio. Inheritance is essential rather than convenient: the
child's own runtime receives the closed pipe and handles it (spec §6), so D-06 survives at the
source and `liss run prog | head` needs no re-buffering.

### Classifying the child

**`finis`** maps a finished `ExitStatus` to `Status(code)` or `Signum`. `ExitStatus::code()` is
`None` **exactly** when a signal ended the process, and this is where that becomes a Latin sentence
and exit 1 instead of the shell's bare `139`. With §8's handler installed and `SIGPIPE` ignored, no
ordinary failure reaches the `Signum` arm: it is a genuine external kill, or a defect in this
compiler.

**What this branch buys, stated accurately.** Spec §9.5 justifies it by saying that without it a
signalled child "exits 139 and the robustness harness silently accepts it". **The second half is not
true and the reasoning should not be repeated**: `judge` reaches `Outcome::Signalled` only when the
process *it* spawned was signalled, which under `liss run` is the parent, never the child. A parent
exiting 139 takes `judge`'s `code != 1` arm and is `Acceptable`; a parent exiting 1 with this
sentence parses as a well-formed structural diagnostic and is *also* `Acceptable`. Both are
accepted, so the mapping closes no hole in the harness. What it does buy is the person holding the
terminal: a Latin diagnostic saying the program was killed, rather than a bare number they must
decode. Enforcing §8's "never a signal" guarantee needs a `judge` rule that maps this sentence to
`no abort`.

The decision is lifted out of `run` so it can be made against real `ExitStatus` values rather than a
re-encoding of them: `Error: programma abrupte cessavit` had no test at all, and collapsing the
whole match to `exit(code().unwrap_or(139))` left the suite green. That is what makes
`a_signalled_child_is_told_apart_from_one_that_exited` writable — `ExitStatus::from_raw` takes a
**`wait(2)` status**, where an ordinary exit is `code << 8` and a signal death is the signal number
itself, so both shapes are built directly rather than through a spawn.

## The diagnostic-site convention

**Each Latin sentence is written inline at its own call site, and there is deliberately no shared
message table** (compiler-backend spec §10.3). This is counter-intuitive and is the single most
important convention in the workspace to get right.

`tests/catalogus.rs` scans `crates/*/src` with a **non-recursive** `read_dir`, strips each file's
`#[cfg(test)]` module, finds each of eleven markers —

```
LinguaError {   .err(   ProgramError::structural(   Warning {   eprintln!(
Err("   .ok_or("   ok_or_else(|| format!(   liss_panic(   Defectus::novus(   NUNTIUS_
```

— and reads **the first string literal within 300 bytes** of it. Four consequences follow, and each
one has already cost something:

- **A site that forwards a constant carries no literal of its own** and is reported `UNPARSED`,
  hiding the message from the ratchet entirely. A shared table was proposed while two
  implementations existed and had to agree; with one implementation it buys nothing and blinds the
  ratchet.
- **The 300-byte window is real.** In `check_stmt`'s `Assign` arm the explanatory comment sits
  *outside* the constructor deliberately: three comment lines in between are enough to push the
  message out of reach and leave the site reading UNPARSED.
- **Two sites with the same wording are permanently `ambiguous`.** `catalogus.rs` grants `fixtured`
  only to a workspace-unique pattern, and **nothing moves a site from `ambiguous` back to
  `fixtured`**. So where two call sites raise the same sentence they share one helper — that is why
  `lookup_or_err` and `reserved_name_error` exist in the checker, why `liss-abi`'s `defectus` has
  `divisio_per_nullum` and `index_extra_fines`, and why `emissio.rs` has `sedes_ignota` and
  `saltus_extra_ansam`. Conversely, where two failures are genuinely different they get different
  tails: `munus vocatum non inventum: definitio` against `: vocatio`, `memoria violata est:
  verificatio` against the handler's `: signum`, and `nexus.rs`'s four link failures.
- **The scanner reads comments too**, so a comment may not spell a marker either.

The catalogue is read as **two metrics**, and only the first is the ratchet's subject: (1)
diagnostics a program can provoke — every site whose sentence does not begin `defectus internus: `,
**115 → 103 sites, 70 → 70 fixtured, 60.9% → 68.0%**; and (2) compiler-internal failures, the
`defectus internus: ` sites, **0 → 20, all UNCOVERED, all permanently so**. Growth in the second is
a compiler that checks itself more, not coverage that got worse; what guards those is `judge`'s `no
internal defect` rule, not a fixture.

---

# Part 2 — The counterfactual record

Every entry names the file and symbol it belongs to, states what the code used to be or would be,
and states what was observed when it was. Numbered defects link to
`docs/defectus/2026-08-03-index.md`, which carries the full narrative; only the one-line "what would
break" is repeated here.

## Front end — lexer, parser, token

- **`tokenize`, the `°` branch** (lexer.rs) — the greedy numeral scan **did not give back a trailing
  `.`, `:` or `/`**. `°X/°II` died with `numerus Romanus invalidus: 'X/'` while the identical `°X /
  °II` evaluated fine, and `°V.summa` gave `numerus Romanus vacuus`. Whitespace was load-bearing.
  **D-14**
- **`tokenize`, the `°` branch** — retreating one *character* at a time instead of over separators —
  the fix D-14's register entry implies — would make `°I:I` fall back to `°I` and leave a stray
  `Ident("I")`, silently killing `fractio sine denominatore`.
- **`tokenize`, the `°` branch** — without the `i > start + 1` guard a bare `°` trims itself away
  instead of reaching `parse_roman` and reporting `numerus Romanus vacuus`.
- **`tokenize`, the string branch** — the body was scanned with a loop that never looked at `'\n'`,
  so every token after a multi-line literal carried a line number short by one per embedded newline.
  **The drift is unbounded, not an off-by-one.** **D-03**
- **`tokenize`, the digit arm** — an Arabic digit used to fall through to the catch-all as `signum
  ignotum: '5'`, which is true but useless for the likeliest first mistake in a Roman-numeral
  language. **D-15**
- **`TokenKind::name`** (token.rs) — `eat` formatted the kind with `format!("expectatum: {:?}",
  kind)`, so a missing semicolon read `expectatum: Semicolon` — user-facing text coupled to a
  private Rust enum, where renaming a variant silently rewrites what the user sees. A catch-all arm
  would reintroduce it for every variant added later; as written, adding one is a compile error.
  **D-02**
- **`Parser::descend`** (parser.rs) — nothing counted nesting depth, so a deeply nested source
  aborted the process: SIGABRT, exit 134, entirely outside the error model. Twenty thousand
  parentheses used to abort the parser. **D-05**
- **`Parser::descend`** — the register cites only `primary`'s `LParen` arm; guarding it alone leaves
  two live routes, `unary`'s self-recursion on `non`/`-` and `block` via `statement → if_stmt →
  block`. Nested `si` blocks alone abort at the same depth nested parentheses do.
- **`ALTITUDO_MAXIMA`** (parser.rs) — sizing the cap to the main thread's 8 MiB is the wrong
  measurement. On a spawned thread with Rust's default 2 MiB — the in-process fixture runner, and
  any embedder calling this library off the main thread — **a limit of 128 still parses and 192
  overflows**. The cap is 64, half the measured ceiling.
- **`func_stmt`, the parameter loop** (parser.rs) — duplicate parameter names were accepted and the
  last binding won at every layer: the parser pushed both, and the checker and the interpreter each
  `insert`ed into a map, so the body saw only the last one and the earlier argument could not be
  named at all. **D-16**
- **`Parser::eat`** — without the quotes around the token name, the robustness suite, which strips
  quoted spans before its lexical checks, reads a glyph like `{` as stray punctuation.

## Front end — checker

- **`check_func_decl`** (checker.rs) — nothing asked whether a declared return was reachable on
  every path, and `interp::call` ended every body with `Int(zero)` regardless of the declared type,
  so a checker-approved `reddit voc` function could hand back a number. **That is the type system
  being unsound, not merely permissive.** **D-01**
- **`always_returns`** — special-casing `indem (verus)` as diverging, which Rust and Java do, is
  unsound here: their rule only holds for a loop with no reachable break, and the checker tracks
  loop depth but not break targets. Treating it as non-returning only over-rejects.
- **`check_stmt`, the `Break`/`Continue` arms** — both were accepted unconditionally. Outside a loop
  the interpreter yielded `Flow::Break`, and the function-body loop matches only `Flow::Return`, so
  it was **dropped on the floor and execution simply carried on**, with no diagnostic and no clue.
  **D-07**
- **`Checker::current_ret`** — it was `Option<Type>`, `None` raising `'redde' extra functionem`.
  That diagnostic was reachable only through `checker::check`, the legacy entry point that walked a
  bare `&mut [Stmt]`; with `check` gone the `None` case cannot be constructed, so the type rules it
  out rather than an `unreachable!` standing in for it. **D-19**
- **`Checker::new`** — `namespace` used to be initialised to `None` here, with a comment explaining
  that the legacy single-block entry point "stays permissive: it has no program around it". `None`
  means *the root*, the one namespace whose `initium` is the entry point and may fall off its end —
  **a default that quietly says "root" hands the entry point's exemption to whoever forgot to set
  the field.**
- **`check_file`** — a fallback used to walk a non-`Func` statement through `check_stmt`. It was
  unreachable, and it was the last path by which a bare `redde` could have been checked outside a
  function; dropping it is what lets `current_ret` be a `Type`.
- **`lookup_or_err`** — the out-of-scope message was spelled out at three sites (the `Ident`
  expression, `Assign` and `IndexAssign`). Three identical literals meant the catalogue could not
  tell which one a message came from, and only the expression form was ever provoked. **D-24**
- **`reserved_name_error`** — both entry points enforced the reserved-builtin rule from
  byte-identical copies: two implementations of one rule that can drift apart, only one of which a
  user ever runs. **D-24**
- **`check_block`** — it replaced two hand-written error exits, which is what makes the pop happen
  on every path, and leaves one nested-function guard instead of two identical ones. **D-24**
- **`check_binary`, the `And`/`Or` arm** — emitting an `And`/`Or` control-flow form, the branch a
  compiler writer's reflexes reach for, is **the easiest silent semantic change in the project**. It
  is pinned by `conformance/errores/{vel,et}-utrumque-latus`, whose right operand divides by zero:
  evaluating both raises, and short-circuiting prints the left operand and exits 0 — a difference on
  all three streams. The rule *originated* with `interp.rs:131-135`, which evaluated both sides and
  then dispatched on the operator; those fixtures are what carries it now.
- **`resolve_floatlit_ir`** — the truncation composition is exact **in one direction only**. Going
  the other way violates it for the great majority of rationals, and nothing textual protects this:
  `resolve_floatlit` recomputes from the original numerator and denominator and so is
  direction-free, which means the two can stay perfectly parallel while this one silently goes
  wrong. Hence the `debug_assert!`.
- **`check_expr`, the bare `Call` arm** — `self.func_id(self.namespace.as_deref(), name)` is the
  **sole namespace-sensitive `FuncId` binding in the compiler**. Bind it to `None` — which means the
  program's root — and a package's own unqualified call silently resolves to the **root's** function
  of the same name. `liss-codegen/tests/vocationes.rs`'s `UMBRA` program is the one shape that
  catches it: the root declares `quadratum` as `n + °C`, the package `alpha` declares its own as
  `n * n`, and `alpha/a.liss`'s `porta` calls `quadratum` bare, so **`alpha.porta(°III)` is `IX`
  through the package's and `CIII` through the root's** — the wrong binding is a wrong *number*,
  not a coincidence that could agree on some input. The decoy has to be of the same name *on the
  call path*: `vocationes.rs`'s `TRES` already carries a root/package `initium` collision, but only
  ever reaches the package's through a qualified call, so it exercises the symbol table and never
  the resolution — the mangled names differ and everything links.
- **`check_expr`, the bare `Call` arm** — nothing else catches that mutation. Binding it to `None`
  leaves `regimen.rs` and `scalares.rs` green and `vocationes.rs` green but for
  `vocatio_nuda_in_fasciculo_in_fasciculo_suo_solvitur`, and kills exactly one other workspace
  test — `programs.rs`, and by an `expect` in `Checker::func_id` firing because the root happens
  not to declare the name, which is a total-function assertion tripping rather than an observation
  of wrong behaviour. **Give the root a declaration of the same name and even that goes quiet: the
  program links, runs, exits 0 and prints the other function's answer.**
- **`check_func_decl`, the root-`initium` exemption** — `interp.rs`'s
  `falling_off_initium_exits_zero` was the pin before Task 19, and for a while afterwards there was
  none: this arm and the `Return(0)` below it were checker- and IR-only, **neither of which observes
  the exit status the exemption is *about***. `conformance/currunt/initium-sine-reditu` is the pin
  now, end to end.
- **`check_stmt`, the `For` arm** — a `dum` header outside the `Block` tree would leak: §7.3.5 emits
  its teardown once after the exit block, which `redde` never reaches, and §7.3.4 frees every
  enclosing *block*.
- **`check_stmt`, the `Assign` arm** — the explanatory comment sits **outside** the constructor
  deliberately: the catalogue reads the first literal within 300 bytes of a constructor, and three
  comment lines in between are enough to push the message out of reach and leave the site reading
  UNPARSED.
- **checker tests, `check_src`** — twenty-nine of them drove `checker::check`, which took a bare
  `&mut [Stmt]` and was strictly laxer than the real pipeline: no `initium` requirement, and
  functions registered with a plain `insert` so duplicates went unnoticed. They certified a shadow
  language. They are wrapped in `munus initium reddit ni { … }` rather than rewritten, on one line
  so asserted line numbers do not move. **D-19**
- **parser tests, `wrap`** — seventeen of them drove `parser::parse`, which looped `statement()`
  over every token and so accepted a bare statement list. Same shadow language, same one-line
  wrapper. **D-19**

## Front end — program loading, CLI, library surface

- **`scida_legi_non_potest`** (program.rs) — these messages used to be a Latin frame with
  `std::io::Error`'s English `Display` interpolated onto the end: `scida 'principale.liss' legi non
  potest: Permission denied (os error 13)`. The frame was right and the tail was a foreign string
  the library did not author, so **every embedder inherited the one rule the language has being
  broken**. **D-22**
- **`scida_legi_non_potest`** — one shared frame `scida '{}' {}` plus an interpolated reason would
  collapse all four causes into one unattributable catalogue pattern, and a single constructor
  wrapped around the match would show up as one site owning the first arm's wording, hiding the
  other three entirely.
- **`collect_liss`** — `dir.display()` in the failure message is an absolute filesystem path, where
  every other diagnostic in the language names a path relative to the program root. **D-22**
- **`collect_liss`** — every directory entry was recursed into, so a program folder that happened to
  be a git checkout died on `.git` failing to lex as a package name: a failure with nothing to do
  with the program. **D-17**
- **`write_or_exit`** (cli.rs) — it replaces a `print!`. Rust's runtime sets SIGPIPE to `SIG_IGN`,
  so that `print!` panicked on `prog | head` — a Rust panic message, a backtrace note and **exit
  101**, for the most ordinary idiom a shell has. **D-06**
- **`write_or_exit`** — the flush is load-bearing rather than tidy: `exit` does not run destructors,
  so nothing else would flush these handles.
- **`load`** (cli.rs) — asking what kind of path it is before asking whether it exists is what told
  a missing directory that it needs a `.liss` extension. **D-11**
- **`liss`, the load-failure arm** — passing a load failure through `strip` would give `scida '…'
  non inventa est` a phantom `in linea 1`.
- **`write_image`** — removing the destination on `Err(false)` deletes a file this process never
  managed to open — an existing program in a writable directory whose own mode refuses us — turning
  a refusal to write into a deletion.
- **`write_image`** — answering `true` merely because the open succeeded is the other error and is
  worse. **`/dev/full` is exactly the shape that provokes it:** the open succeeds, every write fails
  `ENOSPC`, and nothing was ever truncated, so a caller told `true` would unlink the **device
  node**. Run as root in a container that removes `/dev/full` from a devtmpfs which will not
  repopulate it, and every later process that touches it breaks.
- **`write_image`** — setting the mode only through `OpenOptions::mode()` is not enough: it applies
  only when the file is created and is masked by the umask, and `--exi` may well name a file that
  already exists with some other mode entirely.
- **`create_exclusive`** — reusing the write handle instead of reopening read-only turns every `liss
  run` into a failure to start: `execve` refuses with `ETXTBSY` any file that is open for writing
  anywhere in the system.
- **`create_exclusive`** — weakening `create_new` to `create` would silently reintroduce the
  clobber. `create_new` is not about collisions — the name carries this process's own pid — it is
  that `O_CREAT|O_EXCL` refuses an existing file *and refuses to follow a symlink*, which is the
  whole defence against a name pre-planted in a world-writable `TMPDIR`.
- **`stage_in`** — spec §9.5 says to clear `FD_CLOEXEC`; std has no such API and spec §11 keeps this
  crate free of `libc`, so the step is skipped, and it is skipped safely. `execve` resolves its path
  argument in `do_open_execat`, at the very top of `do_execveat_common`, long before
  `begin_new_exec` runs `do_close_on_exec`. **This was measured before it was relied on** — a real
  ELF, created `O_EXCL` 0700, written, closed, reopened read-only (std forces `O_CLOEXEC`),
  unlinked, then exec'd through `/proc/self/fd/<n>`: exit 0, correct output, no name left behind. It
  is also the mechanism behind `fexecve(3)`. **Do not "fix" this back by adding a dependency.**
- **`stage_in`** — returning `Err` when the unlink fails would leave the name behind, which is the
  one thing the two error paths on either side both take care not to do, and would refuse to start a
  perfectly runnable image because a directory entry could not be removed.
- **`stage_in`** — spelling the unlink as `Result::err` composed with `map` adds a phantom UNPARSED
  row to the ratchet for a call that raises nothing: a dotted call to `err` is one of
  `catalogus.rs`'s diagnostic markers.
- **`stage_in`, the `proc_fd` parameter** — hardcoding `/proc/self/fd` leaves both branches
  unreachable in a test suite, since every host that runs these tests mounts `/proc`. A refactor
  that stopped keeping the name or stopped removing it would leak one 0700 image per `liss run` on
  every host without `/proc` — containers, chroots, some sandboxes — **with the whole suite green**.
- **`finis`** — collapsing the whole match to `exit(code().unwrap_or(139))` left the suite green:
  `Error: programma abrupte cessavit` had no test at all.
- **`finis`** — spec §9.5 justifies this branch by saying that without it a signalled child "exits
  139 and the robustness harness silently accepts it". **The second half is not true and the
  reasoning should not be repeated:** `judge` reaches `Outcome::Signalled` only when the process
  *it* spawned was signalled, which under `liss run` is the parent, never the child. A parent
  exiting 139 takes `judge`'s `code != 1` arm and is `Acceptable`; a parent exiting 1 with this
  sentence parses as a well-formed structural diagnostic and is *also* `Acceptable`. What the branch
  does buy is the person holding the terminal.
- **`liss`, the single-file arm** — `Func::file` cannot be repaired downstream: the name is baked
  into the image at codegen time and `run` inherits the child's stderr, so the parent never sees the
  sentence it would have to rewrite.
- **`cli.rs` module doc** — putting the driver in `src/bin/liss.rs` puts it outside the ratchet:
  `catalogus.rs::extract_sites` reads `crates/*/src` with a non-recursive `read_dir`, so every
  sentence written there could be added, reworded or lost with the snapshot header still reading the
  same number of sites and nobody the wiser.
- **`usus`** — two sentences and therefore two sites, which is also what keeps each literal inside
  the 300-byte window `catalogus.rs` reads a pattern from.
- **`lib.rs`** — spec §9.6 said `run_program_with_warnings`, `run_dir_with_warnings` and
  `run_file_with_warnings` would keep their exact signatures. Its premise died when the fixture
  harness moved to `liss run`, which it had to, because Latinity warnings are written by the parent
  before it spawns and a raw image has none. What §9.6 would have preserved is a library function
  that forks a compiler: a small `liss run` with a worse interface.
- **`lib.rs` tests** — five stood in that module and are **re-homed, not dropped**. Each drove
  `run_program_with_warnings`, so each needed the interpreter to produce the `Result` it then
  rendered; what they asserted is behaviour a user sees, so all five say it end to end now, which is
  a stronger claim than a rendering of a value built in process. This is the only index from these
  three defect IDs to where their claims now live:
  - `output_printed_before_an_error_still_reaches_stdout` (**D-08**) →
    `conformance/errores/emissio-ante-erratum`, and now also
    `conformance/currunt/scribe-ante-acervum`, **which is the harder half: the flush has to happen
    inside a SIGSEGV handler**.
  - `warnings_survive_an_error_and_are_rendered_before_it` (**D-09**) →
    `conformance/errores/monitum-cum-errato`.
  - `warnings_survive_a_checker_error_too` →
    `tests/programs.rs::type_errors_halt_but_their_warnings_survive`, which asserts the same two
    lines in the same order.
  - `an_out_of_range_exit_status_warns_before_it_is_truncated` (**D-10**) →
    `conformance/monita/status-exitus-redactus` and
    `tests/cli.rs::an_out_of_range_exit_status_is_truncated_and_says_so`. **The note is the
    *runtime's* now (`liss-abi`'s `ingressus`), not the interpreter's, so only an end-to-end test
    can see it at all.**
  - `an_in_range_exit_status_says_nothing` →
    `tests/cli.rs::an_exit_status_that_already_fits_says_nothing`.

  The four `render` tests left standing beside them survive because `render` survives: they are
  about the shape of the two streams, and they never needed a backend to make one.
- **`programs.rs::type_errors_halt_but_their_warnings_survive`** — a correction to the defect
  register itself. The test's old name and comment claimed "no warnings are produced at all", which
  the register read as this test pinning D-09's behaviour as correct. **It never did** — the
  assertion was only `is_err()`, and it passes unchanged. **It was the prose that was wrong.**
- **`cli.rs::unbounded_recursion_is_a_clean_latin_error`** (tests) — **the sentence changed with the
  backend, and this is the only test in that file that says so.** The interpreter refused the 65th
  frame and said `vocatio nimis alte implicata` at the call's line; compiled code reaches the guard
  page and §8's handler answers with **no line at all** (§8.4), because a faulting thread cannot
  know which call was the last. What is asserted now is `Error: acervus vocationum exhaustus est`
  and an empty stdout — and asserting an *exit* rather than a signal is the whole claim, since the
  child dies by SIGSEGV and `liss` must turn that into a status rather than let it through. The
  test was once `#[ignore]`d and aspirational, because the process used to be signalled. **D-04**

## Latinity and `latinitas`

- **`analyze::analyze`** (latinitas) — it used to hash every one of the `w.len() + 1` stem/ending
  splits, which is **O(n²) in the identifier's length**. The bounds make it constant work however
  long `w` is; against Whitaker's data they are **18 and 7**, pinned by
  `the_dictionary_bounds_stay_small` because the speedup is only real while they stay small and a
  data-file or supplement change could widen them silently. **`whole` is exempt from them**: it is
  matched by one hash of the entire word, not by splitting. **D-23**
- **`Tables::default`** (model.rs) — the two bounds default to `usize::MAX`, **not `0`**, and the
  `Default` is hand-written rather than derived for that reason. A `0` default would make a table
  that never calls `narrow_bounds` **silently reject every word**; `usize::MAX` makes it merely as
  slow as it was before D-23. The failure mode of forgetting to narrow is a performance regression,
  which is recoverable; a false negative in the Latinity checker is not.
- **`Tables::narrow_bounds`** — calling it before every stem and ending has been inserted
  under-counts, and turns the optimisation into a wrong answer.
- **`whitaker::merge_wildcard_endings`** — taken literally, the `Group { pos, decl }` join key files
  the 31 `V 0 0` lines (the conjugation-independent perfect-system personal endings) and the 52 `ADJ
  0 0` lines (comparative and superlative endings) under a group essentially no dictionary stem ever
  references, so both entire inflectional systems become unreachable and `amavit`, `vidi`, `melior`
  and `minimum` are wrongly reported as non-Latin — including the notorious `maximum` accepted but
  `minimum` rejected split.
- **`whitaker::parse_inflect_line`** — without stripping the trailing scholarly citation, real
  INFLECTS.LAT line 972's `-- G+L 130 1` sits after the two flag columns and the ending is misparsed
  as `""` instead of `are`, which then lets any V-1 stem match as a bare word.
- **`whitaker::parse_inflect_line`** — matching only an exact `--` token misses real line 409
  entirely, where the marker is glued onto the following word as `--JFW,`: the last token is then
  non-alphabetic, which injects a bogus empty ending and discards the real ending `is`.
- **`whitaker::add_supplement`** — supplement entries land in `whole`, so they are accepted as
  written and inflect no further. Letting them inflect would silently license every ending of a
  conjugation the word was never checked against. `examples/calc.liss`'s `munus summare` is the real
  example that made the supplement necessary. **D-18**
- **`whitaker::inflecting_pos`** — a known, accepted limitation rather than an oversight: `PRON` and
  `NUM` are unmapped, so `quae`, `quem`, `duo`, `tres` and `ille` are reported as non-Latin. Routing
  their truncated 1–3 character stems (`qu`, `h`, `ill`, `du`, `tr`) permissively would accept
  enormous amounts of junk and gut the checker's precision.
- **`latinity` tests** — an earlier version of `warnings_are_sorted_by_line` used `alpha` and
  `beta`, which are real Latin (borrowed Greek letter names), so only **one** warning came back —
  and a 1-element vector is trivially sorted, making the ordering assertion unfailable. `compute`
  and `value` are poor choices for the same reason: `computare` and `valere` are real Latin verbs
  whose stems are in the dictionary.
- **`latinity` tests** — `reaches_declarations_in_for_init` uses `foobar` as its init variable
  deliberately: a normal single-letter counter (`i`) would be exempt, and would not catch a
  regression that skipped the `init` field of `For`. `reaches_declarations_inside_else_block` is
  built the same way round — its `then` arm is empty, so `result` can only be found if the walk
  visits `else_`, and a regression that dropped `else_` handling from the `If` case would make the
  warning **silently disappear** rather than fail an assertion.
- **`lint_stmt`, the `Func` arm** — reporting parameters at the enclosing `Stmt.line` blames the
  `munus` line for a header split across lines. Every other lint test keeps the header on one line,
  where the two coincide and the bug is invisible.

## `liss-runtime`

- **`Roman::div_rem`** and **`mag_divmod`** (roman.rs) — the zero guard used to be a `debug_assert!`
  inside `mag_divmod`, compiled out of release. With an empty divisor `mag_cmp` never reports
  `Greater`, so the doubling loop **runs forever, doubling `pow` until memory runs out**. Every
  caller happened to guard, so it was unreachable; but `div_rem` is `pub`, and a latent release-only
  hang behind a caller's diligence is not a guarantee. Deliberately an `Option` rather than an
  `assert!`: an unreachable panic is still a panic, and `no panic` is a live rule the fuzzer
  enforces. **D-21**
- **`parse_roman`** (roman.rs) — three of its guards could not fire. `fractio sine numeratore` was
  **impossible**, because `splitn` always yields at least one item for every input including `""`.
  `denominator est nullus` could only fire if `convert_int_roman` returned zero, which
  `convert_single_is_never_zero_or_negative` refutes by enumerating **every string of length 1..=6
  over the seven symbols — 137,256 of them**. The missing-`°` arm is kept, because `parse_roman` is
  `pub` and a direct caller has no such guarantee; it records UNCOVERED, which is the honest label,
  and it has a test, which is what makes keeping it honest rather than dead. **D-25**
- **`Roman::from_digits`** — left unchecked, a digit beyond 9 panics inside `display`'s
  `render_digit` `unreachable!`, or overflows the `u8` addition in `mag_add`, silently wrapping in
  release.
- **`scriptio` module doc** — it used to claim the module allocates nothing. `scribe_int` and
  `scribe_frac` delegate to `Roman::display` / `fraction::display`, which **return a `String`**, so
  every `scribe` of a `ni` or a fraction takes Rust's global allocator — outside §7.5's counted
  pair, and with `panic = "abort"` an allocation failure there reaches `handle_alloc_error` and dies
  in English rather than saying `memoria exhausta est`. The fix is a `display_ad(&mut impl Sink)` on
  `Roman`, which is a change to `roman.rs`, not to `scriptio.rs`. **D-28**
- **`dispositio::ONUS`** — the alternative was a second `24` in `emissio.rs` held to this one by a
  comment, which is precisely the arrangement Task 12 removed for `KIND_INT`. `liss-abi` is never
  linked into the compiler (§11), so the constant lives in `liss-runtime`, which both ends already
  depend on.
- **`dispositio::VERBUM`** — the more dangerous of the two constants, because **the reader never
  spells it**: `liss_abi::series::elementa` returns a `*mut u64` and every access is `.add(i)`, a
  stride the type system fixes at 8. A `VERBUM` of 16 would have codegen store element `i` at `ONUS
  + 16*i` while the runtime reads `ONUS + 8*i`: `quantum` would over-allocate (harmless), every
  element past the first would read as a zero word, and **both `liberare_verbum` and
  `copiare_verbum` skip a zero word** — silent wrong answers and silent leaks, with nothing failing
  to compile. `liss-abi`'s `const _` block now asserts it.
- **`exit_status`** (exitus.rs) — the truncation is specified (packages design §4.3) and stays. What
  was missing was any sign that it happened: `redde °CCC` exited 44 in silence, and `redde °CCLVI`
  exited 0, which reads as success. **D-10**

## `liss-abi`

- **`memoria::abolere`** — a plain `(*h).magic = 0` before the free **does not clear it in a release
  build, and did not.** `free` ends the object's lifetime, so LLVM's dead-store elimination removes
  the store — correctly, by the language rules, since nothing may legally observe the result.
  Nothing *legal*: the store exists precisely for the illegal reader. The shipped `liss_text_free`
  disassembled to a null test, a magic test, a kind test, `call free` and the counter decrement,
  **with no write to the magic anywhere**, and the same held of `liss_int_free`, `liss_list_free`'s
  element loop and `abolere` itself. **It was uniform, not one inlining accident.** **D-27**
- **`memoria::liberare`** — refusing a null pointer would turn a legal program that prints nothing
  and exits 0 into exit 1 with a corruption diagnostic, and the robustness harness would report an
  invariant violation. `pop_scope` yields every slot bound *anywhere* in a block, so a `redde`
  inside an `si` frees a slot whose `Let` never ran; codegen defines every slot to zero on entry, so
  that free sees null. The alternative is codegen proving per-slot, per-path initialisation, which
  is precisely the bookkeeping §5.4 exists to abolish.
- **`memoria` module doc** — making the live-allocation counter debug-only would have meant "debug
  build" becoming a property of *which prebuilt musl archive is embedded*, doubling the artefact
  matrix; an environment-variable gate would make behaviour depend on the environment.
- **`custos` module doc** — an `.init_array` constructor would have removed the need to remember the
  installer call, and is rejected: on a host build `std::rt::init` runs *after* the constructors and
  installs std's own `SIGSEGV` handler over ours, so the mechanism is not merely untested there — it
  is **inert** there, which is the worst way to ship something.
- **`custos` module doc** — a `main` that calls `liss_initium` and forgets `liss_custos_institue`
  **leaves every test in `tests/custos.rs` passing**, because each installs the handler explicitly.
  D-04 would stay open with no symptom until somebody ran a recursive program. **D-04**
- **`ingressus` module doc** — the same constructor on the musl build *would* work, and is still
  rejected, for four reasons: it is inert on the host and live on musl, so the thing tested is not
  the thing shipped; it is an invisible second entry, so the next reader who adds the obvious
  explicit call gets two `sigaltstack`s and two `sigaction`s with nothing to say so; against
  `.init_array` the strongest available statement is "some pointer is in the section", where a call
  can be checked by reading `main`'s relocations; and a constructor would have to survive
  `--gc-sections`, whose root set is the linker's rather than `main`'s. **A call from `main` is live
  because `main` is live.**
- **`custos::fines`** — three other ways of finding the stack bounds were measured on musl and
  x86_64, and **all three are wrong**: `pthread_getattr_np` + `pthread_attr_getstack` and
  `/proc/self/maps`'s `[stack]` both report only the pages already *touched*, because the main
  thread's stack grows on demand while the guard page sits at the bottom of the *potential* stack,
  and `getrlimit` against a local in `main` gets the size right and the top wrong, because `argv`,
  the environment and the auxv sit above `main`'s frame. **`[stack]` high address − `RLIMIT_STACK`**
  is the one that is right. The four measured rows are in `docs/internals/abi.md` §11.
- **`custos` constants** — `MENSURA_ALTERNA` is a fixed **64 KiB** and deliberately not `SIGSTKSZ`:
  musl's is a small compile-time constant, and on aarch64 with SVE the true minimum can exceed it.
  `LIMES` is **1 MiB, not one page**, because Linux's `stack_guard_gap` defaults to 256 pages, so a
  fault caused by stack growth can land that far below the mapping.
- **`custos::manus`** — recovering the faulting stack pointer from `uc_mcontext` would mean
  hand-rolled musl struct offsets with no header to check them against; `si_addr` against
  precomputed bounds is architecture-independent, which is why this module needs no per-target code
  beyond the two signal numbers.
- **`defectus::liss_panic`** — a format hole with no argument used to render as a literal `{}`,
  which produces a *plausible* exit-1 Latin diagnostic: braces are not digits, English or Rust type
  names, so `judge` accepts it and a codegen bug that forgot an argument goes unnoticed. Refusing up
  front turns it into the internal defect it is, and refusing before any output means the sentence
  is never half-written.
- **`defectus::liss_panic`** — the argument-free path is written as direct `write(2)` calls rather
  than one assembled `String`, and that is not tidiness: `memoria_exhausta` is raised from
  `allocare` at the moment `malloc` returned null, so a `String` here would reach
  `handle_alloc_error`, which **aborts** — `SIGABRT` and exit 134, D-04's shape, produced inside the
  one function whose whole job is to prevent it.
- **`defectus::liss_panic`** — `effunde` and not `liss_flush`: `liss_flush` is the *success* path's
  flush and exits 0 on a closed descriptor 1, which from inside a raise is `judge`'s `clean success`
  violation — and, because the flush returns early on an empty buffer, it would fire **only for
  programs that happened to have printed something**. **D-08**
- **`defectus::liss_exit`** — exit **1** for a leak, and not a distinctive code such as 70: `judge`
  treats any status outside {0, 1, 101} as "this came from `initium`, therefore it is the program's
  own choice and legitimate", so a leak reported as 70 would be **silently accepted by the one
  harness most meant to catch it**.
- **`scriptio` module doc** — the pending-output buffer is a `static mut [u8; 8192]` with a separate
  `AtomicUsize` length, not a `Vec`: the signal handler walks it from a signal context, and a `Vec`
  could be mid-`realloc` when the fault arrives, **losing D-08 for exactly the case §8 exists to
  handle**.
- **`scriptio::effunde`** — while the write chose the exit status, a flush with nothing pending
  returned at `n == 0` without reaching a `write` and so never saw `EPIPE`, which meant the *same*
  fault ended differently depending on how much output the program happened to have buffered: exit 0
  with a silent stderr for a program that had printed something, exit 1 with its diagnostic for one
  that had not. **Nothing stated that rule because nobody designed it.**
- **`series::intra_fines`** — `then`, not `then_some`: `then_some` evaluates its argument eagerly,
  so the zero subscript of `errores/index-infra-fines` computed `0usize - 1` before the guard could
  reject it. That is an overflow panic in debug and a wrap to `usize::MAX` in release — and a panic
  crossing an `extern "C"` boundary aborts, which is `SIGABRT` and exit 134, exactly the shape D-04
  recorded as outside the error model entirely.
- **`defectus::index_extra_fines`** — the length interpolated as a bare `usize` was **the one place
  in the language a user could see an Arabic number**. Both numbers are Roman now; an empty list
  reads `longitudo nullus`. **D-12**
- **`numeri::roman_of`** and **`numeri::exige_granum`** — without the boundary checks, a `LissInt`
  arriving with a digit above 9 panics inside `render_digit`'s `unreachable!` or silently wraps the
  `u8` addition in `mag_add` in a release build, and a zero grain from a miscompiled call site is a
  Rust panic crossing an `extern "C"` boundary.
- **`lib.rs`, the `const _` block** — `ONUS` is held by the three `size_of`s, but the element stride
  was held by nothing at all until `VERBUM == size_of::<u64>()` was asserted here. See the
  `dispositio::VERBUM` entry above for what drift would have cost. **Now something does fail to
  compile.**
- **`lib.rs`, the kind codes** — `liss-codegen` used to carry a copied `GENUS_NI = 2` with a comment
  as the only thing tying the two together. The writer and the reader of these numbers now share one
  definition in `liss_runtime::genus`.
- **`lib.rs`, the `ingressus` feature** — an ungated `main` in this crate is a `duplicate symbol:
  main` at link time, because the crate is also built as an `rlib` for its own tests and every test
  binary already defines `main`. **Measured, not feared.**

## `liss-codegen`

- **`Corpus::libera`** (emissio.rs) — without the null store back into a freed slot, a `continua`
  frees a slot declared later in the loop body: null on the first iteration, and on the iteration
  after one that fell through, **the pointer the last scope exit already freed**. Measured, without
  this store: `Error: defectus internus: memoria violata est: verificatio`, exit 1, on a legal
  program. `conformance/currunt/memoria-sedes-post-continua` is that program, and it is in the
  corpus. The store costs one `iconst 0` and one `def_var` per owned slot per teardown, entirely
  inside the SSA `Variable` layer.
- **`Corpus::blocus_intus`** (emissio.rs) — without the terminator guard, `redde nullus;
  scribe("numquam");` — a legal program, because the checker's all-paths analysis asks whether *any*
  statement returns — has the next statement add an instruction to a Cranelift block the `redde`
  already filled: **a panic in debug, and in release a `Defectus` claiming an internal compiler
  failure over a program that is simply valid**. `abrumpe` and `continua` need the same guard,
  because both fill the block with a `jump` exactly as `redde` fills it with a `return`.
- **`Corpus::dum_intus`** (emissio.rs) — the header is a second statement walk and owes the same
  guard, because a `dum` init clause is any statement, terminators included. Before `9f54f57` all
  three shapes were `you cannot add an instruction to a block already filled` in debug and `defectus
  internus: munus componi non potuit` in release — a valid program told it had broken the compiler.
  **Breaking out of the walk is not sufficient, and was verified not to be:** the instruction that
  lands in the filled block is the `jump caput` that starts the loop, so the whole loop must be
  abandoned.
- **`Corpus::sententia`, the `IndexAssign` arm** — a `Load` here would be **the silent version of
  the bug**: it would copy the list, write the element into the copy, and free the copy, leaving the
  program's own list untouched and every later read correct-looking but stale. `liss_list_put`
  borrows, so the pointer comes off the slot by a bare `use_var`.
- **`Ansa::profunditas`** (emissio.rs) — including the `dum` header in the `abrumpe`/`continua` free
  range would be **the second free of the same pointer**: its teardown is emitted once after the
  exit block.
- **`Corpus::series`** (emissio.rs) — elements first, then the header. Evaluating an element can
  raise — `[°I / nullus]` is a program someone will write — and a raise does not unwind, so a header
  allocated before its elements would be a **live allocation at the moment `liss_panic` runs**.
- **`Corpus::comparatio`, the `List` arm** — routing `liss_list_eq` through the shared
  `icmp_imm_s(cc, ordo, 0)` would be **a silent wrong answer rather than a build failure**:
  `liss_list_eq` returns **1 for equal**, so `IntCC::Equal` against zero — which is what `par_est`
  lowers to for every other type — would report exactly the lists that are *not* equal.
- **`Corpus::comparatio`, the scalar path** — the three-way result is signed: `minor_est` on `ni` is
  the `-1` case, and an unsigned compare against zero would read it as 255.
- **`granum`** (emissio.rs) — a second `Grain → 12 | 288` table in this file would be worse than the
  kind-code duplication it resembles. The reader is `liss_abi::numeri::exige_granum`, which accepts
  XII or CCLXXXVIII and rejects everything else — so emitting `12` for an `nr` **passes that gate**
  and silently produces wrong arithmetic: `°II:I/II * °I:I/XXIV` is 30/288 with the right grain and
  720/288 with the wrong one. A wrong answer, not a crash.
- **`possessio`** (emissio.rs) — three independent `match`es tied together by a comment is how a
  seventh `Ty` gets added to two of them. The leak or the missing null store would be silent, and
  the compiler would report nothing, because each `match` is exhaustive on its own.
- **`machina_isa`** (emissio.rs) — the two settings of spec §8.3 fail *silently* if omitted. Without
  `enable_probestack`, a function whose frame is larger than one page can move the stack pointer
  past the guard page in a single `sub`, touch valid memory below it and **never fault**, so §8's
  SIGSEGV handler never runs and deep recursion goes back to being memory corruption instead of
  `acervus vocationum exhaustus est`. With the default `outline` strategy the probe is a call to a
  `__cranelift_probestack` libcall nothing in the embedded runtime defines, so **the link would fail
  only for programs that happen to have a large frame**.
- **`machina_isa`** — with `is_pic` on, every constant reference goes through a GOT that a `-static
  --no-pie` link has no dynamic loader to fill in.
- **`signum`** (emissio.rs) — a literal `CallConv::SystemV` works on both targets today and quietly
  makes the Windows port a rewrite rather than a recompile. `ArgumentExtension::None` on `veritas`
  and `lit` is self-consistent for a liss→liss call and wrong the moment a runtime entry point is on
  the other end — and **the wrong answer is usually invisible**, because the register is usually
  already clean.
- **`each_signature_carries_its_target_s_own_call_convention`** (emissio.rs tests) — the earlier
  form compared `isa.default_call_conv()` with `CallConv::triple_default(isa.triple())`, which is a
  property of Cranelift and holds no matter what this crate does: **it would have passed unchanged
  over a `signum` that wrote `CallConv::SystemV`**.
- **`Runtime::clausura`** (target.rs) — an earlier version added the relocation addend to the symbol
  address in **unsigned** arithmetic. On aarch64, where `A` is 0, it worked. On x86_64 every `0 +
  (-4)` wrapped to `u64::MAX - 3`, resolved to nothing, and the traversal stopped at the handler's
  own body — so **the whole closure was one node and the x86_64 half of the check was inert while
  looking exactly like the aarch64 half**. x86_64's `R_X86_64_PLT32` stores `S + A - P`, so a call
  to a function at section offset `X` is emitted with `A = X - 4`; aarch64's `R_AARCH64_CALL26`
  carries no such bias and uses `A = X`. Clamping is exact only because rustc gives every function
  its own section, which the code asserts loudly rather than assumes.
- **`Runtime::clausura`** — without following local, section-relative relocations transitively, a
  scan bounded by one symbol's byte range sees nothing the function calls: after `ld -r` a call to a
  private Rust `fn` is a relocation against a **section symbol** with no name. On aarch64 that hid
  three hops — `manus` reaches `discerne`, `erratum` and `effunde` that way, and `erratum`/`effunde`
  are where `write` and `__errno_location` live — so **a forbidden call added to `discerne`, the
  discriminator and arguably the component §8.2's "never in the handler" is most about, passed
  silently**. Measured on the shipped objects, the reachable set is **4 functions on both targets**,
  calling exactly `write`, `__errno_location` and `_exit`.
- **`Runtime::argumentum_primum`** (target.rs) — both filters are load-bearing. A scan that accepted
  any destination register would read the `movz w1, #11` / `movz w1, #7` that build the handler's
  signal mask, which sit inside the same `RETRO` window, and would **get the right answer for the
  wrong reason — and keep getting it after the `sigaction` arguments themselves had gone wrong**. A
  `bl` clobbers `w0`, so a `movz w0` on the far side of one says nothing about this call; without
  stopping at calls the scan is a heuristic that happens to be right on today's layout.
- **`the_page_size_is_asked_of_sysconf_and_never_assumed`** (target.rs) — an earlier draft asserted
  `== 1` and said "asked for once, at initialisation, and never in the handler", which **would have
  gone red on a perfectly correct build that inlined `fines`**, and was describing something the
  check could not see. What none of it establishes: nothing here or anywhere in the suite has ever
  executed the handler on a kernel whose pages are 16 or 64 KiB. The call is proven; the arithmetic
  downstream of a 16384 is not.
- **`main_installs_the_guard_before_it_calls_the_program`** (target.rs) — every Task 9 test still
  passes without the call, because each installs the handler explicitly, so the assertion is made
  against the **object**: `main`'s own relocations must name `liss_custos_institue` at a **lower
  offset** than `liss_initium`. Without it, deep recursion reverts to signal death with an empty
  stderr — D-04, reopened with no test failing. **D-04**
- **`the_crt_objects_are_whole`** (target.rs) — architecture and non-emptiness are not enough: a
  truncated `crt1.o` of the right machine passes both and fails at the first program link with an
  undefined `_start`.
- **`the_runtime_defines_the_abi_and_leaves_liss_initium_to_compiled_code`** (target.rs) — a
  definition of `liss_initium` in the runtime would be silently linked instead of the program.
- **`the_prelink_leaves_nothing_undefined_but_the_program_itself`** (target.rs) — this is how
  `libc.a`'s necessity was re-established rather than taken on trust. Measured on this build,
  x86_64: prelinking the staticlib alone leaves **165** strong undefined symbols, `malloc`,
  `memcpy`, `write`, `sigaltstack` and `sysconf` among them; adding `libc.a` leaves **12**, all
  `_Unwind_*`; adding `libunwind.a` too leaves **none**. `nm --defined-only` over the 331-member
  staticlib finds no musl symbol at all.
- **`the_embedded_runtime_costs_what_it_costs`** (target.rs) — the unstripped prelink was **18.5 MB
  and 19.1 MB**, four fifths of it two things nothing downstream reads: `.llvmbc` (7.6 MB on x86_64)
  for an LTO that never runs, and `.debug_*` (7.2 MB) for a design that emits no debug information
  at all. `build.rs::exonera` removes both, giving **3,396,800 B and 3,524,880 B, 6.6 MiB
  together**. Neither `--whole-archive` nor `lto = true` touches either, which is why two earlier
  attempts at this number moved it by nothing and 3.6 MB the wrong way respectively. **The knock-on
  is larger than the compiler:** `--gc-sections` cannot reach debug sections or bitcode, so every
  byte was being carried into every user's binary — a linked program went from **4,627,048 B to
  512,912 B**, and the "4.6 MB floor" first reported was not a floor.
- **`nexus` module doc** — the risk was measured before any of the linking code was written: the
  prelinked runtime carries **general-dynamic** TLS relocations from Rust's std thread-locals, and a
  static image has neither a `__tls_get_addr` to call nor dynamic relocations to apply, so an
  unrelaxed one is either a link failure or a fault at the first TLS access, with no amount of code
  generation able to help. libwild does relax them, and **both images come out with zero TLS
  relocations and no dynamic section**. The relocation counts, the two surviving `%fs:`-relative
  forms and their addresses are in `docs/internals/codegen.md` §9.
- **`WildNexus::iunge`** (nexus.rs) — without `args.on_warning`, libwild's default sink writes
  English to stderr, and the robustness harness enforces an English denylist, so a libwild warning
  would not merely be ugly: it would fail a test far away from here with no hint of where it came
  from. `args.on_warning` is installed before `parse` rather than after, because `parse` itself can
  warn. And `libwild::error::Error` is never forwarded, for D-22's reason. **D-22**
- **`WildNexus::iunge`** — the four link failures said the same four words until the whole-branch
  review, and that was **a permanent trap rather than a wording preference**: `catalogus.rs` decides
  `fixtured` on a pattern being unique across the workspace, so the first provocation of any one of
  them would have marked all four `ambiguous`, and nothing moves a site back. The trap was reachable
  — `nexus.run` fails on link-time resource exhaustion, which a large enough source program can
  cause without any compiler bug at all.
- **`artefacts_of_two_architectures_are_refused`** (nexus.rs tests) — without the architecture check
  libwild is handed `-m elf_x86_64` for aarch64 inputs and fails somewhere in relocation processing,
  which becomes the same opaque `imago non coniuncta est: nexus` with nothing to say what went
  wrong.
- **`Emissor::novus`** (emissio.rs) — `liss_panic` never unwinds and `redde`/`abrumpe`/`continua`
  are syntactic, so there is no unwinder to feed and `.eh_frame` would be dead weight in every
  compiled binary.
- **`ex_horreo`** (liss-codegen/build.rs) — `rerun-if-changed` on the blob *directory* watches its
  listing; replacing a `runtime.o` in place changes no listing, and the stale bytes would survive
  every rebuild until somebody added or removed a file. Each blob is named individually.
- **`ex_horreo`** — with `LISS_RUNTIME_DIR` set, the layout proof runs zero times. The `const _`
  block in `liss-abi/src/lib.rs` is compiled only when `liss-abi` is, which on that path never
  happens: `conde` is skipped, and `liss-abi` is workspace-`exclude`d and a dependency of nothing
  there. **So a stale blob whose `dispositio` disagrees with today's source links cleanly, passes
  every test in the crate, and produces silent wrong answers.** The close is to export the layout
  constants as absolute symbols from the blob and compare them in a `target.rs` test that can see
  both. **D-29**
- **`Defectus::novus`** (liss-codegen/src/lib.rs) — the one constructor, and the sentence must be
  written inline at the call site: spec §10.3 makes `Defectus::novus(` a catalogue marker, and a
  site that forwards a constant carries no literal of its own and the ratchet reports it `UNPARSED`.
  No foreign error is ever interpolated into one — Cranelift's `ModuleError`, `object`'s write
  errors and `libwild::error::Error` all say their piece in English, and forwarding any of them
  would put English on a user's stderr, **which is D-22 exactly**.

## D-26 is an open question, not a closed defect

**`Corpus::dum_intus`** (emissio.rs) marks its own behaviour as **undecided, not settled**. A `dum`
init clause is `self.statement()` — any statement, terminators included — and the checker admits
one, so `dum (redde nullus; verus; k = k + °I) { … }` is a program a user can write. Codegen honours
the terminator: `redde` returns, and `abrumpe`/`continua` bind to the **enclosing** loop, which is
the checker's own rule for the header ("`init`, `cond` and `step` are the loop's header, not its
body: an `abrumpe` there is as loose as one outside").

That reading has **not been chosen**. It is **D-26**, deferred pending a language decision, and it
is not codegen's to resolve. Two candidates survive the interpreter's deletion, and both are new
language semantics and therefore Non-Goals of the compiler-backend milestone:

- **reject a terminator in a `dum` init clause** with a new diagnostic — the more defensible
  reading, since the checker already calls `init`, `cond` and `step` "the loop's header, not its
  body", and it makes the shape a static error rather than something a reader must reason about; or
- **write the compiled reading down as normative** — it costs nothing to implement, since it is what
  happens, but it is still a decision someone has to take rather than one the implementation takes
  by accident. That distinction is the whole reason this was filed instead of fixed.

Three consequences for anyone working near it:

- **There is no `conformance/` fixture and there must not be one yet.** It used to be that a fixture
  would bless one backend's answer as the language's; now it would bless *the* answer as the
  language's, which is worse — the corpus is what the compiler is held to, so a fixture would close
  the question by implication, with no spec and no one having decided.
  `liss-codegen/tests/regimen.rs`'s `caput_dum_terminatum_componitur` pins the compiled behaviour
  instead, and is now the **only** pin on it anywhere.
- **`lingua-serpentis/tests/generative.rs`'s `caput_dum_terminatum` refuses the shape by a check
  rather than by luck**, so a widened generator cannot draw a program whose meaning is undecided.
- **Codegen's duty here is only not to crash.** The historical disagreement — `interp.rs:325`
  discarded the `Flow` from `self.exec(init)`, so a `redde` there looped forever and an `abrumpe`
  there was ignored — is history, kept because it records what was true when the question was found.

## Deferred elsewhere

One further deferred item is named in the sources without a counterfactual of its own: **D-20**
(`x = x * x` in a loop runs forever, because `Roman` is unbounded). It is named twice, and in both
places as the reason a rule cannot be enforced — in `lingua-serpentis/tests/support/diag.rs`, which
is why `terminates` is the one entry in `RECORDED_RULES` rather than an open rule, and in
`tests/robustness.rs`, whose 60-second `T` cannot be the thing that holds it, since a program that
does not terminate *at all* is let through by no finite deadline.

**D-13** (Roman numeral form is never validated) is deferred too, but it is named nowhere under
`crates/`: its fixture is `conformance/defectus/D-13-numerus-non-canonicus/`, which is the only
directory `conformance/defectus/` holds. Both remain deferred language changes; see the register.
