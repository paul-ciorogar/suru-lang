# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fix — generics inside object-literal method bodies

Generic types/functions (e.g. stdlib `Option<T>`, `some<T>()`) can now be used
inside object-literal method bodies. Previously a call like
`some(this.items.at(i))` in an object method emitted an unmangled `@some` (invalid
IR / segfault), or failed with `unknown return type 'Option<Sig>'`. Two
independent compiler gaps were fixed:

- `src/compiler/semantic/resolveTypes.suru` (`rtResolveMethodCallNode`): pass-1
  type resolution now annotates `arr.at(i)` with the array's element type and
  `x.clone()` with the receiver's type. Without these, generic-call arguments
  built from `.at()`/`.clone()` had an empty `resolvedType`, so monomorphization
  could not infer the type argument and never created the instantiation. (This
  was never object-method-specific — it reproduced at top level too; object
  methods just made it the common case.)
- `src/compiler/semantic/mono/monoInstantiate.suru` (`instantiateAll`): converted
  from a single pass into a **fixed-point worklist**. When a concrete function is
  produced (e.g. `some__Sig`), its substituted return/param/`let` type
  annotations are re-scanned for generic applications (`Option<Sig>`), which are
  enqueued and instantiated transitively (`Option__Sig` + its variant structs).
  Termination is guaranteed by the existing mangled-name `seen` set.

New regression fixture `tests/fixtures/objects-option-find/` (a registry object
whose `find` method returns `Option<Sig>`, exercising both `Some` and `None`
paths). 77 tests pass; bootstrap fixed-point holds.

### Refactor — compiler internals cleanup

Behaviour-preserving sweep across `irCodegen`, `monoPass`, and `importPass`.
All 76 tests pass; bootstrap fixed-point still holds.

- `src/compiler/codegen/irCodegen.suru`:
  - Added three helpers (`emitRawIndex`, `emitFilePathArg`, `emitPrintByType`)
    that collapse repeated unbox/dispatch patterns. Applied at five
    Array/String index sites, both file I/O builtins (`writeFile`,
    `appendToFile`), and both print builtins (`printLn`, `printError`).
  - Converted `appendRootSlot` from tail recursion to a `while` loop.
- `src/compiler/semantic/mono/monoPass.suru`:
  - Inlined seven tail-recursive `*At` helpers
    (`extractFnNamesAt`, `extractTypeNamesAt`, `filterGenericDefsAt`,
    `rewriteCallsInExprsAt`, `rewriteCallsInStmtsAt`, `rewriteMatchArmsAt`,
    `rewriteMatchStmtArmsAt`, `rewriteStructFieldsAt`) into their wrapper
    functions as `while` loops; removed the now-redundant `*At` exports.
- `src/compiler/semantic/importPass.suru`:
  - Split the 109-line `buildSubstTable` into four per-kind helpers
    (`buildSubstForFullNs`, `buildSubstForAliasNs`,
    `buildSubstForSelectiveNames`, `buildSubstForSelectiveAliased`) and
    three diagnostic helpers (`importConflictError`,
    `importNotExportedError`, `addSubstEntry`). The dispatcher now hoists
    the shared "unknown namespace" check above the per-kind match.

### Namespace system — include removal + namespace enforcement (Task 13)

Namespace declarations are now **required** in every `.suru` file. Missing a `namespace`
declaration is a hard compile error:
```
/path/to/file.suru: error: namespace declaration required
```

The `include` directive has been **removed** from the language:
- Lexer: `TOK_INCLUDE` token removed
- AST: `IncludeNode` and `IncludeAlias` types removed
- Parser: `parseIncludeDirective()` removed
- Pipeline: `IncludeNode` resolution branch removed; `collectNonInclude` simplified
- Codegen: dead `lookupAliasSrc` helper and `aliases` parameter removed from `generate()`

All remaining `include` directives in the compiler source and test fixtures have been
converted to `import` declarations.

Remaining files that received namespace declarations: `src/compiler/diagUtils.suru`
(`Suru.Compiler.DiagUtils`), `src/compiler/debug/debugPrint.suru` (`Suru.Compiler.Debug.Print`),
`src/compiler/build.suru` (`Suru.Compiler.Build`), `src/cli/main.suru` (`Suru.Cli`),
`tests/runner/main.suru` (`Suru.Tests.Runner`).

New test fixture `namespace-error-missing` pins the compile-error diagnostic.
Bootstrap fixed point confirmed twice (hash `d8e807f464918f81098ccadd9ccdc4d7`). All 71 tests pass.

### Namespace system — compiler source (Task 12)

All 29 compiler source files in `src/compiler/semantic/`, `src/compiler/codegen/`, and
`src/compiler/pipeline.suru` now carry `namespace` declarations and `export` blocks.

