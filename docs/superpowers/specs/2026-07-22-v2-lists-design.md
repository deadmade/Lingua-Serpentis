# v2: Lists (`lista`) — Design

**Date:** 2026-07-22
**Branch:** `rust-rewrite`
**Status:** Approved (brainstorming complete; plan not yet written)

## Summary

Add a growable, statically-typed list type to Lingua Serpentis — the language's
first parametric type and first non-scalar value. A list is written
`<elem-type> lista <name>`, constructed with `[...]` literals, indexed 1-based
with `xs[i]`, and manipulated through two reserved built-ins (`longitudo`,
`adde`) plus an element-write assignment form (`xs[i] = v`). Lists have **value
semantics**: assigning or passing a list copies it, so no two names ever share
the same list.

This is the sole feature of v2. For-each loops, `>=`/`<=`, maps, and list
removal/insert/slice/concat operations are explicitly out of scope.

## Motivation

v1 is entirely scalar: every `Type` is a `Copy` value (`ni`, `mnr`, `nr`,
`voc`, `lit`, `veritas`). Lists are the natural next step — they let programs
hold sequences of values and iterate them. They also exercise two genuinely new
pieces of language machinery (a parametric/recursive type, and a heap-backed
value), which makes them a good, self-contained v2.

## Design decisions

These were settled during brainstorming:

1. **Value semantics** (not reference semantics, not fixed-size arrays). A list
   is a value like any scalar. Assignment and function-argument passing copy it.
   A function cannot mutate its caller's list; to grow a list you reassign:
   `xs = adde(xs, v);`. This maps directly onto the existing `Value: Clone`
   model — a new `Value::List(Vec<Value>)` variant inherits value semantics for
   free, with no `Rc`/`RefCell` and no aliasing.
