# Lingua Serpentis — `capit` / `reddit` Function Headers Design

- **Date:** 2026-07-22
- **Status:** Approved (design) — pending implementation plan
- **Author:** Manuel (with Claude)
- **Supersedes:** `2026-07-21-haskell-style-signatures-design.md`

## 1. Vision

The function header shipped yesterday states a function's type as a standalone
Haskell/ML signature line, then repeats the name on a definition line:

```
munus summare :: ni -> ni -> ni
summare(a, b) {
    ni c = a + b;
    redde c;
}
```

It reads poorly for this language. The `::` and `->` are the least Latin things
in the grammar — ASCII operator notation dropped into a language whose whole
identity is Latin keywords and Roman numerals — and the name appears twice.

This design replaces that header with one that reads as a Latin sentence, on a
single line, with each type sitting next to the thing it describes:

```
munus summare capit (ni a, ni b) reddit ni {
    redde a + b;
}
```

Read aloud: *"munus summare capit (ni a, ni b) reddit ni"* — "function summare
**takes** (int a, int b) **returns** int." Two Latin verbs frame the header:
`capit` ("it takes") introduces the parameters, `reddit` ("it returns")
introduces the return type. The name appears exactly once.

Like its predecessor, this is a **syntax-layer change only**. Arrow types are
gone entirely; `Type` remains a flat five-variant enum. The change buys
readability, not expressive power.

## 2. Goals and Non-Goals

### Goals
- A function header that reads as Latin and is instantly scannable: return type
  clearly marked, each parameter type adjacent to its name.
- Keep the change confined to the lexer and parser — no changes to the type
  checker, the interpreter, or the AST.
- Retire the `::` / `->` notation and its two tokens completely.
- Reduce the error surface: with a single parameter list there is nothing for a
  separate signature to disagree with.

### Non-Goals (v1)
- **First-class function types.** No currying, partial application, or functions
  as values. There is no arrow type any more; a function's type exists only in
  its header.
- **Changing variable declarations.** `ni c = a + b;` is untouched.
- **Signature without definition** (forward declaration / extern). Not added.
- **Type inference**, ADTs, and everything else already deferred to v2.

## 3. Syntax

### Form

A function with parameters:

```
munus summare capit (ni a, ni b) reddit ni {
    redde a + b;
}
```

A function with no parameters omits `capit` entirely — there is nothing to take:

```
munus duo reddit ni {
    redde °II;
}
```

### Rules

1. The header is one syntactic unit: `munus`, the name, an optional
   `capit (...)` parameter clause, `reddit`, the return type, then the block.
2. The name appears **once**.
3. `capit` is present **if and only if** the function has parameters. A function
   with no parameters writes neither `capit` nor `()`. A `capit ()` with empty
   parentheses is a parse error (see §5) — this keeps exactly one canonical
   spelling for each function.
4. When present, the parameter clause lists one or more `type name` pairs,
   comma-separated. Each parameter carries its own type.
5. `reddit` and the return type are always present, including for functions that
   return `ni`. There is no inferred or omitted return type.
6. The old forms are **removed**, not deprecated, and are rejected at different
   stages: the C-style inline form (`munus ni summare(...)`) at the parser (§5),
   and the Haskell signature (`munus summare :: ni -> ni`) at the *lexer* — with
   the `::` and `->` arms gone (§4.2), those characters no longer tokenize and
   fail as `signum ignotum` before the parser runs.

### Grammar

```
func   := 'munus' IDENT [ 'capit' '(' params ')' ] 'reddit' type block
params := type IDENT (',' type IDENT)*
```

The grammar stays LL(1): after the name, a `capit` token means a parameter
clause follows, and a `reddit` token means there are no parameters. No newline
sensitivity is introduced; the language remains brace-and-semicolon delimited.

## 4. Implementation

### 4.1 AST — no change

`FuncDecl` in `ast.rs` already stores **resolved** types, not syntax:

```rust
pub struct FuncDecl {
    pub name: String,
    pub params: Vec<(Type, String)>,
    pub ret: Type,
    pub body: Vec<Stmt>,
    pub def_line: usize,
}
```

The parser fills the same fields from the new syntax. Nothing downstream can
tell the difference, so `checker.rs` and `interp.rs` are untouched — including
the rejection of nested functions, which keys on `StmtKind::Func` and still sees
exactly one `StmtKind::Func` per function.

**`def_line` collapses to the header line.** It exists because the old form
spread a declaration across two lines: `s.line` was the signature line and
`def_line` the (later) definition line where parameters were named. In the new
form both are the same line, so the parser sets `def_line` to the header line
and it always equals `s.line`. The field is kept — removing it would touch the
AST and break the "AST unchanged" property — but it is now redundant and is a
candidate for later removal. The stale comment in `latinity.rs` that explains
the two-line split (see §4.4) must be updated.

### 4.2 Lexer

`crates/lingua-serpentis/src/lexer.rs`.

**Add** two keywords to `word_to_kind`:

