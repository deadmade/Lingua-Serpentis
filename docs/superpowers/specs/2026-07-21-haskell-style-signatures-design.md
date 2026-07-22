# Lingua Serpentis — Separate Function Signatures Design

> **Superseded by** `2026-07-22-capit-reddit-function-headers-design.md`. The `::`
> / `->` signature described here was replaced by the `capit` / `reddit` header
> before it reached production. Kept as a historical record.

- **Date:** 2026-07-21
- **Status:** Approved (design) — pending implementation plan
- **Author:** Manuel (with Claude)

## 1. Vision

Today a function states its types inline, C-style, so the name is buried between
a return type and a parenthesized parameter list:

```
munus ni summare(ni a, ni b) { redde a + b; }
```

This design separates the *type* of a function from its *definition*, the way
Haskell and ML do. The signature becomes a standalone line that reads as a
declaration in its own right, and the definition line carries only names:

```
munus summare :: ni -> ni -> ni
summare(a, b) {
    ni c = a + b;
    redde c;
}
```

The keyword `munus` moves to the signature line, where it plays the role OCaml's
`val` plays in an `.mli` file: it announces *"there is a function called this,
and here is its type."* The body follows separately.

This is a **syntax-layer change only**. Arrow types are notation, not values:
`ni -> ni` is not something a variable can hold, and `Type` remains a flat
five-variant enum. The change buys readability, not expressive power.

## 2. Goals and Non-Goals

### Goals
- Give every function a standalone type signature that reads as documentation.
- Keep the change confined to the lexer and parser — no changes to the type
  checker, the interpreter, or the AST.
- Preserve the language's Latin keyword identity (`munus` is kept, not dropped).
- Keep function definitions greppable and distinguishable from call sites.
- Produce clear, Latin errors when a signature and its definition disagree.

### Non-Goals (v1)
- **First-class function types.** No currying, no partial application, no
  functions as values. `ni -> ni` cannot appear as a variable's type.
- **Changing variable declarations.** `ni c = a + b;` is untouched; `::` is a
  function-signature form only.
- **Signature without definition** (forward declaration / extern). The syntax
  leaves room for it; this design does not add it.
- **Type inference**, ADTs, and everything else already deferred to v2 in the
  interpreter design spec.

## 3. Syntax

### Form

```
munus summare :: ni -> ni -> ni
summare(a, b) {
    ni c = a + b;
    redde c;
}
```

The last type in the chain is the return type; the preceding ones are the
parameter types, in order. A zero-argument function is therefore a lone type:

```
munus duo :: ni
duo() { redde °II; }
```

### Rules

1. The signature and its definition are a **single syntactic unit**. The
   definition must immediately follow the signature; nothing may come between.
2. The name in the signature and the name in the definition **must match**.
3. The definition's parameter count **must equal** the number of arrow-separated
   types minus one.
4. The old inline form (`munus ni summare(ni a, ni b)`) is **removed**. It is a
   parse error, not a deprecated alternative.

Adjacency (rule 1) is stricter than Haskell, which permits a signature and its
definition to be separated. Requiring adjacency costs nothing in expressiveness
and makes both the parser and the error messages substantially simpler.

### Grammar

```
func   := 'munus' IDENT '::' sig IDENT '(' [params] ')' block
sig    := type ('->' type)*
params := IDENT (',' IDENT)*
```

No newline sensitivity is introduced. The language remains brace-and-semicolon
delimited; the type chain ends when the next token is not `->`.

## 4. Implementation

### 4.1 AST — no change

This is the reason the change is cheap. `FuncDecl` in `ast.rs` already stores
**resolved** types rather than syntax:

```rust
pub struct FuncDecl {
    pub name: String,
    pub params: Vec<(Type, String)>,
    pub ret: Type,
    pub body: Vec<Stmt>,
}
```

The parser fills those same fields from two syntactic sources instead of one.
Nothing downstream can tell the difference.

**Consequence:** `checker.rs` and `interp.rs` are untouched. This includes the
existing rejection of nested functions, which keys on `StmtKind::Func` and so
keeps working unchanged — a signature plus definition still yields exactly one
`StmtKind::Func`.

### 4.2 Lexer

Two new tokens: `TokenKind::ColonColon` and `TokenKind::Arrow`.

- **`->`** — `-` is currently matched unconditionally as `Minus` (`lexer.rs:25`).
  The `->` arm must be placed *before* it, mirroring how the `//` comment arm is
  placed before the `/` arm.
- **`::`** — a bare `:` is currently `signum ignotum`, so `::` is unclaimed.

**Known edge case.** `:` *is* a Roman-numeral body character (`lexer.rs:81`,
consumed as a vinculum separator in `roman.rs:52`), but only inside the `°`
branch. This means `°X::ni` would swallow the colons into the numeral. The
sequence is unreachable through valid syntax — `::` only ever follows an
identifier — but it is pinned with a test rather than left to chance.

### 4.3 Parser

Changes are confined to `func_stmt` (`parser.rs:229`).

`statement()` (`parser.rs:249`) needs **no change**: `munus` still leads, so the
existing `Some(TokenKind::Func) => self.func_stmt()` dispatch stands, and the
grammar stays LL(1). The identifier-led definition line never reaches statement
dispatch because `func_stmt` consumes signature and definition together.

`func_stmt` becomes:

