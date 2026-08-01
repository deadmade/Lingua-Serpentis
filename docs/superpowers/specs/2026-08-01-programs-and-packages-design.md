# Lingua Serpentis — Programs and Packages (*fasciculi*) Design

- **Date:** 2026-08-01
- **Status:** Approved — implemented (plan: `../plans/2026-08-01-programs-and-packages.md`)
- **Author:** Manuel (with Claude)
- **Relationship:** Realizes the "Imports / modules" v2 candidate from
  `2026-07-19-lingua-serpentis-rust-interpreter-design.md` §9. **Supersedes that
  document's script-style program model** (§5.5 there): top-level statements are
  removed by this design — a program no longer runs a file top to bottom; it
  starts at a designated entry function.

## 1. Vision

Today the language has no answer to two basic questions: *what is a program?*
and *where does it start?* A "program" is whatever one `.liss` file happens to
contain, executed top to bottom — the same fuzziness Python has, where the entry
point is "whichever file you ran, plus an `if __name__ == "__main__"` idiom",
and a folder only becomes a package by planting an `__init__.py` marker in it.

This design gives both questions concrete answers, in the shape Go chose:

- **A program (*programma*) is a folder.** Running it means running that folder.
- **It starts at exactly one place:** a required entry function,
  `munus initium reddit ni { … }`.
- **A package (*fasciculus*) is a subfolder.** Nothing more — no marker file, no
  manifest, no declaration line repeated in every file. The folder's name *is*
  the package name.

```
computatrum/              ← the program (root folder; its name doesn't matter)
├── principale.liss       ← root files: the program itself, like Go's package main
├── auxilia.liss          ← same folder ⇒ same namespace; calls work across files freely
└── mathematica/          ← a package, because it is a folder — that is the whole rule
    ├── summa.liss
    └── fractio.liss      ← same package as summa.liss; file names never appear in code
```

```
// principale.liss
affer mathematica;

munus initium reddit ni {
    scribe(mathematica.summare(°XXV, °XVII));
    redde nullus;
}
```