- `capit` → `TokenKind::Takes`
- `reddit` → `TokenKind::Yields`

`reddit` is deliberately a **different token** from `redde` (`TokenKind::Return`).
They are different Latin forms doing different jobs: `reddit` is the third-person
indicative ("it returns") that describes the header's return type, `redde` is the
imperative ("return!") that is the statement in the body. Distinct strings, so
the keyword table separates them cleanly.

**Remove** the now-dead notation:

- The `->` arm (`lexer.rs:25–27`) and the `::` arm (`lexer.rs:28–30`).
- The `TokenKind::Arrow` and `TokenKind::ColonColon` variants (`token.rs:22`).

With the `->` arm gone, `-` is always `Minus`; a stray `>` becomes
`signum ignotum`, and a lone or doubled `:` is `signum ignotum` (the `:` inside a
`°` Roman numeral is unaffected — that path is a separate branch). No other part
of the grammar referenced either token.

### 4.3 Parser

Changes are confined to `func_stmt` (`parser.rs:229`). `statement()` needs no
change: `munus` still leads, so the existing
`Some(TokenKind::Func) => self.func_stmt()` dispatch stands.

`func_stmt` becomes:

1. Eat `munus`.
2. **Old-C-form guard.** If a type keyword sits where the name belongs, the
   source is `munus ni summare(...)`; emit the dedicated message from §5 rather
   than a generic `nomen expectatum`.
3. Read the name (a single identifier).
4. Parameters:
   - If the next token is `capit`: eat it, eat `(`, then read one-or-more
     `type name` pairs separated by commas, then eat `)`. An immediate `)` after
     `capit (` is the empty-`capit` error from §5.
   - Otherwise there are no parameters; `params` is empty.
   - If the next token is `(` here (a parameter clause with no `capit`), emit the
     "forgot `capit`" message from §5. A retired `::` / `->` never reaches this
     point — it fails at the lexer as `signum ignotum` (see §4.2), so there is no
     parser case for it.
5. Eat `reddit`. If a `{` appears instead, the return type clause is missing;
   emit the dedicated message.
6. Read the return type.
7. Parse the block.
8. Build `FuncDecl` directly: `params` is the `type name` list from step 4,
   `ret` is the type from step 6, `def_line` is the header line.

Because parameters declare name and type **together**, there is no second list
to reconcile. The old `nomen definitionis … non congruit` (name mismatch) and
`signatura N parametra promittit, definitio M habet` (arity mismatch) errors are
**deleted** — the states they detected are now unrepresentable.

### 4.4 Latinity checker — comment-only change

`latinity.rs` walks the **AST** (`lint` over `&[Stmt]`, matching
`StmtKind::Func`), so its behaviour is unaffected: it still reports the function
name at `s.line` and each parameter name at `f.def_line`, which now equal each
other. Only the comment at `latinity.rs:39–41` — which explains that a
declaration "spans a `::` signature line and a definition line" — is now false
and must be rewritten to note that the header is a single line.

## 5. Errors

Checked at parse time, phrased in the terse Latin the parser already uses.

| Condition | Message (draft) |
|---|---|
| `munus` then a type keyword (old C-form / return-type-first) | `typus reditus post 'reddit' ponendus, non ante nomen` |
| Name then `(` (parameters without `capit`) | `'capit' ante parametra expectatum` |
| `capit ()` with empty parentheses | `munus sine parametris 'capit' omittat` |
| `reddit` (and return type) missing before `{` | `'reddit' et typus reditus expectati` |
| No type after `reddit` | `typus post 'reddit' expectatus` |
| Parameter written without a type | `typus parametri expectatus` |

The retired `::` / `->` notation is **not** in this table on purpose: it is not a
parser error. With the lexer arms removed (§4.2), a leftover `munus f :: ni` or
`ni -> ni` fails at the lexer with `signum ignotum` before the parser sees it.
That message is generic, but the only people who hit it are migrating away from
yesterday's syntax this week; keeping the `::` / `->` tokens alive solely to
print a nicer deprecation message is exactly the cruft this redesign removes.

The Latin wording above is a **first draft**. Before landing it should be
checked against the `latinitas` crate's dictionary — every content word
(`ponendus`, `obsoleta`, `omittat`, `expectati`, etc.) run through `is_latin`,
and any that fail replaced with a form the analyzer recognizes.

## 6. Migration

The switch is breaking. Every program-text occurrence of the retired form moves
to the new form **in the same commit as the parser change**, or the suite is
left red. Thirteen program-text sites, most inside inline `#[cfg(test)]` modules
under `src/` (easy to miss when grepping only `tests/` and `examples/`):

- `examples/calc.liss:1` — `summare` gains `capit (ni a, ni b) reddit ni`.
- `crates/lingua-serpentis/tests/programs.rs:8`.
- `src/interp.rs:310` — `munus adde capit (ni a, ni b) reddit ni`.
- `src/checker.rs:285,290,295,300` — including the zero-arg `f` (`reddit ni`,
  no `capit`) and the nested-function case at 300 (both `outer` and `inner`
  become zero-arg `reddit ni`).