1. Eat `munus`.
2. Read the signature name. **This is the detection point for the old inline
   form:** if a type keyword appears here instead of an identifier, the source is
   using `munus ni summare(...)`, and the dedicated message from §5 is emitted
   rather than a generic `nomen expectatum`.
3. Eat `::`, then read the `->`-separated type chain (at least one type).
4. Read the definition name; check it against the signature name.
5. Eat `(`, read bare parameter names, eat `)`; check the count against the
   signature's arity.
6. Parse the block.
7. Zip parameter names with signature types to build `params`; take the last
   signature type as `ret`.

## 5. Errors

Checked at parse time, phrased in the terse Latin already used by the parser
(`typus reditus expectatus`, `nomen expectatum`).

| Condition | Message |
|---|---|
| Signature name ≠ definition name | `nomen definitionis non congruit cum signatura` |
| Parameter count ≠ signature arity | `signatura N parametros promittit, definitio M habet` |
| `->` with no type after it | `typus post '->' expectatus` |
| `::` not followed by a type | `typus post '::' expectatus` |
| Definition written with no signature | `signatura ante definitionem expectata` |
| Old inline form used | `typus in signatura declarandus, non in definitione` |

The Latin wording above is a **first draft**. It should be reviewed against the
`latinitas` crate's dictionary before landing.

**The missing-signature case needs care.** A definition written without its
signature — `summare(a, b) { ... }` — takes the call-expression-statement path,
parses cleanly as a call (bare identifiers are valid expressions), and then fails
wanting `;` where `{` sits. The default message would be `';' expectatum`: right
line, useless advice. The fix is a targeted special case at exactly that point —
when a call-expression statement is followed by `{` instead of `;`, emit
`signatura ante definitionem expectata`. No lookahead or paren-scanning is
required, since the parser is already past the `)`.

## 6. Migration

Eleven call sites across six files. Note that most of them live in inline
`#[cfg(test)]` modules under `src/`, which is easy to miss when grepping only
`tests/` and `examples/`:

Eleven `munus` occurrences across six files, of which **nine are old-syntax
program text** that must change:

- `src/checker.rs:285,290,295,300` — five occurrences (line 300 nests two).
- `src/interp.rs:310` — one function.
- `src/parser.rs:457` — the existing `function_declaration_and_return` test.
- `crates/lingua-serpentis/tests/programs.rs:7` — one function.
- `examples/calc.liss` — one function.
- `examples/salve.liss` — none.

The remaining two occurrences are **not** call sites and stay as they are:
`src/lexer.rs:93` (the keyword table entry — `munus` keeps its `TokenKind::Func`
mapping) and `src/parser.rs:231` (a `// munus` comment).

**The eight occurrences in Rust code must migrate in the same commit as the
parser change.** The syntax switch is breaking, so any commit that lands the new
`func_stmt` without them leaves the test suite red. `calc.liss` and the docs can
follow separately.

Documentation:

- `docs/superpowers/specs/2026-07-19-lingua-serpentis-rust-interpreter-design.md`
  — two identical code samples, at lines 172-175 and 194-197. **Amend the samples
  in place; do not rewrite the document.** It is a historical record of shipped
  work; only its samples are stale.
- `docs/superpowers/specs/2026-07-19-latin-identifier-checker-design.md` — **no
  change needed.** It names `munus` only in prose, as an example of a Latin
  keyword, which remains accurate.

## 7. Testing

Added to `tests/programs.rs`:

- A function with the new signature form runs and returns correctly.
- Zero-argument function (`munus duo :: ni`).
- Multiple parameters with mixed types (e.g. `voc -> ni -> veritas`).
- Name mismatch between signature and definition → error.
- Arity mismatch, both too many and too few parameters → error.
- Dangling `->` and dangling `::` → error.
- Definition with no signature → the targeted `signatura ante definitionem`
  message, **not** `';' expectatum`.
- **The old inline form is a parse error.** Load-bearing: the removal in §3 rule
  4 is a deliberate breaking change and must be pinned, or it will silently rot
  back into a dual-syntax parser.
- Lexer: `°X::ni` is rejected rather than silently accepted — the colons are
  swallowed into the numeral per §4.2, so this must surface as an error. Assert
  that it fails; do not assert on the specific message, which belongs to
  `roman.rs` and may reasonably change.
- Nested functions are still rejected — confirms §4.1's claim that the checker is
  genuinely untouched.

## 8. Design Notes

**Why the name appears twice.** The signature restates the function name, so
`summare` occurs on both lines. This is the redundancy Haskell and ML both accept
in exchange for a signature that stands alone as documentation. It is a real
cost, accepted knowingly.

**Why `munus` sits on the signature line.** Three arrangements were considered:
`munus` on the definition line (the signature would then announce the function a
line before the keyword did — redundant), `munus` dropped entirely (leaner and
more Haskell-like, but it costs the language's keyword identity, makes
definitions ungreppable, and forces LL(2)), and `munus` on the signature line.
The third keeps the Latin keyword on the line that declares, leaves the
definition line lean, and is the only one of the three that leaves `statement()`
untouched.

## 9. v2 Candidates

Not rejected, just not now:

- **Signature with no definition** as a forward declaration or extern. The syntax
  already accommodates it; only the parser rule forbidding it would relax.
- **Real arrow types** — functions as values, partial application. This would
  make `Type` recursive and non-`Copy`, and would require closures in the
  interpreter. It also interacts with the top-level-only-functions rule.
- **Inference**, so signatures become optional and locals lose their annotations.
