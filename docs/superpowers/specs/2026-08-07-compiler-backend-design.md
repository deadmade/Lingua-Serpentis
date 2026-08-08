# Lingua Serpentis — Compiler Backend Design

- **Date:** 2026-08-07
- **Status:** Proposed
- **Author:** Manuel (with Claude)
- **Relationship:** Adds an ahead-of-time compiler and **removes the
  interpreter**. Supersedes §3.2 of the conformance suite design (in-process by
  default) and closes deferred defect D-19. Changes one conformance expectation
  and no others.

## 1. Vision

Lingua Serpentis can be read, checked and run. It cannot be *given* to anyone.
Every program needs the source tree, a Rust toolchain and a `cargo build` before
it prints a word of Latin. A language whose whole aesthetic is that the finished
artefact should stand on its own has no finished artefact.

This design produces one: `liss build programma/` emits a statically linked
binary that runs on any Linux of its architecture with nothing else installed.

Three constraints shape every decision that follows, and they were chosen before
this document existed:

- **No rustc on the machine running the compiler.** The obvious shortcut —
  translate to Rust and shell out to `cargo` — is therefore unavailable. So is
  every design that ends in `cc`.
- **The runtime is Rust std on musl**, not `no_std` over raw syscalls. The
  language's OS surface today is three things: buffered print, an exit status,
  and stderr for diagnostics. It will grow, and it should grow by writing
  ordinary Rust rather than by hand-rolling syscall stubs.
- **Memory is freed at last use, from day one.** No leak-first milestone, no
  arena that is emptied at exit.

And one decision taken while writing it, which reshaped the rest:

- **There is exactly one backend.** The interpreter is deleted.

## 2. Goals and Non-Goals

### Goals

- `liss build` produces a standalone static ELF for `x86_64` and `aarch64`
  Linux, from a compiler binary that needs no rustc, no `cc` and no linker on
  the host (§8).
- The compiled program is **observationally identical** to what the conformance
  corpus records — same stdout, same Latin diagnostics, same file attribution,
  same exit status, same stream ordering (§10).
- Every heap value is freed at its last syntactic use, and the runtime **proves
  it** with an always-on live-allocation counter that must read zero at exit
  (§7.5).
- Every failure, including stack exhaustion, stays inside the error model: exit
  1 and a well-formed Latin diagnostic (§8).
- Windows is *designed for* — the linker seam, the calling convention and the
  signal handling each have a named extension point (§9.4).

### Non-Goals

- **No macOS.** Windows is a designed-for second milestone; macOS is not
  considered.
- **No optimisation.** `ni` is boxed unconditionally (§5.1), so allocation-bound
  loops will be *slower than the interpreter they replace*. Two future
  optimisations are named here so they have names when they are scheduled:

  - ***Elisio ad ultimum usum*** — eliding the copy when a `Load` is a variable's
    **last** use. Requires dataflow, and ownership is deliberately syntactic
    (§7.2).
  - ***Subscriptio mutuata*** — a **borrowing subscript**. Because `Load` is the
    only copy site, `xs[i]` deep-copies the whole list to read one element, so
    subscripting in a loop is quadratic. `exemplum-series` is exactly that shape.

  The second is *not* covered by the first: in `summa = summa + numeri[i]`,
  `numeri` is not at its last use — it is read again next iteration — so elisio
  would never touch it. And it is **not a divergence**: the interpreter clones on
  every variable read (`interp.rs:86-91`) and every element read (`:176-183`), so
  both backends are O(n) per subscript and the corpus is blessed against it.

  The IR already carries the precedent. `StmtKind::IndexAssign` takes a `SlotId`
  base while `ExprKind::Index` takes an `Expr` base — so the objection that a
  borrowing read needs "a second rule saying which expressions may borrow" is
  already answered for the write form. An `Index` whose base is a `Load` of a slot
  is the same shape.
- **No new language semantics.** In particular `et` and `vel` keep their current
  non-short-circuiting behaviour (§3.4).
- **No debug information.** No DWARF, no backtraces, no PC→line mapping. This is
  what costs the stack-exhaustion diagnostic its line number (§8.3).
- **No resolution of D-13 or D-20.** They remain deferred language changes.

## 3. Key Decisions and Rationale

### 3.1 One backend, not two

The earlier design kept the interpreter as a differential-testing oracle: run
every fixture both ways, compare. That justification does not survive contact
with the corpus.

**The corpus is already the oracle.** Eighty-six fixtures carry blessed
`emissio`, `nuntii` and `exitus` files on disk. They are recorded facts, and
they do not need a second implementation to be true. What a second
implementation *would* need is every Latin diagnostic, the mod-256 exit rule,
1-based list indexing, truncating division and the `mnr`/`nr` grain arithmetic
written twice and kept in step forever. Differential testing is a way of
*detecting* the drift that having two implementations *causes*. Removing one
removes the class.

It also retires **D-19**. Sixty-two of the workspace's unit tests drive the
legacy `parser::parse` / `checker::check` / `interp::execute` triple, which
accepts top-level statements and `initium`-less scripts — a laxer language than
the one the compiler implements. That shadow language has been on the deferred
list because removing it was a test rewrite rather than a fix. It leaves with
the interpreter.

**The deletion is staged.** The interpreter remains as scaffolding for the whole
milestone and is deleted last, gated on the entire corpus passing through the
compiled path. Deleting it first would mean a red suite and no reference for
however long codegen takes; deleting it last costs nothing but patience.

### 3.2 `liss run` is compile-and-exec

With no interpreter, `run` must mean something new. Two candidates were
considered and one rejected.

**Rejected: a Cranelift JIT.** It would be fast and the tests would love it. But
it is a second linkage model, resolving symbols against a *host-gnu* build of
the runtime while `build` links a *musl* one — which reintroduces, at smaller
scale, precisely the two-implementations problem §3.1 just eliminated.

**Chosen: `run` is `build` with a throwaway output path.** Compile, link, write
to a temp file, spawn it, propagate its exit status. One pipeline, so `run` and
`build` cannot disagree about anything, ever. The cost is a Cranelift pass and a
link per run; both are milliseconds (§10.2).

### 3.3 No recursion limit

`VOCATIONES_MAXIMAE = 64` exists because a tree-walking interpreter recurses on
the host stack and 64 Lingua Serpentis frames is a lot of Rust frames. It was
never a language design decision, and 64 is a strikingly low ceiling for a
compiled language. It dies with the interpreter that motivated it.

Compiled code recurses until the operating system's stack is exhausted —
roughly 40,000–80,000 levels on a default 8 MiB stack, against the 2,000 the
interpreter managed before the cap and the 64 after it.

The price is that stack exhaustion is now a `SIGSEGV` on the guard page rather
than a checked condition, and the robustness invariant does not permit signal
death. §8 is the whole answer to that, and it is the highest-risk part of this
design.

### 3.4 The typed IR is a second output of the checker

`ast.rs` carries no type annotations — no field on any `Expr` variant, no side
table. Everything the checker synthesises is discarded when `check_program`
returns `Ok(())`. Codegen needs those types on every node.

Letting codegen re-derive them was rejected outright: that is D-24's failure
mode (one condition inferred in two places, drifting) reproduced across an
entire backend rather than one function.

So the checker gains a second output. Two shapes were considered:

