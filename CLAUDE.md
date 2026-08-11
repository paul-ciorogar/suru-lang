# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Guidelines

- Ask clarifying questions before implementing.
- Implement logic and tests, preferably in a new file.
- Keep all `.suru` files under 500 lines; split by concern when approaching the limit.
- Update `CHANGELOG.md`, `CLAUDE.md`, and `README.md` when a feature is completed.

## Build & Test Commands

All builds and tests run inside Docker. The host needs Docker and docker-compose installed.

```bash
# Run the full test suite (build + tests + valgrind leak checks)
./scripts/test.sh

# Build a single Suru program to a native binary
./scripts/build.sh src/cli/main.suru build/cli

# Advance the bootstrap binary after codegen changes (a single run does the full 3-stage fixed-point check — see Bootstrap section)
./scripts/bootstrap.sh

# Regenerate expected test output baselines
./scripts/generate-expected.sh
```

**There is no way to run a single test in isolation** — the test runner compiles and runs all fixtures in sequence.

## Bootstrap Rules

The compiler is self-hosted: `bin/suru-build` (a frozen C# binary) bootstraps the Suru compiler from source. `./scripts/bootstrap.sh` runs a 3-stage fixed-point check:

1. `bin/suru-build` compiles source → C1
2. C1 compiles source → C2 (first compile on new codegen)
3. C2 compiles source → C3 (must equal C2, bit-for-bit)

If C2 == C3, the test suite passes, and `bin/suru-build` is updated to C2 (≡ C3).

**Critical**: After any codegen change, run `./scripts/bootstrap.sh` **once** — the script already chains all three stages and asserts the C2 == C3 fixed point internally, so a second manual run is unnecessary. If the script reports "fixed point confirmed: C2 == C3" and the suite passes, the codegen is stable.

## Name Mangling

Mangled symbol names use separators the lexer **rejects** in identifiers (so users can never collide with them) that are also valid **unquoted** LLVM IR symbol chars. One char per role:

- **Generics → `-`**: `Box<i64>` → `Box-i64`, `Pair<i64, String>` → `Pair-i64-String`, variant arms `Some` → `Some-i64`. Emitted as `@suru_clone_Box-i64` etc.
- **Namespaces → `.`**: `Suru.Cli.fn` → symbol `@Suru.Cli.fn` (the source dots are kept, not flattened).
- **Compiler-internal reserved → `$`**: the synthetic vtable field `$vtable`; lifted object-literal methods `$m_Type_method_0` (single-`_` joiners *inside* the synthetic name stay `_`).
- **Exception — `__append` / `__at` stay on `__`**: these are builtin intrinsic methods written in real `.suru` source; `$` can't be lexed, so they must remain `__`-prefixed and lexable.

**Bit-for-bit invariant**: the separators are hardcoded string literals duplicated across the semantic and codegen layers (`monoInstantiate.suru`, `passes.suru`, `irCodegenTypeHelpers.suru` produce; codegen looks up by the same string). A producer and its lookup **must agree exactly**. There are also parse/split sites that read a separator back — change these in lockstep with the producers: generic base-name detection in `irRegisterPasses.suru` (`cloneReturnMatchesType` scans for the first `-`), variant-arm prefix match in `monoVariantArms.suru` (`resolveVariantNameAt`), and `isMethodOfMonoType` in `pipeline.suru` (`$m_` prefix slice).

## Compiler Architecture

The compilation pipeline (`src/compiler/pipeline.suru`) flows through these stages:

```
Source → Lexer → Parser → Semantic Analysis → Type Resolution → IR Codegen → clang-18
```

1. **Lexer** (`src/compiler/lexer/lexer.suru`) — tokenizes source into `Array<Token>`
2. **Parser** (`src/compiler/parser/`) — recursive descent, builds `AstModule` AST. Entry point: `parse(tokens)` in `parser.suru`. Token cursor is encapsulated in the `Parser` object type (`parserUtil.suru`); construct with `util.newParser(tokens)`. `parserAst.suru` holds all AST type definitions.
3. **Semantic** (`src/compiler/semantic/`) — multi-pass: name resolution, type checking, function validation. Leaf helpers: `assignable.suru` (the one value-flow assignability predicate) and `nodeType.suru` (`nodeResolvedType`, the pure read of the pass-2 annotation)
   - **Types** (`src/compiler/types/`) — the structured `Type` value (`type.suru`) and its canonical operations: `parseType` (`typeParse.suru`), `printSymbol`/`printSuruType`/`printAnnotation` (`typePrint.suru`), `equalType`/`canonType`/`substitute`/`varizeType`/`unify` (`typeOps.suru`). This tree imports nothing from `semantic/` or `codegen/` and is the foundation both depend on: **a pass that needs to know something about a type parses it once here and asks the structure — new string surgery over type names is a regression.** Design + section map: `semantic_analysis_architecture_rd.md` (source comments cite it by §-number)
4. **Monomorphization** (`semantic/mono/monoPass.suru`) — runs after pass 1 type resolution; collects generic defs, infers instantiation sites, generates and injects concrete copies, then removes generic templates before pass 2. Sub-modules live in `src/compiler/semantic/mono/`:
   - `monoCollect.suru` — collect `GenericRegistry` from stmts with `typeParams.len > 0`
   - `monoInfer.suru` — walk AST to build `Array<InstantiationRequest>`. Purely **mechanical**: it does no type-variable inference of its own, it reads the structured `typeArgs` the resolver recorded on call/method/object nodes plus explicit generic annotations (through `collectGenericAppFromType`). Inference lives in the resolver (`rtCallUnify` / `rtMethodUnify` / `rtRecordArgBindings` / `rtObjectLitRecord`).
   - `monoInstantiate.suru` — deep-clone generic nodes with type substitution; mangle names (`Box<i64>` → `Box-i64`, `Pair<i64, String>` → `Pair-i64-String`). `instantiateAll` is a **fixed-point worklist**: after producing a concrete fn it re-scans that fn's substituted return/param/`let` type annotations for generic applications (e.g. `some-Sig`'s return `Option<Sig>`) and enqueues them, so transitively-needed types (`Option-Sig` + variant structs) get instantiated. The mangled-name `seen` set bounds it.
   - `monoVariantArms.suru` — the final mono step, `rewriteVariantArms`: match-arm patterns still carry the unmangled base name the programmer wrote (`Some:`), and are resolved against the concrete `SumTypeDeclNode`'s variant list (`Some-i64:`). Split out of `monoPass.suru` to keep both under the 500-line limit.
   - `monoSubst.suru` — recursive type-name and AST-node rewriters for substitution. **Everything a node carries that names a type must be substituted when a generic body is cloned** — the `resolvedType`/`typeName` strings *and* the structured `typeArgs` on `CallNode`/`MethodCallNode`/`ObjectLitNode` (via `substTypeArgs`). Inside a template the resolver records the enclosing type parameter itself: `List<T>.clone()`'s inner `newList()` carries `typeArgs=[T]`. Since mono treats `typeArgs` as the authoritative binding for both instantiation and call-site renaming, leaving them verbatim makes the `List-i64` copy ask for `newList-T` — a name never instantiated — and the build fails with `undefined function 'newList'`.