New project file support: a `.suruproject` file at the project root lists additional source
root directories (one per line, relative to the file, `#`-comments ignored). The compiler
walks upward from the entry file to find `.suruproject` and passes all listed directories
to `buildNamespaceRegistryMulti` so imports across sibling directories (e.g. `src/cli/`
importing `src/compiler/` namespaces) resolve correctly.

New pipeline helpers exported from `pipeline.suru`:
- `findProjectFile(startDir)` — locates `.suruproject` by walking up the directory tree
- `readProjectSourceDirs(projectFile)` — parses the project file into a list of absolute paths
- `registryRootsFor(sourcePath)` — combines the entry file's directory with project file dirs
- `buildNamespaceRegistryMulti(dirs)` — scans multiple directories in a single `find` invocation

All `include` directives in those files replaced with `import { alias: Namespace }` declarations,
completing the include→import migration for all compiler source.

Legacy `mangleSym` and `sanitizePath` functions removed from `irCodegen.suru`; the `fnMangleSym`
fallback (for extern functions with no source namespace) now returns the name verbatim.

Semantic analysis extended: `MethodCallNode` receivers that are not `VarRefNode` (e.g. desugared
import-alias calls routed through `QualifiedNameNode`) now also trigger argument-type checking
via `checkCallArgTypesAt`, catching cross-module type mismatches that previously went unreported.

Bootstrap fixed point confirmed twice (hash `fbf0c62d8f00acfec2387b4679b47313`). All 70 tests pass.

### Renamed primitive types to lowercase

The five primitive type names have been renamed to their lowercase, Rust-style equivalents:

| Before | After |
|--------|-------|
| `Bool` | `bool` |
| `Int32` | `i32` |
| `Int64` | `i64` |
| `Float64` | `f64` |
| `Char` | `char` |

`String` and `void` are unchanged.

```suru
let x i64: 42
let ratio f64: 1.5
let flag bool: true
let c char: 'a'
let p i32: 7
```

All compiler source files, test fixtures, stdlib, README, and CLAUDE.md have been updated. The rename was carried out in two bootstrap phases: phase 1 updated the string tables (what the compiler recognises internally) while keeping the compiler's own source annotations unchanged so the old bootstrap binary could still compile it; phase 2 updated the compiler source annotations using the new binary.

---

### Object types supported as sum type variants

Sum type variants can now be object types (types with method signatures). Previously only plain structs could appear as variants. This enables patterns like calling methods on a narrowed match arm value:

```suru
type Animal: {
    fn speak() String
}
type Empty: { }
type MaybeAnimal: Animal, Empty

let a MaybeAnimal: makeAnimal(0)
match a {
    Animal: { printLn(a.speak()) }
    Empty:  { printLn("empty") }
}
```

No compiler changes were required — the codegen (`registerTypes` in `irRegisterPasses.suru`), object lowering (`lowerObjectsStructLit` in `pipeline.suru`), and clone/drop infrastructure (`irTypeCloneDropCodegen.suru`) already handled this case correctly. The change is validated by the new `sum-type-with-methods` fixture (58 tests total, all passing, valgrind clean).

### Mono pass reorganized into `semantic/mono/` subfolder

The five monomorphization files (`monoCollect`, `monoInfer`, `monoInstantiate`, `monoSubst`, `monoPass`) were moved from `src/compiler/semantic/` into a dedicated `src/compiler/semantic/mono/` subfolder, mirroring the existing `lexer/`, `parser/`, and `codegen/` layout. No behaviour changes — include paths updated throughout (`pipeline.suru`, the two unit test fixtures, and cross-references inside the mono files themselves).

### Added `suru debug <pass> <file>` CLI command

A new `debug` command stops the compilation pipeline after a named pass and prints the full AST and symbol table (functions, types, sum types, scopes, errors) to stdout. Useful for inspecting intermediate compiler state without reading LLVM IR.

```sh
suru debug resolve1 src/myprogram/main.suru   # before monomorphization
suru debug mono     src/myprogram/main.suru   # after concrete nodes injected
suru debug resolve2 src/myprogram/main.suru   # after second type resolution
suru debug semantic src/myprogram/main.suru   # full symbol table
```

Implementation:
- **`pipeline.suru`**: `DebugResult { stmts Array<AstNode>, state AnalyzerState }` type and `runPipelineDebug(sourcePath, stopPass)` function — mirrors `runPipelineFull` with early-return checkpoints after each named pass.
- **`debug/debugPrint.suru`** (new file): `printDebugOutput(stmts, state, passName)` — prints the AST via `parserPrint.printModule` and a structured symbol table (functions with signatures, struct types with fields, sum types with variants, scope chain with variable bindings, error list).
- **`cli/main.suru`**: `runDebug` function and `debug` dispatch in `main()`.

### Parser refactored to object type with private state

`Parser` is now an object interface type with private `pos` and `tokens` fields. All cursor logic moved from free functions to methods; `parseTypeAnnotation` returns `String` directly (no more `TypeAnnResult` threading). `newParser(tokens)` is the sole constructor.