| shape | verdict |
|---|---|
| `sink: Option<&mut Emissio>` threaded through every method | **Rejected.** Two paths through the checker; the one nobody runs is the one that rots. |
| The checker *always* builds the IR; a new entry point returns it | **Chosen.** One walk, therefore provably identical diagnostics. |

```rust
pub fn check_program(prog: &mut ParsedProgram) -> Result<(), ProgramError> {
    check_program_typed(prog).map(|_| ())
}
pub fn check_program_typed(prog: &mut ParsedProgram)
    -> Result<liss_ir::Programma, ProgramError>;
```

The cost is an IR built and dropped on paths that do not need it. On a corpus
where the pipeline is dominated by process spawning, that is not measurable, and
it buys a guarantee that the checking walk and the lowering walk cannot diverge
— which matters most during the staging period, when the interpreter is still
consuming `check_program`.

### 3.5 `et` and `vel` do not short-circuit, and must not start

`interp.rs:131-135` evaluates both operands of a binary expression and *then*
dispatches on the operator. So `°V / nullus vel verus` is a runtime error today,
not `verus`, and the corpus is blessed against that behaviour.

Emitting the branch a compiler writer's reflexes reach for would be a silent
semantic change to every program that relies on it. Compiled `et` and `vel`
evaluate both sides, unconditionally. If short-circuiting is ever wanted it is a
language change with its own spec, not a codegen detail.

This is recorded as a decision rather than a footnote because it is the single
easiest divergence to introduce by accident in the entire project.

## 4. The Typed IR

`liss-ir` is its own crate. The checker emits it, so `lingua-serpentis` must
depend on it; if it lived inside `liss-codegen` the dependency would be circular
(codegen needs `Type` and `BinOp`, which live in `ast.rs`) and every `cargo test`
touching the lexer would first build some two hundred crates of Cranelift.

### 4.1 Shape

```rust
pub enum Grain { Uncia, Scripulum }                    // mnr | nr
pub enum Ty { Bool, Char, Int, Frac(Grain), Text, List(Box<Ty>) }

pub struct Expr { pub ty: Ty, pub kind: ExprKind }     // every node typed
pub enum ExprKind {
    Const(Const), Load(SlotId),
    Unary  { op: UnOp,  arg: Box<Expr> },
    Binary { op: BinOp, operand_ty: Ty, lhs: Box<Expr>, rhs: Box<Expr> },
    Call   { func: FuncId, args: Vec<Expr> },
    ListLit{ items: Vec<Expr> },
    Index  { base: Box<Expr>, index: Box<Expr> },
    Len    (Box<Expr>),                                // longitudo
    Append { list: Box<Expr>, item: Box<Expr> },       // adde
}

pub struct Block { pub stmts: Vec<Stmt>, pub declares: Vec<SlotId> }
pub struct Func {
    pub symbol: String, pub file: Option<String>, pub is_entry: bool,
    pub params: u32, pub slots: Vec<Ty>, pub ret: Ty, pub body: Block,
}
```

Four properties earn their keep:

- **Every expression node carries its own type.** Codegen never consults an
  environment and never infers. That is the entire purpose of the IR.
- **`Binary` carries `operand_ty` separately** because comparisons produce
  `Bool` from operands that are not `Bool`, and codegen needs to know which
  `cmp` to call.
- **`Block::declares` comes from the checker**, not from codegen re-deriving
  scopes. It is the ownership machinery (§7.3), and re-deriving it would be the
  same mistake as re-deriving types.
- **`Func::file` is `Option`** because single-file mode strips the path, giving
  `Error in linea N:` rather than `Error in 'x.liss', linea N:`. `None` lowers
  to a zero-length file name at the ABI.

  **The checker cannot set this, and the strip must happen before codegen.**
  `check_program_typed` has no single-file signal — the driver builds an ordinary
  one-file program with a synthetic path and strips `ProgramError::file` *after*
  the pipeline. So whoever constructs the `Programma` must clear `Func::file` in
  single-file mode, exactly as the driver already clears the error's.

  This cannot be deferred to a post-hoc fix-up of output, because §9.5 has `liss
  run` spawn the child with **inherited** stdio: the parent never sees the
  child's stderr and cannot rewrite it. The file name is baked into the binary at
  codegen time. Getting this wrong makes every single-file diagnostic read
  `Error in 'fons.liss', linea N:` where the corpus expects `Error in linea N:`.
  Owned by the CLI task, which introduces the file-versus-directory dispatch.

`Type::Float` and `Type::Double` collapse into `Frac(Grain)`. Every operation on
them is identical modulo one constant (12 or 288), so keeping them apart would
double every arm in codegen for no gain. The *checker* keeps them distinct,
because its diagnostics say `mnr` and `nr`.

`Stmt` carries `line` and nothing finer. That is exactly the interpreter's
granularity — `exec` passes `s.line` into every `eval` — so it is exactly the
attribution the corpus is blessed against.

### 4.2 Slots

The checker gains a slot table parallel to its scope stack. Slots are allocated
**per function and never reused**: leaving a block pops the name binding but not
the number. Two consequences, both wanted — shadowing works with no special
case, since an inner `ni x` simply gets a fresh slot and the outer name is
restored on pop; and a function's locals are a flat `Vec<Ty>`, which is exactly
the shape `cranelift_frontend::Variable` wants. Parameters are slots
`0..params`.

### 4.3 Symbols

Identifiers are Unicode-alphabetic (`lexer.rs:96` uses `is_alphabetic`), so
mangling needs a real escape rather than a passthrough:

```
liss_<n>_<ns>_<m>_<name>
```

where `n` and `m` are the byte lengths of the *escaped* components, and any
**character** — not byte — outside `[A-Za-z0-9_]` becomes `_uXXXX_`, `XXXX`
being its Unicode scalar value in lowercase hex. Length-prefixing the escaped
form keeps the mapping injective.

Escaping per character rather than per byte is the only reading consistent with
both worked examples above, and it is the one that matters: identifiers here are
Unicode-alphabetic (`lexer.rs` uses `is_alphabetic()`), so a per-byte escape
would turn one multi-byte character into several `_uXXXX_` groups and lose the
round-trip. An earlier draft of this paragraph said "byte" in one clause and
"scalar value" in the next; per-character is the rule.

- root `initium` → `liss_0__7_initium`
- `mathematica.duplum` → `liss_11_mathematica_6_duplum`

The character immediately after `liss_` is a **digit** for every user function
and a **letter** for every runtime entry point (`liss_int_add`, `liss_panic`).
That is the collision proof, and it is one rule to check rather than a reserved
list to maintain.

### 4.4 What lowering desugars

| construct | desugared | why |
|---|---|---|
| `aut si` | already, by the parser | `parser.rs:288-291` nests it in a one-statement else block |
| `indem` | no | trivially a CFG |
| `dum` | **no** | `continua` must still run the step. The correct desugaring is a CFG, not a statement tree, so codegen builds it directly |
| `longitudo`, `adde` | yes → `Len`, `Append` | they are not functions and have no symbol |
| root `initium` falling off the end | yes | lowering appends `Return(zero)`; every other function is guaranteed to return by the checker's existing all-paths analysis |

## 5. Representation and the C ABI

### 5.1 One machine word per value

