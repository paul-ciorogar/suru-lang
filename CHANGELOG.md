# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Collision-proof name mangling separators

- **Mangled symbol names no longer use `__`, which users could collide with.** `_`
  is a legal identifier character, so the old `__` separator risked clashing with
  user-written names. Each mangling role now uses a separator the lexer rejects in
  identifiers (and that is valid unquoted in LLVM IR):
  - **Generics** use `-`: `Box<i64>` → `Box-i64`, `Pair<i64, String>` → `Pair-i64-String`.
  - **Namespaces** keep their `.`: `Suru.Cli.fn` → symbol `@Suru.Cli.fn` (dots preserved
    instead of being flattened to `__`).
  - **Compiler-internal reserved names** use `$`: the synthetic vtable field is `$vtable`
    and lifted object-literal methods are `$m_Type_method_0`.
- The builtin intrinsic methods `__append` / `__at` are unchanged — they are written in
  real `.suru` source and must remain lexable (the lexer rejects `$`).
- Pure mangling change: no language-surface or behavior change. Producers and the
  parse/split sites that read separators back (generic base-name detection in
  `irRegisterPasses.suru`, variant-arm prefix match in `monoPass.suru`,
  `isMethodOfMonoType` in `pipeline.suru`) were updated in lockstep; bootstrap
  fixed point (C2 == C3) holds.

### Removed the "struct" concept; `ptr` now feels like an object

- **There is no separate "struct" at the language level — only objects and object
  literals.** A named `type Foo: { ... }` is an *object* (with or without methods); a
  `{ ... }` value is an *object literal*. Pure terminology refactor: the AST nodes
  `StructLitNode`/`StructFieldNode` are renamed to `ObjectLitNode`/`ObjectFieldNode`,
  and the object-literal pathway helpers (`parseObjectLit*`, `rtResolveObjectLitNode`,
  `lowerObjectsObjectLit`, `emitObjectLit`, `printExprObjectLit`, …) follow suit.
  Diagnostics now say `object literal of type '...'` instead of `struct literal ...`.
  No behavior or IR change. ("struct" survives only as an implementation term for the
  in-memory record layout — `irStructCodegen.suru`, `suruStructSize`, `runtime/struct.c`,
  `type_tag 4 = Struct` — and for the C-ABI `cType`.)
- **`ptr` is now an object-like value with method-call memory ops.** The former
  function intrinsics `ptrAdd`/`ptrLoad`/`ptrStore` are replaced by `p.add(n)` /
  `p.load(off)` / `p.store(off, val)`, dispatched on a `"ptr"` receiver (like
  `arr.push`/`5.add`). They are **always available — no `import { ... : stdlib.ffi }`
  required** (the `stdlib.ffi` import gate now applies only to `typeSize`). The old
  function forms and their `ffi.suru` phantoms are removed; stdlib (`string`,
  `stringBuilder`, `list`) and the `ptr-load-store` fixture migrated to method syntax.
  Same emitted IR (GEP `i8` + typed load/store).

### `SuruString.concatStr()` — concatenate two `SuruString`s