**Before:**
```suru
let parser Parser: { pos: 0, tokens: tokens }
parser: util.advance(parser)
parser: util.consume(parser, TOK_IDENT)
if util.currentTokenIs(parser, TOK_EOF) { ... }
let tr TypeAnnResult: util.parseTypeAnnotation(parser)
parser: tr.parser
let t String: tr.typeName
```

**After:**
```suru
let parser Parser: util.newParser(tokens)
parser.advance()
parser.consume(TOK_IDENT)
if parser.currentTokenIs(TOK_EOF) { ... }
let t String: parser.parseTypeAnnotation()
```

Changes:
- **`parser/parserAst.suru`**: `type Parser` is now an object interface declaring 9 method signatures; `pos`/`tokens` removed from public interface.
- **`parser/parserUtil.suru`**: `newParser()` returns an object literal with `_ pos i64: 0` and `_ tokens Array<Token>: tokens` as private fields, plus all method bodies (`advance`, `currentToken`, `currentTokenIs`, `peekToken`, `peekIs`, `consume`, `consumeIf`, `error`, `parseTypeAnnotation`). Old free functions removed.
- **`parser/parser.suru`**: ~215 `util.fn(parser, ...)` call sites converted to `parser.fn(...)`. Dead cleanup boilerplate (`r.parser: {}`, `drop(r)`) removed throughout — safe because field assignment is not RAII and intermediate result Parser copies share the same underlying tokens.
- **Known limitation** (`tests/fixtures/private-fields-constructor`, xfail): `augPrivInBody` in `pipeline.suru` only registers private fields from `LetNode → StructLitNode`, not from `ReturnNode → StructLitNode`. Constructor functions using `return { _ field ... }` must use an intermediate `let` binding as a workaround.

### Rich error diagnostics with source location and code snippets

Compiler errors now include file path, line number, column, and a source code snippet with a caret pointing at the exact error site — matching the Rust/TypeScript diagnostic style.

**Before:**
```
/path/to/file.suru: variable 's' has been moved
```

**After:**
```
/path/to/file.suru:8:13: error: variable 's' has been moved
  8 |     printLn(s)
                  ^
```

All semantic errors carry position. Errors that lack position (e.g. duplicate type declarations in pre-passes) fall back to `path: error: message`.

Implementation:
- **`parser/parserAst.suru`**: Added `line i64, col i64` to 13 node types: `FnDeclNode`, `LetNode`, `AssignNode`, `FieldAssignNode`, `ReturnNode`, `WhileNode`, `IfNode`, `BreakNode`, `ContinueNode`, `VarRefNode`, `CallNode`, `MethodCallNode`, `FieldAccessNode`.
- **`parser/parser.suru`**: All node creation sites now capture the relevant token's `line`/`col` and store it in the node.
- **`semantic/semantic.suru`**: `AnalysisError` extended with `line i64, col i64, srcPath String`; `AnalyzerState` gains `currentSrcPath String`; `addError` takes `line i64, col i64`.
- **`semantic/passes.suru`**: `appendError` updated; `registerFnDecl` sets `currentSrcPath` from `FnDeclNode.srcPath`; all call sites pass position.
- **`semantic/stmts.suru`**, **`semantic/exprs.suru`**, **`semantic/fns.suru`**, **`semantic/resolveTypes.suru`**: All ~25 error call sites updated to pass `node.line, node.col`.
- **`semantic/fns.suru`**: `analyzeFunctionDeclaration` sets/restores `state.currentSrcPath` on function entry/exit so errors inside functions point to the correct included file.
- **`semantic/monoSubst.suru`**, **`semantic/monoPass.suru`**: Deep-clone operations propagate `line`/`col` through all reconstructed nodes.
- **`pipeline.suru`**: Sets `state.currentSrcPath = sourcePath` before analysis passes; `printErrorsFrom` rewritten with `extractSourceLine`/`buildSpaces` helpers to emit the three-line diagnostic format; synthetic nodes (method-lift) get `line: 0, col: 0`.
- **Test fixtures**: `break-error`, `continue-error`, `move-error`, `private-fields-error`, `xmod-error` expected outputs updated to new format.

---

### Added `break` and `continue` for `while` loops

`break` exits the enclosing `while` loop immediately; `continue` skips the rest of the current iteration and jumps back to the loop condition. Both are compile-time errors when used outside a loop.

```suru
// break: exit early
let i i64: 0
while i.lt(10) {
    if i.equals(3) { break }
    printLn(i)
    i: i.add(1)
}
// prints 0, 1, 2

// continue: skip an iteration
let j i64: 0
while j.lt(5) {
    j: j.add(1)
    if j.equals(3) { continue }
    printLn(j)
}
// prints 1, 2, 4, 5
```