- `src/latinity.rs:103,117,149` — the linter tests. Preserve what they test: the
  invented names (`foobar`, `widget`) must stay the linted identifiers. `149`'s
  `summa` is zero-arg (`reddit ni`, no `capit`).
- `src/parser.rs:513,527,540` — the positive parse tests.

The **error tests** at `src/parser.rs:552–586` need more than mechanical
rewriting:

- `552` (name mismatch), `558` / `565` (arity too few / too many), `572`
  (dangling `->`), `578` (dangling `::`) test errors that **no longer exist**.
  Replace them with tests for the new error table in §5.
- `586` (old inline `munus ni adde(...)`) **stays** — that form is still an
  error — with its expected message updated to the §5 wording.

The two `munus` occurrences that are **not** program text stay put:
`lexer.rs:99` (the keyword-table entry — `munus` keeps `TokenKind::Func`) and
`parser.rs:231` (a `// munus` comment).

Documentation:

- `docs/superpowers/specs/2026-07-19-lingua-serpentis-rust-interpreter-design.md`
  — its function-header code samples were amended to the `::` form last time;
  re-amend them to the `capit` / `reddit` form. Amend the samples in place; do
  not rewrite the document.
- `docs/superpowers/specs/2026-07-21-haskell-style-signatures-design.md` and
  `docs/superpowers/plans/2026-07-21-haskell-style-signatures.md` — mark both
  **superseded by this design** with a status line at the top pointing here.
  Keep them as a historical record; do not delete.
- Check for a top-level `README` with syntax examples and update any there.

## 7. Testing

Added to `tests/programs.rs` and the parser/lexer unit modules:

- A function with parameters runs and returns correctly
  (`munus adde capit (ni a, ni b) reddit ni`).
- A zero-argument function in the new form — `munus duo reddit ni` — parses and
  runs, and has **no** `capit`.
- Multiple parameters of mixed types (e.g. `capit (voc v, ni n) reddit veritas`).
- **`capit ()` with empty parentheses is a parse error.** Load-bearing: this
  pins §3 rule 3 (form B's "omit `capit` when empty"), so the parser cannot rot
  into accepting two spellings of a zero-arg function.
- **The retired Haskell form (`munus f :: ni -> ni`) is an error.** With the
  `::` / `->` arms gone it now fails at the *lexer* as `signum ignotum`, not at
  the parser. Load-bearing: pins the breaking removal in §3 rule 6. Assert only
  that it errors — not the message or stage, which may change.
- **The old C-style inline form (`munus ni adde(...)`) is a parse error**, with
  the §5 return-type message.
- Missing `reddit`, no type after `reddit`, and a parameter without a type each
  produce their §5 error.
- Nested functions are still rejected — confirms §4.1's claim that the checker is
  genuinely untouched.
- Lexer: `capit` and `reddit` tokenize as `Takes` / `Yields`; a longer word that
  merely starts with one of them (e.g. `capitulum`, `redditus`) stays a single
  identifier, mirroring the existing `keyword_prefix_does_not_split_identifier`
  test.
- Lexer: the retired `signature_symbols` and `numeral_swallows_adjacent_colons`
  tests are removed (their tokens are gone); `lone_colon_is_still_unknown`
  stays valid and is kept.

## 8. Design Notes

**Why `capit` is omitted for zero-arg functions.** `capit ()` reads as "takes
nothing," which is noise; `munus duo reddit ni` reads cleanly as "function duo
returns int." Tying `capit` to the presence of parameters costs one branch in
the parser and buys a header with no dead ceremony. The alternative — always
writing `capit ()` — was considered and rejected for exactly that readability
reason, which is the whole point of this change.

**Why `capit` and `reddit` become reserved words.** Making them full keywords
(rather than identifiers the parser recognizes only in header position) keeps the
lexer and parser simple and matches how every other Latin keyword in the language
already works. The cost is that a program can no longer use `capit` or `reddit`
as an identifier. For a language that already reserves `munus`, `redde`, `si`,
and the rest, two more common verbs is a consistent, acceptable cost.

**Why the return type trails.** Putting the return type after the parameters,
introduced by `reddit`, keeps the header in reading order — subject (`summare`),
what it takes, what it gives back — and mirrors the body's `redde`. The C-style
alternative (`munus ni summare(...)`) puts the return type before the name, which
reads like C, not Latin, and is the form this language is deliberately leaving.

## 9. v2 Candidates

Not rejected, just not now:

- **Signature without definition** as a forward declaration or extern.
- **Real arrow types** — functions as values, partial application. This would
  make `Type` recursive and non-`Copy` and require closures in the interpreter.
- **Removing the now-redundant `def_line` field** from `FuncDecl` once nothing
  depends on it being distinct from `s.line`.
- **Inference**, so return types and local annotations become optional.