| type | representation | owned |
|---|---|---|
| `veritas` | `i8`, 0 or 1 | no |
| `lit` | `i32`, a Unicode scalar value | no |
| `ni` | `*mut LissInt` | yes |
| `mnr`, `nr` | `*mut LissInt`, a grain count; the grain is static | yes |
| `voc` | `*mut LissText` | yes |
| `T lista` | `*mut LissList` | yes |

`ni` is boxed unconditionally. A small-integer optimisation would be a tagged
value, which the ABI rules out, and unconditional boxing makes the ownership
rules uniform — every `ni` is exactly one allocation to free, with no branch.
The consequence, that `ni i = i + °I` allocates twice per iteration, is stated
in the Non-Goals rather than hidden.

### 5.2 Layouts

```c
#define LISS_MAGIC 0x4C495353u          /* 'LISS' */
struct LissHdr  { uint32_t magic; uint32_t kind; };
struct LissInt  { LissHdr h; uint64_t len; uint64_t neg;   uint8_t  digits[]; };
struct LissText { LissHdr h; uint64_t len; uint64_t _rsv;  uint8_t  bytes[];  };
struct LissList { LissHdr h; uint64_t len; uint64_t ekind; uint64_t items[];  };
```

The payload sits at **offset 24 in all three**, so codegen carries one constant
rather than three. `_rsv` on `LissText` exists solely to buy that.

`magic` is checked on every free. It turns a double free or a use-after-free
into a deterministic Latin exit instead of musl heap corruption — and it covers
the live-allocation counter's one blind spot, since an extra free plus a missing
free nets to zero.

**`LissInt`'s payload is `Roman`'s representation**, after a prerequisite
refactor: `Tier { units: u8, five: u8 }` becomes a packed `u8` digit in `0..=9`.
Every access already goes through `Tier::digit()` and `Tier::from_digit()`, so
the change is mechanical and is pinned by twenty-five existing tests. `Roman`
then becomes `{ negative: bool, tiers: Vec<u8> }` — little-endian, canonical, the
empty vector the unique zero — which is byte-for-byte the payload. Conversion is
one `memcpy` each way.

The zero-copy alternative, a header pointing at the `Vec`'s own buffer, costs a
second allocation per integer. For numbers almost always under twenty digits, a
twenty-byte copy beats a `malloc`.

### 5.3 Kind codes

A list must know its element type at runtime in order to free, copy and print
recursively.

```
base: 0 Bool  1 Char  2 Int  3 Frac(Uncia)  4 Frac(Scripulum)  5 Text
kind = base | (depth << 3)
```

`ni lista` is 10; `ni lista lista` is 18; the element kind of kind `k` is `k-8`.
Closed form, unbounded nesting, no interning table.

This is a type descriptor **on the container**, written at every construction
site as a compile-time immediate. Codegen only ever writes it and never
dispatches on it, so it is categorically not the tagged value the ABI forbids.

### 5.4 Everything consumes

**Every runtime *operation* takes ownership of its owned arguments and returns
exactly one new owned value.** Two functions borrow instead:

- `liss_list_put`, which mutates the list it is given;
- **every `liss_*_copy`**, which must borrow its source. `Load` is §7.2's only
  unconditional copy site and reads a slot the function still owns; a consuming
  copy would empty the slot on first read and make every later `Load` of that
  variable a use-after-free. It is doubly forced: `liss_list_copy` is also called
  internally to deep-copy an element the source list still owns.

This is the key simplification of the entire design. It means a statement's
expression tree leaves exactly **one** live temporary — its root — so codegen
never maintains a set of pending temporaries, and "free temporaries at end of
statement" collapses to "free the root if nothing consumed it".

There is **no `liss_enter` / `liss_leave`**. With the recursion cap gone, calls
carry no bookkeeping whatsoever.

#### Freeing a null pointer is a no-op

`liss_int_free`, `liss_text_free` and `liss_list_free` accept a null pointer and
do nothing, exactly as C's `free` does.

This is not a convenience — it is what keeps §7.3.4's "one uniform walk over
`Block::declares`, with no special case" true. `Block::declares` lists every slot
bound *anywhere* in the block, regardless of position, so a `redde` taken before
a later declaration frees a slot whose `Let` never executed:

```
munus initium reddit ni {
  si (verus) { redde nullus; }
  voc s = "salve";
  redde nullus;
}
```

Cranelift requires a `Variable` be defined on every path reaching a use, so
codegen zero-initialises owned slots at entry and frees unconditionally. If null
were fatal, that correct program would exit 1 with a corruption diagnostic. The
alternative — codegen proving initialisation per slot per path — is exactly the
bookkeeping this design exists to avoid.

**The worked example above is a `redde`, and that is why this rule looked
narrower than it is.** A `redde` leaves the function, so it can never observe the
slot again — it cannot see the gap that makes the null store necessary. The rule
that actually matters is §7.3.2's: every free stores null back, so a later
`continua` through the same `declares` finds null rather than a dangling pointer.
Reading this section alone, and then writing a free loop without the store, is
the bug.

#### How diagnostics cross the boundary

```c
_Noreturn void liss_panic(const char *file, uint64_t line,
                          const char *fmt,
                          const LissInt *arg0, const LissInt *arg1);
```

Three properties, each forced rather than chosen:

- **The message is a C string, not pointer-plus-length.** The catalogue ratchet
  extracts the first string literal near a marker, so the sentence must appear
  *inline at the call site*. A length would mean writing the sentence twice at
  one site, or hiding it behind a `const` the ratchet cannot see. For codegen a C
  string is also one `.rodata` symbol and one address, rather than a symbol plus
  a second immediate to keep in step with it.
- **Substitution happens inside `liss_panic`,** because
  `index extra fines: {} (longitudo {})` has two holes and must remain one
  literal at one site. Both slots are `LissInt`, so **a diagnostic needing a
  non-numeric hole cannot use this entry point** — extending it means adding
  `liss_panicN`, never changing this signature, which Tasks 11–15 emit against at
  every raising site.
- **Every fallible operation carries a trailing `(file, line)`** rather than
  setting an ambient locus. An ambient locus is not merely more bookkeeping, it
  is wrong: a raise inside a callee would report whatever line the caller last
  set, unless every call site saved and restored it — which is the shadow call
  stack this design abolished.

#### The runtime's own vocabulary

Beyond the three diagnostics inherited from the interpreter — `divisio per
nullum`, `modulus per nullum`, `index extra fines: {} (longitudo {})` — the
runtime owns four sentences of its own:

| sentence | raised when |
|---|---|
| `memoria non liberata est` | the live-allocation count is nonzero at exit (§7.5) |
| `defectus internus: memoria violata est` | a header's magic fails, or any corrupt input |
| `memoria exhausta est` | `malloc` returns null, or a length computation overflows |
| `acervus vocationum exhaustus est` | the guard page is hit (§8) |

All four are permanently UNCOVERED by design: each means the compiler is broken
or the machine is out of memory. They follow the existing register — subject plus
perfect passive participle plus `est`, agreeing in gender — and the three
`memoria` sentences are deliberately one family distinguished by participle.

### 5.5 `value.rs` is deleted, not extracted

`Value` was the interpreter's tagged runtime representation. Nothing tagged
crosses into compiled code and nothing walks a tree any more, so the enum has no
role.