Nested loops are supported — `break`/`continue` always refer to the **innermost** enclosing loop.

Implementation:
- **`lexer/lexer.suru`**: `TOK_BREAK` (37) and `TOK_CONTINUE` (38) added to the token set and `keywordKind` dispatch.
- **`parser/parserAst.suru`**: `BreakNode {}` and `ContinueNode {}` appended to `AstNode` (indices 30, 31 — appended last to keep prior indices stable).
- **`parser/parser.suru`**: `parseBreakStmt` / `parseContinueStmt` consume the keyword and return the empty node; dispatched from `parseStatement`.
- **`parser/parserPrint.suru`**: pretty-printer cases for both nodes.
- **`semantic/semantic.suru`**: `AnalyzerState` gains `insideLoop i64` (incremented/decremented around `while` body analysis).
- **`semantic/stmts.suru`**: `analyzeWhileStatement` tracks loop depth; `analyzeBreakStatement` / `analyzeContinueStatement` emit a compile error when `insideLoop == 0`.
- **`codegen/irCodegenTypes.suru`**: `IrCodegenContext` gains `loopCondLabels Array<String>` and `loopAfterLabels Array<String>` — a stack of label names for the current loop nest.
- **`codegen/irCodegen.suru`**: `emitWhile` pushes/pops the cond/after labels around body emission; `emitBreak` branches to `while_after_N`; `emitContinue` branches to `while_cond_N`; both set `blockOpen: false`.
- **Test fixtures**: `break-valid`, `continue-valid` (runtime output), `break-error`, `continue-error` (compile-error assertions).

### Added `move()` builtin — ownership transfer without cloning

`move(x)` transfers ownership of a variable to the caller without performing a deep copy. The compiler enforces that `x` cannot be read or passed again after the move — any subsequent use is a compile-time error. Re-assigning `x` with `x: newValue` clears the moved state.

```suru
fn consume(s String) void {
    printLn(s)
    drop(s)
}

fn main() i64 {
    let s String: "hello".append(" world")
    consume(move(s))        // s is moved; consume owns it and drops it

    s: "goodbye".append(" world")   // re-assign: s is valid again
    printLn(s)
    drop(s)
    return 0
}
```

Compile-time enforcement:

```suru
fn main() i64 {
    let s String: "hello".append(" world")
    let t String: move(s)
    printLn(s)    // compile error: variable 's' has been moved
    drop(t)
    return 0
}
```

Design:
- `move(x)` is a 1-arg builtin; its argument must be a variable (a `VarRefNode`), not an arbitrary expression.
- Move state is tracked per-function; each function body starts with a clean slate.
- Re-assigning a moved variable (`x: expr`) is valid and clears the moved flag after the RHS is analysed.
- **Known limitation:** tracking is linear, not flow-sensitive. A `move(x)` inside one branch of an `if` is treated as unconditional — `x` is considered moved on all paths after that point.

Implementation:
- **`semantic/semantic.suru`**: `AnalyzerState` gains `movedVars Array<String>` and helpers `isMoved`, `markMoved`, `clearMoved`, `resetMovedVars`.
- **`semantic/exprs.suru`**: `VarRefNode` analysis checks `isMoved` and emits "variable 'x' has been moved"; `CallNode` analysis adds `move` to the 1-arg builtins and, when the argument is a `VarRefNode`, calls `markMoved`; `inferType` returns the variable's declared type for `move(x)`.
- **`semantic/stmts.suru`**: `analyzeAssignmentStatement` calls `clearMoved` on the target after analysing the RHS; `analyzeLetStatement` calls `clearMoved` after the RHS.
- **`semantic/fns.suru`**: `analyzeFunctionDeclaration` saves and restores `movedVars` around each function body, resetting it to `[]` on entry.
- **`codegen/irCodegen.suru`**: `move(x)` emits a plain variable load (identical to reading `x` directly) — no clone is performed; the receiver takes ownership.
- **Test fixtures**: `move-valid` (valid move + re-assign, runtime output) and `move-error` (use-after-move compile error).

### Added private fields and methods on object literals

Object literals now support access-controlled members using a standalone `_`
modifier token. Private members are invisible from outside the object; only
`this.name` inside a method body is permitted.

```suru
type Counter: {
    fn increment()
    fn get() i64
}

fn main() i64 {
    let c Counter: {
        _ count i64: 0,
        fn increment() { this.count: this.count.add(1) }
        fn get() i64 { return this.count }
    }
    c.increment()
    c.increment()
    c.increment()
    printLn(c.get().toString())   // 3
    // c.count  ← compile error: cannot access private field 'count' from outside its object
    return 0
}
```

Design:
- `_ name Type: value` — private data field. The name is stored **without** the
  underscore; `this.name` is the only valid access form inside the object's methods.
- `_ fn name(params) Ret { ... }` — private method, likewise accessible only via
  `this.name(...)` inside sibling methods.