- **Added `SuruString.concatStr(other SuruString) SuruString`.** Returns a fresh
  `SuruString` equal to `this ++ other`; modifies neither operand. One `malloc` +
  two `memcpy` (this half, then `other`'s bytes at offset `this.count`), then
  NUL-terminate — no `StringBuilder`, no per-byte loop.
- **Added `SuruString.dataBuffer() ptr`.** Returns the internal borrowed,
  read-only byte buffer so a sibling `SuruString` can `memcpy` its bytes (used by
  `concatStr`, since private fields are accessible only via `this`). Demonstrates
  that an object-literal **method may return `ptr`** even though a regular `fn`
  cannot. Fixture `surustring-basic` extended to cover `concatStr`.

### `ptr` allowed as a regular function parameter; faster `SuruString.clone()`

- **`ptr` is now accepted as a regular `fn` parameter** (previously rejected in
  `semantic/passes.suru`'s `registerFnDecl`). This is safe: Suru never auto-drops
  parameters, so a `ptr` param is just a borrowed raw pointer. `ptr` remains banned as a
  regular-function *return type* and as a struct field (no `type_tag` for clone/drop).
- **`SuruString.clone()` no longer allocates a `StringBuilder`.** It now does a single
  `malloc` + `memcpy` (copying the bytes and the NUL terminator) and hands the buffer to a
  new raw-buffer factory `suruStringFromBuffer(buf ptr, n i64, cap i64)`, which is now the
  single object-literal site for `SuruString`. `suruStringFromBuilder` and the other
  constructors delegate to it.
- **New `ptrAdd(p ptr, n i64) ptr` FFI intrinsic** (`stdlib.ffi`, gated behind
  `import { [ptrAdd]: stdlib.ffi }`): byte-addressed pointer arithmetic emitting a
  `getelementptr i8`, mirroring `ptrLoad`/`ptrStore`. Produces an offset pointer for
  `memcpy` and similar libc calls.
- **`SuruString.slice()` now uses `malloc` + `memcpy`** from an offset source pointer
  (`ptrAdd(this.data, from)`) instead of a per-byte `StringBuilder` loop.

### Refactor — `List`, immutable `SuruString`, and new `StringBuilder`

Three stdlib refactors, plus a compiler fix that import cycles between stdlib modules
exposed.

- **`DynArray<T>` → `List<T>`.** Renamed the generic resizable array: file
  `src/stdlib/dynarray.suru` → `src/stdlib/list.suru`, type `DynArray` → `List`,
  constructor `newDynArray` → `newList`, namespace `Suru.Stdlib.DynArray` →
  `Suru.Stdlib.List`. Fixtures `dynarray-basic`/`dynarray-string` → `list-basic`/`list-string`.
- **`SuruString` is now immutable.** It exposes no in-place mutating methods; `append`
  is renamed `concat` and (like `slice`/`clone`) returns a fresh `SuruString`. Because a
  raw `ptr` cannot be a regular-function parameter, every constructor and transforming
  method funnels through `suruStringFromBuilder(b StringBuilder)` — the single
  object-literal site — which copies the builder's bytes (read via its public
  `len()`/`charAt()`) into a new immutable buffer.
- **New `StringBuilder`** (`src/stdlib/stringBuilder.suru`, namespace
  `Suru.Stdlib.StringBuilder`): a mutable, in-place byte accumulator with
  `append(String)`, `appendChar(char)`, `appendStr(SuruString)`, `len()`, `charAt()`,
  `clone()`, `drop()`, and `toString()` to snapshot an immutable `SuruString`. Fixture
  `surustring-build` → `stringbuilder-basic`; `surustring-basic` updated to use `concat`.

`SuruString` and `StringBuilder` are mutually referential (`SuruString` builds via
`StringBuilder`; `StringBuilder.toString()` returns `SuruString`). Compiling each as its
own root module surfaced a latent bug:

- **Compiler fix — module dedup under import cycles** (`pipeline.suru`,
  `resolveIncludes`): the root path was never seeded into the `resolved` set, so an
  import cycle that led back to the root (e.g. `String → StringBuilder → String`)
  re-included the root as a transitive dependency. Its declarations were added twice and
  codegen emitted every root function and object-literal (and its vtable / `clone`/`drop`
  wrappers) twice, producing `invalid redefinition` link errors. The root path is now
  seeded into `resolved` as a cycle guard and excluded from the returned `includedPaths`
  so `runBuildDriver` does not recompile the root as a dependency.

### Refactor — Per-module import isolation (proper module system)

Modules are now first-class compilation units. Previously, the pipeline merged all
transitive source files into one flat statement array and used two global boolean flags
(`typeSizeImported`, `ptrFfiImported`) set from the root file's imports to gate
compiler intrinsics (`typeSize`, `ptrLoad`, `ptrStore`). This broke the fundamental
invariant that a module's imports are private to that module.

The new architecture:

- `resolveIncludes` returns `Array<Module>` (each `Module` carries `srcPath`, `ns`,
  `stmts`, and `importedNs`). Modules are the primary output; the flat merged stmts
  array is derived by concatenation for global passes.
- `resolveIncludesRec` builds one `Module` per file as it recurses, collecting the
  file's import namespace strings into `importedNs`.
- `AnalyzerState` gains `currentModuleImports Array<String>` (the active module's
  imports during type resolution) and `moduleImportRegistry Array<SrcModImports>`
  (the full per-file import map, built once and shared across both semantic passes).
- `resolveModuleTypes` updates `state.currentModuleImports` via registry lookup on
  each `FnDeclNode` entry. This is correct for both pass 1 (per-module loop) and pass 2
  (merged stmts — concrete copies inherit `srcPath` from their generic template, so the
  registry lookup gives the right answer).
- Pass 1 in `runPipelineFull` and `runPipelineDebug` is now a per-module loop calling
  `resolveModuleTypes(state, module.stmts)` for each module. Pass 2 still runs on
  merged stmts (needed to cover concrete copies injected by monomorphization).
- Import gate checks in `resolveTypes.suru` now call
  `currentModuleHasImport(state.currentModuleImports, "stdlib.ffi")` instead of reading
  the old boolean flags.
- The hardcoded scan functions `ownStmtsImportFfiTypeSize` and `ownStmtsImportFfiPtrFfi`
  in `pipeline.suru` are deleted. Adding a new intrinsic namespace requires no new flags
  or scan functions — just check `currentModuleHasImport(state.currentModuleImports, "ns")`.

Files changed:
- `src/compiler/semantic/semantic.suru`: added `SrcModImports` type; replaced bool flags
  with `currentModuleImports`/`moduleImportRegistry`; added `currentModuleHasImport` and
  `lookupModuleImports` helpers.
- `src/compiler/pipeline.suru`: added `Module` type; added `modules` field to
  `ResolveState` and `ResolveResult`; added `buildModuleImportRegistry`; updated
  `resolveIncludesRec`, `resolveIncludes`, `runPipelineFull`, `runPipelineDebug`,
  `compileOneFile`.
- `src/compiler/semantic/resolveTypes.suru`: per-FnDecl registry lookup in
  `resolveModuleTypes`; replaced all three gate checks.

### Add — `DynArray<T>` stdlib generic resizable array (FFI Phase D1c)

`src/stdlib/dynarray.suru` implements a generic resizable array backed by a raw `malloc`'d
buffer. `newDynArray<T>() DynArray<T>` constructs an empty array with capacity 8; the buffer
doubles on overflow (via `realloc`). Named `DynArray<T>` to leave the existing built-in
`Array<T>` codegen untouched.

```suru
namespace My.App
import { [DynArray, newDynArray]: Suru.Stdlib.DynArray }

fn main(args Array<String>) {
    let arr DynArray<i64>: newDynArray()
    arr.add(10)
    arr.add(20)
    arr.add(30)
    printLn(arr.at(0).toString())   // 10
    printLn(arr.len().toString())   // 3

    let copy DynArray<i64>: arr.clone()
    let sl   DynArray<i64>: arr.slice(1, 3)
    drop(arr)
    drop(copy)
    drop(sl)
}
```

**Interface** (`type DynArray<T>`):

| Method | Description |
|--------|-------------|
| `add(val T) void` | append, growing the buffer if needed |
| `at(idx i64) T` | element at index (no bounds check) |
| `set(idx i64, val T) void` | overwrite element at index |
| `len() i64` | number of live elements |
| `clone() DynArray<T>` | deep-copy (clones each element individually) |
| `drop() void` | drop each element and free the buffer |
| `slice(from, to) DynArray<T>` | deep-copy of `[from, to)` range |

**Runtime requirements**: `extern fn malloc / realloc / free` (already in libc), plus
`typeSize(T)` and `ptrLoad` / `ptrStore` from `stdlib.ffi` (D1a + D1b).

**Key compiler additions**:

- `hasCustomLifecycle` flag on `SemTypeEntry` / `TypeDecl` (set when the interface declares
  both `fn clone() T` and `fn drop() void`). Codegen emits vtable-dispatch
  `@suru_clone_T` / `@suru_drop_T` wrappers instead of field-walking — necessary because
  `_ buf ptr` has no type_tag.
- `inferZeroArgFnsFromAnnotation` in `monoInfer.suru` infers the type parameter for a
  zero-argument generic constructor call from the surrounding `let`-binding annotation
  (e.g. `let arr DynArray<i64>: newDynArray()` → `T = i64`).

Files changed:
- `src/stdlib/dynarray.suru` (new, 122 lines)
- `tests/fixtures/dynarray-basic/` (new — `DynArray<i64>` fixture, valgrind-clean)
- `tests/fixtures/dynarray-string/` (new — `DynArray<String>` fixture, valgrind-clean)
- `src/compiler/semantic/passes.suru` — `hasCustomLifecycle` detection
- `src/compiler/codegen/irTypeCloneDropCodegen.suru` — `emitCustomLifecycleCloneDrop`
- `src/compiler/codegen/irRegisterPasses.suru` — `cloneReturnMatchesType` + `hasCustomLifecycle` on `TypeDecl`
- `src/compiler/semantic/mono/monoInfer.suru` — `inferZeroArgFnFromAnnotation` / `inferZeroArgFnsFromAnnotation`
- `tests/runner/main.suru` — two new test cases

---

### Add — `ptrLoad` / `ptrStore` intrinsics (FFI Phase D1b)

`ptrLoad(p ptr, offset i64) T` and `ptrStore(p ptr, offset i64, val T)` are
byte-addressed typed pointer reads and writes, gated behind
`import { [ptrLoad, ptrStore]: stdlib.ffi }`. Together with `typeSize(T)` (D1a),
they provide the primitives needed to implement `DynArray<T>` entirely in Suru.

```suru
namespace MyBuf
import { [ptrLoad, ptrStore]: stdlib.ffi }

extern fn malloc(size i64) ptr
extern fn free(p ptr) void

fn main(args Array<String>) {
    let buf ptr: malloc(24)
    ptrStore(buf, 0, 42)          // i64 at offset 0
    let i32val i32: 77
    ptrStore(buf, 8, i32val)      // i32 at offset 8
    ptrStore(buf, 12, true)       // bool at offset 12
    let a i64: ptrLoad(buf, 0)    // → 42
    let b i32: ptrLoad(buf, 8)    // → 77
    let c bool: ptrLoad(buf, 12)  // → true
    free(buf)
}
```

`T` for `ptrLoad` is inferred from context (declared type of the `let` binding,
function return type, etc.); a compile error is emitted when the type cannot be
inferred. `ptrStore` always resolves to `void`. The bool memory convention matches
the array runtime: ptrLoad reads `i8` → `trunc i8 to i1`; ptrStore `zext i1 to i8`
→ stores `i8`.

Implementation:
- `src/stdlib/ffi.suru`: non-generic phantom exports `fn ptrLoad(p i64, offset i64) i64`
  and `fn ptrStore(p i64, offset i64, val i64) void` (non-generic so the mono pass
  does not mangle call sites).
- `src/compiler/semantic/semantic.suru`: `AnalyzerState` gains `ptrFfiImported bool`.
- `src/compiler/pipeline.suru`: `ownStmtsImportFfiPtrFfi` scans for the import;
  `state.ptrFfiImported` set before each type-resolution pass.
- `src/compiler/semantic/resolveTypes.suru`: `CallNode` arm intercepts `ptrLoad`/
  `ptrStore` — import-gate check + return-type inference from `expected` context.
- `src/compiler/semantic/exprs.suru`: `checkCallArgTypesAt` returns early for
  `ptrLoad`/`ptrStore` to skip the i64-placeholder phantom signature check.
- `src/compiler/codegen/irCodegen.suru`: intercepts in both `emitValue` `CallNode`
  arm (pre-desugaring) and at the top of `emitMethodCall` (post selective-import
  desugaring which rewrites calls to `MethodCallNode`); emits `getelementptr i8` +
  typed load/store with bool widening/narrowing.
- Tests: `ptr-load-store` fixture; `ptrloadstore_test.suru` unit tests (14 assertions).

### Change — `size T` replaced by `typeSize(T)` from `stdlib.ffi`

The `size T` contextual-keyword expression has been replaced by `typeSize(T)`, a
compiler intrinsic that must be imported from `stdlib.ffi`. The behaviour is
identical — `typeSize(T)` returns an `i64` compile-time constant equal to the
element storage size of `T` (bool/char → 1, i32 → 4, everything else → 8) — but
the new form is function-call style and requires an explicit import, keeping the
name out of module scope in files that don't need it.

```suru
namespace My.Ffi
import { [typeSize]: stdlib.ffi }

fn bufBytes<T>(dummy T) i64 {
    return typeSize(T)     // generic — resolved after monomorphization
}

fn main(args Array<String>) {
    let sz i64: typeSize(i64)   // → 8
    let s2 i64: typeSize(i32)   // → 4
    let s3 i64: typeSize(bool)  // → 1
    printLn(sz.toString())
}
```

Using `typeSize(T)` without the import is a compile-time error:
```
error: typeSize is a compiler intrinsic; add: import { [typeSize]: stdlib.ffi }
```

Implementation:
- `src/stdlib/ffi.suru` (new): namespace `stdlib.ffi`; exports a phantom
  `fn typeSize<T>() i64 { return 0 }` that satisfies the import validator —
  the body is never executed because `typeSize(T)` is parsed as a `SizeNode`
  before semantic analysis.
- `src/compiler/parser/parser.suru`: contextual trigger changed from
  `size IDENT` to `typeSize(`, new `parsePrimaryTypeSize` method.
- `src/compiler/parser/parserAst.suru`: `Parser` type method renamed
  `parsePrimaryTypeSize`; `SizeNode` doc comment updated. `SizeNode` stays
  at index 36 in the `AstNode` sum type (bootstrap stability — never remove).
- `src/compiler/semantic/semantic.suru`: `AnalyzerState` gains
  `typeSizeImported bool`.
- `src/compiler/semantic/resolveTypes.suru`: `SizeNode` arm emits a compile
  error if `state.typeSizeImported` is false.
- `src/compiler/pipeline.suru`: `ownStmtsImportFfiTypeSize` helper scans the
  root module's own statements for the stdlib.ffi import; sets
  `state.typeSizeImported` before each type-resolution pass so the check is
  live when SizeNode is encountered.
- Tests: `size-expr` fixture and `size_expr_test.suru` updated to the new syntax.

### Remove — fileio codegen special-case (FFI Phase C)

`irFileioCodegen.suru` (410 lines of hand-written LLVM IR emission for `fopen`/
`fseek`/`ftell`/`rewind`/`malloc`/`fread`/`fwrite`/`fclose` sequences) is deleted.
Its logic is replaced by three plain C functions in `runtime/fileio.c`
(`suru_readfile`, `suru_writefile`, `suru_appendtofile`). The codegen dispatch
for `readFile`, `writeFile`, and `appendToFile` in `irCodegen.suru` now emits
a single `call` to the corresponding C runtime function rather than a multi-step
IR sequence. `exec` no longer depends on `irFileioCodegen`'s `extractStringData`
helper — it uses `irStringCodegen`'s `emitExtractStringData` instead. No syntax,
semantic, or test fixture changes.

### Add — `extern type` declaration (FFI Phase B2)

`extern type Foo` declares a named opaque C handle type (e.g. `FILE*`,
`LLVMContextRef`). The name is used for compile-time call-site type checking but
erased to `ptr` in codegen — no IR emitted for the declaration, no runtime header,
no clone/drop. `clone()`/`drop()` and use as a regular struct field are compile
errors with caret diagnostics.

```suru
extern type Buffer

extern fn malloc(size i64) Buffer
extern fn free(b Buffer) void

fn main(args Array<String>) {
    let buf Buffer: malloc(64)
    free(buf)
    printLn("ok")
}
```

### Add — `ptr` type in `extern fn` (FFI Phase B1)

`ptr` is now a first-class type name in the Suru type system, representing an
untyped C pointer (`void*` / LLVM `ptr`). Valid in `extern fn` param/return types
and `let` declarations; rejected in regular `fn` signatures and struct fields
(compile errors with caret diagnostics). No boxing or unboxing: a `ptr` value is
stored as a raw LLVM `ptr` alloca and passes to/from C functions directly.

```suru
extern fn malloc(size i64) ptr
extern fn free(p ptr)  void
extern fn memset(p ptr, c i32, n i64) ptr

fn main(args Array<String>) {
    let buf ptr: malloc(64)
    let zeroed ptr: memset(buf, 0, 64)
    free(zeroed)
}
```

Semantic changes: `isBuiltinType("ptr")` = true (`passes.suru`); `ptr` rejected in
`registerFnDecl` (regular fn params/return) and `collectTypeDeclarations` (struct
fields); `clone()`/`drop()` on a `ptr` value is a compile error in both method
form (`resolveTypes.suru`) and function form (`exprs.suru`). Codegen: explicit
`"ptr": "ptr"` arm added to `irLlvmTypeOf` in `irCodegenTypeHelpers.suru`
(the `_: "ptr"` catch-all already handled it; the new arm documents the intent).

Covered by `extern-fn-ptr` and `extern-fn-fileio` fixtures (malloc/free/memset
round-trips, valgrind-clean) and 11 in-process `ptr` unit-test assertions.

### Change — Compact struct layout (FFI Phase E3)

Regular `type` structs now use a **compact, natural-alignment field layout** instead
of the old "every field is an 8-byte `i64` slot". The 32-byte runtime header is
unchanged; fields start at offset 32, each at its natural size/alignment (bool/char
1, i32 4, i64/f64/heap-ptr 8), and are **reordered into three alignment buckets
(8 ++ 4 ++ 1, stable within each)** so padding is minimised — e.g. `{a bool, b i64,
c i32}` drops from 24 to 16 bytes of field space. The synthetic `__vtable` ptr stays
at offset 32, so object dispatch is unaffected.

Pure codegen change — no syntax or semantics change, identical program output. New
`reorderSuruFields`/`suruFieldOffset`/`suruStructSize` helpers in `irStructCodegen.suru`
(reusing the E2 C-ABI geometry, based at offset 32); `irRegisterPasses.suru` reorders
non-`cType` `TypeDecl.fields` at registration so every consumer agrees; field
access/assign and `emitStructLit` store at the field's natural LLVM type; clone/drop
gain a natural-width `copyScalarField` and walk the reordered offsets (heap fields
keep the recursive 8-byte clone/drop). Covered by the `struct-layout` fixture and 22
in-process `struct-layout` unit-test assertions; bootstrap holds the self-hosting
fixed point (C2 == C3). `cType` (header-less, order-preserving) is unchanged.

### Add — `cType` declarations + C ABI layout (FFI Phase E2)

New top-level `cType Foo: { ... }` declaration for header-less, C-ABI-laid-out
structs, the foundation for passing structured data to C functions by pointer.
Unlike a regular `type` (32-byte runtime header, every field an 8-byte `i64`
slot), a `cType`:

- has **no runtime header** — field 0 starts at offset 0;
- uses **natural C alignment** with padding between misaligned fields, total size
  rounded up to the max field alignment (x86_64 sizes: bool/char 1, i32 4,
  i64/f64 8); **field order is preserved** (the C ABI is order-sensitive);
- is **erased to a raw C pointer** at the `extern fn` boundary, so a `cType` value
  passes straight to libc functions (e.g. `memcpy`, `gettimeofday`);
- is **managed manually** — it has no type_tag, so `clone()`/`drop()` are rejected
  and its memory is freed via an `extern fn free`.

Fields are limited to fixed-layout primitives (`bool`, `char`, `i32`, `i64`,
`f64`); `String`/`Array`/named/nested-`cType` fields, methods on a `cType`, and a
`cType` used as a field of a regular type are all compile errors. Implementation:
`TOK_CTYPE` keyword + `parseCTypeDeclaration` (lexer/parser), `isCType` threaded
through `TypeDeclNode`/`SemTypeEntry`/`TypeDecl`, the rejections in
`semantic/passes.suru`/`resolveTypes.suru`/`exprs.suru`, and the natural-alignment
geometry (`cFieldSize`/`cFieldAlign`/`cAlignUp`/`cFieldOffset`/`cStructSize`) +
header-less alloc/access/assign in `codegen/irStructCodegen.suru`/`irCodegen.suru`.
Covered by the `ctype-struct` fixture (memcpy round-trip, valgrind-clean), four
`ctype-*-error` diagnostic fixtures, and the in-process `ctype-layout` unit tests.
Self-hosting fixed point (C2 == C3) holds.

Two supporting codegen fixes landed alongside: a `void`-returning `extern fn`
(e.g. `free`) is now called with no SSA result (`call void @free(...)`) instead of
emitting invalid `%t = call void`; and `toString()` on a narrower integer scalar
(`i32`/`char`/`bool`, e.g. a `cType` field) now widens to `i64` before the
`suru_int64_to_string` call.

### Remove — deprecated LLVM IR runtime files (FFI Phase A5)

Deleted the five hand-written `runtime/*.ll` files (`array`/`box`/`string`/`struct`/`variant`),
superseded by the C runtime (`runtime/*.c`) linked since Phase A4. Documentation (CLAUDE.md,
README.md) and the stale `.ll` comments in `irArrayCodegen.suru`/`irStringCodegen.suru` now
reference the C runtime. No build, output, or IR change — the self-hosting fixed point (C2 ==
C3) is unaffected. This completes Phase A of the FFI plan.

### Change — runtime linked from C objects instead of LLVM IR (FFI Phase A complete)

Compiled programs now link against the C runtime (`runtime/*.c`) instead of the
hand-written LLVM IR (`runtime/*.ll`). This completes Phase A of the FFI plan:
tasks A1–A3 had already ported `box`/`struct`/`array`/`string` to C; A4 adds the
final file and flips the link step. No language- or output-visible change — the
emitted IR is byte-for-byte identical, so the self-hosting fixed point (C2 == C3)
holds unchanged.

- New `runtime/variant.c`: 1:1 port of `variant.ll`
  (`suru_variant_create`/`_tag`/`_inner`/`_drop`) over the shared `SuruHeader`
  layout in `runtime/suru_runtime.h`. Verified symbol-identical to `variant.ll`.
- The Docker scripts (`scripts/test.sh`, `bootstrap.sh`, `build.sh`,
  `generate-expected.sh`) now compile `runtime/*.c` → `runtime/*.o` once per run
  (the runtime dir is bind-mounted, so objects are produced at container runtime).
- Both link sites (`src/compiler/pipeline.suru` `compilePipeline`,
  `tests/runner/main.suru`) glob `runtime/*.o` instead of `runtime/*.ll`.
- `runtime/*.ll` files are kept (marked `DEPRECATED`, no longer linked) and will
  be deleted in task A5; `runtime/*.o` is gitignored.

### Add — hexadecimal integer literals

Integer literals can now be written in hex with a `0x` / `0X` prefix
(`0xFF`, `0X10`, `0xDEAD`, mixed case `0xAbCd`). A hex literal lexes to an
ordinary decimal `TOK_INT`, so it is usable anywhere an integer literal is and
type inference into `char` / `i32` / `i64` works unchanged. A bare `0x` with no
following hex digits is a fatal lex error with a caret diagnostic.

- New `src/compiler/lexer/lexerHex.suru` (`Suru.Compiler.Lexer.Hex`): pure,
  unit-testable helpers `isHexDigit`, `hexCharToVal`, `hexDigitsToI64`.
- `src/compiler/lexer/lexer.suru`: `readNumber` branches to a new `readHexLiteral`
  method on the `0x`/`0X` prefix; the decoded value is emitted via
  `value.toString()` so no downstream stage changes.
- New in-process unit-test harness: `tests/unit/assert.suru` (shared
  `TestResult` + `assertEqI64`/`assertTrue`/`assertEqStr`/`reportResults`) and
  `tests/unit/compiler/lexer_test.suru` (`lexerTests`), run by the test runner
  before the fixtures. A dedicated `tests/.suruproject` exposes the source roots
  to the tests tree.

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
