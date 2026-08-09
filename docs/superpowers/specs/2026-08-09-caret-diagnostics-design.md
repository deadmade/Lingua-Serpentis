# Lingua Serpentis — Caret Diagnostics Design

- **Date:** 2026-08-09
- **Status:** Proposed
- **Author:** Manuel (with Claude)
- **Relationship:** Amends `2026-08-07-compiler-backend-design.md` §5.4 and §10 — the
  diagnostic grammar it fixes at one line becomes three levels, and the runtime's
  `(file, line)` pair renders its line as a Roman numeral. Supersedes the four-shape
  `Display` table recorded in `internals/rationale.md`. Adds **no** dependency: §11's
  table is untouched, and `lingua-serpentis` stays at zero third-party crates.

## 1. Vision

A diagnostic names a line and leaves the reader to find the rest:

```
Error in 'principale.liss', linea 3: variabilis ignota: 'y'
```

Line 3 might hold one identifier or forty. The message knows which one is wrong — the
checker had it in hand — and then throws that away, because there is nowhere to put it.

`Token` is `{ kind, line }`. `Expr` carries no position **at all**; `checker.rs`
compensates by threading a `line: usize` parameter by hand through `check_expr`,
`resolve_expr`, `check_args`, `check_list_elems`, `check_binary` and `lookup_or_err`, so
every diagnostic about a subexpression reports the line of the whole statement enclosing
it. Nothing in the workspace stores a byte offset or a column. `FuncDecl::def_line` is
the single hand-added sub-statement position in the entire AST, and it exists because one
warning needed to land on the line a parameter was written rather than the line `munus`
was.

So this is not a rendering change with a renderer at the end of it. **The renderer is the
small part.** The work is giving the front end positions it never had, and then absorbing
that in a test suite whose entire purpose is to notice when a diagnostic changes.

What it buys:

```
erratum: variabilis ignota: 'y'
  in 'principale.liss', linea III
     |
 III | ni x = y + °I;
     |        ^
```

And, incidentally but not accidentally, the last Arabic numeral leaves the language.
`liss-abi/src/defectus.rs` currently carries a function whose doc comment calls it *"the
one number this crate formats without going through `Roman`"*. §7 deletes it.

## 2. Goals and Non-Goals

### Goals

- **A caret under the offending expression**, not under the statement containing it (§4).
- **One grammar for every diagnostic the language emits** — compiler, back end, and the
  compiled image alike (§3, §7).
- **No new dependency.** Spec §11's table is not touched (§6).
- **Widen `no Arabic digits` from the message body to the whole diagnostic** (§8). The
  robustness suite's rules may only ever get stronger; this is a rare chance to take one.
- **Warnings render like errors**, because they are read in the same stream by the same
  reader (§3).

### Non-Goals

- **Not spans in the IR.** `ir::Stmt.line` is what a *runtime* diagnostic reports, and it
  keeps its present value. A runtime has no source text and cannot draw a snippet.
  Pointing `divisio per nullum` at the dividing expression means widening the ABI, which
  §5.4 pins as `(file, line)`. Out of scope, and a reasonable successor.
- **Not multi-line snippets.** A span crossing lines shows its first line (§5).
- **Not terminal colour.** Nothing here writes an escape sequence. The corpus compares
  bytes.
- **Not a language change.** No syntax, no semantics, no new fixture programs — only what
  the compiler *says* about the programs already there.

## 3. The grammar

A diagnostic has three levels, and **each is independent of the one below it**:

```
erratum: <sententia>          ← always
  in '<scida>', linea <N>     ← when the diagnostic has a location
     |
 <N> | <the source line>      ← when the source is also in hand
     |        ^^^
```

That independence is the load-bearing part of the design. It is what lets the compiled
image — which knows its file and line but has no source text at run time — emit the first
two levels and stop, rather than needing a grammar of its own. Every diagnostic in the
language is a prefix of the same three-level shape.

`monitum:` replaces `erratum:` for a warning; nothing else differs.

**Six shapes result**, on severity × location:

```
erratum: <sententia>                erratum: <sententia>
  in 'principale.liss', linea III     in linea III
     |                                   |
 III | ni x = y + °I;                III | ni x = y + °I;
     |        ^                         |        ^

erratum: <sententia>                erratum: <sententia>
  in 'principale.liss'
```