- Private members are **not** part of the `type` declaration (the public interface);
  they are declared inline on the object literal.
- A read (`obj.field`), call (`obj.method()`), or write (`obj.field: v`) of a
  private member from outside the object is a compile-time error.

Implementation:
- **Parser** (`parser.suru`): `parseStructLitMemberInto` dispatches on `TOK_WILDCARD`
  to two new parsers — `parseStructLitPrivateFieldInto` (requires an explicit type
  annotation) and `parseStructLitPrivateMethodInto`. `StructFieldNode` gained
  `isPrivate bool` and `privateTypeName String`.
- **Semantic layer** (`semantic/semantic.suru`, `semantic/resolveTypes.suru`):
  `SemTypeEntry` gained `privateFieldNames Array<String>` and
  `privateMethodNames Array<String>`. During `StructLitNode` resolution
  `rtAugmentPrivateFieldsAt` populates those lists from the literal so
  `this.fieldname` resolves correctly in method bodies. Privacy checks in the
  `FieldAccessNode`, `MethodCallNode`, and `FieldAssignNode` arms of
  `resolveExpr` emit errors when a private member is accessed from outside.
- **Codegen layer** (`pipeline.suru`): `augmentPrivateFields` runs at the start
  of `lowerObjects` and injects private data fields from object literals into the
  corresponding `TypeDeclNode.fields` so the struct layout contains the correct
  GEP offsets for those fields.
- **Test fixtures**: `private-fields` (Counter with hidden `count`, outputs `3`)
  and `private-fields-error` (compile-time error on external field access).

### Added generics (monomorphization)

Suru now supports generic type and function definitions. The compiler
monomorphizes all generic usage sites at compile time, producing concrete
copies with mangled names (e.g. `Box<i64>` → `Box__i64`).

```suru
type Box<T>: { value T }

fn identity<T>(x T) T { return x }

fn main(args Array<String>) {
    let b Box<i64>: { value: 42 }
    printLn(b.value)    // 42
    drop(b)
    let r i64: identity(42)
    printLn(r)          // 42
}
```

Added:
- `typeParams Array<String>` field on `TypeDeclNode` and `FnDeclNode` — parsed
  from `<T, U, ...>` after the definition name.
- **Multi-arg generic type annotations**: `parseGenericTypeAnnotation` in
  `parserUtil.suru` now collects comma-separated type arguments with a while
  loop, enabling `Pair<i64, String>`, `Map<K, V>`, etc. Previously only
  single-arg forms (`Box<T>`) were parsed correctly.
- **Mono pass** (`src/compiler/semantic/monoPass.suru`) — orchestrates four sub-modules:
  - `monoCollect.suru` — collects generic definitions into a `GenericRegistry`
    and filters them out of the statement list.
  - `monoInfer.suru` — infers concrete type arguments at each call/usage site
    and builds a list of `InstantiationRequest`s via a full AST walk.
  - `monoInstantiate.suru` — deep-clones generic AST nodes with type-parameter
    substitution (`monoSubst.suru`) and produces concrete `TypeDeclNode` /
    `FnDeclNode` copies with mangled names.
  - `monoSubst.suru` — recursive type-name and expression/statement rewriters
    that replace type-parameter names with concrete counterparts throughout an
    AST subtree.
- **Name mangling**: `Box<i64>` → `Box__i64`, `Pair<i64, String>` →
  `Pair__i64__String`. Mangling is applied consistently in:
  - `passes.suru` `resolveTypeName` (semantic validation accepts both raw and
    mangled forms)
  - `irCodegenTypeHelpers.suru` `makeSuruType` (maps annotations to mangled
    names for struct-type registry lookups)
  - `irCodegen.suru` LetNode/StructLitNode handler (uses mangled `annSuruType`
    for `emitStructLit` and `addVar` so field lookups resolve correctly)
  - `monoPass.suru` `rewriteCallSites` (rewrites generic call-site names to
    mangled form after pass-2 resolvedTypes are populated)
- Pipeline integration (`pipeline.suru`): mono pass runs after pass 1 (name
  resolution), concrete nodes are injected and originals filtered, then pass 2
  (type resolution) sees only concrete types.
- **Test fixtures**: `generics-type-basic` (one type, two concrete args),
  `generics-fn-basic` (one function, two concrete args including `String`),
  `generics-multi-param` (`Pair<T, U>` two-parameter type), `generics-nested`
  (same type instantiated with three different args), `generics-cross-module`
  (generic type defined in an included file).

### Added generic sum types and `src/stdlib/option.suru` / `src/stdlib/result.suru`

Generic sum types are now fully supported. The compiler monomorphizes generic
sum type definitions (e.g. `Option<T>`) into concrete instances (`Option__i64`)
and rewrites unmangled match-arm patterns (`Some:`, `None:`) to the mangled
variant names expected by the codegen.

