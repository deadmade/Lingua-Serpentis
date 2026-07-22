# Lingua Serpentis — `nr` / double as a Distinct Type Design

- **Date:** 2026-07-22
- **Status:** Approved (design) — pending implementation plan
- **Author:** Manuel (with Claude)
- **Amends:** `2026-07-19-lingua-serpentis-rust-interpreter-design.md` §5.2, §6.2
  (resolves its open question about whether `mnr` and `nr` collapse to one type)

## 1. Vision

The design shipped in July lists two floating keywords, `mnr` (float) and `nr`
(double), but v1 collapsed them: both lex to `TokenKind::TyFloat`, both map to
`Type::Float`, and both are backed by `f64`. The `nr` keyword parses but buys
nothing — it is a synonym for `mnr`. The original design left this open
("Whether `mnr` (float) and `nr` (double) stay distinct types or collapse to
one"); this design closes it by making them genuinely distinct.

`nr` becomes its own type with its own backing: **`mnr` is `f32`, `nr` is
`f64`** — a real precision and range difference, exactly what "float vs double"
means in C, Java, and every language a learner will meet next. The distinction
is observable:

```
mnr x = °I:I/III; scribe(x);   → 1.3333334
nr  y = °I:I/III; scribe(y);   → 1.3333333333333333
```

Same literal (1 + 1/3), two precisions, two printed results. This is the payoff:
a type whose name now carries information, in a language whose whole point is
that types are real and checked.

The change reaches deeper than the `capit`/`reddit` header work did — it touches
the `Type` and `Value` enums, the checker, and the interpreter — but it stays
small and is anchored by one new idea (§3.2): a fractional literal has no fixed
precision until context gives it one.

## 2. Goals and Non-Goals

### Goals

- Make `nr` a first-class type distinct from `mnr`, backed by `f64` while `mnr`
  is re-backed by `f32`, so the two differ in real precision and range.
- Keep the existing `°...:.../...` literal syntax usable for **both** types with
  no new literal notation: a fractional literal adapts to the float type its
  context wants (§3.2).
- Preserve the language's "no implicit conversions" invariant: `mnr` and `nr`
  never mix or convert silently, exactly as `ni` and `mnr` never do today (§3.3).
- Keep the interpreter a dumb second walk. All precision decisions are made once,
  by the checker, and written into the AST (§4.1).

### Non-Goals (v1)

- **Casts / explicit conversions** between `mnr` and `nr` (or between `ni` and
  either). Mixing stays a type error. A conversion feature — which needs cast
  syntax the language does not yet have — is deferred (§9).
- **Implicit numeric promotion** of any kind. No `mnr → nr` widening, no
  `ni → mnr`. Unchanged from today.
- **Whole-number float/double literals.** A floating value still requires a
  fraction (`°V:I/II`); `°V` alone is `ni`. This pre-existing limitation is
  untouched — you cannot write `2.0` as such (§9).
- **A third precision** (f16, big-decimal, etc.). Two floating types only.
- Type inference, arrays, and everything else already deferred to v2.

## 3. Semantics

### 3.1 The two floating types

| Latin | `Type` variant | Rust backing | `type_name` |
|---|---|---|---|
| `mnr` | `Type::Float` | `f32` | `"mnr"` |
| `nr` | `Type::Double` | `f64` | `"nr"` |

`mnr` re-backed as `f32` is the substantive runtime change: a value declared
`mnr` really is single-precision and can lose precision or overflow to infinity
where an `f64` would not. That faithfulness is the point of the feature (§8).

Integer literals (`°V`) remain `ni`/`Int`, untouched. Only **fractional**
literals — those with a `:num/den` part, which `roman.rs` already resolves to a
floating value — participate in the float/double distinction.

### 3.2 Fractional literals adapt to context

A fractional literal has no fixed float-vs-double type at the point it is
written. It takes the float type its context requires; if the context imposes
none, it defaults to `nr` (double), matching C and Java, where an unsuffixed
floating literal is `double`.

The checker models this with an internal, never-user-visible inference tag:

```
InferTy = Known(Type) | FloatLit
```

`FloatLit` is the synthesized "type" of an as-yet-unresolved fractional literal.
Every other expression synthesizes `Known(_)`. The resolution rules:

**Synthesis (bottom-up):**

- A fractional literal → `FloatLit`.
- `Neg` of `FloatLit` → `FloatLit` (a negated literal is still polymorphic).
- Arithmetic `FloatLit ⊕ FloatLit` → `FloatLit` (stays polymorphic).
- Arithmetic `FloatLit ⊕ Known(F)` with `F ∈ {Float, Double}` → **resolve the
  literal side to `F`**, result `Known(F)`. (Symmetric for `Known(F) ⊕ FloatLit`.)
- Arithmetic `Known(Float) ⊕ Known(Double)` (either order) → **type error**
  (§3.3, the mixing rule).
- `FloatLit ⊕ Known(Int)` (either order) → **type error** (int/float never mix,
  unchanged).
- `Known(V) ⊕ Known(V)` → the existing per-operator rules, extended so
  `Double`/`Double` behaves like `Float`/`Float` (Add/Sub/Mul/Div yield that
  type; `Mod` stays `Int`-only).
- **Comparisons** (`par_est`, `par_non_est`, `maior_est`, `minor_est`) follow the
  same literal-adaptation as arithmetic on their operands — a `FloatLit` beside a
  concrete `mnr`/`nr` resolves to it; `Float` beside `Double` is the mixing error
  — but always synthesize `Known(Bool)`. Two `FloatLit` operands are the
  untyped-sink case below (both default to `nr`), then compared, yielding `Bool`.

**Resolution at a demand site — every `FloatLit` is forced to a concrete type
exactly once:**

- **Typed slot** (let/assign target type, `redde` against the return type, a call
  argument against its parameter type): if the value is `FloatLit` and the
  expected type is `mnr` or `nr`, resolve the value to it. If the expected type
  is not a float type, it is a type error (§5).
- **Untyped sink** (`scribe(e)`, a bare expression statement, or a `FloatLit`
  compared against another `FloatLit`): resolve to `nr` (the default).
- **Condition** of `si`/`indem`/`dum`: must be `veritas`; a `FloatLit` there is
  the existing "condition must be bool" error.

Resolving an expression to a concrete float type `F` walks its subtree and
rewrites **every** `FloatLit` leaf reachable through `Neg` and arithmetic
operators to a concrete node of type `F` (narrowing to `f32` when `F = Float`).
Because every expression is reached by exactly one demand site that resolves-or-
errors, and mixed literal/concrete arithmetic resolves eagerly during synthesis,
**no `FloatLit` survives type checking** (§4.4 makes this an invariant the
interpreter relies on).

Worked examples:

| Program fragment | Literal resolves to | Why |
|---|---|---|
| `mnr x = °I:I/III;` | `mnr` (f32) | typed slot wants `mnr` |
| `nr y = °I:I/III;` | `nr` (f64) | typed slot wants `nr` |
| `nr z = °I:I/II + °I:I/IV;` | both `nr` | slot wants `nr`; pushed into both leaves |
| `scribe(°I:I/III);` | `nr` | untyped sink → default |
| `mnr a = ...; nr b = a + °I:I/II;` | error | `a` is `mnr`, slot wants `nr`: literal takes `mnr`, then `mnr` value into `nr` slot mismatches |
| `ni n = °I:I/II;` | error | fractional literal into a non-float slot (§5) |

### 3.3 Mixing is a type error (strict)

Two **already-typed** floating values of different precision never combine
silently. Specifically, a compile-time error is raised when:

- an `mnr` and an `nr` value are operands of the same arithmetic operator;
- an `mnr` and an `nr` value are compared (`par_est`, `maior_est`, …);
- an `mnr` value is assigned to an `nr` target, or vice versa;
- an `mnr` value is passed where an `nr` parameter is declared, or vice versa.

This is the same rule the language already applies to `ni` + `mnr`, extended to
the new pair. There is no widening and no cast. To combine values of the two
types, a programmer would need a conversion the language does not provide in v1
(§9). Note this governs **typed values**; a bare `FloatLit` still adapts (§3.2),
so `nr z = mnr_expr + °I:I/II;` fails not because the literal is stubborn but
because `mnr_expr` fixes the sum to `mnr`.

## 4. Implementation

### 4.1 Architecture: an elaborating checker

The interpreter is an independent second walk over the AST (`interp.rs`); it
carries no type information from the checker and decides a value's runtime shape
purely from the expression node it evaluates. A fractional literal's precision,
however, is decided by **context**, which only the checker computes. The resolved
precision therefore has to be recorded **in the AST**, where the interpreter can
see it.

The chosen mechanism (approaches weighed in §8): the parser emits an **abstract**
literal node, and the checker — as it resolves each literal per §3.2 — **rewrites
that node in place** to a concrete-precision node. Concretely:

- `checker::check` takes `&mut [Stmt]` instead of `&[Stmt]`, and `check_expr`
  takes `&mut Expr`. The checker's own state stays read-only (`&self`); only the
  expression tree is mutated.
- After `check` returns, `latinity::lint(&program)` and `interp::execute(&program)`
  run against the now-resolved tree, unchanged in signature.
- `lib.rs`'s `run_with_warnings` binds `program` as `mut` and calls
  `check(&mut program)?`.

This keeps one owner for the adaptation rules (the checker) and leaves the
interpreter trivial.

### 4.2 Tokens and keywords — `token.rs`, `lexer.rs`

- Add `TokenKind::TyDouble` (`token.rs:13`); update the adjacent comment, which
  currently reads `TyInt, TyFloat, TyText, TyChar, TyBool // ni | (mnr|nr) | …`,
  to list `TyDouble` and stop pairing `mnr|nr`.
- In `word_to_kind` (`lexer.rs:101`), split the alias:
  `"mnr" => TokenKind::TyFloat`, `"nr" => TokenKind::TyDouble`. The
  "both float in v1" comment is deleted.
- `TokenKind::Float(f64)` is **unchanged** — the lexer still produces it from a
  fractional Roman literal (the f64 is narrowed later, only if the literal
  resolves to `mnr`). The lexer test at `lexer.rs:176`
  (`°V:I/II` → `TokenKind::Float(5.5)`) stays valid as-is.

### 4.3 AST — `ast.rs`

- `Type` gains `Double`:
  `enum Type { Int, Float, Double, Text, Char, Bool }`. Still `Copy` — the
  addition is flat, no recursion.
- `Expr` splits the single float node into an abstract literal plus two concrete
  forms:
  - `FloatLit(f64)` — what the parser emits for every fractional literal
    (pre-elaboration). The f64 is the exact value from `roman.rs`.
  - `Float(f32)` — a literal resolved to `mnr` (narrowed).
  - `Double(f64)` — a literal resolved to `nr`.

  The current `Expr::Float(f64)` is replaced by these three. Only `FloatLit`
  exists before checking; only `Float`/`Double` exist after.

### 4.4 Parser — `parser.rs`

- Where a `Float` token becomes an expression (`parser.rs:111`), emit
  `Expr::FloatLit(f)` rather than `Expr::Float(f)`.
- `type_keyword` (`parser.rs:147`) maps `TokenKind::TyDouble => Type::Double`.
- The "a type keyword may start here" lookahead used for declarations and
  headers (`parser.rs:237`, the `Some(TokenKind::TyInt | TyFloat | TyText | …)`
  set) must include `TokenKind::TyDouble`, so `nr x = …;` declarations and
  `reddit nr` / `capit (nr a, …)` headers parse.

### 4.5 Checker — `checker.rs` (the bulk of the work)

- `type_name` (`checker.rs:25`) gains `Type::Double => "nr"`; `Float` stays
  `"mnr"`.
- `check_expr` returns the internal `InferTy` (§3.2) instead of `Type`, and takes
  `&mut Expr` so it can rewrite `FloatLit` nodes. A small helper —
  `resolve_floatlit(e: &mut Expr, ty: Type)` — performs the recursive rewrite
  described in §3.2, narrowing to `f32` for `Type::Float`.
- `check_binary` is reformulated over `InferTy` per §3.2: the `FloatLit`
  combinators, the `Double`/`Double` arithmetic arms, and the
  `Float`×`Double` → error case.
- `Neg`'s rule (`checker.rs:43`) extends to accept `Double` and `FloatLit`.
- The demand sites — `Let`, `Assign`, `Return`, call arguments, and the untyped
  sinks (`Print`, `ExprStmt`) — apply the resolution/default rules in §3.2. The
  existing exact-match comparisons (`got != want`) are replaced by "is this
  `InferTy` acceptable for this expected `Type`, resolving if `FloatLit`."

### 4.6 Value and interpreter — `value.rs`, `interp.rs`

- `Value::Float` changes from `f64` to **`f32`**; add `Value::Double(f64)`
  (`value.rs`). `display` prints each with its native `to_string()` — an `f32`
  and an `f64` of the "same" number can print differently, which is the intended,
  observable behavior (§1).
- `interp.rs::eval`: `Expr::Float(f) => Value::Float(*f)` (now f32),
  `Expr::Double(f) => Value::Double(*f)`. A `FloatLit` must not survive checking;
  defensively map any residual `Expr::FloatLit(f) => Value::Double(*f)` (its
  default) so the interpreter can never panic on one.
- `eval_binary`, `cmp`, and the unary `Neg` arm gain `Double` cases mirroring the
  existing `Float` ones; the `Float` arms now operate on `f32`. `Mod` stays
  `Int`-only. These functions use `_ => Err(...)` catch-alls, so the new arms
  must be added deliberately — the compiler will not flag their absence.

**Compiler-forced vs. manual:** `Value::display` (exhaustive `Value` match) and
`interp::eval` (exhaustive `Expr` match) will fail to compile until the new
variants are handled — a useful backstop. The catch-all sites above
(`eval_binary`, `cmp`, both `Neg` arms, `check_binary`) will compile without the
new arms but be wrong; they are called out here so they are not missed.

### 4.7 `roman.rs` — unchanged; `latinity.rs` — verify only

`roman.rs` still returns `RomanNumber::Float(f64)`; narrowing to `f32` happens in
the checker's `resolve_floatlit`, not here. `latinity.rs` walks statements for
declaration **names** and does not depend on the shape of float types; confirm it
still compiles against the new `Type`/`Expr`/`Value` and adjust only if a match
becomes non-exhaustive.

## 5. Errors

Two new conditions, phrased in the terse Latin the checker already uses. Wording
is a **first draft** and must be validated against the `latinitas` dictionary
(`is_latin`) before landing — each content word checked and any it rejects
replaced, exactly as the `capit`/`reddit` design required.

| Condition | Message (draft) |
|---|---|
| A fractional literal where a non-float type is expected (`ni x = °I:I/II;`) | `numerus fractus typum 'mnr' vel 'nr' requirit, non '<ty>'` |
| `mnr` and `nr` values mixed in one operation, compared, assigned, or passed | reuses the existing mismatch message, now naming `'mnr'` and `'nr'`: `operatio typis 'mnr' et 'nr' non convenit` / `'<name>' typum 'nr' expectat, sed 'mnr' datum est` |

The mixing case needs **no new message** — the existing `check_binary`
`mismatch()` and the `Let`/`Assign`/`Return`/argument mismatch messages already
produce the right text once `type_name` distinguishes `mnr` from `nr`. Only the
fractional-literal-into-non-float case is genuinely new.

## 6. Migration

Small, because no existing program text exercises the floating types end to end.

- **No `.liss` example or integration test** declares an `mnr`/`nr` variable or
  does float arithmetic (`examples/` has none; `tests/programs.rs` has none).
  Nothing there needs rewriting.
- **The only float-literal test sites** are `lexer.rs:176`
  (`°V:I/II` → `TokenKind::Float(5.5)`) and `roman.rs:114`
  (`parse_roman("°V:I/II")` → `RomanNumber::Float(5.5)`). Both are below the type
  layer and **stay unchanged** — the token and the `roman.rs` result are the
  same.
- **`checker.rs:25`** (`type_name`) and **`token.rs:13`** (the comment) are the
  only existing non-test code lines that mention the collapsed `mnr|nr`; both are
  updated as in §4.

Documentation:

- Amend `2026-07-19-lingua-serpentis-rust-interpreter-design.md` **in place**:
  §5.2's table note "Distinct keyword, same backing as float in v1" becomes
  "distinct type, `f64` (float is `f32`)", and §7's / §9's open question about
  collapsing the two types is marked resolved (pointing here). Do not rewrite the
  document.
- Update the top-level `README` if it lists the type keywords or claims `nr` is
  an alias.

## 7. Testing

Checker (`checker.rs` unit tests):

- A fractional literal adapts: `mnr x = °I:I/II;` and `nr y = °I:I/II;` both
  type-check.
- Default: `scribe(°I:I/II);` type-checks (literal → `nr`).
- Literal pushed through arithmetic: `nr z = °I:I/II + °I:I/IV;` type-checks and
  both leaves resolve to `nr`.
- **Mixing rejected:** an `mnr` value and an `nr` value added, compared, and
  cross-assigned each error. Load-bearing: pins §3.3.
- **Fractional literal into a non-float slot rejected:** `ni n = °I:I/II;` errors
  with the §5 message.
- Return-type and argument adaptation: a function `reddit nr` returning a bare
  fractional literal type-checks; passing one to an `mnr` parameter resolves to
  `mnr`.

Interpreter (`interp.rs` / `tests/programs.rs`):

- **The precision divergence:** `mnr x = °I:I/III; scribe(x);` → `1.3333334`;
  `nr y = °I:I/III; scribe(y);` → `1.3333333333333333`. Load-bearing: this single
  pair proves `mnr` is really `f32` and `nr` really `f64`, end to end.
- `nr` arithmetic and comparison run and print correctly.
- A value equal in both precisions (`°V:I/II` = 5.5) prints `5.5` under both
  types — guards against a test that accidentally depends on divergence for a
  representable value.

All existing tests (117 at last count) stay green; none assert on float output
today, so none should shift.

## 8. Design Notes

**Why real `f32`/`f64`, not two names over one `f64`.** A nominal-only split —
two incompatible types with identical bits — teaches that types are checked by
name but gives the distinction no observable meaning, which for a teaching
language is a worse lesson than none: "why are these two identical things
incompatible?" Backing `mnr` with `f32` makes `mnr` vs `nr` mean exactly what a
learner will find in C, Java, or Rust next, and the difference is visible in one
`scribe`. The cost — an extra `Value` variant and literal narrowing — is small
and paid once.

**Why literals adapt instead of defaulting to one type.** Under the language's
strict "no implicit conversions" rule, a literal pinned to a single float type
would make the other type impossible to initialize without a cast — and there is
no cast syntax. Adapting the literal to its slot keeps both types usable with the
existing `°...` notation and adds no syntax to memorize. It is a small, literal-
only form of type-directed checking, deliberately **not** the general type
inference deferred to v2: it never gives a *variable* an inferred type; it only
picks the precision of a literal that context already constrains.

**Why unconstrained literals default to `nr`.** When nothing pins a literal's
precision (`scribe(°I:I/III)`), some default is required. `nr`/double matches C
and Java, where an unsuffixed floating literal is `double` — the wider, less-
surprising choice, and the one that loses no precision at the point of writing.

**Why mixing stays a hard error.** The language already forbids `ni`+`mnr` silent
mixing; extending the same rule to `mnr`+`nr` keeps one consistent story ("the
compiler never converts numbers behind your back") rather than carving out a
C-style promotion exception that would be the language's only implicit
conversion.

**The elaboration mechanism, alternatives.** Two rejected: (a) a separate
post-check elaboration pass, which would re-derive the same types the checker
already computes and invite drift; (b) letting the interpreter thread expected
types and narrow literals itself, which would place the adaptation rules in two
places. Rewriting `FloatLit` nodes during checking keeps a single owner and a
trivial interpreter, at the cost of making `check` take `&mut`.

## 9. v2 Candidates

Not rejected, just not now:

- **Explicit conversions / casts** between `ni`, `mnr`, and `nr` — the natural
  companion to the strict mixing rule, and what unblocks combining the two float
  types. Needs cast syntax the language lacks.
- **Whole-number floating literals** (`°V` as an `mnr`/`nr`, not just `ni`), so a
  float need not carry a fraction. A lexer/literal change, independent of this
  one.
- **Retiring `Expr::FloatLit` from the runtime AST** by giving the checker a
  separate elaborated AST type, if the defensive interpreter arm ever feels like
  a smell.
- Inference proper, arrays, and the rest of the standing v2 list.