But `Value::display` *is* the `scribe` formatter, and it must survive. It does,
monomorphically: its seven one-line arms become free functions in
`liss_runtime::scriptio` writing into a sink rather than returning a `String` —
`scribe_int`, `scribe_frac`, `scribe_text`, `scribe_char`, `scribe_bool`. The
list case is inherently `ekind`-driven and lives in `liss_abi::liss_print_list`.

Sink-rather-than-`String` is not cosmetic: it avoids an allocation per print and
is the shape the buffered output in §6 requires.

## 6. Output, Streams and Entry

**Stream *ordering* is not observable and must not be designed for.** The
fixture format stores stdout and stderr in separate files (`emissio`, `nuntii`),
and `cli.rs` captures them separately too. No test in the suite can see how the
two interleave. What *is* observable, and what the design owes:

- **D-08** — everything `scribe` produced before a failure still reaches stdout;
- **D-09** — warnings still reach stderr even when the program then fails;
- **D-10** — the truncated-exit-status note reaches stderr before exit.

All three hold with direct, unordered writes to the two file descriptors. An
earlier draft of this design required stderr-strictly-before-stdout and an
accumulate-everything buffer flushed at exit; that was over-strict, and the
accumulating buffer was actively worse — it would have destroyed streaming for
long-running programs and made memory grow without bound.

**The buffer is a `static mut [u8; 8192]` with a separate `AtomicUsize`
length**, flushed on overflow, at exit, and from the signal handler (§8). Not a
`Vec`: no allocator involvement, no `realloc` window, no lock. `scribe` writes
the bytes *first* and then stores the new length with release ordering, so any
length the signal handler observes names bytes that are fully written. The
program is single-threaded, so there is no second writer.

This is what lets §8's handler flush pending output from a signal context, which
in turn preserves D-08 for the stack-exhaustion case — something the
flush-at-exit design could not have done.

**`SIGPIPE` must be ignored explicitly.** Because `std::rt::init` never runs
(§8.1), the runtime does *not* inherit std's `SIG_IGN` disposition, so the
default would kill the child on a closed pipe. The runtime sets it at startup
and treats `EPIPE` from a raw write as an ordinary end of output, exiting 0 —
reproducing `main.rs::write_or_exit`'s semantics at the source. D-06's handling
moves from the CLI into the runtime, unchanged in behaviour.

The runtime provides `main`; compiled code provides `liss_initium`. Because the
entry point is `#[no_mangle] extern "C" fn main`, `lang_start` is bypassed and
`std::rt::init` never runs — which is what §8.1 turns to advantage.

**`liss_panic` never unwinds.** It flushes and exits; everything live leaks,
exactly as the interpreter dropped everything on its error path. Two
consequences follow, and the design leans on both: the leak assertion runs only
on the success path, and unwinding for `redde` / `abrumpe` / `continua` is a
purely *syntactic* problem — no landing pads, no exception edges, no
`eh_frame`. That is what makes §7 tractable.

## 7. Ownership

### 7.1 The invariant

Every owned word is, at every moment, in exactly one of four places: one slot;
one pending temporary of the executing statement; one element position of one
list; or in flight as a call's return value, owned by the caller on return.

And the dual, which is the half this design originally left unstated: **a slot
holds a live owned value or null, at every moment.** Codegen zero-initialises
owned slots at entry, and every free stores null back into the slot it emptied.
Together those make the free discipline **idempotent**, which is what lets
`Block::declares` stay a single flat set.

**No aliasing, ever.** This is achievable precisely because the language has
pure value semantics — the interpreter cloned on every variable read
(`interp.rs:88`) and on every element read (`interp.rs:182`), so there is no
user-observable sharing to preserve.

### 7.2 Copies

**`Load` is the only unconditional copy site.** Everything else is a move: call
arguments, list literal elements, `Let`, `Assign`. `Index` copies inside
`liss_list_get`. `adde` consumes its list and may therefore *steal* its element
words — allocate a header one longer, `memcpy` the words, store the new element,
free the old header — because the caller has already handed over a private copy.
That is what keeps `adde` in a loop linear in words rather than in deep copies.

### 7.3 Frees

1. **Statement root** — after each statement, free the expression tree's root if
   nothing consumed it. At most one call, by §5.4.
2. **Scope exit** — at the end of each block, free `block.declares` in reverse
   declaration order, **storing null into each slot as it is freed**.

   The null store is not tidiness; without it a legal program double-frees.
   `declares` lists every slot bound anywhere in the block, regardless of
   position, so a `continua` taken before a later declaration frees a slot the
   next iteration's scope exit will free again:

   ```
   indem (cond) {
       si (x) { continua; }
       voc s = "salve";
       si (y) { continua; }   // this one must free s
   }
   ```

   The two `continua`s need *different* free sets for the same block. A
   position-sensitive `declares` could express that, but only by becoming
   per-program-point liveness — precisely the "codegen proving initialisation per
   slot per path" that §5.4 exists to abolish. Storing null keeps `declares` flat
   and makes redundant frees harmless.

   **The cost, and it is worth naming:** once frees are idempotent,
   **over-freeing becomes systematically unobservable**. Only under-freeing (a
   leak, caught by §7.5) and wrong jump targets remain detectable. A test that
   adds a redundant free and still passes has proved nothing.
3. **`Assign` is evaluate → free old → store.** The order is load-bearing:
   `x = x + °I` must read `x` before the old value dies, and if evaluation
   panics the slot must still hold a valid pointer.
4. **`Return`** evaluates, then frees every slot of every enclosing block of the
   function, innermost first. The returned temporary is never one of them — it is
   a fresh copy if it came from a `Load`.

   **The function body block's `declares` *is* the parameter frame** — the
   parameter slots are already its first entries. So this rule is one uniform
   walk over `Block::declares` with no special case, and `Func::params` exists
   for the signature and the ABI only. Do not free `0..params` as a separate
   step: that double-frees every heap parameter on every `redde`.
5. **`Break`** frees from the current block up to and including the loop body,
   then jumps to the exit block. It does *not* free a `dum` header scope; that
   teardown is emitted once, after the exit block, so the loop variable is freed
   exactly once on every path.

   **`Return` is the exception, and it is the one that bites.** `abrumpe` and
   `continua` both reach the exit block, so deferring header teardown there is
   sound for them. `redde` leaves the function without ever reaching it. A
   `dum` header is therefore an ordinary enclosing scope as far as `Return` is
   concerned, and its slots must appear in a `Block::declares` like any other —
   which is why `StmtKind::For` carries its header as a `Block` rather than a
   bare statement. Otherwise
   `dum (voc s = "salve"; verus; s = s) { redde nullus; }` leaks `s`, and §7.5
   turns a legal program into `Error: memoria non liberata est`, exit 1.

   The invariant that makes this tractable: **every owned slot lives in some
   `Block::declares`**, with no exceptions codegen has to remember.
6. **`Continue`** frees the same set, then jumps to the step block (`dum`) or the
   condition block (`indem`).

`IndexAssign` frees the displaced element inside `liss_list_put`.

The subtle case is a `redde` nested inside a loop: the returned value must be
**copied out before its scope is torn down**. That copy-then-free pair is
exactly what *elisio ad ultimum usum* would elide, and exactly what this design
deliberately does not do.

### 7.4 Cranelift discipline