```suru
include "../../src/stdlib/option.suru" as opt

fn describeOpt(x Option<i64>) {
    match x {
        Some: { printLn(x.value) }    // arm pattern rewritten to Some__i64
        None: { printLn("none") }
    }
}

fn main(args Array<String>) {
    let s Option<i64>: opt.some(42)  // namespace-qualified generic fn call
    describeOpt(s)                     // 42
    drop(s)
}
```

Added:
- `typeParams Array<String>` on `SumTypeDeclNode` — parsed from `<T, U, ...>` after
  the type name, using `parseTypeAnnotation` so variant entries like `Some<T>` are
  stored as full type expressions.
- `GenericSumTypeDef` / `sumTypes Array<GenericSumTypeDef>` in `GenericRegistry`
  (`monoCollect.suru`) — collects generic sum type definitions alongside structs.
- `registryHasSumTypeAt` in `monoInfer.suru` — `inferFromAnnotation` now checks
  both the struct and sum type registries so `Option<i64>` annotations trigger
  instantiation.
- Namespace-qualified generic function call inference: `opt.some(42)` (a
  `MethodCallNode` whose receiver has no `resolvedType`) is now detected and
  inferred by `inferFromGenericFnCall` in `monoInfer.suru` and rewritten by
  `rewriteCallSites` in `monoPass.suru`.
- `instantiateSumType` / `wrapSumTypeDeclNode` / `findInstSumTypeAt` /
  `instantiateVariantTypeAt` in `monoInstantiate.suru` — produce concrete
  `SumTypeDeclNode` and variant struct `TypeDeclNode` copies (e.g. `Some<T>` →
  `Some__i64`) with `seen`-based deduplication.
- `rewriteVariantArms` pass in `monoPass.suru` — called from `pipeline.suru`
  after `rewriteCallSites`; maps unmangled arm names (`Some`, `None`) to the
  concrete mangled variant names the codegen expects, using `resolvedType` of the
  match condition to find the sum type's variant list.