5. **Type Resolution** (`semantic/resolveTypes.suru`) — infers and annotates `resolvedType` on every AST expression node; codegen depends on these annotations
6. **Code Generation** (`src/compiler/codegen/`) — emits **one `.ll` for the whole program** (entry file + every transitively imported module); links with `runtime/*.o` (compiled from `runtime/*.c`) via `clang-18`, naming the `.ll` explicitly (never globbing, so a stale file is inert). Leaf helper: `irOwnedLocals.suru` (`collectMutatedLocals`) — a pure AST walk deciding which `String` locals are owned slots (see Memory Management)

**One `IrCodegenContext` == one whole program.** There is no `define`-vs-`declare` ownership gate: every `FnDeclNode`/`TypeDeclNode` that reaches codegen is defined there. Two consequences the old per-module split used to hide, and which any new lookup must respect:
- **Bare names are not unique across modules.** `isSumTypeName` is declared by both `semantic/exprs.suru` and `codegen/irCodegenTypes.suru`; `joinDots`, `containsName` and the `rewriteMatchArm*` family too. `types.lookupFn` and `lookupFnSrcPath` (`irCodegen.suru`) therefore resolve **module-locally first** — matching `ctx.currentFnSrcPath` (set per body in `emitFunction`) before falling back to first-match. `lookupExternalFnSrcPath` compares against `currentFnSrcPath` for the same reason. A new bare-name registry keyed only by name is a regression.
- **`declare`s are deduped by symbol, not by text** (`declKey` in `irCodegenTypes.suru`), first writer wins. The compiler's hardwired `declare ptr @memcpy(…)` (`emitMainWrapper`) and stdlib's `extern fn memcpy(…) void` are two signatures for one name, which LLVM rejects. This is safe only because opaque pointers make every `call` carry its own explicit function type, so the callee's declared signature is never consulted.
- `@suru_clone_T`/`@suru_drop_T` are **not** namespace-mangled, so `emitAllTypeCloneDrop` guards on an already-emitted-name list (walked in `ctx.typeDecls` registration order, to keep output deterministic for the bootstrap fixed point).

Pipeline order in `pipeline.suru` (`runPipelineFull`):
```
lex → parse → collectFnNames/TypeNames → resolveIncludes → buildModuleImportRegistry
→ runPrePasses(mergedStmts)                  // global type/fn registry
→ for each Module: resolveModuleTypes        // pass 1 — per-module import context
→ collectGenericDefs → runMonoPass
→ inject concreteNodes; filterGenericDefs
→ augment ownFnNames / ownTypeNames
→ runPrePasses(mergedStmts) → resolveModuleTypes(mergedStmts)  // pass 2 (concrete types)
→ rewriteCallSites (monoPass.suru)           // rename generic calls from node.typeArgs
→ rewriteVariantArms (monoVariantArms.suru)  // unmangled "Some:" → "Some-i64:"
→ lowerObjects                               // augmentPrivateFields then method lift
→ analyzeModuleStatements
→ rewriteQualifiedCalls(mergedStmts)         // imported bodies are emitted from this list too
→ append resolved.externFns                  // imported externs rejoin for their `declare`s
→ codegen                                    // one .ll for everything
```

Imported `extern fn`s are held in a side channel (`ResolveResult.externFns`) so `runPrePasses` doesn't see the same C name declared by several modules as a duplicate; they are folded back into the codegen statements at the very end so `emitModule` emits each `declare` (deduped by symbol) and `generate` registers each signature.

Pipeline entry points in `pipeline.suru`:
- `runBuildDriver(sourcePath, outputPath)` — full pipeline, writes the program's single `.ll`
- `compilePipeline(sourcePath, buildDir)` — full pipeline + invokes `clang-18` to link a binary
- `generateIRText(sourcePath)` — full pipeline, returns IR as a string
- `runPipelineDebug(sourcePath, stopPass)` — stops after the named pass and returns `DebugResult { stmts, state }`; valid pass names: `resolve1`, `mono`, `resolve2`, `semantic`

## Error Diagnostics