2. **Keyword `lista`** (user's choice, over the more classical `series`).
   `lista` is post-classical, but keywords are not run through the Latinity
   checker, so it is allowed.
3. **1-based indexing.** The first element is `xs[°I]`. Thematically correct
   (Romans counted from I) and practically forced: Roman-numeral literals cannot
   express zero, so a 0-based scheme would have no natural literal for the first
   index.

## Surface language

### Declaring & constructing

```
ni lista xs = [°I, °II, °III];          // list of ni
voc lista nomina = ["Marcus", "Livia"]; // list of voc
ni lista vacua = [];                    // empty; element type read from annotation
```

The type form is `<elem-type> lista <name>`. The element type is any existing
scalar type. Nested lists (`ni lista lista`) are permitted and fall out of the
recursive type representation (see below), but are treated as lightly-tested
rather than a headline feature.

### Reading & writing elements (1-based)

```
scribe(xs[°I]);      // first element
xs[°II] = °X;        // in-place element write on the variable's own copy
```

### Built-in operations

Recognized specially by name, the way `scribe` already is. `longitudo` and
`adde` are call-expressions (they nest inside other expressions); `xs[i] = v` is
a statement form.

| Form              | Meaning        | Type                    | Notes                                             |
| ----------------- | -------------- | ----------------------- | ------------------------------------------------- |
| `longitudo(xs)`   | length         | `T lista → ni`          | element count                                     |
| `adde(xs, v)`     | append         | `(T lista, T) → T lista`| **returns a new list**; use `xs = adde(xs, v);`   |
| `xs[i] = v`       | element write  | statement               | mutates the variable's own copy in place          |

`longitudo` and `adde` are **reserved** — a user-defined function may not shadow
them.

### Iteration

No new loop construct. Iterate with the existing C-style `pro` loop, using
`longitudo` as the bound. Because there is no `<=`, the bound is expressed with
`minor_est` against `longitudo(xs) + °I`:

```
pro (ni i = °I; i minor_est longitudo(xs) + °I; i = i + °I) {
    scribe(xs[i]);
}
```

## Type system

### Structural change

`Type` becomes recursive to carry an element type:

```rust
// before:  enum Type { Int, Float, Double, Text, Char, Bool }   // derives Copy
// after:   add a variant →  List(Box<Type>)
```

Because `Box<Type>` is not `Copy`, `Type` **loses `#[derive(Copy)]` and derives
`Clone` instead**. This is the single largest edit: every site that currently
copies a `Type` by value must clone or borrow it. Mechanical, but it ripples
through `ast`, `parser`, `checker`, and `interp`.

### Static rules (all strict — no implicit conversions)

- **List literal** `[a, b, c]`: every element must have the same type `T`; the
  literal's type is `T lista`. Mixed element types (e.g. `[°I, "x"]`) → error.
- **Empty literal** `[]`: carries no element type of its own, so it is legal
  **only where an expected list type is statically known** — a declaration with
  a list annotation, an assignment to a list-typed variable, an argument to a
  function/built-in whose parameter type is a list, or a `redde` in a function
  whose return type is a list. Anywhere else → error: *"cannot determine element
  type of empty list."* No inference engine is required — every one of those
  sites already carries an explicit type.
- **Index** `xs[i]`: `xs` must be some `T lista`; `i` must be `ni` (not `mnr` or
  `nr`); the result type is `T`.
- **Index-write** `xs[i] = v;`: `xs` is `T lista`, `i` is `ni`, `v` is `T`.
- **`adde(xs, v)`**: `xs` is `T lista`, `v` is `T`; result `T lista`. A type
  mismatch between the list's element type and `v` → error. Because `adde` is
  polymorphic, its element type `T` is resolved from `v` when the list argument
  is a bare empty literal — so `adde([], °I)` types as `ni lista`. (In practice
  the list argument is almost always a typed variable, `xs = adde(xs, v);`, and
  this rule is only exercised by a literal `[]` passed directly.)
- **`longitudo(xs)`**: `xs` is any list; result `ni`.

## Runtime

- `Value` gains `List(Vec<Value>)`, inheriting `Clone`/`PartialEq`, so value
  semantics are automatic (list equality via `PartialEq` works for free —
  `[°I] par_est [°I]` is `verus`; not a headline feature, just a consequence).
- **Index out of range** (`< °I` or `> longitudo`) → a runtime error through the
  existing error path, naming the offending index and the length. Applies to
  both reads and writes.
- `xs[i] = v` mutates the `Vec` inside the variable's own environment slot in
  place. This is safe with no aliasing precisely because value semantics
  guarantee nothing else holds that `Vec`.
- **Printing**: `scribe(xs)` renders a list as `[a, b, c]` — each element via its
  own `display`, comma-space separated, no quotes (matching how `scribe` already
  prints scalars). Example: `ni lista` `[°I, °II]` prints `[1, 2]`;
  `voc lista` `["Marcus"]` prints `[Marcus]`.

## Implementation surface

Files touched, in pipeline order:

- **`token.rs`** — new tokens `LBracket` (`[`) and `RBracket` (`]`); `lista`
  becomes a keyword token. `longitudo`/`adde` remain plain identifiers,
  recognized at check time.
- **`lexer.rs`** — lex `[`, `]`, and the `lista` keyword.
- **`ast.rs`** — `Type::List(Box<Type>)` with `Copy` dropped and `Clone` added;
  new `Expr::ListLit(Vec<Expr>)` and `Expr::Index { base, index }`; new
  `StmtKind::IndexAssign { base, index, value }`.
- **`parser.rs`** — parse list types (the `lista` suffix, including nesting),
  `[...]` literals, `xs[i]` indexing in expressions, and `xs[i] = …;` statements.
- **`checker.rs`** — all static rules above; built-in signatures for
  `longitudo`/`adde`; the empty-`[]` contextual rule; reserved-name enforcement.
- **`value.rs`** — `Value::List(Vec<Value>)` and its `display` formatting.
- **`interp.rs`** — evaluate list literals, 1-based bounds-checked indexing,
  index-assignment, and the two built-ins.
- **`latinity.rs`** — no change. `lista` is a keyword (exempt); list-variable
  names are still linted like any identifier.

## Non-goals (explicitly deferred)

- For-each loop, `>=`/`<=`, maps/dictionaries.
- List removal, insert-at, `contains`, slicing, concatenation.
- Casts/conversions between list types.

## Testing

Mirrors the existing per-stage test style (the approach that got v1 to 96 green):

- **Lexer:** `[`, `]`, and `lista` tokens.
- **Parser:** list literal, nested list type, index expression, index-assign
  statement.
- **Checker** (the bulk): mixed-type literal rejected; `mnr` index rejected;
  empty `[]` accepted with an annotation and rejected bare; `adde` element-type
  mismatch rejected; `longitudo` typed as `ni`; reserved-name shadowing rejected.
- **Interp:** construct/read/write round-trips; `adde` returns a grown copy while
  leaving a passed-in list untouched (proving value semantics); out-of-range read
  *and* write both error; nested-list read; `scribe` output format.
- **End-to-end:** one `examples/series.liss` exercising build → grow → iterate →
  print, wired into the existing example test.

**Verification:** full `cargo test` green across both crates (after importing the
MSVC environment per the known build steps), and the new example runs clean.