Blocks are sealed the moment their complete predecessor set is known — entry at
creation, a loop header only after its back-edge is emitted. `seal_all_blocks`
is not used, and `is_filled()` is asserted before every `switch_to_block`.
Incorrect sealing is the most common source of `cranelift-frontend` bugs and
fails in ways that do not look like sealing bugs.

### 7.5 The proof

Every allocation passes through one pair of helpers maintaining a live count.
`main` checks it on the success path; a nonzero count writes
`Error: memoria non liberata est` and exits **1**.

Exit 1 specifically, and **not** a distinctive code such as 70. A richer exit
code is tempting — a leak is a distinct failure and deserves a distinct
signal — but `judge` treats any status other than 0, 1 and 101 as "this came
from `initium`, therefore it is the program's own choice and legitimate". A leak
reported as 70 would be *silently accepted* by the robustness harness, which is
the one place we most want it caught. The diagnostic text carries the
distinction; the exit code carries the invariant.

The counter is **always on, in every build** — one relaxed atomic against a
`malloc`. This deviates from the earlier decision to make it debug-only, for two
reasons. With a prebuilt musl archive, "debug build" would be a property of
*which archive is embedded*, doubling the artefact matrix; and an environment
variable gate would make behaviour depend on the environment. If the counter
ever fires the compiler is broken, and a cheap loud invariant is the right shape
for that.

## 8. Stack Exhaustion

Removing the recursion cap means deep recursion faults on the guard page. The
robustness invariant requires every failure to be exit 0 with warnings, or exit
1 with a diagnostic matching `^Error( in '<path>')?(, linea \d+)?: .+$` bearing
no ASCII digits, no Rust type names and no English. `judge` maps a signalled
process straight to a violation, and **D-04 recorded exactly this failure** —
`SIGABRT`, exit 134 — as "outside the error model entirely".

### 8.1 What `std::rt::init` not running buys us

Because the entry point is `#[no_mangle] extern "C" fn main`, std installs no
`SIGSEGV` handler. There is therefore no English `has overflowed its stack`
message to suppress and no handler to coexist with — only a bare kernel signal,
which we catch ourselves. Should the entry point ever become a Rust `fn main`,
our installer must run *after* `std::rt::init` and override it; that is recorded
as an invariant in the source.

### 8.2 The handler

Everything expensive happens at initialisation, before `liss_initium`:

- `sigaltstack` onto a **static** array — by the time the handler runs there is
  no stack left to allocate on. Size is a fixed **64 KiB**, not `SIGSTKSZ`:
  musl's is a small compile-time constant, and on aarch64 with SVE the true
  minimum can exceed it. 64 KiB is unconditionally enough for a handler that
  allocates nothing.
- page size from `sysconf(_SC_PAGESIZE)`, never a hardcoded 4096, because
  aarch64 kernels commonly ship 16 KiB and 64 KiB pages;
- stack bounds, computed once here and never in the handler. **Measured, not
  assumed — and the measurement overturned the obvious answers.** See below.
- `sigaction` for **both `SIGSEGV` and `SIGBUS`** with `SA_ONSTACK | SA_SIGINFO`,
  because on aarch64 a stack fault may arrive as either.

The handler compares `si_addr` against the low bound, flushes the output buffer
to fd 1, writes one of two preformatted byte constants with raw `write(2)`, and
calls `_exit(1)`. No allocation, no formatting, no `std::io` (it locks), and
`_exit` rather than `exit` so no `atexit` handler runs on a corrupted stack.

```
si_addr ∈ [low − 1 MiB, low + page)  → Error: acervus vocationum exhaustus est
otherwise                            → Error: defectus internus: memoria violata est
bounds unknown                       → the first
```

The guard band is **1 MiB**, not one page: Linux's `stack_guard_gap` defaults to
256 pages, so a fault caused by stack growth can land that far below the mapping.

#### Getting the bounds right — the measured result

The main thread's stack **grows on demand**. Every interface that reports the
stack as it *currently exists* therefore reports a small fraction of what it may
become, and the guard page sits at the bottom of the *potential* stack, megabytes
below. Measured on musl, x86_64, with `RLIMIT_STACK` at 16 MiB and a deliberate
overflow:

| method | reports | classification |
|---|---|---|
| `pthread_getattr_np` + `pthread_attr_getstack` | 124 KiB | ✗ wrong |
| `getrlimit` against a local in `main` | 16 MiB, top ~32 KiB low | ✗ wrong |
| `/proc/self/maps` `[stack]` | 152 KiB | ✗ wrong |
| **`[stack]` mapping's high address − `RLIMIT_STACK`** | **16 MiB, correct** | **✓ right** |

The first and third describe only the pages already touched. The second gets the
*size* right but the *top* wrong, because `argv`, the environment and the auxv
sit above `main`'s frame — roughly 32 KiB on a normal invocation, which is
enough to push the computed floor past the real one and misclassify the fault.

**So: take the high address of the `[stack]` mapping from `/proc/self/maps`, and
subtract `RLIMIT_STACK`.** Fall back to `getrlimit` against a local only if
`/proc` is unavailable, and record that the fallback is known-approximate. If
`RLIMIT_STACK` is `RLIM_INFINITY`, treat the bounds as unknown.

An earlier draft of this section listed the three wrong methods in preference
order and did not list the right one. The consequence would not have been an
invariant violation — both branches are well-formed exit-1 Latin diagnostics, so
the robustness suite would still have passed — but every real stack overflow,
the single most common runtime failure a recursive program can have, would have
reported `defectus internus: memoria violata est`. The `errores/acervus-exhaustus`
fixture is what pins this, and it would have failed.

**The handler never reads `ucontext_t`.** Recovering the faulting stack pointer
from `uc_mcontext` would mean hand-rolled musl struct offsets — `gregs[REG_RSP]`
on x86_64, a field after `regs[31]` on aarch64 — with no header to check them
against. Using `si_addr` against precomputed bounds is architecture-independent
and is the reason §8 needs no per-target code beyond the two signal numbers.

The discriminator can afford to be generous, and its failure mode is the point:
**both branches produce a well-formed Latin diagnostic and exit 1**, so
misclassifying yields a wrong-but-well-formed message, never an invariant
violation. A Lingua Serpentis program has no pointers, no FFI and no `unsafe`,
so the only ways compiled code can fault are stack exhaustion and a compiler
bug. When bounds detection fails entirely, the handler reports exhaustion: in a
program whose only unbounded recursion is user-written, that is overwhelmingly
the right guess, and a wrong guess still satisfies the invariant.

`acervus vocationum exhaustus est` — "the stack of calls has been exhausted".
Keeping the `vocatio` root means a reader who knew `vocatio nimis alte
implicata` recognises its successor.

### 8.3 Two requirements that fail silently if omitted

**Stack probes.** `enable_probestack` with `probestack_strategy = "inline"`.
Without them a function whose frame exceeds the guard page can step over it
entirely and fault far below the stack, which both defeats the discriminator and
is a genuine memory-safety hazard. Our frames are small, so this is
belt-and-braces — but it is what makes the guard-page heuristic sound rather
than lucky.

**The chunk-list buffer** (§6). A `Vec<u8>` could be mid-`realloc` when the fault
arrives.

### 8.4 No line number

A signal handler has a return address and no line table, and this design emits
no debug information. The message is therefore structural, which
`parse_diagnostic` classifies as `Diagnostic::Structural` and `judge` accepts —
no file check, no line check, and the text clears the English, digit and
Rust-name rules.