- `monoFnNames` field in `IrCodegenContext` and `monoFnNames` parameter on
  `irCodegen.generate` — separates mono-instantiated function names from own
  function names in the codegen. `emitFunction` uses this to emit full `define`
  bodies for concrete instantiations whose `srcPath` points to an included file
  (the generic template's origin), without incorrectly claiming ownership of
  same-named functions from unrelated included files.
- `compileOneFile` (pipeline.suru) now passes `filterGenericDefs(resolved.stmts)`
  to codegen — prevents generic template bodies from being emitted in included
  file IR.
- `src/stdlib/option.suru` — `Some<T>`, `None`, `Option<T>`, `some<T>()`.
- `src/stdlib/result.suru` — `Ok<T>`, `Err<E>`, `Result<T, E>`, `ok<T>()`, `err<E>()`.
- Test fixtures: `stdlib-option`, `stdlib-result`.

### Added objects: interface types with per-instance methods

Types can now declare method signatures (`fn name(params) Ret`, no body)
alongside data fields, and a struct literal supplies both field values and
per-instance method bodies:

```
type Greeter: { name String, fn greet(p String) String }

let g Greeter: { name: "World", fn greet(p String) String { return p.append(this.name) } }
g.greet("Hello ")          // "Hello World"
```

Added:
- New `this` keyword (`TOK_THIS`, ordinal 35 — appended last; ordinals are
  bootstrap-stable). Inside a method body `this` is the receiver; it is
  lowered to an ordinary `this` parameter so `this.field` reads and
  `this.field: x` writes reuse existing field access/assignment.
- `MethodSig` on `TypeDeclNode`; method members on `StructFieldNode`; a
  `VtableRefNode` AST variant (**appended last**) carrying the vtable symbol
  and its entry list.
- **vtable object model:** each object stores a single per-instance vtable
  pointer in a synthetic leading `__vtable` slot; one
  `@vtable.<sanitised-srcPath>.<Type>.<n> = private constant [N x ptr]` is
  emitted per struct-literal site (so two literals of one type may carry
  different bodies; instances from the same site share a vtable). The
  symbol includes the owning file's sanitised path so two `include`d files
  that each declare a literal of the same type don't mint colliding globals
  at link time. Method calls load the vtable, index by declared-method
  order, and indirect-call with `this` first.
- Method lambda-lift (`lowerObjects` in pipeline.suru, after the
  resolved-type pass): each method body becomes a top-level `FnDeclNode` with
  an implicit `this` param; the literal is rewritten to a `__vtable` field.
- `@suru_clone_T` / `@suru_drop_T` treat the `__vtable` slot as **non-owned**
  (shallow copy on clone, never dropped — it is a static constant pointer).
- Strict interface contract: a literal must implement every declared method;
  a method body for a type that declares none is rejected.

Self-hosting note: `parserExpr.suru` was **merged into `parser.suru`**.
Method bodies make expression parsing depend on statement parsing
(`parsePrimaryStruct → parseBlock`) while statement parsing already depends on
expressions; Suru's acyclic per-file include model requires mutually recursive
functions to share one file. Behaviour is unchanged and the 3-stage bootstrap
fixed point (C2 == C3) holds.

### Added `char` value type

Introduces a `char` value type (an `i8`, scalar, never heap-allocated, never
dropped) to give the language an allocation-free way to read a character.
`String.at(i)` allocates a fresh one-character heap `String` on every call.


Added:
- `char` scalar type — `i8` LLVM type, type_tag `9`, box/unbox, array
  (`i8` storage) and struct (`zext`/`trunc`) encoding, recognised as a
  builtin type name by the parser and semantic passes.
- Single-quote char literals: `'x'`, `'\n'`, `'\t'`, `'\\'`, `'\''`. A new
  `CharLitNode` is **appended last** in the `AstNode` sum type — variant tags
  are derived from declaration order, so appending keeps every existing
  variant index stable and the bootstrap fixed point intact.
- `String.__at(i) -> char` — the interim, allocation-free character accessor
  (mirrors the existing `__append` interim-name convention). `String.at(i)
  -> String` is **unchanged**.
- `char.equals(char) -> bool` (`icmp eq i8`), `char.ord() -> i64`
  (`zext i8` to `i64`), `char.toString() -> String`, and a
  `String.append(char) -> String` overload.
- Runtime (`runtime/string.ll`): `suru_string_char_at` (no allocation),
  `suru_string_append_char`, `suru_string_from_char`.


### Fixed self-hosting: `Array<AstNode>` / sum-type clone-drop codegen

Repaired a self-hosting regression introduced by the `resolvedType` migration
that prevented the compiler from rebuilding itself (it surfaced as `cli-lex`
failing to link with invalid `@suru_clone_Array:AstNode` identifiers and
undefined `@suru_clone_AstNode` / empty `@suru_clone_` symbols). Two root
causes, both fixed:
- `src/compiler/semantic/resolveTypes.suru` — the `MethodCallNode` handler
  resolved `.add(x)` arguments with an empty expected type, so struct-literal
  arguments such as `params.add({ ... })` never received a `resolvedType`.
  It now propagates the receiver's `Array<T>` element type to `add`
  arguments.
- `src/compiler/codegen/irCodegen.suru` — `emitStructLit` now returns a null
  pointer for an empty `{}` whose type is not a registered struct (a sum
  type or array), i.e. the `x.field: {}` ownership-release idiom, instead of
  allocating a bogus zero-field struct that referenced an undefined
  per-type clone/drop symbol. `suru_drop_dyn` is a no-op on null, so the
  subsequent `drop(x)` stays safe.

A reconstructed `tests/fixtures/resolved-type-smoke/main.suru` (its source was
never committed — only `expected.txt`) is restored. `./scripts/test.sh` and
`./scripts/bootstrap.sh` now report 18/18 passing; the lone remaining
`cli-lex` memcheck failure is a pre-existing leak the runner does not block
on.

### Build/test workflow hardening

- `Dockerfile` no longer `COPY`s `bin/suru-build` into the image — the
  `docker-compose.yml` bind-mount already provides it. The bake-in let a
  broken binary ride forward invisibly across commits.
- `scripts/bootstrap.sh` is now a verified 3-stage self-hosting bootstrap:
  C1 (current binary) → C2 → C3, requiring `C2 == C3` (a true fixed point;
  C1 may legitimately differ when codegen changed) and a fully green test
  suite before it replaces `bin/suru-build`.
- `scripts/bootstrap.sh` and `scripts/test.sh` now wipe `/tmp/suru-test-*`
  and stale `build/` directories first. Stale binaries there previously
  produced false-PASS results because the runner masks `suru compile`
  errors with `2>/dev/null`.
- `tests/runner/main.suru` gains an `xfail bool` escape hatch on `TestCase`
  (XFAIL = expected failure, not counted; XPASS = remove the marker). No
  test is currently `xfail`.

### AST: `resolvedType` field on expression nodes

Refactor that replaces the brittle `ctx.currentReturnTypeName`
fallback in codegen with a `resolvedType String` field carried by every
expression AST node, written during semantic analysis via combined top-down
and bottom-up type inference. This fixes the nested-struct-literal segfault
and unblocks nested arrays of structs, struct literals as arguments, and
match-expression struct results.

The AST field is declared and parser-init'd to `""`; a
new semantic pass (`resolveTypes.suru`) walks each function body and writes
a canonical SuruType onto every expression node using top-down expectations
from `let` annotations, return types, struct field types, array element
types, and call parameter types; codegen reads `resolvedType` directly for
struct literals, array literals, and return values; the legacy
`ctx.currentReturnTypeName` field and its save/restore dance are deleted;
the original segfault is covered by a dedicated regression fixture.


### Added `if` / `else if` / `else` to the language

Added two new AST node types to `src/compiler/parser/parserAst.suru`:
- `BlockNode { stmts Array<AstNode> }` — a brace-delimited statement block; empty `stmts` signals an absent branch
- `IfNode { condition AstNode, thenBranch BlockNode, elseBranch BlockNode }` — if/else if/else statement; `else if` chains are represented recursively as a single `IfNode` inside `elseBranch.stmts`


### User-Facing CLI (`suru`)

A new user-facing CLI entry point (`src/cli/main.suru`) wraps the compiler pipeline:
- `suru build <file.suru>` — compiles to a native binary in `build/` next to the source file
- `suru lex <file.suru>` — prints all tokens produced by the lexer, one per line (`KIND text line:col`)
- `suru parse <file.suru>` — prints the parsed AST as an indented tree (includes are expanded)
- `suru ir <file.suru>` — prints the generated LLVM IR without invoking clang

`src/compiler/pipeline.suru` was extracted from `build.suru` as a shared library so both the
bootstrap driver and the CLI can use the same lex→parse→semantic→codegen pipeline.
`build.suru` is now a thin 12-line wrapper around `pipeline.suru`.

Added `cli-lex` test fixture: runs `suru lex` on `tests/fixtures/simple/main.suru` and compares
output against a captured snapshot.

### Zero-Cost Static String Literals

String literals (`"hello"`) are now zero-allocation static globals instead of heap-allocated values:
- Each unique literal emits a pair of module-level LLVM `constant` globals: a raw byte buffer
  (`@.str_N`) and a `%suru.String` header (`@.str_hdr_N`) with `type_tag=8` (StaticString).
- No `malloc`, no `memcpy`, no ownership — returning or passing a literal is a pointer to `.rodata`.
- `suru_string_drop` (and `suru_drop_dyn`) treat `type_tag=8` as a no-op; `drop()` on a literal
  is safe and free.
- `suru_println`, `suru_printerror`, and `suru_dyn_len` dispatch `type_tag=8` identically to
  tag=6 (heap String).
- `isPrintTempString` no longer includes `StrLitNode`; static globals never need dropping after use.
- Existing heap Strings (tag=6) are unchanged; clone/append/slice all produce owned heap copies.

### Typed Element Sizes in Arrays

Arrays now store elements at their native size — no heap boxing for scalars:
- `bool` elements use 1-byte `i8[]` buffers; `i32` uses 4-byte `i32[]`; `i64`, `f64`,
  and heap types (String, Array, Struct) use 8-byte `i64[]` buffers.
- `at()` and `set()` are inlined as typed GEP+load/store — no runtime call.
- `add()` dispatches to a typed runtime variant (`suru_array_add_i8/i32/i64`).
- Clone and drop fast-path scalar arrays via `memcpy` / direct free, skipping per-element dispatch.
- Fixed a pre-existing bug where non-empty array literals with struct elements (e.g.
  `let xs Array<MyType>: [{ ... }, ...]`) would generate an invalid `@suru_clone_` reference.

### Added Self-Hosted Test Runner

- `tests/runner/main.suru`: Suru program that compiles and runs the full test corpus using
  `exec()` + `readFile()`. Iterates over 10 fixtures, invokes `suru-build` and `clang-18`
  directly, captures output, compares against `expected.txt`, and reports `PASS`/`FAIL`.
- `scripts/test.sh` now builds the runner via `./scripts/build.sh`, then launches it inside
  Docker — no shell test logic remains in the script itself.

---

## v0.1.0 — Self-Hosting Achieved

### Added

- The Suru compiler (`src/compiler/build.suru`) is written in Suru and compiles itself.
  The bootstrap binary was seeded from the frozen C# compiler
  ([v0.1.0-bootstrap](https://github.com/paul-ciorogar/suru-lang-bootstrap/releases/tag/v0.1.0)).
- Docker-based build environment (`Dockerfile`, `scripts/build.sh`, `scripts/bootstrap.sh`).
  All build and bootstrap operations run inside the container — no host toolchain required.
- Language features: `bool`, `i32`, `i64`, `f64`, `String`, `Array<T>`, named struct types,
  sum types, `match` (statement and expression), `while`, `include`, `clone`/`drop`,
  `readFile`/`writeFile`/`appendToFile`, `exit`, `printLn`, `printError`, `exec`.
- `exec(cmd String) i64` — runs a shell command via `system()` and returns its exit code.
  First language feature added purely in Suru, with no C# changes.
- Fixed `scripts/bootstrap.sh`: the C# bootstrap binary emits one `.ll` per module; the
  script now links all generated modules rather than only the entry-point file.
- All Docker scripts mount `bin/suru-build` as a volume override so bootstrapping no
  longer requires rebuilding the Docker image.