The bare fourth shape must survive: a package-folder Latinity warning has no file and no
line by construction — a directory name is not written on a line of anything — and neither
does the runtime's exit-status warning.

**`Error` becomes `erratum`.** It was the one English word the language said. Lowercase,
matching `monitum`, and matching the sentence that follows it.

**The line number is a Roman numeral**, in the location row and in the gutter. The
language renders every other number that way; `docs/internals/abi.md` records D-12 as the
defect of having let a `usize` reach a user. The line number was the last survivor, and it
survived only because nothing had made it awkward enough to fix.

### Framing rules

- **Gutter width is the width of the Roman numeral.** `LXXXVIII` is eight characters wide;
  nothing may assume two. Bare rows are `width + 2` spaces, then `|`.
- **The location row is indented two spaces regardless of gutter width.** That keeps the
  header a two-line unit whose shape does not depend on whether a snippet follows it.
- **No separator between consecutive diagnostics.** The column-0 `erratum:`/`monitum:` is
  the separator, and is what a parser keys on. `conformance/monita/ordo-trans-scidas` has
  three in a row. A blank line would put a genuinely empty line into stderr, which several
  consumers iterate over.
- **No trailing whitespace, ever.** An empty gutter row ends at its `|`; a snippet of an
  empty source line ends at its `|`. This repository has no `.gitattributes` and no
  `core.whitespace` setting, so a fixture file with trailing spaces is one editor-on-save
  away from a diff nobody meant to make.

## 4. Positions, in three units that must not be confused

| quantity | unit | who reads it |
|---|---|---|
| `Spatium` | **byte** offsets into the file's source | AST, `LinguaError`, `Fragmentum::extract` |
| the lexer's cursor | **char** index into `Vec<char>` | `lexer::tokenize`, and nothing else |
| the caret column | **display** column | the renderer, computed at render time and never stored |

```rust
/// Byte offsets into the file's source text. `linea` is 1-based.
pub struct Spatium { pub principium: u32, pub finis: u32, pub linea: u32 }
```

**Bytes, because the only consumer slices a `&str`.** Finding the line a span sits on is
`src[..lo].rfind('\n')`, which is a byte operation; with char offsets every lookup pays a
re-walk from the start of the file for nothing.

**The lexer keeps its `Vec<char>`.** Converting it to `char_indices()` would rewrite every
`chars.get(i + 1)` peek and every `chars[start..i]` slice — a large diff whose only product
is a different spelling of the same loop. Instead `tokenize` builds one `offsets: Vec<u32>`
of prefix sums of `len_utf8`, of length `chars.len() + 1`, and every `start`/`i` pair the
lexer *already computes and throws away* becomes `offsets[start]..offsets[i]`.

**`linea` rides on the span rather than being derived from the offset.** D-03 makes it
non-derivable: a multi-line string literal is attributed to the line it *opens* on, so that
a diagnostic about the string points at the string and not at its last line. No
offset-to-line function reproduces that, and `conformance/errores/linea-post-chordam`
exists to pin it.

### Display columns without `unicode-width`

Spec §11 forbids the dependency, so the rule is chosen to be correct by construction
rather than by table lookup:

- each `\t` becomes **one space** and counts one column;
- every other `char` counts **one** column.

The renderer prints the echoed line and counts the caret **from the same character
iteration**, so alignment is exact for everything a terminal draws at width 1 — all ASCII,
`°`, and every accented letter `is_alphabetic()` admits into an identifier. A wide
character (CJK, emoji) misaligns the caret by one column each. That is a real limitation,
it earns a defect-register entry, and it is not worth a dependency.

**Tabs are not expanded to a tab stop.** Whichever stop is chosen, some terminal disagrees,
and the disagreement is invisible until a fixture blessed on one machine is compared on
another. One space removes the class.

## 5. `Fragmentum`, and every edge case

The snippet is extracted **where the source is still in hand**, not at render time:

```rust
pub struct Fragmentum {
    pub linea: usize,    // 1-based
    pub textus: String,  // the source line: tabs substituted, trailing \r stripped
    pub caret: usize,    // 0-based display column
    pub latitudo: usize, // caret count, >= 1
}
pub fn extract(src: &str, spatium: Spatium) -> Option<Fragmentum>
```

`extract` is pure and does no I/O, which is what lets every rule below be a unit test
rather than a fixture.

**Why extraction happens early rather than in `render`.** `strip` clears `e.file = None`
*after* the error exists, so that a single-source program reads `in linea III` rather than
naming the synthetic `fons.liss`. A renderer that looked the source up by filename would
therefore have to guess "the first root file" for every single-source program — reviving
exactly the coupling that makes `fons.liss` delicate today. Extracting early costs
nothing: `strip` goes on clearing the *header's* filename, and the fragment beside it stays
correct. It also keeps `render`'s signature, and keeps `cli::finish` callable on the `load`
failure path, where no source exists at all.

| case | rule | why |
|---|---|---|
| span crosses lines | first line only, caret run clamped at end of line | one gutter row is the whole grammar; reachable today through D-03's `"prima\nsecunda"` |
| span at EOF (`lo >= len`) | `None` | header and location, never a phantom empty line |
| empty file | `None` | `program.rs` admits empty `.liss` files deliberately |
| line 0 | `None` | reachable: `Parser::line()` returns 0 on an empty token stream |
| line longer than 200 chars | `None` | an elision marker needs a re-based caret and a second family of edge cases; one branch removes both |
| tab | one space, one column | §4 |
| CRLF | strip the trailing `\r` | otherwise the caret row overprints the source row |
| zero-width span | one caret | the common case: a token-position error has no width |
| non-ASCII | one column per `char` | §4 |

## 6. Why the renderer is written here and not taken from crates.io

`annotate-snippets` and `codespan-reporting` both draw this shape well. Neither is taken,
for two reasons that compound.

**It would be the first third-party runtime dependency of `lingua-serpentis`.** Spec §11
names three crates that "stay at zero" and this is one of them. The precedent for an
exception is `libc` in `liss-abi`, which earned a row in §11's table and a paragraph of
justification, on the grounds that `sigaltstack` has no spelling in std and §8 is not
optional. A snippet renderer is roughly eighty lines of arithmetic over a `&str`. It does
not meet that bar.

**And the frame would not be Latin.** Both crates take their severity word from a fixed
enum of English labels. The whole surface this project has built — the catalogue, the
`Latin only` rule in `judge`, `TokenKind::name` existing so no Rust variant can leak —
exists to keep English out of what the language says. Buying a dependency in order to
reintroduce `error:` above a Latin sentence inverts the point.

## 7. The compiled image says the same thing

`liss-abi` hand-writes the diagnostic grammar byte by byte, and it adopts the first two
levels of §3's grammar. It never draws a snippet: it has no source text at run time, which
is precisely the case §3's independence rule is built for.

This is smaller than it sounds. **`liss-abi` is not `no_std`** — it is a byte writer by
choice, and it already calls `Roman::display()` when filling a `{}` hole. Rendering the
line number as a Roman numeral is the same call it already makes. `decimales`, and the
twenty-byte buffer it writes into, are deleted.

`ingressus.rs`'s `MONITUM_PRAEFIXUM` moves with it, or
`conformance/monita/status-exitus-redactus` is left as the one warning in the corpus still
speaking the old grammar.

Thirteen fixtures produce runtime diagnostics. If the compiler moves and the runtime does
not, those thirteen read differently from the other seventy-three for no reason a user
could be told.

**The risk here is the build, not the diff.** `liss-codegen/build.rs` cross-compiles
`liss-abi` into two prebuilt musl staticlibs during the outer build. A stale archive leaves
the two workspaces disagreeing while *both* test commands pass.

## 8. What the robustness suite checks, after

`parse_diagnostic` becomes a block parser over the whole of stderr rather than a line
parser over the last line — the last physical line of a failing run is now a caret row.

Two rules are added and one is widened.

**New — `caret aligns`.** Where a block has a snippet, the caret column must be no greater
than the echoed line's length in `char`s, and the gutter numeral must equal the location
row's. This is the only automated check that the renderer is not lying about where it
points; everything else would accept a caret under the wrong column.