D-04 stays **fixed** by its own acceptance criterion, whose Expected line reads
"a Latin diagnostic and exit 1, *or* a documented recursion limit". The fix
moves from the second disjunct to the first.

## 9. Building and Linking

### 9.1 Crates

```
liss-runtime  (rlib, zero deps)   Roman, fractions, formatters, message constants
     ↑                    ↑
  liss-ir  (rlib)     liss-abi  (staticlib, own workspace, cross-built for musl)
     ↑    ↑               │ embedded as bytes
     │    └────────── liss-codegen  (cranelift + libwild + build.rs)
     │                     ↑
lingua-serpentis ──────────┘
     ↑
 latinitas
```

`liss-abi` is split from `liss-runtime` because every `#[unsafe(no_mangle)]`
symbol — and the runtime's `main` — must live in the crate built as
`crate-type = ["staticlib"]`. In an rlib they are garbage-collected out; in
`liss-runtime`, which the compiler links, `main` would be a duplicate symbol. It
is excluded from the root workspace so `build.rs` can invoke cargo on it without
contending for the outer `.cargo-lock`.

### 9.2 A staticlib is not a program

The locked decisions said "embed the musl staticlib". That is necessary and not
sufficient. Producing a static ELF without rustc also requires, per target, from
`$(rustc --print target-libdir --target=T)/self-contained/`: `crt1.o`, `crti.o`,
`crtn.o`, **`libc.a`** and **`libunwind.a`**.

**Measured: rustc does not bundle `libc.a` into `staticlib` output.** The
archive holds Rust codegen units and `compiler-builtins` objects only — 329
members, not one of which defines `malloc`, `memcpy`, `write` or
`__libc_start_main`. Embedding `libc.a` separately is therefore mandatory, not
conditional.

**`libunwind.a` is required too, and `panic = "abort"` does not avoid it.** The
precompiled musl `libstd` references eighteen `_Unwind_*` symbols regardless of
the crate's panic strategy. Setting `panic = "abort"` on `liss-abi` is still
worth doing — it strips landing pads from our own code and is what makes
"`liss_panic` never unwinds" (§6) true at the codegen level rather than by
convention — but it cannot remove references that arrive precompiled.

The archives are **prelinked once at compiler build time** by a relocatable link
(`-r`), so every program link is one program object plus one runtime object plus
three CRT objects. This removes archive-member resolution from a path that runs
for every compiling fixture in the test suite.

**The host `ld` will not do.** Distribution binutils is single-architecture: the
foreign-target prelink fails with `Relocations in generic ELF (EM: 183)` and
writes a **zero-byte object**. Use a multi-target linker — `rust-lld` from the
toolchain's own sysroot, which is already present because building the compiler
requires rustc anyway. This does not compromise "no rustc on the machine running
the compiler": the shipped compiler holds four `include_bytes!` blobs and spawns
nothing.

#### Strip the prelinked objects

`rust-objcopy --remove-section=.llvmbc --remove-section=.llvmcmd --strip-debug`,
from the same sysroot as the linker.

This is not an optimisation, it is removing weight that is dead **by this
design's own rules**: `.llvmbc` is embedded LLVM bitcode for an LTO that never
runs, and §8.4 states that this design emits no debug information at all. Left
in, they are 7.6 MB of bitcode and ~7 MB of DWARF per target.

| | unstripped | stripped |
|---|---|---|
| x86_64 blob | 18.5 MB | **3.39 MB** |
| aarch64 blob | 19.1 MB | **3.52 MB** |
| both, embedded in the compiler | 35.9 MiB | **6.6 MiB** |
| **every shipped program** | 4.63 MB | **513 KB** |

The last row is the one that matters. Nine-tenths of a compiled Lingua Serpentis
binary is otherwise debug information and bitcode inherited from the runtime
blob. Combine with `--gc-sections` at the program link, worth a further ~0.92 MB.

The compiler binary therefore carries **6.6 MiB** of embedded blobs for two
targets. That is the price of "no rustc on the machine" plus "standalone static
binary", and given those two it is not negotiable — but it is only that small
because the blobs are stripped. Unstripped it is 36 MB, and the difference is
entirely weight this design already declared dead.

### 9.3 Linking in-process

```rust
pub trait Nexus: Send + Sync {
    fn iunge(&self, artefacta: &[(&str, &[u8])], nomen: &str)
        -> Result<Vec<u8>, Defectus>;
}
```

`WildNexus` builds a `MemoriaFS` implementing `libwild::FileSystem` entirely in
memory, parses an argv through `libwild::Args`, and calls
`Linker::with_file_system(fs).run(&args)`. Nothing touches the disk until the
caller writes the finished bytes, with its own permissions and its own Latin
errors.

Two rules are easy to miss and both are load-bearing:

- **`args.on_warning` must be installed.** libwild's default writes English to
  stderr, which breaks the language's one standing rule and would be caught by
  the robustness harness's English denylist. `libwild::error::Error` is likewise
  never forwarded — any link failure becomes one structural Latin diagnostic,
  for the same reason D-22 was a defect.
- **`Signature::call_conv` always comes from `module.isa().default_call_conv()`**,
  never a literal. That single line is what makes the Windows port a recompile
  rather than a rewrite.

### 9.4 The Windows seam, honestly

Carried over unchanged: the IR, the ownership rules, `liss-runtime`, every
`liss_*` signature, and all of codegen bar two constants. Changed:
`BinaryFormat::Coff`, `CallConv::WindowsFastcall`, a windows-gnu staticlib with
a MinGW CRT, and a structured-exception version of §8.

**Absent: a PE/COFF backend in libwild.** Its `Args` type is `Elf | MachO |
Wasm`. The `Nexus` trait is drawn at "objects in, image out" precisely so the
Windows answer is one new implementor and no other change — but the design will
not pretend the answer exists. Either libwild grows PE support or a minimal PE
linker must be budgeted, and that is resolved *before* Windows is scheduled.

### 9.5 The CLI

`lingua-serpentis` remains as a second binary, byte-identical, because its
twenty-three tests are the regression net for the whole pipeline and losing them
in the milestone that swaps the backend is exactly the wrong time. `liss` is the
new surface:

```
liss run   <via.liss | directorium>
liss build <via.liss | directorium> [--exi <via>] [--ad <machina>]
liss       <via.liss | directorium>      ≡ liss run
```

The first argument is a subcommand iff it is exactly `run` or `build`; a folder
named `run` must be given as `./run`, as `git` and `cargo` require.
`write_or_exit` and `finish` move into a shared `cli` module so broken-pipe
handling and stream order have one implementation.

**The driver lives in a library module, not in `src/bin/`.** `catalogus.rs`
scans `crates/*/src` **non-recursively**, so a diagnostic written in
`src/bin/liss.rs` would be invisible to the ratchet — silently, with the header
still reporting the old count. `src/bin/liss.rs` is a shim; the messages live in
`lingua_serpentis::cli`.

`--exi` defaults to the `.liss` stem or the program folder's name, in the cwd;
the artefact is mode 0755, against 0700 for `run`'s unlinked temp. **If the
resolved output path exists and is a directory, refuse** with a Latin diagnostic
naming `--exi`. That case arises whenever a program folder is built by name from
its parent, where the default output path *is* the input directory. Overwriting
an existing file is ordinary compiler behaviour and is allowed.