Compiler errors include file path, line, column, and a source snippet with a caret:
```
/path/to/file.suru:8:13: error: variable 's' has been moved
  8 |     printLn(s)
                  ^
```

**Error infrastructure** (in `semantic/semantic.suru`):
- `AnalysisError: { message String, line i64, col i64, srcPath String }`
- `AnalyzerState.currentSrcPath` — set to the compilation target at pipeline entry; updated to `FnDeclNode.srcPath` when entering each function body (so errors inside imported functions point to the right file).
- `addError(state, msg, line, col)` — uses `state.currentSrcPath` as the error's file.
- `passes.appendError(state, msg, line, col)` — same pattern for pre-pass errors.

**Position in AST nodes**: `FnDeclNode`, `LetNode`, `AssignNode`, `FieldAssignNode`, `ReturnNode`, `WhileNode`, `IfNode`, `BreakNode`, `ContinueNode`, `VarRefNode`, `CallNode`, `MethodCallNode`, `FieldAccessNode` all carry `line i64, col i64` populated by the parser. Pass `0, 0` for synthetic/position-less errors (snippet is suppressed).

**Snippet helpers** (in `pipeline.suru`): `extractSourceLine(src, lineNum)` and `buildSpaces(n)` — iterative helpers used by `printErrorsFrom`.