"Package" here means the *program-structure* sense (Go's package), not the
*distribution* sense (npm/Cargo): manifests, versions, and dependencies are
explicitly out of scope (§7).

## 2. Goals and Non-Goals

### Goals

- A **concrete definition of "program" and "package"**: program = folder (or a
  single file as the degenerate case), package = direct subfolder, package name
  = folder name. No marker files, ever (§4.1–§4.2).
- A **single unambiguous starting point**: every program — even a single file —
  must define `munus initium reddit ni` exactly once in its root; execution
  starts there and nowhere else (§4.3).
- **Top level is declarations only**, everywhere: `munus` declarations and
  `affer` imports. Script-style top-level statements and top-level variables are
  removed (§4.4).
- **Go-style imports**: a file brings in a package by name (`affer
  mathematica;`) and calls its functions qualified
  (`mathematica.summare(…)`) (§4.5).
- **Everything is public**: every function in a package is callable by
  importers; no visibility keyword (§4.5).
- **Flat**: only direct subfolders of the program root are packages; deeper
  `.liss` files are an error (§4.2).
- Errors gain **file attribution** (path + line); the Latinity checker extends
  to **package folder names** (§4.6).
- The program's **process exit status** comes from `initium`'s returned `ni`
  (§4.3).

### Non-Goals (deferred, not rejected — see §7)

- Manifests, versions, dependencies — the distributable sense of "package".
- Private functions / any visibility distinction.
- Nested sub-packages.
- Unqualified imports (`ex mathematica affer summare;`).
- Package-level constants (top level stays functions-and-imports only).
- Unused-import warnings.
- A standard library shipped as packages (*bibliotheca*).

## 3. Key Decisions and Rationale

| Decision | Choice | Why |
|---|---|---|
| Package model | **Go-style: folder = package, files invisible** | The user's two requirements — a definite starting point, and folders that work without an init file — describe Go's model exactly. Rejected: C-style whole-tree loading (no namespaces), JS/Python-style per-file path imports (file names leak into code; folders need marker/index files to act as one thing). |
| Entry point | **Entry function, strict — required even for single files** | "A program has a starting point" becomes a hard rule with no exceptions. Rejected: dual mode (file = script, folder = program) because "what a program is" would have two answers. Hello world grows a wrapper; concreteness wins. |
| Visibility | **Everything public** | YAGNI at this scale. Privacy can be added later without breaking any existing program. |
| Nesting | **Flat, one level** | Package names are unique by construction (sibling folders), imports are always a single name, no path syntax or collision rules needed. Additive later. |
| Package naming | **Folder name alone** — no declaration line in files | Zero ceremony, matches the no-init instinct. Rejected: Go's `package x` line per file (deliberate redundancy the user doesn't want). |
| Entry return type | **`ni`, used as process exit status (mod CCLVI)** | The language has no void type, and adding one is a bigger change than using the type it has. `redde nullus;` = success; exit codes in Roman numerals. |
| Import placement | **Per file, before all declarations** | Each file states what it brings in (Go's rule); reading any file shows its dependencies at the top. |
| Import cycles | **Allowed — harmless by construction** | With top level restricted to declarations, a package has no initialization code, so there is no init order to get wrong. The no-`__init__` requirement buys this for free. |

### Vocabulary

| Concept | Pick | Why | Alternatives considered |
|---|---|---|---|
| package (docs/errors term) | *fasciculus* | classical word for a bundle of papers/letters | *sarcina*, *capsa* |
| import keyword | `affer` | imperative "bring!" — matches the `scribe`/`redde` imperative family | `porta`, `adduc` |
| entry function | `initium` | "the beginning"; reads well after `munus` | `principium`, `exordium`, `origo` |

*caput* ("head") was deliberately avoided for the entry point — it would collide
visually with the existing `capit` keyword.

## 4. Language Specification

### 4.1 The program (*programma*)

- A program is a **folder**. Running it means running that folder (§4.7).
- The `.liss` files directly in the root folder form the program's own code —
  the analogue of Go's `package main`. The root is **not importable** and the
  root folder's name carries no meaning to the language.
- A **single `.liss` file** may also be run directly: it is a complete program
  with no packages. All rules below apply to it unchanged (it needs `initium`).
- Files without the `.liss` extension are ignored wherever they appear.
- Folders containing no `.liss` files anywhere beneath them (`.git`, assets,
  editor droppings) are ignored entirely.

### 4.2 Packages (*fasciculi*)

- Every **direct subfolder** of the program root that directly contains `.liss`
  files is a package. (A direct subfolder with no `.liss` files of its own
  cannot smuggle them in deeper down either — any such deeper file trips the
  flat rule below.)
- **The folder's name is the package name.** It must be shaped like a valid
  identifier (letters and underscores as the lexer defines them — no spaces, no
  hyphens); a folder whose name is not a valid identifier is an error, because
  the name must be writable in code.
- The Latinity checker warns (warning-only, as always) on package folder names
  that are not recognizable Latin, exactly as it does for identifiers. File
  names are **not** checked — they never appear in code.
- All `.liss` files directly inside one folder form **one namespace**: a
  function in `summa.liss` calls a function in `fractio.liss` with no ceremony,
  no imports between files of the same folder. Splitting or merging files
  inside a folder is invisible to every caller.
- Declaring two functions with the same name **within one folder** is an error
  (this also covers "two `initium` in the root" — it is the same duplicate-name
  error).
- **Flat rule:** packages cannot contain packages. Any folder deeper than the
  first level that contains `.liss` files (directly or further down) is an
  error. Package names are therefore unique by construction.

### 4.3 The starting point (`initium`)

- The root must define, **exactly once** across its files:

  ```
  munus initium reddit ni {
      ...
      redde nullus;
  }
  ```

- Execution starts here and nowhere else. There is no other entry mechanism and
  no top-level execution (§4.4).
- `initium` takes **no parameters** — by the existing header rule, a
  parameter-less function omits `capit` entirely.
- The returned `ni` becomes the **process exit status**, taken modulo CCLVI
  (256), matching how Unix truncates exit codes to their low 8 bits.
  `redde nullus;` means success. A negative return is first reduced to the
  0–255 range the way Unix does (only the low 8 bits survive).
- A function named `initium` inside a **package** is an ordinary function with
  no special meaning; only the root's `initium` is the entry point.
- A program whose root defines no `initium` is an error.

### 4.4 Top level: declarations only

- At file scope — in the root and in packages alike — only two things may
  appear: `affer` imports (§4.5) and `munus` declarations.
- Any statement at top level (`scribe(…);`, `ni x = °V;`, `si (…) { … }`, …) is
  an error. Today's script-style programs stop being valid and are rewritten to
  wrap their statements in `munus initium`.
- Consequence: the language has **no global variables and no package state**.
  All state lives in locals and parameters. This is also what makes import
  cycles harmless (§4.5): there is no initialization code, hence no
  initialization order.

### 4.5 Imports (`affer`) and qualified calls

- A file that wants a package brings it in by name:

  ```
  affer mathematica;
  ```

- Imports are **per file** and must precede all `munus` declarations in that
  file. An import makes the package's name usable in *that file only*.
- Calls into a package are **always qualified**: `mathematica.summare(a, b)`.
  The dot is unambiguous in this grammar — the language has no field access,
  and the dots inside Roman literals live inside the `°…` token.
- **Everything is public**: every function of an imported package is callable.
- A local function may share a name with an imported package's function;
  qualification keeps them apart (`summare(…)` is the local one,
  `mathematica.summare(…)` the imported one). A package name may likewise
  coexist with an identically-named variable or function — the qualifier
  position (`name.`) is syntactically distinct from every other use of an
  identifier.
- Any file in any folder may import any package (packages may import each
  other). **Cycles are allowed** and harmless — imports grant name visibility
  only; there is no code to run at load time (§4.4).
- `affer` naming a package that does not exist is an error. Importing the same
  package twice in one file is an error (it can only be a mistake). Importing a
  package and never using it is **not** checked in v1 (candidate in §7).
- Using `nomen.functio(…)` where `nomen` was never imported in this file, or
  where the package has no such function, is an error.

### 4.6 Errors and warnings across files

- With multiple files, errors carry **file attribution**: the path relative to
  the program root, plus the line, phrased in Latin as today:

  > `Error in 'mathematica/summa.liss', linea V: 'x' typum 'ni' expectat, sed 'voc' datum est.`

- The single-source library entry points (`run`, `run_with_warnings` — what the
  test suite feeds strings to) keep today's file-less phrasing
  (`Error in linea V: …`), so existing error-message tests change only where
  the new rules themselves bite.
- New error conditions introduced by this design (final Latin wordings settled
  during implementation, per house tradition; indicative forms shown):
  - missing entry point — `munus 'initium' in programmate non inventum`;
  - duplicate `initium` — the ordinary duplicate-function error (§4.2);
  - statement at top level — `extra munera nihil nisi declarationes licet`;
  - unknown package in `affer` — `fasciculus 'nomen' non inventus`;
  - duplicate `affer` of one package in one file;
  - nested package — `fasciculus in fasciculo non licet`, naming the offending
    folder;
  - package folder name not a valid identifier;
  - qualified call whose package was not imported in this file, or whose
    function does not exist in that package.
- The Latinity pass gains one warning site: package folder names (§4.2).
  Everything else about warnings (stderr, never blocking) is unchanged.

### 4.7 CLI

```
lingua-serpentis <folder>        runs a program folder
lingua-serpentis <file.liss>     runs a single-file program (still needs initium)
```

- The `.liss` extension check remains for the file form; the folder form
  requires the path to be a directory.
- Program output and warnings behave exactly as today (stdout / stderr). The
  one addition: the **process exit status** is `initium`'s return value
  (§4.3) on success, and remains the current failure status on any
  lex/parse/check/runtime error.

## 5. Implementation Sketch

Crate-level shape only; the implementation plan will do the real decomposition.

- **Loader** (new module, e.g. `program.rs`): walks the program folder once,
  applies the ignore/flat rules (§4.1–§4.2), and produces a
  `Program { root: Vec<SourceFile>, packages: BTreeMap<String, Vec<SourceFile>> }`
  where each `SourceFile` keeps its relative path for error attribution. The
  single-file and single-source entry points build the same structure with one
  root file.
- **AST**: a new top-level form for a parsed file (its `affer` list + its
  `FuncDecl`s); `Expr::Call` gains an optional package qualifier. Statement
  forms lose nothing — they just stop being legal at top level.
- **Lexer/parser**: a `.` token in expression position (only for qualified
  calls), the `affer` keyword, and the top-level grammar
  `file := affer* munus*`.
- **Checker**: two phases — first collect every package's function signatures
  program-wide, then check each file's bodies against its own import list plus
  its folder's namespace. All existing per-function checking is unchanged.
- **Interpreter**: the function table becomes keyed by (package, name) with the
  root as one namespace; execution begins by calling the root's `initium` and
  the returned value surfaces as the exit status. No other evaluation changes.
- **Latinity**: one new declaration-site kind (package folder names).
- **`main.rs`**: dispatch on file-vs-directory, thread the exit status out.
- **Tests**: integration tests gain a multi-file entry point that takes labeled
  sources (path + text) without touching disk; existing single-source tests are
  rewritten to wrap their statements in `munus initium`.

## 6. Testing Strategy

- **Loader rules**: non-`.liss` files ignored; `.liss`-free folders ignored;
  nested package rejected; invalid folder name rejected.
- **Entry point**: missing `initium` rejected; duplicate `initium` rejected;
  exit status is the returned value mod 256, including a negative return;
  `initium` in a package is callable like any function and not an entry point.
- **Top level**: each statement kind at top level rejected with the Latin
  error; declarations-only files accepted.
- **Imports**: qualified call across packages works; same-folder cross-file
  calls work unqualified; unknown package rejected; duplicate import rejected;
  unimported qualifier rejected; unknown function in a real package rejected;
  local and imported functions with the same name coexist; a two-package
  import cycle runs.
- **Errors**: multi-file errors name the right file and line; single-source
  errors keep the file-less phrasing.
- **Latinity**: non-Latin package folder name draws a warning; file names draw
  none.
- **Examples**: `examples/*.liss` rewritten to the `initium` form and kept
  running in CI as today.

## 7. Deferred (v2 candidates)

Recorded, not rejected — each is additive on top of this model:

- Manifests, versions, dependencies (the distributable/npm sense of "package").
- Private functions (`publicum` or any visibility rule).
- Nested sub-packages (`affer mathematica.fractio;`).
- Unqualified imports (`ex mathematica affer summare;`).
- Package-level constants.
- Unused-import warnings.
- A standard library shipped as packages (*bibliotheca*).

## 8. Open Questions

- Final Latin wording of the new error messages (§4.6) — cosmetic; settled
  during implementation, as with the original interpreter design.
- The exact shape of the multi-file test/library entry point (labeled sources
  vs. temp dirs) — an implementation-plan decision, not a language one.