`run` spawns with **inherited** stdio. This is essential rather than convenient:
the *child's* runtime receives the closed pipe and handles it itself (§6), so
D-06 survives at the source; and `liss run prog | head` needs no re-buffering.
The parent therefore writes no stdout at all, and must emit its Latinity
warnings **before** spawning.

**The temp file is unlinked before it is executed.** Create it `O_EXCL` with
mode 0700, write the image, `unlink` the path, clear `FD_CLOEXEC`, then exec
`/proc/self/fd/<n>`. After the unlink the file has no name and the kernel frees
the inode when the last descriptor closes, so cleanup is unconditional — it
covers normal exit, error exit, panic, `SIGINT`, `SIGTERM` and **`SIGKILL`**,
none of which a `Drop` guard or a signal handler can cover. Concurrent runs
cannot collide, because after the unlink there is no name to collide on.

Where `/proc` is not mounted, the fallback keeps the name, execs the path and
removes it after `wait` returns, with parent-side `SIGINT`/`SIGTERM` handlers
that unlink and re-raise. That path is genuinely best-effort — a `SIGKILL`ed
parent leaves a file behind — which is acceptable for a fallback and is why it
is not the primary design.

A child killed by a signal is reported as `Error: programma abrupte cessavit`
and exit 1. With the handler of §8 installed and `SIGPIPE` ignored, no ordinary
failure reaches this branch, so it fires only on a genuine external kill or a
compiler bug — without it, such a child would exit 139 and be silently accepted
by the robustness harness.

**The subcommands are English**, against the project's otherwise total Latin
discipline. The consequence is concrete: the robustness harness's English
denylist requires an explicit, commented carve-out scoped to the usage string —
not a general weakening of the rule.

### 9.6 The library API is unchanged

`render`, `run_program_with_warnings`, `run_dir_with_warnings` and
`run_file_with_warnings` keep their exact signatures; the `String` is now the
child's captured stdout and the `u8` its exit status.

This is the move that lets the corpus tooling survive the backend swap: the
fixture runner calls what it calls today and receives what it receives today, so
fixture discovery, comparison, blessing and the diagnostic catalogue all keep
working unchanged, and `conformance.rs` needs no edit at all.

## 10. Testing

### 10.1 The property

> The compiled path reproduces the corpus's blessed `emissio`, `nuntii` and
> `exitus` exactly.

Checkable over all eighty-six existing fixtures with no new expectation files.

**Exactly one expectation changes**: `errores/vocatio-nimis-alte-implicata`
becomes `errores/acervus-exhaustus`, with a rewritten `NOTA.md`. Every other
fixture must reproduce byte for byte, and one that does not is a codegen bug.
Of `cli.rs`'s twenty-three tests, twenty-two are untouched and one is updated.

Six new `currunt` fixtures target the free rules where they are most likely
wrong: nested list build/read/free, `adde` in a loop, assignment in a loop,
`redde` from inside `si` inside `dum` inside `indem`, `abrumpe` and `continua`
from a nested `si`, and overwriting an owned list element. The always-on counter
means they need no extra wiring — a leak fails the fixture with a visible stderr
diff.

Three more become possible only once the cap is gone, and two of them cannot
exist today: `currunt/recursio-profunda`, a 5,000-level recursion proving the
handler does not fire spuriously; `currunt/scribe-ante-acervum`, which prints and
then recurses to death, pinning the handler-side flush of §6; and
`currunt/scribe-omnium-typorum`, one value of each of the seven types plus a
nested list, as the end-to-end check on the formatters.

**During the transition there is a differential target.** `tests/compilata.rs`
runs every fixture *both* ways — interpreted and compiled — and requires the two
`Rendered` values to be equal. This is scaffolding with a defined lifetime: it
exists only while both backends do, it is what makes "the compiled path
reproduces the corpus" a checked claim rather than an assertion, and it is
deleted along with the interpreter, at which point `conformance.rs` inherits its
job unchanged.

### 10.2 The cost of a subprocess per fixture

This reverses §3.2 of the conformance suite design. Of eighty-six fixtures,
seventy-four are `errores` and most fail in the lexer, parser or checker,
never reaching codegen; roughly twenty-five actually compile.

**Measured, not estimated.** A prelinked link of a hello-world through libwild
in memory takes a **median of 11.5 ms** (mean 14.5 over 20 runs, min 10.9; the
45 ms first run is cold cache). Adding Cranelift, the image write and a
fork/exec/wait, a compiling fixture costs on the order of 25 ms. That is well
under a second single-threaded for the whole corpus, and a fraction of it with
the harness's default parallelism.

The linked image for a trivial program is **513 KB** — the floor for any Lingua
Serpentis binary, with the runtime blobs stripped and `--gc-sections` on.

That number was 4.63 MB before the blobs were stripped, and the gap is worth
recording: nine-tenths of a compiled binary was debug information and LLVM
bitcode inherited from the runtime object, neither of which this design emits or
uses. "Static musl plus Rust std is not small" turned out to be true of the
build, not of the language.

The earlier "~100× faster in-process" figure was measured against spawning the
*interpreter* — a very different baseline from spawning a static binary that
does almost nothing.

If it does need help, in order: content-hash caching in the harness (perhaps
twenty lines, harness only — never in the shipped compiler, where a stale cache
is a correctness hazard), then one ISA per test thread rather than per fixture.

**The robustness suite is the real risk, not conformance.** Its generator
produces well-typed programs that reach codegen every time. That generator is
capped separately from token soup, defaulted low, and the number is recorded —
this is where a careless default turns `cargo test` from seconds into minutes.

### 10.3 The catalogue, and why there is no shared message table

The ratchet grows the three new crates in its scanned set and the markers
`liss_panic(` and `Defectus::novus(`.

**Runtime diagnostics are written as inline byte-string literals at their call
site, never as references to a shared constant table.** This is counter-intuitive
and is the single most important thing to get right here. `catalogus.rs` reads
the first string literal within 300 bytes of a marker; a site written
`liss_panic(nuntii::DIVISIO_PER_NULLUM.as_ptr(), …)` contains *no literal of its
own* and would be reported `UNPARSED`, hiding the message from the ratchet
entirely. Written inline it is one site, one literal, unique in the workspace,
and therefore `fixtured`. A shared table was proposed while two implementations
existed and had to agree; with one implementation it buys nothing and blinds the
ratchet.

The signal handler's messages are not call sites at all — they are `static`
byte constants with no marker near them, the same situation that already forced
`Err("` and `.ok_or("` into `MARKERS`. The rule is therefore: **every
async-signal-safe message constant is named `NUNTIUS_*`**, and `NUNTIUS_` is a
marker. Enforceable by grep.

Deleting the interpreter makes the catalogue **sharper, not weaker**. Around a
dozen sites disappear with `interp.rs` — nearly all defensive arms the checker
already makes unreachable, reported UNCOVERED today — and only three are reborn
in the runtime: `divisio per nullum`, `modulus per nullum` and
`index extra fines`. New and permanently UNCOVERED by design:
`defectus internus: memoria violata est`, `programma abrupte cessavit` and
`memoria non liberata est`. Expect a re-blessed snapshot with *fewer* sites, a
higher fixtured fraction, and no `ambiguous` entries from cross-crate
duplication — because there is no longer a second implementation to duplicate.

