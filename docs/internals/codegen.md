# Code generation internals

This document is the back end's design commentary, extracted from the source so that the source can
stop carrying it. It covers `crates/codegen`: the crate layout (`lib.rs`), the Cranelift code
generator (`emissio.rs`), the target and embedded-runtime layer (`target.rs`), the in-process linker
(`nexus.rs`) and the musl cross-build that fills them (`build.rs`). It is a reference, not a
narrative: the ownership discipline is the largest section because it is the part a reader most
needs before touching anything, and every claim here was written against the code and its
measurements rather than deduced from them.

---

## 1. Overview

`codegen` is the back end: the typed IR in, a static ELF out. Three pieces, in the order a
program passes through them.

| Module | |
|---|---|
| `target` | The two targets, and the four musl blobs embedded for each — put into the compiler's `.rodata` at *its* build time by `build.rs`. This is spec §2's first constraint made good: **no rustc on the machine running the compiler**, because there is no toolchain to invoke. |
| `emissio` | Cranelift. `compila` takes a `ir::Programma` and gives back one relocatable ELF object. |
| `nexus` | libwild, driven entirely from memory. `nexus::WildNexus` takes that object plus the four blobs and gives back the finished image. |

**Nothing here touches the filesystem.** `compila` returns bytes and `iunge` returns bytes; whoever
wants a file writes one, with its own permissions.

### `Defectus` (lib.rs)

A back-end failure. Structural and always Latin — it carries no line, because there is no line: by
the time the IR exists the source has type-checked, so everything that can still go wrong is either
the machine (out of memory) or this compiler (a bug). Both read the same to the person holding the
terminal, and the register is the runtime's: subject, perfect passive participle, `est`. `Display`
writes `Error: {nuntius}` — the same shape `ProgramError`'s structural arm writes, so a back-end
failure is indistinguishable in form from a front-end one.

**No foreign error is ever interpolated into one of these.** Cranelift's `ModuleError`, `object`'s
write errors and `libwild::error::Error` all say their piece in English; forwarding any of them
would put English on a user's stderr, which is D-22 exactly — an OS string spliced into a Latin
sentence was a defect then and is one now. What the English *would* have said is recovered from a
debugger or a stack trace, not from the diagnostic.

`Defectus::novus` is the one constructor. Spec §10.3 makes `Defectus::novus(` a catalogue marker,
so the sentence must be written **inline at the call site** and never behind a shared constant — a
site that forwards a constant carries no literal of its own and the ratchet reports it `UNPARSED`.

---

## 2. The IR in, the object out

### What `emissio.rs` covers

**The whole language** — every statement, every type, every operator. Tasks 12–13 brought the
constants, the slots, the binary and unary operators, the four comparisons, the division and modulus
raises, and the whole of control flow (`si`/`aut`, `indem`, `dum`, `abrumpe`, `continua`, `redde`)
with the ownership discipline of spec §7; Task 15 closed it with `lista`.

`ExprKind::Call` is emitted here, one task before the one that owns it, because
`conformance/currunt/exemplum-praecisio` — Task 12's checkpoint — calls a two-parameter `munus`.
What is emitted is the direct call and nothing else: mangled cross-package symbols, deep recursion
and the stack-exhaustion fixture remain Task 14's, and the machinery a call needs beyond
`Module::declare_func_in_func` is exactly zero, because spec §5.4 abolished the shadow call stack.

### `compila` — two passes

**`compila`** declares every function first and only then emits any body, because a call may name a
function declared later in the program — and because `Func::symbol` is the *only* thing tying an
IR `FuncId` to a linker symbol. The bytes it returns are `programma.o` in
`target::Runtime::artefacta`'s link line. Nothing is written anywhere.

### `machina_isa` — the four settings that are load-bearing

**`machina_isa`** builds the ISA for a target with the four settings spec §8.3 and §9.3 make
load-bearing. All failures collapse to `defectus internus: machina institui non potuit`.

| Setting | Why |
|---|---|
| `opt_level = speed` | |
| `is_pic = false` | Static, non-PIE (spec §2). With `is_pic` on, every constant reference would go through a GOT that a `-static --no-pie` link has no dynamic loader to fill in. |
| `enable_probestack = true` | Spec §8.3, and **both halves fail silently if omitted**. A function whose frame is larger than one page can move the stack pointer past the guard page in a single `sub`, touch valid memory below it, and never fault — so the SIGSEGV handler of §8 never runs and deep recursion goes back to being memory corruption instead of `acervus vocationum exhaustus est`. |
| `probestack_strategy = inline` | Rather than the default `outline`: an outline probe is a call to a `__cranelift_probestack` libcall that nothing in the embedded runtime defines, so the link would fail — and it would fail only for programs that happen to have a large frame. |

`the_isa_probes_the_stack_inline_and_is_not_pic` asserts all three of the last three on both
targets.

### `Emissor` — one compilation

`Emissor` holds the `ObjectModule`, the Cranelift `Context` and `FunctionBuilderContext`, and three
maps:

- **`ordo`** — IR `ir::FuncId` → the Cranelift `FuncId` it was declared under, indexed
  directly: `Programma::funcs[id]` *is* the function `id` names (spec §4.1), and the checker
  asserts it as it numbers them, so a `Vec` built in declaration order is the whole mapping a `Call`
  needs. Indexed directly means every function takes its own slot whether or not anything calls it:
  **a `FuncId` that no `Call` names still has to be declared and defined at its index**, or every
  later index is one function out. `tests/vocationes.rs`'s three-package program leaves
  `beta.triplum` deliberately never called for that reason — a package that exports more than its
  users take is the ordinary shape of a library.
- **`externa`** — ABI entry point → its `FuncId`, declared on first use, so an object only names
  the runtime functions its program actually calls. An undefined symbol nothing references still
  costs nothing at link time, but it makes the object's symbol table a lie about what the program
  does.
- **`principium`** — the entry function, for the `INGRESSUS` alias.

**`Emissor::novus`** sets `unwind_info(false)`: `liss_panic` never unwinds (spec §6) — it flushes
and exits, and `redde`/`abrumpe`/`continua` are syntactic rather than exceptional. So there is no
unwinder to feed, and `.eh_frame` would be dead weight in every compiled binary.

**`Emissor::defini`** builds one body. The entry block is sealed at creation (spec §7.4: seal the
moment the complete predecessor set is known — for the entry block that is at creation, it has
none). One `Variable` per slot is declared before anything can use one: slots are allocated per
function and never reused (spec §4.2), so the function's locals are a flat `Vec<Ty>` — exactly
the shape `cranelift_frontend::Variable` wants. Parameters are slots `0..params` and they arrive
**owned**; `Func::params` exists for the signature and that store, and is not a free list — the
parameter frame *is* the body block's scope, so the ordinary teardown frees them exactly once. Every
other slot is zero-initialised here (see §4).

The checker proves every function returns on every path, and lowering appends `Return(zero)` to root
`initium` (spec §4.4), so an unterminated block after the body walk is a codegen bug rather than a
program that can fall off its end; a trap says so at the one moment it could matter instead of
letting Cranelift's verifier say it in its own words.

Constant blobs are declared inside the body walk but can only be *defined* once the builder has let
go of the module, which is why `Corpus::data` exists. They are aligned to 1 — **bytes, not
words**: both a UTF-8 literal and a little-endian digit string are read one `u8` at a time on the
far side.

### `INGRESSUS` and the entry-point alias

`INGRESSUS` is `"liss_initium"`, the symbol the runtime leaves undefined and compiled code supplies
(spec §6). It is *not* the entry function's mangled name — `initium` mangles to
`liss_0__7_initium` like any other root function, because the user may call it recursively and needs
a name to call. So the entry point carries **two symbols at one address**.

**`Emissor::finge`** serialises the object and adds that second name. An ELF alias is not an
assembler `.set`: it is a second symbol-table entry pointing at the same `(section, offset)` with
the same size and kind. `cranelift-module` has no concept of one — a `FuncId` is a declaration and
a declaration is a name — but `cranelift-object` hands back the `object::write::Object` it built,
and `object` has supported exactly this all along. So the entry point is emitted **once** and named
twice, which is what an alias should cost: nothing.

`Symbol` is not `Clone`, hence the field-by-field copy. Every field carried is `Copy`, and every one
of them must be carried: a `value` or `section` that drifted from the original would put
`liss_initium` somewhere that is not `initium`, and the link would still succeed.
`the_object_defines_the_entry_point_and_calls_only_what_it_needs` asserts that the two names share a
section index, address and size, and that the object of `exemplum-salve`'s shape asks for exactly
`["liss_int_new", "liss_print_text", "liss_text_new"]`.

That is the relocatable object, which is this compiler's own output and therefore the easy half. Two
claims can only be made about the **finished image**, and `tests/vocationes.rs`'s
`imago_coniuncta_symbola_servat` makes them on the host target: that `liss_initium` and
`liss_0__7_initium` came out of the linker at **one address** rather than as two definitions it was
free to place apart, and that `--gc-sections` did not take the uncalled export
`liss_4_beta_7_triplum` with it on its way through (§9). Both symbols are looked up with an `expect`
rather than compared as `Option`s — two absent symbols compare equal, and a vacuous pass is exactly
what that test exists to rule out.

---

## 3. Types, kinds and the shared tables

### `codex` — the kind code

The kind code stamped into one value's header (spec §5.3). **Imported, not copied.** The constants
moved down into `runtime::genus`: `abi` is cross-compiled for musl and never linked into
the compiler (spec §11), so codegen cannot depend on *it* — but `runtime` is already a
dependency of both ends and needs no new row in spec §11's table, so the writer and the reader of
these numbers finally share one definition rather than a comment. `abi` re-exports them, so
`abi::KIND_INT` still names this constant.

Only the three numeric codes are *written*, and only by `Corpus::numerus`: `voc`'s is stamped inside
`liss_text_new`, and `veritas` and `lit` are immediates with no header to stamp. The whole closed
form is written out anyway because it is one expression rather than five, and because the container
kind is the thing Task 15 reads off a `Ty` (`Ty::List(inner)` is `codex(inner) + GRADUS`).

### `GRANUM_UNCIAE` / `GRANUM_SCRIPULI` / `granum`

The grain denominators, **read out of the shared table** rather than written again. A second
`Grain → 12 | 288` here would have been the kind-code duplication one function lower down, and a
worse one: the reader is `abi::numeri::exige_granum`, which accepts XII or CCLXXXVIII and
rejects everything else — so emitting `12` for an `nr` **passes that gate** and silently produces
wrong arithmetic (`°II:I/II * °I:I/XXIV` is 30/288 with the right grain and 720/288 with the wrong
one). A wrong answer, not a crash.

`runtime::genus::granum` returns an `Option` because it is total over every kind code, and
these two are the kinds that have a grain. The `Option` is discharged **at compile time**: a `const`
initialiser that panics is a build failure, not a runtime one, so this costs neither a branch nor a
`Defectus` sentence that could never be raised.

The grain denominator is XII for `mnr` and CCLXXXVIII for `nr` (spec §5.1). It travels as an
*argument* rather than in the value, because the grain is a property of the type — which is what
lets `ni`, `mnr` and `nr` share `add`, `sub`, `neg` and `cmp` outright.