**Type-mismatch checking** (value-flow sites): every place a value flows into a slot with a declared type is checked by one shared predicate, `assign.isAssignable(state, expected, actual)` in `semantic/assignable.suru`. It returns true when either side is unknown (`""` — an un-inferred node is skipped, never rejected), and otherwise **parses both sides into a structured `Type`, canonicalizes them with `ops.canonType`, and compares with `ops.equalType`** (§3.4). `canonType` folds a generic application into the `Prim` naming its instantiation, so a substituted-but-unmangled annotation such as `Option<i64>` compares equal to its registered mangled form `Option-i64`, and the colon array form `Array:i64` compares equal to `Array<i64>`; it deliberately does **not** fold the built-in `Arr`. It is `printSuruType`-preserving, which is why the string-keyed sum-variant queries below still see exactly the `makeSuruType` strings they did before. Accepted: structural equality, declared sum-type variant in **either** direction (variant→sum for a `Some<i64>` into an `Option<i64>` slot; sum→variant for the compiler's own typed-bridge downcast idiom `let n MatchStmtNode: someAstNode`, which codegen relies on), the String family (`Str` ↔ `String`), or the integer family (`isIntegerFamily`: `i64`/`i32`/`char` are mutually assignable because integer literals infer as `i64` but validly initialize narrower slots; `f64` is excluded — no implicit int↔float; both family tests match on the `Prim` name of the canonical type). There is exactly **one** definition of this predicate — a duplicate second definition once existed and made the compiler fail to build itself ("function 'isAssignable' is already declared"); extend the single one rather than adding a variant. Wired into: return statements + call arguments (`analyzeReturnStatement`, `checkCallArgTypesAt`, `checkFnCallArgTypes`), **`let` bindings** (`analyzeLetStatement`), **assignments** (`analyzeAssignmentStatement`), field assignments, object-literal fields, and array elements. The let/assign/return sites additionally gate on both types being known before reporting (to keep the diagnostic text honest) and use `exprs.inferTypeForMismatchCheck` — `inferType` but returning `""` (skip) for kinds whose type is an unreliable heuristic for user types: `MethodCallNode` (e.g. `toString → String` is wrong for `StringBuilder.toString()` → `SuruString`) and `MatchExprNode`. **Do not** make those kinds reliable by changing `inferType` — its `MethodCallNode` arm also feeds `resolveTypes.suru` `resolvedType` (codegen). This is what turns a `List<Token>`-into-`Array<Token>` mismatch from a runtime segfault into a compile error. `obj.f: expr` (field assignment), `{ f: expr }` (object-literal fields), and array elements are now covered too — those sites read the pass-2 annotation via `ntype.nodeResolvedType` (`semantic/nodeType.suru` — context-narrowed, so no false positives) and pass it to the same predicate ungated, which is why the unknown-type guard lives *inside* `isAssignable`. Match-arm agreement remains a gap. Tests: `tests/unit/compiler/assignability_test.suru` and `semantic_type_mismatch_test.suru` (unit), fixtures `let-type-mismatch` / `assign-type-mismatch` (compileError). The let/assign diagnostics read `type mismatch: variable 'x' declared T but initialized with A` and `… has type T but assigned A` — the unit test asserts these strings, so reword both together.

## Runtime

`runtime/` contains C source files (compiled to `.o` and linked into every compiled program;
shared header `suru_runtime.h`):
- `box.c` — Scalar boxing; typed print variants (`suru_println_bool/i32/i64/f64`, `suru_printerror_*`). The type-erased `suru_println`/`suru_printerror`/`suru_dyn_len` have been **deleted** (Step 1b).
- `string.c` — String heap management; `suru_string_clone`/`suru_string_drop` (null-safe, called directly by static dispatch). `suru_string_drop`'s `type_tag == 8` early return is the **one remaining `type_tag` read** in the system — see the String-split note below for the two boundaries that still need it.
- `array.c` — Array heap operations; **static-dispatch helpers** `suru_array_clone_scalar(arr, esz)` / `suru_array_clone_heap(arr, clone_fn)` / `suru_array_drop_scalar` / `suru_array_drop_heap` (no `elem_tag` read — element size / per-element fn passed from the static call site). The `*_dyn` variants have been deleted; the `suru_array_at`/`_set`/`_add` shims near the top of the file are pre-typed-element-ABI leftovers marked for removal.
- `struct.c` — Struct layout; **null-safe vtable trampolines** `suru_clone_via_vtable`/`suru_drop_via_vtable` (dispatch through the value's `clone_fn`/`drop_fn` pointer with NO `type_tag` read — used for all struct/variant clone/drop). The old `suru_clone_dyn`/`suru_drop_dyn` have been deleted.
- `variant.c` — Sum type dispatch
- `fileio.c` — `readFile`/`writeFile`/`appendToFile` backends (still reached through hardwired builtins in `irCodegen.suru`, not yet a stdlib module)

**Static clone/drop/print dispatch (Step 1 of `type_tag` removal)**: codegen no longer
emits any runtime function that reads `type_tag`. `heapCloneSym`/`heapDropSym`
(`codegen/irCodegenTypeHelpers.suru`) map a static suruType to its clone/drop symbol:
struct/variant → `suru_clone_via_vtable`/`suru_drop_via_vtable`; `String` →
`suru_string_clone`/`suru_string_drop`; `Array<T>` → a `linkonce_odr` per-element-type
wrapper `@suru_array_clone_<T>`/`@suru_array_drop_<T>` emitted by `ensureArrayHelper`
(`codegen/irArrayCodegen.suru`). Recursive field clones (`cloneHeapField`/`dropHeapField`),
the `clone(x)`/`drop(x)` builtins (`emitCloneDyn`/`emitDropDyn`), struct `.clone()`/`.drop()`
(`emitStructClone`/`emitStructDrop`), and `printLn`/`printError` (`emitPrintByType`) all use
this. The 32-byte header (`type_tag`/`clone_fn`/`drop_fn`) layout is unchanged in Step 1 —
the tag is still written, just never read. **All recursive clone/drop leaves are null-safe**
(the `{}` ownership-release idiom stores null heap fields). Step 2 will shrink the header.

type_tag encoding (still written; the only surviving READ is `suru_string_drop`'s `== 8` static-literal check): 0=bool 1=i32 2=i64 3=f64 4=Struct 5=Array 6=String(heap) 7=SumType 8=StaticString

**Whole-program type-id table (`codegen/irTypeIds.suru`, refactor task T2)**: the successor
numbering to `type_tag`, built into `ctx.typeIds` by `emitModule` immediately after
`registerTypes` (so ids derive from the very registry entries codegen looks up and cannot
drift). **Nothing reads it yet** — T3 points `suruTypeTag` at it; until then any change here
must leave emitted IR byte-identical. Two bands: **0–15 reserved builtins** with fixed
constants (`bool` 0, `i32` 1, `i64` 2, `f64` 3, `char` 4, `String` 5, `Str` 6 — `Str` has
its own id because it encodes *ownership*, not identity, and outlives the refactor), and
**≥16 dynamic**, dense and **sorted by mangled name** — every non-`cType` `TypeDecl`, every
`SumTypeDecl` and its arms, and one id per mangled `Array<T>` name. Sorting is the whole
determinism story: the table is a function of the *set* of names, never of `stmts` order.
`cType` and `extern type` are header-less and get **no** id; `lookupTypeId` returns `-1`, a
sentinel that must never be defaulted. `None` is one shared `TypeDecl` across every
`Option<T>`, so it gets exactly **one** id that appears in every `Option-*` arm set —
deliberate, and pinned by `tests/unit/compiler/typeIdsTest.suru`. `Array<T>` needs
`collectArrayTypeNames` because `ensureArrayHelper` discovers element types lazily during
emission; it recovers them from type annotations and is deliberately over-inclusive (a
spurious name wastes an id, a missing one surfaces as `-1`). Design + task order:
`todo_type_refactor.md`.

## Memory Management

No garbage collector. All heap values (String, Array, Struct, SumType) use explicit `clone()` and `drop()`. The codegen tracks which expressions produce temporaries that need to be dropped after use.

Key codegen helpers in `irCodegen.suru`:
- `isPrintTempString` — decides whether to drop a String result after printing. **Must NOT include `CallNode`** (user functions return borrowed refs; including them causes double-free). `StrLitNode` is also excluded (static globals, no-op drop).
- `isExprTemp` — used for general drop decisions. `Array.at()` returns borrowed refs; never use `isExprTemp` for drop decisions on array element access.

**String ownership (`Str` vs `String`, String-split B3)**: a `Str` is a borrowed
pointer into `.rodata` (every string literal); a `String` is an owned heap allocation.
A borrowed value entering an **owned slot** is materialized into a heap copy by
`irStr.materializeStr` (`codegen/irStringCodegen.suru`) — object-literal fields, field
assignment, array literals, `arr.add`/`arr.set`, and `String` locals. `drop` is then a
purely static decision: `drop(x)` on a `Str` emits nothing, and the in-place `__append`
only frees its old receiver when that receiver is owned.

**Ownership is a property of the slot, fixed at its declaration — never tracked
through the value it holds.** Codegen emits statements in source order while control
flow does not, so a per-variable ownership state that moved with each assignment is
wrong on the second trip around a loop (a `drop` inside the body is emitted once, from
the state of the first iteration). `codegen/irOwnedLocals.suru` (`collectMutatedLocals`,
run once per function into `ctx.mutatedLocals`) decides it up front: a `String` local is
owned when the function assigns to it or uses it as the root of an in-place `__append`
chain; otherwise it keeps its initializer's ownership and allocates nothing. Both
directions of imprecision are memory-safe — a missed mutation leaks, an extra one costs
a clone — so extend the walk freely, but never make the registered type follow the value.

**Storing a borrowed value in an owned slot needs an explicit `clone()`.** The type
system distinguishes literals (`Str`) from owned strings, but NOT borrowed
*parameters* from owned values — both are `String`. A callee that keeps a `String`
parameter beyond its own frame (`makeToken(text String, …) Token` storing it in a
field) is aliasing, and so is copying a string field into a second owned record
(`substParam` in `semantic/mono/monoSubst.suru`, which now clones). These are real
double frees; they are currently survivable only because `suru_string_drop` still
skips `type_tag == 8`. That check — the last `type_tag` read anywhere — cannot be
deleted until this boundary and the `String`-returning-a-literal boundary are closed.

## CLI Commands

```bash
suru compile <file> <out.ll>        # Emit the whole program as ONE LLVM IR file at <out.ll>
suru build   <file>                 # Compile to native binary (placed next to source in build/)
suru lex     <file>                 # Print tokens (KIND text line:col)
suru parse   <file>                 # Print AST (with includes expanded)
suru ir      <file>                 # Print the whole program's LLVM IR to stdout (directly linkable:
                                    #   suru ir x.suru | clang-18 -x ir - /usr/local/lib/suru/runtime/*.o)
suru debug   <pass> <file>          # Stop after pass, print AST + symbol table
                                    # Passes: resolve1, mono, resolve2, semantic
```

## Test Infrastructure

Tests live in `tests/fixtures/`. The runner (`tests/runner/main.suru`) for each fixture:
1. Compiles source via `suru compile` → `.ll`
2. Links with `clang-18` → binary
3. Runs binary, compares stdout against `expected.txt`
4. Runs `valgrind --leak-check=full` for memory safety

Tests can be marked `xfail` (known broken) or `compileError` (expected diagnostic output).

**Unit tests** (`tests/unit/`) run *in-process* inside the runner, with no per-test
compile→link→run→valgrind cost. A test file (e.g. `tests/unit/compiler/lexer_test.suru`)
exports a function returning `Array<TestResult>` built with the shared assert helpers in
`tests/unit/assert.suru` (`assertEqI64`/`assertTrue`/`assertEqStr`, `reportResults`). The
runner imports that function, calls it before the fixture loop, and folds the failure count
into the suite tally. A dedicated `tests/.suruproject` (roots `../src/compiler`,
`../src/stdlib`, `unit`) governs both the runner and fixture builds — it is the nearest
ancestor project file for everything under `tests/`.

## Language Features

- **Types**: bool, i32, i64, f64, char, String, Array<T>, objects (named types declared `type Foo: { ... }`, with or without methods), sum types (discriminated unions), generic types/functions. There is no separate "struct" concept at the language level: a data-only `type` is just an object with zero methods, and a `{ ... }` value is an **object literal** (AST `ObjectLitNode`/`ObjectFieldNode`). "struct" survives only as an implementation term for the in-memory record layout (see Object layout below) and the C-ABI `cType`.
- **Generics**: `type Box<T>: { value T }` / `fn identity<T>(x T) T` — monomorphized at compile time; concrete copies get mangled names (`Box-i64`); type args inferred at usage sites; no explicit instantiation syntax. Generic sum types (`type Option<T>: Some<T>, None`) also supported; variant arm patterns (`Some:`) are automatically rewritten to mangled names by `rewriteVariantArms`. Generic calls work inside object-literal method bodies too (e.g. an object method returning `Option<T>`); this relies on pass-1 type resolution annotating `arr.at(i)` (element type) and `x.clone()` (receiver type) in `rtResolveMethodCallNode` so monoInfer can bind the type argument.
- **Stdlib** (`src/stdlib/`): `option.suru` (`Option<T>`, `some<T>()`), `result.suru` (`Result<T,E>`, `ok<T>()`, `err<E>()`), `list.suru` (`List<T>`, `newList<T>()` — generic resizable array backed by malloc/free/realloc; namespace `Suru.Stdlib.List`; **geometric growth with a shrinking factor**, in the private helper `_ fn computeGrowth(currentCap)` called from `add`: double while `cap < 1024`, then grow 1.5x (`cap + cap/2`) once large — stays amortized O(1) while bounding wasted memory to ~50%, mirroring Go's slice growth), `string.suru` (`SuruString`, `newSuruString()`, `suruStringFrom(s String)`, `suruStringFromI64(n i64)`, `suruStringFromChar(c char)` — the latter two are boundary factories that build a fresh `SuruString` via `newStringBuilder()` → `appendI64`/`appendChar` → `toString()` (no round-trip through built-in `String`); Suru-native **immutable** string; `concat(other String)` / `concatStr(other SuruString)` each return a new string; `compare(other String)` / `compareStr(other SuruString)` give three-way ordering with `strcmp` sign semantics (negative / 0 / positive — common-prefix byte scan over the shorter length, then length tie-break; signed byte compare, correct for ASCII); no in-place mutation; no calls to runtime/string.c; `suruStringFromBuffer(buf ptr, n, cap)` is the single object-literal/raw-buffer factory all constructors delegate to; `clone()` uses `malloc`+`memcpy`, `slice()` uses `malloc`+`memcpy` from `this.data.add(from)`, and `concatStr()` uses `malloc`+two `memcpy` (the second from `other.dataBuffer()`, a `ptr`-returning method exposing the internal buffer)), `stringBuilder.suru` (`StringBuilder`, `newStringBuilder()` — mutable in-place accumulator with `append(String)`/`appendChar(char)`/`appendStr(SuruString)`/`appendI64(i64)`/`appendF64(f64)`; `toString()` snapshots to an immutable `SuruString`; namespace `Suru.Stdlib.StringBuilder`. `appendI64` builds decimal digits directly into the buffer in pure Suru — no modulo op exists, so each digit is `m.take(q.multiply(10))` with `q = m.split(10)` (sdiv), and digit→byte is `"0123456789".__at(d)`; extraction runs in the non-positive domain so it is overflow-safe at `i64` min. `appendF64` formats fixed 6-digit-fraction decimal (`1.5` → `"1.500000"`, `NaN` → `"nan"`) — the algorithm is pure Suru, but the `f64`↔`i64` cast (no `fptosi`/`sitofp` primitive exists) is delegated to the FFI bridge `suru_f64_to_i64` (truncate toward zero) / `suru_i64_to_f64` (widen) in `runtime/string.c`, declared as `extern fn`; no codegen change, so no bootstrap)
- **Control flow**: `while`, `if/else if/else`, `match` (statement and expression). A `match` scrutinee may be `bool`/`i64`/`f64`/`char`/`String`/`SuruString` (allow-list in `semantic/exprs.suru`) or a sum type. For a `SuruString` scrutinee, `emitPatternComparisons` (`codegen/irCodegen.suru`) keeps the scrutinee as an object ptr (skips the named-type i64 unbox) and lowers each `"..."` literal arm to a vtable `scrutinee.equals(pattern)` call (helper `emitSuruStringEqualsPattern`, returns `i1` directly) — the pattern stays a built-in `String`, so no per-arm `suruStringFrom`; `String` scrutinees still use the `@strcmp` path.
- **Modules**: every `.suru` file must declare a namespace (`namespace A.B.C`). Cross-file dependencies can be referenced by fully-qualified name without any import (`A.B.C.fn()` — the compiler auto-discovers the file). `import` is a convenience for shorter local names; four forms: `import { A.B.C }` (FullNs), `import { alias: A.B.C }` (AliasNs), `import { [fn1, fn2]: A.B.C }` (SelectiveNames), `import { [alias: fn1]: A.B.C }` (SelectiveAliased). Auto-discovery uses a namespace registry built from `.suruproject`. `include` has been removed. **Exports are a declaration prefix**: a top-level `fn`/`type`/`cType`/`let`/`extern fn`/`extern type` becomes part of the module's public surface by writing `export` in front of it (e.g. `export fn parse(...)`, `export type Parser: { ... }`). There is no `export { ... }` block (removed) and no aliasing — a declaration is always exported under its own name. The parser folds every `export`-prefixed declaration into one synthetic `ExportNode` (appended to the module's stmts) via the `declName` helper, so downstream passes (`exportPass.suru`, the `pipeline.suru` pre-scan, `importPass.suru`, codegen alias mapping) are unchanged. `export` before a non-declaration token is the error "expected a declaration after 'export'" (fixture `tests/fixtures/namespace-export-error`). **Module-header order is fixed and enforced by the parser** (`parse()` in `parser/parser.suru`): `namespace`, then **at most one** `import` block (a second is the error "a module may declare at most one import block"), then declarations (any of which may carry an `export` prefix). An `import` after a declaration → "import block must appear before declarations" (fixture `tests/fixtures/import-after-export-error`). A single `import` block may list many entries (one per line or comma-separated). Fixtures: `tests/fixtures/multiple-import-error`, `tests/fixtures/import-after-export-error`. **Imports are private to each module** — a file's `import` declarations only affect that file's own code; included files use their own import context. Implementation: `qualNamePass.suru` (collect + rewrite); called from `pipeline.suru` (`resolveIncludesRec` Pass 3 + after `desugarImports`). Module isolation: `resolveIncludes` returns `Array<Module>` (each `Module` has `srcPath`, `ns`, `stmts`, `importedNs`); `AnalyzerState.moduleImportRegistry Array<SrcModImports>` maps each file's path to its imported namespaces; `resolveModuleTypes` updates `state.currentModuleImports` per `FnDeclNode` entry via `lookupModuleImports`.
- **Methods**: value method calls (`5.add(3)`, `arr.push(x)`)
- **Objects**: every named `type` is an object. A `type` declaration lists data fields and (optionally) method signatures with no bodies; an **object literal** (`{ ... }`) supplies field values and the per-instance method bodies. `this` inside a method body refers to the receiver. A type that declares methods gets per-instance vtable-based virtual dispatch; a data-only type (zero methods) is just a record with no vtable. Both share the same in-memory layout (see Object layout). The distinction lives only in `lowerObjects` (`pipeline.suru`), which adds the synthetic `$vtable` field + lifts method bodies to top-level functions when `methods.len() > 0`.
- **Private members**: `_ name Type: value` in an object literal declares a private data field; `_ fn name(...) Ret { ... }` declares a private method. Both are accessible only via `this.name` inside sibling methods (public→private, private→private, and private methods reading private fields all work); access from outside is a compile-time error (the `privateMethodNames`/`privateFieldNames` gate in `rtResolveMethodCallNode`/field-access). Private members are NOT listed in the `type` declaration (the public interface). **Private methods are lifted and vtable-dispatched exactly like public ones** — they are injected into the same `methods` lists so the lambda-lift + vtable machinery picks them up; the only difference from a public method is the external-access gate. Implementation: the semantic layer (`rtAugmentPrivateFields` in `resolveTypes.suru`, pass-2 `ObjectLitNode` resolution) augments `SemTypeEntry.fields` with private data fields and `SemTypeEntry.methods` with private method signatures (`rtHasMethod` dedup), and populates `privateFieldNames`/`privateMethodNames`; the codegen layer (`augmentPrivateFields` in `pipeline.suru`, run at start of `lowerObjects`) injects private data fields into `TypeDeclNode.fields` (correct GEP offsets) via `augPrivAddField` and private method signatures into `TypeDeclNode.methods` via `augPrivAddMethod` (so `registerTypes` → `methodIndex` → vtable dispatch instead of the old `-1` fall-through that segfaulted to `unbox(null)`). `augPrivInBody` handles both `LetNode → ObjectLitNode` and `ReturnNode → ObjectLitNode` (using `fd.returnType` as the type name for the latter). Both the semantic and codegen `methods` lists end up ordered `[public-interface] ++ [private-literal]`, so the lowerObjects vtable-entry order and codegen `methodIndex` agree. **Single-literal assumption**: a private member is recorded on the type's shared `SemTypeEntry`/`TypeDeclNode`, so if a type had two object literals and only one declared a given private method, `lowerObjects` would demand it in the other ("missing required implementation"). Every current stdlib type has exactly one literal, so this is fine.
- **`ptr` type**: `ptr` is a first-class untyped C pointer type (`void*` in C, `ptr` in LLVM IR). Valid in `extern fn` param/return types, **regular `fn` parameters**, and `let` declarations. Still rejected as a regular `fn` **return type** and as a struct field (no type_tag, so `clone()`/`drop()` are errors). **An object-literal method MAY return `ptr`** — the return-type rejection lives only in `registerFnDecl` (top-level `FnDeclNode`); method signatures are registered without that check (`passes.suru` ~line 306). Used by `SuruString.dataBuffer()` to expose its borrowed internal buffer for a sibling `memcpy`. Allowing `ptr` as a regular-fn parameter is safe because Suru never auto-drops parameters — a `ptr` param is just a borrowed raw pointer; this enables raw-buffer factories like `suruStringFromBuffer(buf ptr, n i64, cap i64)` in `src/stdlib/string.suru`. No boxing or unboxing: a `ptr` value is stored/loaded as a raw LLVM `ptr` alloca. The `_: "ptr"` catch-all in `irLlvmTypeOf` already handled this; the explicit `"ptr": "ptr"` arm in `irCodegenTypeHelpers.suru` documents the intent. Rejections in `semantic/passes.suru` (`isBuiltinType`; `registerFnDecl` return-type only, param accepted; `collectTypeDeclarations`), `resolveTypes.suru` (method-form clone/drop), `exprs.suru` (function-form clone/drop). Example: `extern fn malloc(size i64) ptr` / `extern fn free(p ptr) void`.
- **`extern type` (opaque C handles)**: `extern type Foo` declares a named opaque handle type for FFI (e.g. `FILE*`, `LLVMContextRef`). It has no fields, no methods, and no runtime header — it is erased to `ptr` in codegen via `irLlvmTypeOf`'s default `_: "ptr"` arm. No IR is emitted for the declaration. Call-site type checking uses exact name matching so `FILE*`-typed values cannot be passed where `LLVMContextRef` is expected. `clone()`/`drop()` and use as a regular struct field are compile errors (same rejections as `cType`/`ptr`). Implementation: `ExternTypeNode` in `parser/parserAst.suru`; `parseExternDecl()` in `parser.suru` dispatches `extern fn` vs `extern type` via `this.peekIs(TOK_TYPE)`; `isExternType: true` on `SemTypeEntry` in `semantic/semantic.suru`; `isExternTypeNamed` in `semantic/passes.suru`; rejections in `resolveTypes.suru` (method-form clone/drop) and `exprs.suru` (function-form clone/drop); no-op `ExternTypeNode: {}` in `irRegisterPasses.suru` and `irCodegen.suru`. Example: `extern type Buffer` / `extern fn malloc(size i64) Buffer` / `extern fn free(b Buffer) void`.
- **cType (C-ABI structs)**: `cType Foo: { a i64, b i32 }` declares a header-less, C-ABI-laid-out struct for FFI. Unlike a regular `type` (32-byte runtime header, then **compact natural-alignment fields** — see Object layout below), a `cType` has **no header** (field 0 at offset 0), uses **natural C alignment** with padding (x86_64: bool/char 1, i32 4, i64/f64 8), **preserves field order** (C ABI is order-sensitive — no reordering, unlike a regular `type`), and is **erased to a raw `ptr`** at the `extern fn` boundary so a `cType` value passes straight to libc (e.g. `memcpy`). It has **no type_tag**, so `clone()`/`drop()` are rejected and memory is freed manually via an `extern fn free` (codegen emits no `@suru_clone_T`/`@suru_drop_T` and never auto-drops it). Fields must be fixed-layout primitives (`bool`, `char`, `i32`, `i64`, `f64`); methods on a `cType`, a `cType` used as a regular-type field, and `String`/`Array`/named/nested-`cType` fields are compile errors. Implementation: `TOK_CTYPE` + `parseCTypeDeclaration` (`lexer.suru`/`parser.suru`); `isCType` on `TypeDeclNode`/`SemTypeEntry`/`TypeDecl`; rejections in `semantic/passes.suru` (field-type + nested, order-independent second loop in `collectTypeDeclarations`), `resolveTypes.suru` (method-form clone/drop, methods), `exprs.suru` (function-form clone/drop); C-ABI geometry `cFieldSize`/`cFieldAlign`/`cAlignUp`/`cFieldOffset`/`cStructSize` + header-less alloc/access/assign in `codegen/irStructCodegen.suru` and `emitObjectLit` in `irCodegen.suru`. A `void` `extern fn` (e.g. `free`) is called via `emitCallDiscard` (no SSA result); `toString()` on `i32`/`char`/`bool` widens to `i64` first.
- **Object layout (regular `type`)**: an object value is a `ptr` to a 32-byte runtime header (type_tag/variant_idx/clone_fn/drop_fn at offsets 0/8/16/24) followed by a **compact, natural-alignment field block** starting at offset 32. (The in-memory record is still called a "struct" in codegen/runtime — that is an implementation term, like LLVM's `struct`.) Each field is stored at its natural LLVM type/size (bool/char 1, i32 4, i64/f64/heap-ptr 8) — not the old "every field an 8-byte `i64` slot". Fields are **reordered into three alignment buckets (8 ++ 4 ++ 1, stable within each)** to minimise padding; the synthetic `$vtable` ptr (added first for types with methods) stays at index 0 / offset 32. All field access is by name (`fieldIndex` → offset), so reordering is transparent. Implementation: `reorderSuruFields`/`suruFieldOffset`/`suruStructSize` in `codegen/irStructCodegen.suru` (offset walk based at 32, reusing the `cType` geometry helpers); `irRegisterPasses.suru` reorders non-cType `TypeDecl.fields` at registration; field access/assign + `emitObjectLit` store at the field's natural LLVM type (with a value-width normalise so a wide RHS can't clobber a neighbour); `irTypeCloneDropCodegen.suru` uses `copyScalarField` (natural-width scalar copy) over the reordered offsets, heap fields keep the recursive 8-byte clone/drop. `cType` keeps declaration order and is header-less (see above).
- **Integer literals**: decimal, or hexadecimal with a `0x`/`0X` prefix (`0xFF`, `0xDEAD`, mixed case `0xAbCd`). Hex is lexer-only: `readNumber` in `lexer.suru` branches to `readHexLiteral`, which decodes the digits and emits a plain decimal `TOK_INT`, so every later stage (and type inference into `char`/`i32`/`i64`) is unchanged. Bare `0x` (no digits) is a fatal lex error with a caret diagnostic. Pure conversion helpers live in `src/compiler/lexer/lexerHex.suru` (`isHexDigit`, `hexCharToVal`, `hexDigitsToI64`).
- **String literals**: `StrLitNode` emits static `%suru.String` globals (type_tag=8, zero allocation); `.append(StrLitNode)` uses `emitStringAppendLit` optimization
- **char/String escape sequences**: `\n`, `\t`, `\\`, `\"`, `\'`, `\0` (null byte). `escapeChar` in `lexer.suru` handles all six; the `\0` arm embeds a literal null byte in the source (the match arm value is a 1-byte string with byte 0, not the digit `'0'`). `peek()` uses `'\0'` as the EOF sentinel so call sites need no explicit bounds check on `pos+1`.
- **`typeSize(T)` intrinsic**: `typeSize(T)` returns an `i64` compile-time constant equal to the element storage size of type T (bool/char=1, i32=4, everything else=8). Requires `import { [typeSize]: stdlib.ffi }` — using it without the import is a semantic error. `typeSize` is a contextual keyword (lexed as `TOK_IDENT`); `parsePrimary` dispatches to `parsePrimaryTypeSize` only when `typeSize` is followed by `TOK_LPAREN`. The argument inside the parentheses is a type name, not a value expression. Works inside generic functions: `monoSubst.suru` rewrites `SizeNode.typeName` so `typeSize(T)` → `typeSize(i64)` after monomorphization. Import gating: `currentModuleHasImport(state.currentModuleImports, "stdlib.ffi")` in `resolveTypes.suru` checks the active module's import context (private per file, never the root's); `state.currentModuleImports` is updated per `FnDeclNode` entry via the `moduleImportRegistry`. Implementation: `SizeNode` in `parserAst.suru` (index 36 — do NOT move or remove); `parsePrimaryTypeSize` in `parser.suru`; `SizeNode` arms in `resolveTypes.suru` (→ i64 + import check), `monoSubst.suru` (type subst), `irCodegen.suru` (→ `irArr.elemSizeBytes(typeName)` constant). Phantom export: `src/stdlib/ffi.suru` declares `fn typeSize<T>() i64 { return 0 }` so the import validator finds the name — this body is never called.
- **`ptr` methods (`ptr.load` / `ptr.store` / `ptr.add`)**: a `ptr` value behaves like an object exposing byte-addressed memory ops — **no import required** (intrinsic builtins, like `arr.push`). `p.load(offset i64) T` emits `getelementptr i8` + typed load; `T` is inferred from the surrounding expected-type context (let-binding type annotation, return type, etc.) — error if type cannot be inferred. `p.store(offset i64, val T)` emits GEP + typed store; returns void. `p.add(n i64) ptr` emits `getelementptr i8, ptr p, i64 n` and returns the offset `ptr` (used to produce an offset source/destination pointer for `memcpy`, e.g. `SuruString.slice` via `this.data.add(from)`). bool memory convention: `load` loads i8 then `trunc i8 to i1`; `store` `zext i1 to i8` then stores i8. Implementation: dispatched on a `"ptr"` receiver in `rtResolveMethodCallNode` (`resolveTypes.suru` — `load`→expected, `store`→void, `add`→ptr) and in `emitMethodCall` (`irCodegen.suru`, alongside the Array/String receiver-type dispatch). There is no phantom declaration and no `stdlib.ffi` import gate (unlike `typeSize`).
- **Move semantics**: `move(x)` is a 1-arg builtin that transfers ownership without cloning. The semantic layer marks `x` as moved in `state.movedVars`; any subsequent `VarRefNode` for `x` is a compile error. Re-assignment (`x: expr`) clears moved state after analysing the RHS. Each function body resets `movedVars` to `[]` on entry. Codegen emits a plain variable load — no clone. Tracking is linear (not flow-sensitive). Implementation lives in `semantic/semantic.suru` (helpers), `semantic/exprs.suru` (VarRefNode + CallNode), `semantic/stmts.suru` (clearMoved), `semantic/fns.suru` (resetMovedVars), `codegen/irCodegen.suru` (move case).