**New — the Roman line number must round-trip.** `display(parse(s)) == s`. The
`decimales` era could not fail this check because it had no numeral to parse.

**Widened — `no Arabic digits`.** The honest formulation is *the rule covers every byte the
language wrote, and exempts only the bytes the user wrote*:

| region | rule |
|---|---|
| header | no Arabic digits outside quoted spans — unchanged, since messages quote the user back |
| location row | none, **except inside the quoted file path** — `a1.liss` is a legal filename |
| gutter numeral | none |
| echoed source, caret row | **exempt by construction** |

The last row is not a loophole, it is the point. The snippet is user input reproduced
verbatim, and the likeliest thing to appear in it is the Arabic digit the diagnostic is
complaining about. Any stricter rule makes `conformance/errores/numerus-arabicus`
self-contradictory.

The exit-0 branch changes too, and less visibly: it currently requires every line of stderr
to begin with `Monitum`, which a warning's location, gutter and caret rows do not.

## 9. What this does to the catalogue

`catalogus.rs::observed_messages` splits every line of fixture stderr at its first `": "`
and treats the tail as an observed message. Once stderr carries echoed source, that is
wrong in two ways, and the second is serious:

1. a gutter row containing `": "` yields a garbage observed message — harmless noise;
2. **a source line that happens to contain a diagnostic sentence would mark that pattern
   `fixtured` when no fixture provokes it.** That is a false *green* in the ratchet, which
   is worse than any failure it could report.

So `observed_messages` must collect only lines carrying a severity prefix, and must do so
in the same commit that introduces the prefix.

The header will move. `118 sites · 70 fixtured · 0 ambiguous · 46 uncovered · 2 unparsed`
changes because the `Display` impls are themselves catalogue sites and their patterns
change, and because `LinguaError` gaining a field shifts roughly fifty rows' line numbers.
Every difference must be enumerated with its cause. The two `Display` impls that are
permanently `UNCOVERED` — they own the prefixes `observed_messages` strips — stay
`UNCOVERED`.

## 10. Blessing, and the one way this suite fails silently

Eighty-six fixture files pin expected stderr byte for byte, and `LISS_BLESS=1` rewrites all
of them from actual output. Blessing a regression is the single failure mode the
conformance corpus cannot report.

Three things guard it.

**A snapshot of the diagnostics, taken on the old grammar and committed before anything
changes.** All eighty-six files parsed into `(genus, file, line, message)` tuples —
ninety-nine of them — and snapshotted. The extractor then learns both grammars and
normalises: `erratum` ≡ `Error`, `monitum` ≡ `Monitum`, the Roman numeral parsed back to a
`usize`. **The snapshot must not move across any blessing.** If it moves, a message
changed, and a message changing is not a framing change.

**Three commits, never one.** Framing first, then warning snippets, then error snippets,
separated by tasks that move no fixtures at all. Each diff has a different shape and each
is small enough to read.

**A pairing check on the diff.** Because the line number moves off the header onto its own
row, the message text becomes the entire remainder of the header line — so stripping the
prefix from every added and removed line and counting must leave every message an even
number of times. An odd count is a changed sentence.

## 11. A coupling this creates

`conformance/currunt/exemplum-praecisio` reaches `examples/praecisio.liss` through a `VIA`
file, and has a `nuntii` because that example provokes two Latinity warnings. Once a
warning echoes the line it is about, **editing that example moves a conformance
expectation file.** Low probability, high confusion, and it belongs in `CLAUDE.md` beside
the other rules the corpus imposes on the rest of the repository.

## 12. Success criteria

1. A checker diagnostic about a subexpression carries a caret under **that subexpression**,
   not under the statement.
2. Every diagnostic the language emits — front end, back end, compiled image — is a prefix
   of §3's three-level grammar.
3. `judge` enforces `no Arabic digits` over the whole diagnostic, with §8's one exemption.
4. `RECORDED_RULES` does not grow.
5. `cargo test` and `cargo test --manifest-path crates/liss-abi/Cargo.toml` both green.
6. `cargo tree -e normal` shows no new package. §11's table is unchanged.
7. The diagnostics snapshot of §10 is byte-identical to the one taken before any change.