### `possessio` and `possidet`

**The one place that decides whether a type owns heap memory, and which entry points manage it.**
`None` is an immediate — spec §5.1's `veritas` and `lit`, which are values and nothing more.
`Some((free, copy))` is one pointer to one header.

| `Ty` | free | copy |
|---|---|---|
| `Bool`, `Char` | — | — |
| `Int`, `Frac(_)` | `liss_int_free` | `liss_int_copy` |
| `Text` | `liss_text_free` | `liss_text_copy` |
| `List(_)` | `liss_list_free` | `liss_list_copy` |

A single table rather than three `match`es, because the three questions ("does the teardown free
it", "must the slot be left null", "what does `Load` copy it with") have to be answered the same way
by construction. Three independent enumerations tied together by a comment is how a seventh `Ty`
gets added to two of them: the leak or the missing null store would be silent, and the compiler
would report nothing, because each `match` is exhaustive on its own.

`ni`, `mnr` and `nr` are all one `*mut LissInt`; the grain is a property of the type and never
travels in the value, which is what lets them share these entry points outright.

### `genus` — the machine type

The machine type one value of `ty` occupies (spec §5.1): one word, always, and the two unowned
scalars are narrower than a pointer — `Bool` is `I8`, `Char` is `I32`, everything else is the
pointer type.

### `verbum` and `ex_verbo` — the generic element word

**`Corpus::verbum`** widens one value to the **`u64` element word** every generic position takes: a
list element, `liss_list_append`'s value, `liss_list_put`'s value (spec §5.1).

**Zero-extension, and the two types it applies to are the two that own nothing.** `veritas` is an
`i8` that is 0 or 1 and `lit` an `i32` holding a scalar value below 0x110000, so both are
non-negative and a sign extension would agree — but the runtime compares element words with `==`
(`aequum_verbum`'s `KIND_BOOL | KIND_CHAR` arm) and frees them by a `word == 0` test, so the upper
bits are not "nobody's business" the way a return value's are. A `veritas` element carrying garbage
above bit 0 compares unequal to an identical one built elsewhere.

A pointer is already the full word and passes through untouched; the test is on the value's own type
rather than on a `Ty`, because both callers have already lost the `Ty` by the time they have a
`Value`.

**`Corpus::ex_verbo`** is the inverse. `liss_list_get` returns a `u64` whatever it read, so a
`veritas lista`'s element arrives 64 bits wide and every consumer of it — `brif`, `band`,
`liss_print_bool` — wants the `i8` `genus` gives that type. The truncation is exact in the only
direction that exists: the runtime zero-extended the same word on the way in.

---

## 4. Ownership and the free discipline

§5.4's rule — **every operation consumes its owned arguments and returns one new owned value**
— is why there is no temporary set in `emissio.rs` and never will be. A statement's expression
tree leaves exactly **one** live temporary, its root, and only `StmtKind::ExprStmt` fails to consume
it. Copies happen at exactly one place: `Load` (§7.2). Everything else moves.

### The whole of §7.3, and where each rule lives

| § | rule | here |
|---|---|---|
| 1 | free the statement root if nothing consumed it | `Corpus::sententia`'s `ExprStmt` |
| 2 | scope exit frees `declares` in reverse order | `Corpus::libera`, called by `Corpus::blocus` |
| 3 | `Assign` is evaluate → free old → store | `Corpus::sententia`'s `Assign` |
| 4 | `redde` frees every enclosing block, innermost first | `Corpus::libera_usque(0)` |
| 5 | `abrumpe` frees to the loop body, not a `dum` header | `Ansa::profunditas`, and `Corpus::dum`'s exit block |
| 6 | `continua` frees the same set, then jumps to the step | `Ansa::gradus` |

**The one invariant that makes this tractable: every owned slot lives in some `Block::declares`.** A
`dum` header is a `Block` for exactly that reason (`ir`'s `StmtKind::For` says so at length),
so `redde` reaches the loop variable without a special case. Restated on `Corpus::ambitus`: **every
owned slot in a function is in exactly one entry of that stack while it is live.**

### §7.1's dual: a slot holds a live owned value or null

This is the half the design originally left unstated: **a slot holds a live owned value or null, at
every moment.** Codegen zero-initialises owned slots at entry, and `Corpus::libera` stores null back
into every slot it frees (§7.3.2). Together those make the free discipline **idempotent**, which is
what lets `Block::declares` stay a single flat set instead of becoming per-program-point liveness.

At function entry (`Emissor::defini`) the zero-initialisation is spec §5.4's contract doing its
work: `Block::declares` lists every slot bound anywhere in the block regardless of position, so a
`redde` taken before a later `Let` frees a slot whose binding never ran — and freeing a null
pointer is a deliberate no-op precisely so that walk needs no special case. For an unowned slot it
is only Cranelift's requirement that a `Variable` be defined on every path reaching a use.

**Why `declares` cannot be made position-sensitive instead.** Two `continua`s in one block need
*different* free sets for the same `declares` — the one before a `Let` must not free that slot,
the one after must — and the only rule that expresses both is per-program-point liveness, which is
exactly the bookkeeping §5.4 exists to abolish. Storing null keeps `declares` one flat set and
makes a redundant free harmless. Spec §7.3.2 carries the argument.

Straight-line code never notices, because a scope is entered once and its slots are never read
again. A **loop** notices, and deterministically:

```text
indem (i minor_est °IV) {
  i = i + °I;
  si (i par_est °II) { continua; }
  voc s = "salve";                 // declared after the `continua`
}
```

`s` is in the body block's `declares` wherever its `Let` sits, so the `continua` frees it — null
on the first iteration, and on the iteration after one that fell through, **the pointer the last
scope exit already freed**. Measured, without this store:
`Error: defectus internus: memoria violata est: verificatio`, exit 1, on a legal program.
`conformance/currunt/memoria-sedes-post-continua` is that program, and it is in the corpus.

The store costs one `iconst 0` and one `def_var` per owned slot per teardown, entirely inside the
SSA `Variable` layer — on the paths that do not need it, it is a constant that no `use_var` ever
reads and it reaches no machine instruction.

### What idempotent frees cost you as a test author, and it is not small

Spec §7.3.2 names this and it is worth repeating where the tests are: **once frees are idempotent,
over-freeing is systematically unobservable.** A second teardown of the same scope — an `abrumpe`
that also swept a `dum` header, a `redde` that walked a scope twice — frees null and does nothing,
and every test still passes.

What remains detectable is **under-freeing** (§7.5's count), **a slot read after its scope died**
(`verifica` treats null as fatal on a *read*, deliberately), and **a wrong jump target** (a wrong
answer, or a hang). A duplicated free of a **temporary** is caught too, but only by the magic word,
and only since D-27: `abolere`'s clear was dead-store-eliminated in every shipped build until that
task made it volatile.

So: **"I added a redundant free and the suite stayed green" proves nothing.** A mutation meant to
test an over-free must free through a fresh `use_var`, the way a real `0..params` sweep would,
rather than by calling `Corpus::libera` twice.

**Task 15 met it, and this is what it found.** The over-free that `libera`'s null store makes benign
really is inert — a `liss_list_free` added after `liss_list_put` changes nothing about the slot,
because the slot is what the teardown reads and it still holds a live pointer. What killed that
mutation was a *different* consequence: the second free clears the magic, so the ordinary scope exit
then frees a header that reads as not-a-`LISS`-object and the program exits 1 with
`defectus internus: memoria violata est: verificatio`. The same is true of freeing after
`liss_print_list`, which already consumed. So the rule to carry forward is narrower than
"over-freeing is invisible": a redundant free of a **slot the discipline has already nulled** is
invisible; a redundant free of anything still reachable is caught, by the magic, and only since
D-27.

### The functions

- **`Corpus::liber`** frees one value if its type owns anything (spec §7.3). Unconditional, and it
  can be: **freeing a null pointer is a deliberate no-op** on the far side (spec §5.4), which is
  what keeps the scope walk one uniform loop with no special case for a slot whose `Let` did not
  run.
- **`Corpus::exemplar`** makes a private copy if the type owns anything. **`Load` is the only
  unconditional copy site** (spec §7.2); everything else — call arguments, `Let`, `Assign`, list
  elements — is a move. The `_copy` family is one of the two that **borrow** rather than consume,
  and it is forced: a consuming copy would empty the slot on first read and make every later `Load`
  of that variable a use-after-free.
- **`Corpus::libera`** frees a scope's slots in **reverse declaration order** (spec §7.3.2),
  leaving each one **null** — see above.
- **`Corpus::libera_usque`** tears down every scope from `profunditas` to the innermost, in that
  order: what `redde` (`profunditas` 0), `abrumpe` and `continua` each do before they jump (spec
  §7.3.4–6). The clone of the open-scope stack is taken because `libera` needs `&mut self`; slot
  lists are short and a jump is rare, and the alternative is threading an index through every free.

### Statement-level ownership

- **`Let`** is a move, not a copy (spec §7.2): the temporary the expression produced becomes the
  slot's, and there is nothing to free — the slot was zero-initialised at entry and a `Let` is a
  fresh binding, never a rebinding.
- **`Assign`** is **evaluate → free old → store** (spec §7.3.3), and the order is load-bearing
  in both directions: `x = x + °I` must read `x` before the old value dies, and if the evaluation
  raises, the slot must still hold a valid pointer for the teardown that never comes — a raise
  does not unwind, but a half-freed slot would still be a double free if anything later walked it.
- **`Print`** — every `liss_print_*` **consumes** its argument (spec §5.4), so the value built
  one line above is freed inside the call and nothing is left to tidy up. That is the whole of
  §7.3.1 for `scribe`. `Frac` is the one printer that needs the grain, because `mnr` and `nr` are
  the same word and different sentences. `List` is the one printer that recurses, and it does so on
  the **child's own header** rather than on a kind code codegen computed: the container's `ekind`
  names the element type, and a nested element is a `LissList` whose own `ekind` names *its*
  elements. So `scribe` of an `ni lista lista` needs no more from here than `scribe` of an `ni
  lista` — one pointer, consumed.
- **`ExprStmt`** is **the one statement whose root nothing consumes** (spec §7.3.1). Because
  everything else consumes, "free temporaries at end of statement" collapses to exactly this one
  call.
- **`Return`** evaluates, then frees every slot of every enclosing block, innermost first (§7.3.4).
  The returned temporary is never among them — if it came from a `Load` it is a fresh copy. "Every
  enclosing block" is exhaustive and needs no supplement: the parameter frame *is* the body block,
  so freeing `0..params` again off `Func::params` would double-free every heap parameter on every
  `redde`.
- **`Call` arguments** are **moved** (spec §7.2): the callee consumes them and returns one new
  owned value, so there is no bookkeeping at all — no `liss_enter`, no `liss_leave`, no shadow
  stack.

---

## 5. Lists, the only recursive type

`lista` is the only recursive type, the only place a kind code is read at run time, and the only
place a deep copy and a deep free can disagree. Three consequences live in `emissio.rs`:

- **`ekind` is written once, at construction, and never read here.** Codegen is the writer and the
  runtime the reader (`codex`); the walks that free, copy, compare and print a nested list recurse
  on the *child's own header*, so nothing above `Corpus::series` ever computes a `k - GRADUS`.
- **`liss_list_new`'s element words are codegen's to write.** There is no per-element setter in the
  ABI, because there is nothing for one to do that a `store` does not; `Corpus::repone` is the whole
  of the layout, and `ONUS` and `VERBUM` are shared with the runtime through
  `runtime::dispositio` rather than written a second time here. The words do come back
  **zeroed** — `allocare_nulla` is `calloc` — but nothing here leans on that: `Corpus::series`
  emits the constructor and then exactly `items.len()` stores with no call in between, so no
  partially built list is ever observable. The zeroing is a net for the *failing* path, where an
  element expression raises before its store; see `abi::memoria::allocare_nulla`.
- **`liss_list_put` borrows.** It is one of exactly two non-consuming entry points, and the only one
  this file reaches with a bare `use_var` — see `Corpus::sententia`'s `IndexAssign`.

### `Corpus::series` — a list literal

One `liss_list_new` of the final length, then one **direct store** per element (spec §5.2).

The **element** kind is taken off the node's own type rather than computed as `codex(ty) - GRADUS`:
an empty literal has no item to read one off, and a subtraction is a thing that can underflow where
a projection cannot.

**Elements first, then the header.** Evaluating an element can raise — `[°I / nullus]` is a
program someone will write — and a raise does not unwind, so a header allocated before its
elements would be a live allocation at the moment `liss_panic` runs. That costs nothing on the
success path and it is what keeps the failing path's diagnostic the only thing the user sees.

Each element is **moved** into the list. Nothing is copied and nothing is freed: the temporaries the
item expressions produced become the list's, exactly as `Let` takes a temporary into a slot.

### `Corpus::repone` — the payload layout

Stores one element word at `ONUS + VERBUM*i` — the whole of the list payload's layout, in one
place.

`MemFlagsData::trusted()`: the store cannot trap, because the pointer came from `liss_list_new` two
instructions ago and `i` is below the length it was given; and it is aligned, because `ONUS` and
`VERBUM` are both multiples of eight and the allocator's alignment is at least that.

The `i32` offset is Cranelift's, not the layout's. A literal with more than 2^28 elements cannot be
written — the source would be gigabytes — but it is one `iadd_imm` to be total rather than a
`try_from` whose error arm is a sentence nobody can provoke. `saturating` arithmetic for the same
reason: an overflow that no `Vec<Expr>` can reach is still not a panic worth shipping.

**One test would notice a store folded to the wrong displacement**, and it is `series.rs`'s
`series_longa_ad_ultimum_elementum_pertingit`: three hundred elements built by `adde`, read back at
both ends and summed, whose last element sits **2416 bytes from its header**. It is also the shape
that would go quadratic first if the steal (below) ever regressed to a copy.

### The list expressions

- **`ExprKind::Index`** — `xs[i]` is **1-based, bounds-checked, and a deep copy** (spec §5.4).
  The copy is not an optimisation this file could skip: it was `interp.rs:182`'s
  `items[i - 1].clone()`, and it is what keeps the no-aliasing invariant — after the call the
  element is still in the list *and* the result is a value this expression owns and something above
  must free.
  Both operands are consumed inside the call, `base` included, so an `xs[i]` whose base is a `Load`
  allocates and frees a whole copy of the list. That is the price of "`Load` is the only copy site"
  — a borrowing `get` would need a second rule about which expressions may borrow, which is spec
  §13's still-unscheduled *subscriptio mutuata*. The result crosses as a `u64` whatever the element
  is (spec §5.1), and `e.ty` is the **element** type: the IR carries it on the node, so there is
  nothing to strip off `base.ty`.
- **`ExprKind::Len`** — `longitudo(xs)` consumes the list and returns a `ni`.
- **`ExprKind::Append`** — `adde(xs, v)` is a **new** list one longer; the old one is consumed and
  its element words are **stolen**, not copied (spec §5.4). Nothing about that stealing is visible
  from the call site, and that is the point: this site would be identical if `_append` deep-copied
  every element, so the property is the ABI's to hold and `abi`'s
  `append_in_a_loop_stays_balanced_and_copies_no_element` is the only thing that can measure it —
  it reads the live count after each of a hundred appends and finds `1 + i` rather than `1 +
  i(i+1)/2`. What codegen owes is to reach this entry point and add nothing, which
  `tests/series.rs`'s `series_addita_nihil_praeter_appendicem_vocat` reads off the object — and
  what that reading can and cannot see is worth knowing. A codegen that implemented `adde` as "a new
  list one longer, then copy each element across" would produce identical output and would show up
  there as **an extra `liss_list_new`** — and that is the whole of what catches it. The per-element
  copies would *not* show: the program is `ni`-typed and the test's `SPECTANDA` filter carries
  `liss_text_copy` but no `liss_int_copy`, deliberately, because a list of `voc` is the only element
  kind whose copies would survive the filter, and adding every scalar `_copy` would drown the
  sequence in the `Load`s. The extra `new` is sufficient, and it is the assertion doing the work.
- **`StmtKind::IndexAssign`** — `xs[i] = v`, **the one statement that mutates through a slot
  instead of rebinding it** (spec §7.3). The list is **borrowed**: `liss_list_put` is one of the
  ABI's two non-consuming entry points, so the pointer comes off the slot by a bare `use_var` and
  neither a copy before nor a free after. A `Load` here would be the silent version of this bug —
  it would copy the list, write the element into the copy, and free the copy, leaving the program's
  own list untouched and every later read correct-looking but stale. Nothing is left to free:
  `index` and `value` are consumed inside the call, and the element they displace is freed there too
  — that is the whole of §7.3 for this statement, and it is why the element type never appears.

  **Index, then value.** Pinned by `series.rs`'s `ordo_evaluationis_a_fonte_legitur`, whose recorded
  answer is the interpreter's: an `xs[f()] = g()` whose two calls both raise names the wrong one if
  the two lines swap. `liss_list_put`'s bounds check happens after both, so nothing else in the
  suite can see the order. It was `interp.rs:267-268`'s. The slot is read **after** both operands,
  for the same reason `Assign` evaluates before it frees: the slot must be observed at the moment
  the write happens, not before the expressions that decide what is written.

### Evaluation order, and the only way to observe it

**Order is otherwise invisible in this language** — it has no side-effecting expression except a
raise, and a raise ends the process — so the only way to ask which operand went first is to make
both of them fail and read which diagnostic came out. `divisio per nullum` and `modulus per nullum`
are two sentences from the same shape of expression, which is exactly the pair needed.

`series.rs`'s `ordo_evaluationis_a_fonte_legitur` pins four orders, and each is the interpreter's,
recorded from it before the deletion:

- a literal's elements, **left to right** (`eval` walked the `Vec`);
- `xs[i]`: **base then index** (was `interp.rs`'s `Index` arm);
- `adde(xs, v)`: **list then item**, which is argument order;
- `xs[i] = v`: **index then value** (was `interp.rs:267-268`) — the one a codegen author is most
  likely to get backwards, for the reason given under `IndexAssign` above.

---

## 6. The two silent rules

### `et` and `vel` evaluate both operands (spec §3.5)

**The rule a codegen author is most likely to break, and it is silent.** They do not short-circuit.
The rule originated with `interp.rs:131-135`, which evaluated both sides and *then* dispatched on
the operator; what holds it now is `conformance/errores/{vel,et}-utrumque-latus`, whose right
operand divides by zero, so evaluating both raises and short-circuiting prints the left operand and
exits 0 — a difference on stdout, stderr and status at once. So
`verus vel (°V / nullus par_est °I)` is a runtime error rather than `verus`.

**Until Task 19 no fixture covered this at all**, and three comments including the module header
said one did. Not one of the corpus's `.liss` programs used `et` or `vel` as an operator; the rule
rested entirely on `scalares.rs`'s two unit tests — `{vel,et}_dextrum_latus_quoque_aestimat` —
which those comments described as a redundant second check. Anyone tidying them away on that basis
would have left the invariant unguarded.

There is no `And`/`Or` control-flow form in the IR and there must not be one; `Corpus::expressio`'s
`Binary` arm evaluates `lhs` and then `rhs` unconditionally, and `et`/`vel` are `band`/`bor` over
the two results — one bitwise instruction over two `i8`s that are 0 or 1. Emitting the branch a
compiler writer's reflexes reach for would be a silent semantic change to every program that relies
on it. If short-circuiting is ever wanted it is a language change with its own spec, not a codegen
detail.

### `Signature::call_conv` always comes from `module.isa().default_call_conv()` (spec §9.3)

There is one place in `emissio.rs` that builds a `Signature` — **`signum`** — and that is why: a
literal `CallConv::SystemV` works on both targets today and quietly makes the Windows port a rewrite
rather than a recompile. A free function rather than a method, because both `Emissor` and `Corpus`
need it while holding different halves of the same borrow.

`each_signature_carries_its_target_s_own_call_convention` asserts this **on the `Signature` `signum`
builds, not on the ISA**. The earlier form compared `isa.default_call_conv()` with
`CallConv::triple_default(isa.triple())`, which is a property of Cranelift and holds no matter what
this crate does — it would have passed unchanged over a `signum` that wrote `CallConv::SystemV`.
What the rule is about is the signature, so that is what is inspected.

### Why the extension, and why it is not optional

`signum` is also the only place an `AbiParam` is built, which is what makes the zero-extension of
`veritas` and `lit` a property of the file rather than a habit.

`Ty::Bool` and `Ty::Char` are the only values narrower than a register: `i8` holding 0 or 1, and
`i32` holding a Unicode scalar value (spec §5.1). `ArgumentExtension::None` is self-consistent for
a liss→liss call — both ends agree to look only at the low bits — and **wrong the moment a
runtime entry point is on the other end**. Rust's `extern "C" fn(v: i8)` carries an extension
attribute that the callee is entitled to rely on, and Cranelift will not add one for you; a
`liss_print_bool` reached with garbage in the upper 56 bits of the register is a call that satisfies
neither side's idea of the ABI.

Zero rather than sign extension, for the two values that exist: `veritas` is 0 or 1 and `lit` is a
scalar value below 0x110000, so both are non-negative and the two extensions coincide. Where they
coincide, the one that matches how the word is passed *generically* — list elements and
`liss_list_get`'s result are `u64`, zero-extended (spec §5.1) — is the one to write, so there is
one rule for narrow scalars rather than two.

Returns are left unextended, and that is not an oversight: this crate never reads more of a returned
value than its declared type, so `liss_int_cmp`'s `i8` is used as an `i8` and the bits above it are
nobody's business.

`the_two_narrow_scalars_cross_the_abi_zero_extended` asserts all four cases — `Uext` on `veritas`
and `lit`, `None` on a full-width pointer parameter, `None` on the return. It is asserted there
rather than left to a behavioural test because the wrong answer is *usually* invisible: the register
is usually already clean.

---

## 7. Control flow

### `Nodus`, `terminatus` and `transi`

`cranelift_codegen::ir::Block` is aliased to **`Nodus`** — one machine-level basic block —
because `ir::Block` is a *lexical* block, a list of statements and the slots it declares, and
the two appear within three lines of each other everywhere control flow is built.

**`Corpus::terminatus`** answers whether the block being built already ends in a terminator. Spec
§7.4 says to assert `is_filled()` before every `switch_to_block`, and that is still the rule —
but `FunctionBuilder::is_filled` is **private** as of `cranelift-frontend` 0.134, so the question
has to be put to the layout instead. The answer is the same one: the last instruction of the current
block, if any, is a terminator.

**`Corpus::transi`** moves the cursor, having first satisfied `switch_to_block`'s precondition —
and **the precondition is a disjunction, so asserting only one leg is wrong**. `switch_to_block`
accepts

```text
position.is_none() || is_unreachable() || is_pristine(pos) || is_filled(pos)
```

and *pristine* is the leg that matters here: a block the builder has switched to but emitted nothing
into is still pristine, which is exactly an empty `aut { }` arm and a `dum` whose step lowered to
nothing. Asserting `terminatus()` alone would fire on both — legal programs, both of them, and
neither an error a user could act on.

The layout answers all three legs at once: no current block, or a current block into which nothing
has been emitted (pristine), or one whose last instruction is a terminator (filled).
`is_unreachable` needs no separate test — an unreachable block is one of the other two here,
because this crate never leaves one half-built.

`debug_assert!` rather than `assert!`, deliberately: it mirrors `switch_to_block`'s own, which is
compiled out of a release `cranelift-frontend` too, and a codegen bug must not become a panic in a
user's release build. The `Defectus` for a genuinely broken CFG is Cranelift's verifier, one call
later, which does run in release.

### `Corpus::blocus` — one lexical block

`blocus` pushes `block.declares` onto `Corpus::ambitus` and pops it on **every** path, the error one
included, exactly as `Corpus::dum` does for a header — the two are the only pushers of that stack
and they must agree about it. (A `Defectus` abandons the whole function today, so an unbalanced
stack is invisible; that is an argument about the caller, not about this function, and the next
caller may be a different one.)

`blocus` is **not** used for the `dum` header, which is a `Block` whose scope outlives its statement
list.

**Stop at the first terminator**, because the checker admits a statement after one. Its all-paths
analysis asks whether *any* statement returns, not whether the last does (`checker.rs`'s
`block_returns` — "a `redde` followed by unreachable code still returns, and this language has no
dead-code lint to disagree with"), so `redde nullus; scribe("numquam");` is a legal program that
runs to an empty stdout and exit 0. Without the guard the `redde` fills the Cranelift block and the next statement
adds an instruction to a filled one: a panic in debug, and in release a `Defectus` claiming an
internal compiler failure over a program that is simply valid.

**`abrumpe` and `continua` need the same guard**, because both fill the block with a `jump` exactly
as `redde` fills it with a `return`. `sententia_post_abrumpe_neglegitur` exercises both spellings.

**Every walk over a statement list that can hold a terminator owes it.** `Corpus::dum_intus`'s walk
over a header does not, because a `dum` init clause is a declaration or an assignment and nothing
else parses (D-26; see *A terminator in the header* below). An earlier draft claimed this guard
was the only one needed at a time when the header could hold a `redde`, and was wrong; the header
crashed the compiler on `dum (redde nullus; …)`. **Whoever adds a statement walk over a list that can hold one owes it.**

Scope exit (spec §7.3.2) runs at the end of the block *unless* it already ended in a `redde`, an
`abrumpe` or a `continua`, each of which freed this scope itself on its way out. Running it again is
not a missed free but a **double** one, and the same `terminatus()` prevents both.

### `Ansa` — one enclosing loop

`Ansa` records what `abrumpe` and `continua` see (spec §7.3.5–6). The loops currently open are a
stack rather than a pair, because both keywords bind to the **innermost** enclosing loop.

- **`exitus`** — `abrumpe`'s target: the block after the loop, where a `dum`'s header teardown is
  emitted once for every path that reaches it.
- **`gradus`** — `continua`'s target, **and the two loops differ**. For `dum` it is the **step**
  block, because `continua` must still run the update clause
  (`conformance/currunt/dum-continua-currit` is blessed against exactly that — it was
  `interp.rs`'s `continue_skips_iteration` before Task 19 — and a jump to the condition instead is
  an infinite loop, not a wrong answer, so the fixture hangs rather than diffs). For `indem` it is
  the condition block, which is all there is.
- **`profunditas`** — the index in `Corpus::ambitus` of the loop **body**'s scope. Both keywords
  free `ambitus[profunditas..]`, innermost first: "from the current block up to and including the
  loop body". A `dum` header sits at `profunditas - 1` and is deliberately *not* in that range —
  its teardown is emitted once after the exit block, so freeing it here would be the second free of
  the same pointer.

**The two jumps differ only in where they land** (spec §7.3.5–6): both free from the current
block up to and including the loop body, and neither frees a `dum` header.

### `Corpus::si` — `si` / `aut`

Spec §4.4's flattened form, so there is no else-if arm.

```text
       brif cond
       ├── tunc ──┐
       └── aliud ─┤     (`exitus` directly, when there is no `aut`)
               exitus
```

A `veritas` is an immediate and owns nothing (spec §5.1), so the condition leaves no temporary for
§7.3.1 to free — the `brif` is the one place in this file a value is consumed by an instruction
rather than by a call.

**Sealing.** `tunc` and `aliud` have exactly one predecessor each, the `brif`, and are sealed the
moment it is emitted. `exitus` is sealed only after both arms have had their chance to jump to it
— that is its complete predecessor set, and sealing it a line earlier is the most common
`cranelift-frontend` bug there is.

**When neither arm falls through** — both `redde`, which is how a function with no trailing
statement is written — `exitus` is left *pristine*: created, sealed, never switched to, never
inserted into the layout, and therefore exempt from `finalize`'s "every block is filled" check. The
cursor stays on the second arm's filled block, so `blocus`'s `terminatus()` guard sees the statement
list is over and skips the scope teardown that `redde` already did.

### `Corpus::indem` — a condition-only loop

```text
       jump ──▶ caput ──brif──▶ corpus ──┐
                  ▲                      │
                  └──────────────────────┘   (and every `continua`)
                    └─false─▶ exitus         (and every `abrumpe`)
```

**Sealing.** `caput` is the loop header and is sealed **last**, after the body's back edge exists
— that is the one block in this file whose predecessor set is not complete when it is created, and
the reason `seal_all_blocks` is not used. `corpus` is sealed at the `brif`, its only predecessor.
`exitus` takes the `brif`'s false edge plus one edge per `abrumpe`, so it is sealed after the body
too.

`indem` has no header scope of its own: the loop introduces no binding, so `Ansa::profunditas` is
the depth at which `blocus` is about to push the body's.

The condition is re-evaluated on every iteration, and `Corpus::linea` is reset to the `indem`'s own
line before it, so any raise inside it names that line and not whatever the body last set.

### `Corpus::dum` — init, condition, step, and the one loop with a scope of its own

```text
  header.stmts ─jump─▶ caput ──brif──▶ corpus ──▶ gradus ──┐
                         ▲                 (and `continua`)│
                         └────────────────────────────────-┘
                           └─false─▶ exitus ─▶ free(header.declares)
                                       ▲ (and every `abrumpe`)
```

**The header teardown is emitted once, in the exit block** (spec §7.3.5), rather than on the back
edge — which is what makes the loop variable die exactly once however many `abrumpe`s and
iterations there are. It is *not* what makes `redde` correct: a `redde` inside the body leaves the
function without ever reaching `exitus`, so the header is pushed onto `Corpus::ambitus` as an
ordinary scope and §7.3.4's walk frees it there. Both, and neither alone.

The header's scope opens *before* its init statement, so a `redde` anywhere below finds the loop
variable on the scope stack. Not `blocus`, because `blocus` would tear the scope down at the end of
the init clause, which is where the loop begins rather than where it ends. It is popped on the error
path too: a `Defectus` abandons the function, but leaving the stack unbalanced would make the *next*
thing to read it wrong rather than absent.

**Sealing** is `indem`'s, with `gradus` added: it takes the body's fall-through and one edge per
`continua`, so it is sealed after the body and before the cursor moves onto it. `Corpus::linea` is
restored to the `dum`'s own line after the init clause, which moved it (spec §4.1: `Stmt` carries
`line` and codegen carries the line of the statement it is emitting).

#### A terminator in the header (D-26)

**The parser makes the shape unconstructible, so `dum_intus` walks the header plainly.** A `dum`
init clause is a declaration (`TYPE ident = expr ;`) or an assignment (`ident = expr ;`), decided on
lookahead in `Parser::for_stmt`; anything else is `clausula prima capitis 'dum' declaratio vel
assignatio esse debet`. A `Let` or an `Assign` cannot fill a Cranelift block, so the header walk is
`for stmt in &header.stmts { self.sententia(stmt)?; }` with no `terminatus()` check, and the
`jump caput` that follows it always lands in an open block.

Until D-26 closed the init clause was `self.statement()` — any statement — and the walk carried the
same guard as `blocus`, returning rather than breaking, because the instruction that panicked was the
`jump caput` and not a later header statement. The compiled reading it produced (`redde` returned,
`abrumpe`/`continua` bound to the enclosing loop) was never chosen; the record of it is
`rationale.md` Part 2, and the four `conformance/errores/*-in-capite-dum` fixtures are what hold the
rejection now. If the header grammar ever widens to a statement that can terminate, the guard comes
back with it.

---

## 8. Constants, operators and comparisons

### `Corpus::constans` — a read-only blob

A pointer to a fresh read-only blob and its length — the `(ptr, len)` pair every `liss_*_new`
takes.

**Empty is a null pointer and no blob at all.** Both constructors on the far side document `len ==
0` as not reading the pointer — `liss_text_new` for `""` and `liss_int_new` for `nullus` — and a
zero-length data object is not something `cranelift-object` will give a symbol to, so there is
nothing to point at even if it did.

Every constant is anonymous and local: a literal is not a symbol anyone can name, and two identical
literals in one program are two blobs. Interning them is a size optimisation with no ownership
question attached — both constructors copy — but it is not a walking skeleton's.

`a_text_constant_is_static_data` checks the literal reached the object verbatim.

### `Corpus::locus` — the `(file, line)` pair

The pair every fallible entry point takes (spec §5.4). **Not an ambient locus.** A raise inside a
callee must name the callee's line, and an ambient one would name whatever line the caller last set
unless every call site saved and restored it — which is the shadow call stack this design
abolished. So it travels as two arguments.

The file name is a **C string**, and null and empty both mean "no file": spec §4.1 lowers
`Func::file: None` to a zero-length name, `liss_panic` accepts either, and a null costs no `.rodata`
at all. One blob per function, however many raises it contains.

`Corpus::linea` is the line of the statement being emitted. `Stmt` carries `line` and nothing finer,
which is exactly the interpreter's granularity — `exec` passed `s.line` into every `eval` — so
it is exactly the attribution the corpus is blessed against.

### `Corpus::numerus` — one boxed number

`ni`, `mnr` or `nr`, from a static digit blob. The blob is `Roman`'s own representation, byte for
byte: little-endian digits `0..=9`, canonical, the empty payload the unique zero (spec §5.2).
Emitting it as static data is why `liss_int_new` takes a blob rather than an `i64` — the
language's integers are unbounded and no immediate could carry one — and why the sign is a
separate argument.

### `Corpus::unarium` — `non` and unary `-`

`veritas` is 0 or 1, so complementing the low bit *is* `non`.

**`-<literal>` is folded, and this is the first place `Roman::negative()` matters.** `-°V` parses
to `Unary{Neg}` over a `Const` — the lexer builds every numeral through `from_u64`, so a negative
`Const::Int` was unreachable until then and the `neg` argument of `liss_int_new` was a literal `0`
in every object this compiler had ever emitted. Folding makes it real: the sign travels in the
emitted call rather than through an allocate-negate-free round trip, and
`numeri_negativi_in_constantes_conflantur` is the first test of the accessor that reads it back.

**One level, deliberately.** `- -°V` still calls `liss_int_neg` once, because folding transitively
is a constant folder and a constant folder is a pass, not a match arm. The case that a fold gets
wrong is `-nullus`, and it is `Roman::neg`'s to get right: negating an empty magnitude must stay the
canonical, non-negative zero, or structural equality stops being numeric equality on the far side.

### `Corpus::binarium`

The dispatch is on `operand_ty`, **not** on the node's own `ty`: a comparison yields `veritas` from
operands that are not, which is the whole reason the IR carries the two separately (spec §4.1).

`add` and `sub` are **shared** by `ni`, `mnr` and `nr`: adding two counts of the same grain is
adding two integers. `mul` and `div` are where the grain finally shows — a fraction's product
rescales by XII or CCLXXXVIII, an integer's does not — so `liss_frac_mul` and `liss_frac_div` take
the grain as an argument. Concatenation is the only arithmetic `voc` has (`liss_text_concat`).

**The divisor is not pre-checked here, and must not be:** the runtime raises `divisio per nullum` /
`modulus per nullum` itself, from the one site that carries each sentence. A check emitted here
would duplicate the sentence and blind the catalogue's ratchet to whichever copy stayed uncovered
(D-24).

### `Corpus::comparatio`

`par_est`, `par_non_est`, `maior_est`, `minor_est`. The owned types go through **one three-way
compare** each rather than four predicates: codegen compares the result against an immediate, which
is one instruction, and the runtime carries one implementation of the ordering. `liss_int_cmp` and
`liss_text_cmp` both consume their operands, and their result is `-1`, `0` or `1` and **signed** —
`minor_est` on `ni` is the `-1` case, and an unsigned compare against zero would read it as 255.

The two unowned scalars are immediates, and the machine's own compare *is* their ordering: `veritas`
is 0 or 1 and `lit` is a scalar value below 0x110000, so both are non-negative and the unsigned
comparison is the one `Value::cmp` gave them. There is no `liss_bool_cmp` or `liss_char_cmp` and
there must not be.

**Lists are the one comparison that is not a three-way compare**, and routing them through the
`icmp_imm_s(cc, ordo, 0)` tail would be a silent wrong answer rather than a build failure:
`liss_list_eq` returns **1 for equal**, so `IntCC::Equal` against zero — which is what `par_est`
lowers to for every other type — would report exactly the lists that are *not* equal. Hence the
inverted condition codes, and the result is normalised back to the 0-or-1 `veritas` every consumer
expects rather than returned raw: `liss_list_eq` documents `1` or `0` and this does not depend on
it.

Lists have no ordering to return a `-1` from. `Value`'s `PartialEq` recursed but its `Ord` never
covered `List`, and the checker refuses `maior_est`/`minor_est` on a series
(`errores/series-comparatio-ordinis`), so only the two equality arms can arrive.

---

## 9. The linker and TLS (`nexus.rs`)

Linking, in process and in memory (spec §9.3). **`WildNexus`** hands libwild a `FileSystem` that is
a `HashMap`, so the five inputs are never written anywhere and the finished image is never read from
anywhere — it is returned as bytes and the caller decides what a file is. No `cc`, no `ld`, no
temporary directory, no toolchain on the machine.

The **`Nexus`** trait exists so that the Windows answer is one new implementor and no other change
(spec §9.4) — libwild's `Args` is `Elf | MachO | Wasm` and has no PE/COFF backend, so that answer
does not exist yet and this seam is where it will go.

**`libwild::run` cannot be used.** The free function hard-codes `OsFileSystem`. The only way in is
`Linker::with_file_system(fs).run(&args)`.

### The risk this was measured against first

The prelinked runtime of `crate::target` carries **general-dynamic** TLS relocations — 36
`R_X86_64_TLSGD`, 16 `TLSLD` and 28 `DTPOFF32` on x86_64; 53 each of
`R_AARCH64_TLSDESC_{CALL,ADR_PAGE21,ADD_LO12,LD64_LO12}` on aarch64 — all of them from Rust's std
thread-locals, which are compiled GD because a `staticlib` does not know it will end up in a static
executable. A static, non-dynamic image has no `__tls_get_addr` to call and no dynamic relocations
to apply, so the **linker** must rewrite every one of those sequences to local-exec. If it cannot,
either the link fails or the image faults at its first TLS access, and no amount of code generation
helps.

It was checked before any of this was written, and libwild does relax them. Both images contain
**zero TLS relocations and no dynamic section**, and no `@tlsgd` sequence survives anywhere; the
aarch64 one runs under `qemu-aarch64`. Measured on the x86_64 image of `exemplum-salve`, 22 `%fs:`
-relative accesses remain, in the two forms the two relaxations produce — and they are **different
instructions**, which matters to anyone reading a disassembly against this document:

- **LD→LE**, 7 sites: `data16 data16 data16 mov %fs:0x0,%rax`, then a direct `-0xNN(%rax)` load.
  The prefixes are padding, where the linker overwrote a longer local-dynamic sequence in place.
- **GD→LE**, 3 sites (`0x426466`, `0x434365`, `0x44de47`): a *bare* `mov %fs:0x0,%rax` followed by
  `lea -0xNN(%rax),%rax`. **No prefixes.**

Three of the 36 general-dynamic sites rather than 36 because `--gc-sections` removed the rest —
the sequences it did keep were all relaxed, which is the property in question.

The one surviving `__tls_get_addr` in each image is musl's own, called from its static
`dl_iterate_phdr` — hand-written code, not a relaxed relocation, and unreachable from compiled
code.

### Why aarch64's relaxation is not asserted anywhere

Task 11 confirmed the aarch64 TLSDESC → local-exec relaxation by hand, and `tests/aemulatio.rs` —
the file that runs aarch64 images rather than merely building them — deliberately does not re-check
it. The reason is not "running the image is stronger": running exercises only the TLS sites the four
programs in that file actually execute, not all 53 in the image. It is that **an unrelaxed TLSDESC
is not representable in what ships** — the linked aarch64 image has zero relocations of any kind
and no dynamic section, so there is nowhere for one to live, and `imago_est_statica_aarch64`'s
`PT_DYNAMIC` assertion already covers every site statically.

### Why libwild's own words never reach a user

Two rules, both from spec §9.3 and both easy to omit because omitting them is silent:

- **`args.on_warning` is installed.** libwild's default sink writes English to stderr. The
  language's one standing rule is that everything the user sees is Latin, and the robustness harness
  enforces it with an English denylist — so a libwild warning would not merely be ugly, it would
  fail a test far away from here with no hint of where it came from. It is set **before `parse`
  rather than after**, because `parse` itself can warn.
- **`libwild::error::Error` is never forwarded.** Every link failure becomes the same structural
  Latin diagnostic, for D-22's reason: an English string interpolated into a Latin sentence was a
  defect, not a convenience.

The English in the `libwild::error!` calls inside `MemoriaFs` is deliberate and is not a
user-visible string: that trait's error type is libwild's, these values never leave the linker, and
`iunge` replaces whatever comes back with one Latin sentence of its own. They exist so that a
`MemoriaFs` bug is legible in a debugger rather than silent.

### The link line

```text
wild -m <emulatio> -static --no-pie --gc-sections -o <nomen> \
    crt1.o crti.o programma.o runtime.o crtn.o
```

`-static --no-pie` is spec §2's promise: one ELF that runs on any Linux of its architecture with
nothing else installed. `--gc-sections` is worth ~0.92 MB on the image, because the prelinked
runtime carries the whole of `libc.a` and most of it is unreachable from any one program. What it
must **not** take is a package's uncalled export: `tests/vocationes.rs`'s
`imago_coniuncta_symbola_servat` links the three-package program and finds `liss_4_beta_7_triplum`
still in the finished image, alongside the entry-point alias check of §2.

### Four failures, four sentences

`iunge`'s four failure sites said the same four words until the whole-branch review, and that was a
permanent trap rather than a wording preference: `catalogus.rs` decides `fixtured` on a pattern
being *unique across the workspace*, so the first provocation of any one of them would have marked
all four `ambiguous` — and nothing can move a site from `ambiguous` back to `fixtured`. The trap
was reachable: `nexus.run` fails on link-time resource exhaustion, which a large enough source
program can cause without any compiler bug at all. The tails are what the linker was doing, so a
user's report now names the stage as well as the outcome.

| Sentence | Raised when |
|---|---|
| `imago non coniuncta est: argumenta` | `libwild::Args::new` |
| `imago non coniuncta est: perscrutatio` | `args.parse` |
| `imago non coniuncta est: nexus` | `nexus.run` |
| `imago non coniuncta est: imago abest` | the map holds no output under `nomen` |

The output borrows the linker's arena; both must go before the map is read, or the image is still
half-owned by libwild. Once the linker is gone the `Arc` is the only handle and the bytes move out
rather than being copied.

### `MemoriaFs` and the poisoned lock

`aperi` takes the lock **poisoned or not**. libwild links on a Rayon pool. A worker that panics
while holding this lock poisons it, and then every later `lock().unwrap()` panics too — so a link
failure would kill the compiler instead of becoming an `imago non coniuncta est: …`, which is the
one thing this module promises.

Ignoring poison is safe here rather than merely convenient: the map has no invariant spanning two
operations. Every use is one insert, one remove or one lookup, so whatever a panic interrupted, the
contents are a consistent set of name→bytes pairs and there is nothing for poisoning to protect.

`open_input` returns `None` for the descriptor: there is no descriptor behind this, so libwild must
not try to mmap it. `canonicalize` is the identity — every name is already canonical, they are map
keys, not paths. `Egressus::finish` is libwild's commit point: the bytes land in the map, where
`iunge` picks them up after the linker has been dropped.

### `machina_ex_artefactis`

The `-m` emulation the inputs call for, read from the objects themselves. libwild infers nothing
from an argv that names no architecture, and the alternative — passing a `target::Target`
alongside the bytes — would let a caller link one target's object against the other's runtime and
be told so only by the relocation failures. The `e_machine` at offset 18 of every input is the fact
that settles it, and every input must agree.

Its four sentences are `defectus internus: artefactum non est ELF`,
`defectus internus: artefacta machinis diversis composita sunt`,
`defectus internus: machina ignota est` and `defectus internus: nihil coniungendum est`.

`artefacts_of_two_architectures_are_refused` is the one mistake this seam exists to catch: an x86_64
object linked against the aarch64 runtime. Without the check libwild is handed `-m elf_x86_64` for
aarch64 inputs and fails somewhere in relocation processing, which becomes the same opaque
`imago non coniuncta est: nexus` with nothing to say what went wrong.
`a_link_that_cannot_resolve_fails_in_latin` links the four runtime blobs with nothing defining
`liss_initium` and asserts exactly `imago non coniuncta est: nexus` — libwild reports an undefined
symbol in English, and none of that English survives the boundary (spec §9.3, D-22).

---

## 10. The target layer and the embedded runtime (`target.rs`)

`build.rs` cross-builds `abi` for both musl triples, prelinks each with `libc.a` and
`libunwind.a`, and copies the three CRT objects out of the toolchain's `self-contained/` directory.
`target.rs` is where those bytes surface: a program link is `Runtime::artefacta` with codegen's own
object spliced in, and nothing else — no archives, no member resolution, no toolchain on the
machine.

### `Target`

Linux only, and exactly two: spec §2 names `x86_64` and `aarch64`, and §9.4 draws the Windows seam
somewhere else entirely.

| | `X86_64` | `Aarch64` |
|---|---|---|
| `triple()` | `x86_64-unknown-linux-musl` | `aarch64-unknown-linux-musl` |
| `machina()` (ELF `e_machine`) | 62 (`EM_X86_64`) | 183 (`EM_AARCH64`) |
| `emulatio()` (linker `-m`) | `elf_x86_64` | `aarch64linux` |

The triple is **always musl**: spec §2 requires a static ELF that runs on any Linux of its
architecture with nothing else installed, and glibc cannot be linked statically and mean that.
`machina()` is the one number that says an embedded blob is not the *other* target's blob;
`the_machine_numbers_are_the_elf_ones` checks the two literals against `object::elf`. `emulatio()`
lives here so that the two facts that must agree — which triple was cross-built, and which
emulation links it — are one `match` apart.

`Target::hospes()` is the target this compiler is itself running on, if it is one of the two; `None`
on anything else, which is a compiler that can cross-compile but cannot `liss run`.
`Target::ex_nomine` parses a `--target`: the full triple, or the architecture alone.

### `Runtime` — the four blobs

Everything a program link needs beyond the object codegen emits. Four blobs, all `'static`: they are
in the compiler's `.rodata`, put there at *its* build time by `build.rs`. Reading a field costs
nothing and copies nothing.

| Field | |
|---|---|
| `crt1` | The C runtime's entry stub: `_start`, which calls `__libc_start_main`, which calls `main`. |
| `crti` | The prologue half of `.init`/`.fini`. |
| `nucleus` | **The runtime.** `abi` cross-built for musl and prelinked (`ld -r`) with `libc.a` and `libunwind.a`, because rustc bundles neither into a `staticlib` — measured, spec §9.2. One strong undefined symbol survives in it, `liss_initium`, and compiled code is what supplies it. |
| `crtn` | The epilogue half. Must come last. |

`Runtime::artefacta` returns the link line in order — `crt1.o`, `crti.o`, `programma.o`,
`runtime.o`, `crtn.o`. **The order is not decoration.** `crt1.o` must precede `crti.o`, both must
precede everything with code in it, and `crtn.o` must come last or the `.init`/`.fini` sections are
left without their epilogue. The names are what the in-memory `FileSystem` of spec §9.3 answers to,
so they must be distinct and must look like filenames.

### What the target tests establish, and what they measure

These live in `target.rs`'s inline test module and are the only checks on the blobs `build.rs`
produced. They read the *shipped object*, which is a question no comment can answer.

- **`every_blob_is_present_and_is_this_target_s_architecture`** — non-empty (a GNU `ld` built for
  the other architecture writes exactly an empty file) and the right `e_machine`.
- **`the_crt_objects_are_whole`** — the CRT objects are copied verbatim out of the toolchain and
  never inspected by `build.rs`, so this is the only thing that looks at their *contents*.
  Architecture and non-emptiness are not enough: a truncated `crt1.o` of the right machine passes
  both and fails at the first program link with an undefined `_start`. `crt1.o` is the one with
  symbols; `crti.o` and `crtn.o` define no global symbol at all, so the assertion for them is that
  `.init` and `.fini` are present.
- **`the_runtime_defines_the_abi_and_leaves_liss_initium_to_compiled_code`** — and asserts the
  runtime does **not** define `liss_initium`: a definition here would be silently linked instead of
  the program.
- **`the_prelink_leaves_nothing_undefined_but_the_program_itself`** — the proof of a complete
  prelink is a subtraction: after `ld -r` there is **exactly one** strong undefined symbol left, and
  it is the one the program provides. This is also how `libc.a`'s necessity was re-established
  rather than taken on trust. Measured on this build, x86_64, prelinking the staticlib alone:
  **165** strong undefined symbols, `malloc`, `memcpy`, `write`, `sigaltstack` and `sysconf` among
  them. Adding `libc.a`: 12, all `_Unwind_*`. Adding `libunwind.a` too: none. (`nm --defined-only`
  over the 331-member staticlib finds no musl symbol at all — spec §9.2's measurement,
  reproduced.) Each of those three configurations fails here before it can fail at a program link
  fifteen tasks later.
- **`the_prelink_pulled_musl_in`** — `__libc_start_main`, `malloc`, `write`, `memcpy`,
  `sigaltstack`, `sysconf`: the six the probe named. If the separate `libc.a` ever stops being
  embedded they go missing here first.
- **`main_installs_the_guard_before_it_calls_the_program`** — **the hard requirement carried from
  Task 9.** `liss_custos_institue` had no caller, and a `main` that called `liss_initium` and forgot
  it would revert stack exhaustion to signal death with an empty stderr — D-04, reopened with no
  test failing, because every Task 9 test installs the handler explicitly. So the assertion is made
  against the object: `main`'s own relocations must name `liss_custos_institue`, at a **lower
  offset** than `liss_initium`. It reads *positional* order, not a proof about execution, and
  requires both calls to be in `main`'s own body — so inlining `liss_custos_institue`, moving the
  call into a helper, or reordering the two sites would all fail it. All three are fail-safe and
  none is a bug: a failure is a prompt to re-establish the property some other way, not to delete
  the call.
- **`the_guard_traps_both_sigsegv_and_sigbus`** — spec §8.2 asks for `sigaction` on *both*
  `SIGSEGV` and `SIGBUS`, "because on aarch64 a stack fault may arrive as either".
  `liss_custos_institue` must call `sigaction` **twice** on both targets — one call is what a
  `custos.rs` that had quietly lost its `SIGBUS` line would leave, and every existing test would
  still pass, because every existing test faults with `SIGSEGV`. On aarch64 the two signal numbers
  handed to those calls must be **7 and 11**, read out of the `movz w0` that precedes each `bl`. A
  `SIGBUS` cannot be provoked reliably from a Lingua Serpentis program on any architecture, and on a
  4 KiB-page x86_64 host under `qemu-aarch64` it cannot be provoked at all. **What it assumes, for
  whoever it breaks on:** it reads instruction encodings, so it is a statement about *this* code
  generator's output. Three legitimate changes would fail it — setting `w0` more than `RETRO`
  instructions before the call, materialising a signal number some way other than a `MOVZ` (a shared
  constant register, say), and routing the two calls through a loop over an array of signal numbers,
  which would be a perfectly good refactor and would leave one `bl` and no immediates at all. All
  three are fail-safe, none is a bug, and a failure here is a prompt to re-establish the property,
  never to delete a disposition.
- **`the_page_size_is_asked_of_sysconf_and_never_assumed`** — spec §8.2 requires the page size to
  come from `sysconf(_SC_PAGESIZE)` (30 on Linux, in musl and glibc alike), "never a hardcoded 4096,
  because aarch64 kernels commonly ship 16 KiB and 64 KiB pages". This host reports 4096 and
  `qemu-aarch64` inherits it, so a runtime that hardcoded 4096 would behave identically to a correct
  one everywhere this suite can reach. What is measured is narrower than "asked once":
  `liss_custos_institue`'s own body references `sysconf` at least once, and on aarch64 at least one
  of those sites is handed `_SC_PAGESIZE`. An earlier draft asserted `== 1` and would have gone red
  on a perfectly correct build that inlined `fines` — which calls `pagina()` a second time at run
  time. **What none of this establishes:** nothing here or anywhere in this suite has ever executed
  the handler on a kernel whose pages are 16 or 64 KiB. The call is proven; the arithmetic
  downstream of a 16384 is not.
- **`the_handler_asks_nothing_and_allocates_nothing`** — §8.2's other half: "Everything expensive
  happens at initialisation, before `liss_initium`." Measured on the shipped objects: the reachable
  set is **4 functions on both targets** — `manus`, `discerne`, `erratum`, `effunde` — and the
  code it calls beyond them is exactly `write`, `__errno_location` and `_exit`. All three are
  permitted: §8.2 names raw `write(2)` as the handler's only output mechanism, and
  `__errno_location` is how musl reports that call's result. The denylist is `malloc`, `calloc`,
  `realloc`, `free`, `sysconf`, `getrlimit`, `open`, `read`, `close`, `sigaction`, `sigaltstack`,
  `exit`; `_exit` is required to be present.
- **`the_embedded_runtime_costs_what_it_costs`** — see §11 below for the figures.
- **`the_link_line_puts_the_crt_around_everything_else`** — the one test here that reads no blob:
  `Runtime::artefacta` hands back the five inputs under the names `crt1.o`, `crti.o`, `programma.o`,
  `runtime.o`, `crtn.o`, in that order, with the caller's own bytes third.

#### Three questions about the object, and the function that answers each

`target.rs` asks the shipped object three different questions, and telling them apart is most of
navigating the file.

- **`vocationes`** answers *which symbols, and how early*: the offset within `main` of the first
  relocation naming each symbol. It is what `main_installs_the_guard_before_it_calls_the_program`
  reads, and it is deliberately stronger than the Task 9 tripwire it replaced — that one asked
  whether a file containing `fn main(` also contained `liss_custos_institue(` somewhere, which is
  satisfied by a call in an unrelated function, a call placed after `liss_initium`, or a comment.
- **`loci`** answers *how many times, and where*: every offset inside one named function's body at
  which a relocation names a given symbol. That is the question `sigaction` needs, since installing
  one disposition and forgetting the other is exactly the failure spec §8.2 warns about and a
  first-offset map cannot see. It is **scoped to that function's own bytes, and deliberately so**:
  its two callers read *instruction operands* at the offsets it returns, which only means anything
  within one body.
- **`clausura`** answers *what this code can reach*, and anything asking that wants it rather than
  `loci` — `loci` stopping at the symbol boundary was a real hole in
  `the_handler_asks_nothing_and_allocates_nothing` before `clausura` existed.

#### `clausura` — why the reachability walk is transitive

After `ld -r`, a call to a private Rust `fn` in the same object is a relocation against a **section
symbol** with an addend, carrying no name at all. A scan bounded by a single symbol's byte range
(`loci`) therefore never matches one. On aarch64 that hid three hops — `manus` reaches `discerne`,
`erratum` and `effunde` that way, and `erratum`/`effunde` are where `write` and `__errno_location`
actually live. A forbidden call added to `discerne`, which is the discriminator and arguably the
component §8.2's "never in the handler" is *most* about, passed the un-closed check silently.

Traversal follows local (unnamed) targets, and **named** ones only when the name begins with `liss_`
— our own runtime, which is `#[unsafe(no_mangle)]` and so appears by name the moment the optimiser
declines to inline it. `liss_scribe_erratum` is exactly that case and would otherwise be a second
hole of the same shape. It does **not** descend into musl: `write` is defined in this object because
§9.2 prelinks `libc.a`, and following it would drag in the syscall layer and much of the C library
— the "whole-object scan nobody can reason about". So the result describes the **boundary the
handler presents to libc**, and the denylist is a check on that boundary. What a permitted callee
does once entered is musl's business.

**The addend of a PC-relative relocation carries an architecture's own bias, and getting this wrong
fails silently.** x86_64's `R_X86_64_PLT32` stores `S + A - P`, so a call to a function at section
offset `X` is emitted against the *section* symbol with `A = X - 4` — the four bytes between the
field and the end of the instruction. aarch64's `R_AARCH64_CALL26` carries no such bias and uses `A
= X`. An earlier version added the addend to the symbol address in unsigned arithmetic. On aarch64,
where `A` is 0, it worked. On x86_64 every `0 + (-4)` wrapped to `u64::MAX - 3`, resolved to
nothing, and the traversal stopped at the handler's own body — so the whole closure was one node
and the x86_64 half of the check was inert while looking exactly like the aarch64 half. Signed
arithmetic clamped at zero is what makes the bias harmless, and the clamp is exact only because
rustc gives every function its own section; `clausura` asserts that, loudly, rather than assuming
it.

`LIMES_CLAUSURAE` is 32: measured today the set is 4 on both targets, and 32 leaves room for a
refactor while still failing loudly if the handler ever calls into something large — which would
itself be the finding, since §8.2 wants the handler tiny.

#### Reading a call's argument off a stripped object

`argumentum_primum` is how a *call's argument* is read with no disassembler in the dependency list:
on aarch64 every instruction is four bytes, a relocation against a `bl` sits on the `bl` itself, and
the first argument of a C call is `w0`. It scans back `RETRO` = 8 instructions, and **two filters
are load-bearing**:

- **`Rd == 0`.** The `sigaddset` calls that build the handler's signal mask take their signal number
  in `w1`, and `movz w1, #11` / `movz w1, #7` sit inside the same window as the `sigaction` calls
  that follow. A scan that accepted any destination register would read those and get the right
  answer for the wrong reason — and would keep getting it after the `sigaction` arguments
  themselves had gone wrong.
- **Stopping at a call.** A `bl` clobbers `w0` — it is both the argument register and the return
  register — so a `movz w0` on the far side of one says nothing about this call. Without this the
  scan is a heuristic that happens to be right on today's layout; with it, a value found is a value
  this call actually receives.

---

## 11. The build script and the musl cross-build (`build.rs`)

`build.rs` cross-builds `abi` for both musl targets and prelinks each one into a single
relocatable object, so the shipped compiler can produce a static ELF with **no rustc, no `cc` and no
linker on the host** (spec §2, §9.2).

Everything here runs at *compiler* build time. It is allowed to need a Rust toolchain, and it is
allowed to panic: a cross-build that silently produced half a runtime would surface as an undefined
symbol fifteen tasks later.

### What gets embedded, and why it is more than the staticlib

**Measured, in Task 2's probe and again here: rustc does not bundle `libc.a` into `staticlib`
output.** The archive is 330 members of Rust codegen units plus `compiler-builtins`, and not one of
them defines `malloc`, `write` or `__libc_start_main`. So each target embeds, from
`$(rustc --print target-libdir --target=T)/self-contained/`:

- `crt1.o`, `crti.o`, `crtn.o`, copied verbatim;
- **one prelinked object** — `ld -r` of the staticlib, `libc.a` and `libunwind.a`, then stripped
  of bitcode and debug information (`exonera`) — so every program link is three CRT objects plus
  one program object plus one runtime object, with no archive-member resolution on a path that runs
  once per compiling fixture.

`libunwind.a` is **not** in spec §9.2's list and is required anyway. The precompiled musl `libstd`
is built with unwinding, so it references `_Unwind_Resume`, `_Unwind_GetIP` and ten more regardless
of what `panic` this crate's profile asks for. Without it the prelinked object carries twelve strong
undefined `_Unwind_*` symbols and the program link fails. With it, the only strong undefined left is
`liss_initium` — which is exactly the one compiled code supplies.

### Three things the probe did not encounter

1. **`ld` cannot be GNU `ld`.** The probe prelinked x86_64 on an x86_64 host. binutils is configured
   for one architecture, and the aarch64 prelink fails with `Relocations in generic ELF (EM: 183)`
   — a zero-byte output and exit 1. `rust-lld` is multi-target, ships with the rustc this script
   already requires, and does the same job; `LISS_LD` overrides it.
2. **`-u __libc_start_main` is load-bearing.** The prelink names the symbols it must keep rather
   than using `--whole-archive` (18.5 MB against 22.1 MB before stripping, and the difference is
   dead Rust codegen units). Selecting only the `liss_*` entry points leaves `crt1.o`'s own
   undefined symbols unresolved, so the undefined set of `crt1.o` is read out of the object and
   added to the list rather than written down.
3. **Four fifths of the prelinked object is dead**, and neither `--whole-archive` nor `lto` touches
   it: embedded LLVM bitcode for an LTO that never runs, and DWARF for a design that emits none
   (spec §2, §8.4). `exonera` removes both, taking the two blobs from 18.5 MB and 19.1 MB to **3.4
   MB and 3.5 MB** — under §9.2's 8–15 MB prediction rather than two and a half times over it
   — and a linked program from 4.6 MB to **513 KB**. `src/target.rs` reports the figures as a
   test.

### The four traps

- **Recursive cargo deadlock.** The inner build gets an `OUT_DIR`-local `--target-dir`. Cargo's
  exclusive lock lives in the target directory, so that is what keeps the two builds apart.
- **Inherited environment.** Outer `RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS`, `RUSTC_WRAPPER`,
  `CARGO_BUILD_TARGET` and the make jobserver variables all break a cross-build; each is removed
  explicitly, and rustflags are *set empty* rather than removed, because removal leaves a
  `.cargo/config.toml` in charge. See `cargo_mundum`.
- **Building outside the nix shell fails**, unavoidably: musl std must exist. `bibliotheca` detects
  that and says which target and which flake.
- **Staleness.** `rerun-if-changed` on both crates' sources and on each `LISS_RUNTIME_DIR` blob by
  name, and `rerun-if-env-changed` on every variable that changes the answer — including `RUSTC`,
  which is kept. `RUSTC` is an *input*: switching toolchains changes the runtime, and without the
  declaration the blobs from the old one survive the switch.

### `bibliotheca` — proving musl std is installed

`rustc --print target-libdir` answers for a target it has never seen — it is a path computation,
not a lookup — so the path alone proves nothing. What proves it is `libstd-*.rlib` beside
`self-contained/crt1.o`. The failure message names the target, then three ways out: `nix develop -c
cargo build`, `rustup target add <triple>`, or setting `LISS_RUNTIME_DIR` to a prebuilt tree.

### `aedifica` — the inner cargo

`cargo build --release --features ingressus --target <meta>` on `abi`.

**Release always**, whatever profile the compiler itself is being built in. The alternative makes
"debug build" a property of *which* prebuilt musl archive shipped, which doubles the artefact matrix
for no benefit — the runtime is not what anyone debugs.

**`--features ingressus`** is what adds `main`. It is off by default because `abi` is also
built as an rlib for its own tests, and a test binary already has a `main`.

The target directory is `OUT_DIR`-local, **and this is the deadlock fix**. Cargo's exclusive build
lock is a file in the target directory, so a private one is what stops the inner build contending
with the outer. This was long believed to be only half of it, the other half being that `abi` was
its own workspace root; `abi` became an ordinary member on 2026-08-09 and the inner build is
unaffected. `internals/abi.md` records what that did change.

**`--config profile.release.panic="abort"`** is here rather than in `abi/Cargo.toml` because a
workspace member's own profile is ignored — cargo warns and builds it with unwinding anyway. This
argument is the only thing keeping landing pads out of the runtime, and `tests/pondus.rs` is the
only thing that would notice if it went.

### `cargo_mundum` — the environment scrub

| Removed | Why |
|---|---|
| `RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS` | The nix shell and CI both set host-specific flags (`-C target-cpu`, `-C link-arg=…`) that are wrong or fatal for a musl cross-build. |
| `RUSTC_WRAPPER`, `RUSTC_WORKSPACE_WRAPPER` | `sccache` and friends key on an environment this inner build does not share. |
| `CARGO_BUILD_TARGET` | Would silently override the `--target`. |
| `CARGO_TARGET_DIR`, `CARGO_BUILD_TARGET_DIR` | Would silently override `--target-dir`, putting the inner build back in the contended directory. |
| `CARGO_MAKEFLAGS`, `MAKEFLAGS`, `MFLAGS` | The jobserver. The file descriptors they name belong to the *outer* cargo, and an inner build that inherits them either hangs waiting for tokens it will never be given or warns on every invocation. |
| `RUSTDOCFLAGS`, `CARGO_ENCODED_RUSTDOCFLAGS` | Same reasoning as `RUSTFLAGS`, for completeness rather than from an observed failure. |
| `CARGO_BUILD_RUSTFLAGS`, `CARGO_BUILD_RUSTDOCFLAGS` | The environment spellings of the `[build]` config keys, and the exact siblings of `CARGO_BUILD_TARGET`. |

`RUSTC` is deliberately **kept**: it names the toolchain the outer build is using, and the runtime
should be built by the same one.

**`env_remove` is not enough for rustflags, and only for rustflags.** `--target` and `--target-dir`
are on the command line, and a command-line argument beats `~/.cargo/config.toml`. **`rustflags` has
no command-line form**, so `[build] rustflags` and `[target.<musl triple>] rustflags` in a user's or
a packager's config apply to this inner build after every removal above — with host `-C
target-cpu` or a `linker` that cannot see the musl sysroot, which is the failure this function
exists to prevent.

Setting `CARGO_ENCODED_RUSTFLAGS` to the **empty string** is what closes it: cargo takes the
environment in preference to the config file, and an empty encoded value is a well-formed "no flags"
rather than an absence. Removing the variable hands the decision back to the config; setting it
takes the decision away. Nothing in this repository is affected — there is no `.cargo/config.toml`
here — and that is precisely why it had to be reasoned about rather than observed.

### `praeliga` — the prelink

```text
rust-lld -flavor gnu -r -o <runtime.o> \
    -u main -u __libc_start_main -u liss_int_add … (39 in all) \
    <libabi.a> <libc.a> <libunwind.a>
```

**`-u` rather than `--whole-archive`.** `ld -r` pulls an archive member only to satisfy an undefined
symbol, and at prelink time there are none — so without either flag the output is empty.
`--whole-archive` on the staticlib works and costs 3.6 MB per target in Rust codegen units nothing
will ever call. Naming the symbols keeps only what is reachable from an entry point, and the entry
points are not a list anyone has to maintain: they are every `liss_*` symbol the archive defines
(`signa_archivi`), plus `main`, plus whatever `crt1.o` says it needs of us (`desiderata_crt`).

The `main` assertion is asked **against `signa_archivi` alone, before the `extend`, and that is the
whole point.** `crt1.o`'s strong undefined set is exactly `{__libc_start_main, main}` on both
targets, so after merging it in, the name set contains `"main"` whether or not the archive defines
one. The assertion was originally written after the merge and was therefore vacuous: dropping
`--features ingressus` still passed it, still prelinked, and still embedded a runtime with no entry
point — caught only later, by the symbol tests.

A zero-length output is checked for explicitly, because a GNU `ld` built for one architecture does
exactly that on the other.

`signa_archivi` reads the symbols out of the archive rather than having them written down, so that
adding an entry point to `abi` needs no edit here — and so that *removing* one cannot leave a
stale `-u` that fails the prelink with an undefined symbol. `desiderata_crt` skips weak undefined
symbols (`_DYNAMIC`, `_init`, `_fini` all resolve to zero or come from `crti.o`) and the
linker-synthesised `_GLOBAL_OFFSET_TABLE_`, which no archive defines and `-u` would only fail to
find.

### `exonera` — the strip, and what it is worth

Removes the two things in the prelinked object that nothing downstream will ever read: embedded LLVM
bitcode (`.llvmbc`, `.llvmcmd`) and debug information (`--strip-debug`).

**This is 82 % of the object.** Measured on the x86_64 prelink before it was applied: `.llvmbc`
7,602,988 B, `.rela.debug_info` 1,964,832 B, `.debug_str` 1,900,280 B, `.debug_info` 1,119,257 B,
`.rela.debug_ranges` 1,054,848 B and about 1.2 MB of other `.debug_*` — out of 18,538,608.

Both are dead by construction rather than by guess:

- **`.llvmbc`/`.llvmcmd`** exist so a later LTO can re-optimise the object. No LTO runs here and
  none runs in the shipped compiler, which links with libwild (spec §9.3) and never invokes LLVM at
  all.
- **`.debug_*`** are dead **by spec**: §2 lists "no debug information" as a non-goal and §8.4
  turns it into a design consequence — the stack-exhaustion diagnostic has no line number
  precisely because none of this is emitted. Carrying DWARF for the runtime while emitting none for
  the program is paying for half a feature.

**The knock-on is larger than the compiler.** Before this, a linked Lingua Serpentis program was
4,627,048 B and roughly nine-tenths of it was debug info and bitcode that `--gc-sections` cannot
touch, because they are not reachability-analysed. After, it is **512,912 B**. The "4.6 MB floor"
this task first reported was not a floor; it was ballast.

Neither the symbol table nor a single relocation is touched: `main`'s relocations keep their
offsets, `liss_initium` stays the sole strong undefined, and the stripped runtime links, runs and
still faults into the Latin stack-exhaustion diagnostic.

### The embedded size, measured

Spec §9.2 predicted "roughly 8–15 MB of embedded blobs for two targets". Measured: **3,396,800
B** and **3,524,880 B**, 6.6 MiB together — inside the prediction, with room. The unstripped
prelink was 18.5 MB and 19.1 MB; `.llvmbc` was 7.6 MB on x86_64 and `.debug_*` 7.2 MB. Neither
`--whole-archive` nor `lto = true` touches either, which is why two earlier attempts at this number
moved it by nothing and 3.6 MB the wrong way respectively.

`the_embedded_runtime_costs_what_it_costs` asserts each target's total is in `1 MiB .. 6 MiB` and
the sum is under 15 MiB. The band is wide on purpose: it is a tripwire for a change of *kind* — a
`--whole-archive` prelink, a strip that silently stopped running, an empty blob — not a size
ratchet. Its **upper** bound is what fails if `exonera` is removed, and that is the bound that
matters now.

### `LISS_RUNTIME_DIR`, and D-29

The escape hatch: a packager or a CI image that has already built the four objects points
`LISS_RUNTIME_DIR` at a tree of `<triple>/{crt1,crti,crtn,runtime}.o` and no rustc cross-build
happens at all. This is the only supported way to build the compiler without musl std installed, and
it is deliberately dumb: the files are copied, not checked — except for the heap layout, below.

`rerun-if-changed` is emitted for **each blob, not the directory**. `rerun-if-changed` on a
directory watches its listing; replacing a `runtime.o` in place changes no listing, and the stale
bytes would survive every rebuild until somebody added or removed a file.

**What checks these blobs, and — more usefully — what does not.** `src/target.rs`'s tests run
against whatever landed here and cover the object's *shape*: the `e_machine` of every blob, the
symbols the runtime defines and leaves undefined, that the prelink pulled musl in, `main`'s call
order and the handler's relocation closure, the CRT's `.init`/`.fini` epilogues, and the size band.
A blob wrong in any of those fails them exactly as a wrong cross-build would.

Until **D-29** closed it read **no layout constant at all**. `ONUS`, `VERBUM` and the three
`size_of`s are held by the `const _: () = { … }` block in `abi/src/lib.rs`, which proves that
crate's source against `runtime::dispositio` — never against the blob's bytes — and a crate-scoped
build never evaluates it: `conde` is skipped and `abi` is a dependency of nothing here. So a stale
blob whose `dispositio` disagreed with today's source linked cleanly and passed every test in this
crate.

The blob now says its own layout. `abi` exports `ONUS` and `VERBUM` as the data symbols `liss_onus`
and `liss_verbum`, which the prelink keeps because `signa_archivi` hands every `liss_*` definition to
`-u`, and `the_blob_s_layout_is_the_compiler_s` reads each one's eight bytes out of the embedded
object and compares them with `runtime::dispositio`. That test sees both sides on every build path,
`LISS_RUNTIME_DIR` included, which is the one property the `const _` block cannot have.

### `transfer` — why the destination is removed first

**Not tidiness.** `fs::copy` preserves the source's permissions, and the CRT objects come out of the
nix store at mode 0444; the first build copies a read-only file into `OUT_DIR` and the *second*
fails to open it for writing with `Permission denied`. It took one incremental rebuild to find and
would have looked like anything but a permissions problem.

### `instrumentum` — the cross-capable binutils

`$(rustc --print sysroot)/lib/rustlib/<host>/bin/` holds `rust-lld` and `rust-objcopy`, both LLVM's,
both multi-target. That last word is the whole reason they are used in preference to whatever is on
`PATH`: the host binutils cannot read the other architecture's relocations, and this script has to
handle both. `LISS_LD` and `LISS_OBJCOPY` override them.

---

## 12. Diagnostics in codegen

### Why some failures are `unimplemented!` rather than a Latin `Defectus`

Every `unimplemented!` in `emissio.rs` names a shape the **checker refuses** — a binary operator
on a type that cannot carry it, an ordering comparison on a series, a list literal typed as
something other than a series. None is reachable from a valid program. They are deliberately not a
`Defectus`: **a Latin sentence here would be a claim about the user's program, and it would be a
false one.**

The three sites are `Corpus::series`'s non-`List` type (the checker's `ListLit` arm types every list
literal as `Type::List`), `Corpus::binarium`'s fallback arm (the checker's `check_binary` admits no
other pairing, and every combination it does admit is enumerated above it), and
`Corpus::comparatio`'s ordering-on-a-series arm.

By contrast, a `SlotId` outside `Func::slots` **is** a `Defectus` rather than an index panic: it is
the checker and codegen disagreeing about a function's frame, which is a compiler defect and not a
statement about the user's program — but it is still not something to discover as a Rust panic
crossing into a user's terminal.

### One sentence per site, because the catalogue's ratchet is a one-way door

`catalogus.rs` grants `fixtured` to a pattern that is **unique across the workspace**, and
`ambiguous` is a one-way door: nothing moves a site back. So two sites that raise the same sentence
are a permanent trap, and D-24 established the rule — where two sites raise the same sentence they
share a *function*, so the literal stays unique in the workspace and stays uniquely attributable.

Two helpers exist for exactly that reason:

- **`sedes_ignota`** → `defectus internus: sedes ignota`. A slot index outside the function's
  `slots` table — from a `SlotId` (`Corpus::sedem`) or from `Func::params` (`Emissor::declara`,
  which uses `get(..n)` rather than an index for this reason).
- **`saltus_extra_ansam`** → `defectus internus: saltus extra ansam`. `abrumpe` or `continua` with
  no loop to name, or a loop whose recorded scope depth is deeper than the scope stack it indexes
  into. Unreachable from a valid program — `checker.rs` refuses both keywords at `loop_depth == 0`
  with `'abrumpe' extra ansam` — but reachable from a codegen that pushed a loop and forgot to pop
  it, and that is a defect to name rather than an index panic. The empty loop stack and the
  out-of-range scope index are the same inconsistency seen from either end, so they get one sentence
  rather than two the catalogue would have to keep apart.

Where two failures are genuinely different, they get **different tails**, for the same ratchet
reason:

| Sentence | Site |
|---|---|
| `defectus internus: munus declarari non potuit: externum` | `externum` — an imported ABI entry point |
| `defectus internus: munus declarari non potuit: proprium` | `Emissor::declara` — the program's own function |
| `defectus internus: munus vocatum non inventum: definitio` | `Emissor::defini` — `declara` and `defini` disagree about how many functions there are |
| `defectus internus: munus vocatum non inventum: vocatio` | `ExprKind::Call` — a `FuncId` in the IR names none |
| `defectus internus: datum declarari non potuit: constans` | `Corpus::constans` — a literal's blob |
| `defectus internus: datum declarari non potuit: scida` | `Corpus::locus` — the file-name blob a raise needs |
| `defectus internus: datum definiri non potuit` | `Emissor::defini` — defining a declared blob |
| `defectus internus: munus componi non potuit` | `Emissor::defini` — `Module::define_function` |
| `defectus internus: objectum institui non potuit` | `Emissor::novus` — `ObjectBuilder::new` |
| `defectus internus: objectum scribi non potuit` | `Emissor::finge` — serialising |
| `defectus internus: machina institui non potuit` | `machina_isa` — all four settings and the ISA lookup |
| `defectus internus: munus 'initium' in programmate non inventum` | `Emissor::finge` — no entry point (`a_program_without_initium_is_refused`) |

The `: externum` / `: proprium` distinction is the one a reader most needs: one blames the runtime's
declared surface, the other the program's. Declaring and defining a blob are two failures and get
two sentences, as `munus declarari` / `munus componi` already do for functions — one sentence for
both would make the catalogue report a single site for two unrelated ways of being broken.