### 10.4 Re-homing what dies with the interpreter

**The timing matters more than the mechanics: everything that uses the
interpreter as a source of truth happens while it still exists**, early in the
milestone, not during the deletion. That is what makes the ported expectations
provably *today's* behaviour rather than the new backend's opinion.

Three things are captured early:

1. **Golden display vectors.** A committed table of `(kind, payload, expected
   string)` generated from the live interpreter — zero, negatives, multi-tier
   vinculum, every twelfth, the maximal sub-uncia stack, empty and nested lists.
   This is the highest-value item in the whole deletion: it captures the
   formatter's behaviour at the one moment both implementations exist, and it
   keeps working forever after.
2. **The ~20 semantic tests in `interp.rs`'s legacy block** — arithmetic and
   precedence, boolean rendering, string concatenation, `dum` with `abrumpe` and
   `continua`, 1-based index reads and writes at both bounds, `longitudo`,
   `adde`, list equality, nested list reads, unbounded arithmetic. Every one is
   *language* behaviour that belongs in the corpus regardless of backend.
   Porting them is ~20 three-line `initium` programs, and it is a strict
   upgrade: a fixture asserts full rendered output rather than a Rust `String`.
3. **A test-only reference formatter.** `value.rs` moves into `liss-runtime` as
   `#[cfg(test)] mod speculum`, renamed so nobody mistakes it for production and
   never compiled into any artefact, so the randomised display-parity property
   test survives verbatim. This *is* a second formatter, which is what the
   deletion exists to end — it earns its place because it is 29 lines, test-only,
   and the alternative is having no randomised oracle at all.

The `roman` and `fraction` unit tests move wholesale with their modules; they
never touched the interpreter. Lexer, `latinitas`, program-assembly, error and
token tests are untouched. Parser tests get a mechanical `munus initium`
wrapper; checker tests on `check_src` are audited and their gaps ported rather
than bulk-rewritten, since the corpus was built from the diagnostic catalogue
and already covers most of them.

Two checker sites become genuinely unreachable once the legacy API is gone.
They are **removed structurally, not left as `unreachable!`**: `current_ret`
becomes non-optional and `root_namespace` a constructor parameter, so
`'redde' extra functionem` and the permissive namespace default cannot be
written at all. That retires an `UNCOVERED` site, which the ratchet should
record as progress rather than absorb.

## 11. Dependencies

| crate | where | why |
|---|---|---|
| `cranelift-codegen`, `-frontend`, `-module`, `-object` | `liss-codegen` | code generation |
| `libwild` | `liss-codegen` | in-process linking; MSRV 1.94, which raises the workspace floor |
| `object` | `liss-codegen` | reading the embedded archives, and test assertions about output |
| `target-lexicon` | `liss-codegen` | triples |
| `libc` | **`liss-abi`** | `sigaltstack`, `sigaction`, `_exit`, `write` — see below |

**`libc` breaks a standing project constraint and does so knowingly.** Every
prior design here carried "zero runtime dependencies" as a global rule, and
`liss-abi` is the first crate to need one. There is no way around it:
`sigaltstack` and `sigaction` are not in std's public API, and §8 is not
optional — without it, stack exhaustion is signal death and the robustness
invariant fails. `liss-abi` already links libc through std-on-musl; the crate
supplies bindings, not new machinery. The constraint is narrowed rather than
abandoned: `liss-runtime`, `liss-ir` and `lingua-serpentis` stay at zero, and
`liss-abi`'s single dependency is justified in one place.

**What "zero" means, now that `lingua-serpentis` is the compiler driver.** The
rule is about *third-party* dependencies. `lingua-serpentis → liss-codegen` is a
first-party edge, and Cranelift and libwild are tabled above with their reasons —
that is the dependency table working, not being circumvented. So the CLI links
them, and still has zero third-party dependencies of its own.

That distinction was tested by the CLI and held. §9.5's temp-file dance names two
operations std cannot do — clearing `FD_CLOEXEC`, and parent-side `SIGINT`
handlers for the `/proc` fallback — and neither is worth a third-party
dependency:

- **The `FD_CLOEXEC` clear is unnecessary on Linux**, measured rather than
  assumed: `execve` resolves the path in `do_open_execat` before
  `do_close_on_exec` runs, which is exactly why `fexecve` plus
  `memfd_create(MFD_CLOEXEC)` is the standard idiom. The step's purpose is
  satisfied without it.
- **The temp file must be reopened read-only before exec**, not exec'd through the
  write handle, or `execve` fails `ETXTBSY`. This is a requirement, not a style
  choice, and it is easy to undo by reordering two lines.
- **The fallback loses its signal handlers**, so `SIGINT` and `SIGTERM` join
  `SIGKILL` in the set that leaves a file behind. That path is already documented
  as best-effort and already requires `/proc` to be unmounted to be reached at
  all; this widens a stated gap rather than opening a new one.

`liss-runtime` is compiled **twice** — for the host, inside the compiler, where
`liss-ir` needs `Roman` for integer constants; and for musl, inside `liss-abi`.
`liss-abi` is compiled once, only for musl, and is never linked into the
compiler. This is precisely why the two crates cannot be one: they need
different targets, different panic strategies and different link fates in the
same build graph.

The nix dev shell gains a `rust-overlay` input pinning one toolchain with both
musl targets, and `qemu` for the aarch64 checkpoint. MSRV rises 1.85 → 1.94.

## 12. Success Criteria

- `liss build examples/computatrum` produces a statically linked ELF that runs
  on a machine with no Rust toolchain and matches its conformance fixture.
- **Before the interpreter is deleted**, every fixture produces byte-identical
  `Rendered` values under both backends, on both architectures. This is the
  gate on the deletion, not a report after it.
- All eighty-six fixtures pass through the compiled path, with exactly one
  intentional re-blessing (§10.1).
- The live-allocation counter reads zero at exit for every `currunt` and
  `monita` fixture, including the six new `memoria-*` cases.
- Unbounded recursion produces `Error: acervus vocationum exhaustus est` and
  exit 1 — never a signal.
- `cargo test --workspace` is green with `interp.rs` deleted, and
  `docs/defectus/2026-08-03-index.md` records D-19 closed and D-04 amended.
- The catalogue snapshot has fewer sites and more fixtured than today.
- A cross-built aarch64 binary runs under `qemu-aarch64`.

## 13. Open Questions

1. **The Windows linker.** Whether libwild grows PE/COFF support or a minimal PE
   linker must be written. Blocks scheduling the Windows milestone; blocks
   nothing in this one.
2. ~~**Whether rustc bundles `libc.a`**~~ — **resolved.** It does not; `libc.a`
   must be embedded per target (§9.2). Measured by the probe.
3. **When to schedule *elisio ad ultimum usum* and *subscriptio mutuata*.** With
   the interpreter gone there is no faster path to fall back to, which is a mild
   argument for sooner than "someday".

   The borrowing subscript has measured instances: `exemplum-series` and
   `series_longa_ad_ultimum_elementum_pertingit` are both quadratic today, and
   both would become linear. It is also the cheaper of the two, needing no
   dataflow — only the rule that an `Index` whose base is a `Load` of a slot may
   read through the slot rather than through a copy.
