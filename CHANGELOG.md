# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### String split B3 — ownership is a property of the slot, decided statically

B2 gave a literal its own type (`Str`); B3 makes the codegen act on it. A borrowed
`Str` is now **materialized into an owned heap copy wherever it enters an owned
slot**, and `drop` is decided from the static type alone — no runtime tag read on
any path the compiler emits.

- **Materialization sites** (`irStr.materializeStr`, `codegen/irStringCodegen.suru`):
  object-literal fields, field assignment, array literals, `arr.add`/`arr.set`, and
  `String`-annotated locals. Call **arguments** and **returns** are deliberately not
  materialized: a parameter is borrowed and never auto-dropped, so a copy made there
  would be freed by nobody.
- **`drop` and `__append` decide statically.** `drop(x)` on a `Str` emits no call at
  all (`emitDropDyn`, `irExprHelpers.suru`), and the in-place `__append` only frees
  its old receiver when that receiver is owned. Both were previously runtime no-ops
  that depended on `suru_string_drop` inspecting `type_tag == 8`.
- **Ownership is fixed at the declaration, not tracked through the value.** The
  first cut of B3 registered a local under the ownership of whatever value it
  currently held, promoting it on assignment. That is unsound: codegen walks the
  statement list once while control flow does not, so in

      let s String: ""
      while k.lt(3) { let tmp String: s.append("x")   drop(s)   s: tmp }

  the `drop(s)` was emitted once, as a no-op, and executed three times — leaking one
  string per trip around the back edge (`while-loop` and `cli-lex` both leaked under
  valgrind). A `String` local is now an owned slot for its whole lifetime whenever
  the function can ever store an owned value into it, which the new pure analysis in
  **`src/compiler/codegen/irOwnedLocals.suru`** (`collectMutatedLocals`) answers:
  the local is an assignment target, or the root of an in-place `__append` chain
  (`buf.__append(x)` writes the fresh string back into buf's slot). An owned local
  materializes its literal initializer once, at the `let`; a local that is never
  mutated — the overwhelmingly common `let day String: "Monday"` — keeps its
  initializer's ownership, allocates nothing, and still drops to a no-op. Both
  directions of imprecision are memory-safe (a missed mutation leaks, an extra one
  costs a clone); neither can free `.rodata`.
- **A latent double free surfaced and was fixed.** `substParam`
  (`semantic/mono/monoSubst.suru`) built the instantiated `Param` with
  `name: p.name`, aliasing one string across two independently-owned records. While
  every such string was a literal, both drops were runtime no-ops; with
  materialization they became a real double free (`variantInstantiationTest`, which
  drops both the registry and the instantiated nodes, aborted the bootstrap on it).
  The name is cloned now.
- **The `type_tag == 8` net in `runtime/string.c` STAYS**, and the header comment
  there now says why. Removing it was the point of B3, and it is genuinely closer —
  every declared owned slot is materialized — but two boundaries can still put a
  borrowed pointer behind the static type `String`: a function whose declared return
  type is `String` returning a literal, and a `String` parameter that the callee
  stores into an owned slot (`makeToken(text String, …) Token`). Deleting the check
  with those open does not fail at one site: a full suite run with it removed
  produced 29 invalid-free contexts across the lexer, the resolver and mono. Closing
  those two boundaries is the next step.
- Coverage: new `tests/unit/compiler/ownedLocalsTest.suru` (14 assertions — both
  mutation forms, nested loop/if bodies, the functional `.append` that must NOT
  count, and non-local `__append` roots); `tests/fixtures/str-type/` gains the
  ownership-transition cases (loop-carried builder, in-place `__append` builder,
  literal into a `String` local), where valgrind is the real assertion — a missing
  copy is an invalid free, a missing drop a leak. Full suite green (106 passed,
  0 failed, 0 memcheck); bootstrap fixed point confirmed (C2 == C3).

### String split B2 — a string literal is typed `Str`, not `String`

`Str` (the borrowed static-string type) has existed as a full string-family member
since B1, but literals were never given it: `resolveTypes.suru` annotated every
`StrLitNode` as `String`, so the type system could not tell a `.rodata` global from
an owned heap allocation. It is the reason `suru_string_drop` still peeks at
`type_tag == 8` to decide whether freeing is safe — the last `type_tag` **read** in
the codebase, and the blocker for Step 2 (shrinking the 32-byte header).

- **Literals now infer as `Str`** in both inference paths, which must agree:
  `resolveTypes.suru`'s `StrLitNode` arm (the `resolvedType` codegen reads) and
  `exprs.inferType` (the value-flow mismatch checks). `emitStringLiteralValue`
  reports `Str` as the result's suruType to match.
- **`x.clone()` on a `Str` resolves to `String`.** `clone()` is the explicit
  "give me an owned copy" request and is now the one method whose result type is
  not simply the receiver's — `suru_string_clone` mallocs a fresh tag-6 header and
  buffer, so typing the result `Str` would have called a heap allocation borrowed.
- **Owned slots normalize `Str` away.** Two places infer a type *from a value* and
  store the answer in a slot that participates in deep drop, so both had to be
  taught that a borrow cannot be the slot's type:
  - `emitArrayLit` took the array's element type from element 0, so
    `let a Array<String>: ["x"]` would have recorded `elem_tag` 8, type `Array:Str`
    and a `@suru_array_clone_Str` helper — none of which match the declared type.
    New `irArr.ownedElemSuruType` normalizes it.
  - Type-argument inference reads the *argument's* type, so `res.err("oops")` bound
    `E := Str` and asked monomorphization for `err-Str` / `Result-i64-Str`,
    instantiations no annotation ever names; the mangled call name and the mangled
    declaration diverged and the build failed. Normalized at `rtAppendBinding`, the
    single funnel through which every solved binding becomes a recorded type
    argument, so return-vs-expected and param-vs-arg agree.
- The `type_tag == 8` net in `runtime/string.c` is **deliberately untouched**: a
  `Str` still reaches owned slots un-materialized, and the net is what keeps that
  safe. Removing it is B3, together with materializing `Str` → `String` at the
  ownership boundaries.
- Coverage: new `tests/unit/compiler/strLiteralTypeTest.suru` (15 assertions —
  literal inference, the family relation in both directions and its two rejections,
  owned-element normalization per shape, and `Str`'s round-trip through the codegen
  type helpers); `tests/fixtures/str-type/` gains the owned-slot cases (array
  literal + `add` + read-back + drop, and a literal bound to a `String` slot).
  Full suite green (106 passed, 0 failed, 0 memcheck); bootstrap fixed point
  confirmed (C2 == C3).

### §3.4 — `isAssignable` compares `Type` values, and the last duplicated mangler is gone

The two string-surgery hubs left in the semantic layer after monomorphization was
cleaned out. Both now parse a type once at the boundary and ask the structure.

- **`isAssignable` moved to `src/compiler/semantic/assignable.suru`** and compares
  structured types. It used to canonicalize *both sides to strings* through
  `tyh.makeSuruType` and `.equals` them; it now parses both, canonicalizes with the new
  `canonType`, and compares with `equalType`. The String/integer family predicates moved
  with it and match on the `Prim` name of the canonical type instead of a raw string.
  Every documented rule is unchanged — unknown (`""`) on either side still skips, the sum
  variant relation is still accepted in **both** directions (the compiler's own typed-bridge
  downcast depends on the sum→variant one), `f64` is still outside the integer family. The
  sum-type queries stay string-keyed; re-keying the registries is §4.B.
- **`canonType` (new, `src/compiler/types/typeOps.suru`)** is what makes that comparison
  correct. `parseType` deliberately does not read the hyphen-mangled form, so after mono a
  slot's annotation (`Box<i64>` → `Named`) and the instantiated declaration it denotes
  (`Box-i64` → `Prim`) are structurally different values for one type. `canonType` folds a
  generic *application* into the `Prim` naming its instantiation (via `printSymbol`),
  recursing into `Arr` elements but never folding the `Arr` itself — the built-in array is
  the one generic codegen still spells positionally (`Array:i64`), and folding it would make
  `Array<i64>` equal to a user type literally named `Array-i64`. It is
  `printSuruType`-preserving, which is the argument that no accepted pair changed: the
  string-keyed registries downstream still get exactly what `makeSuruType` produced.
- **`semantic/passes.suru` lost its copy of the mono mangler.** `indexOfAngle`, `ltrimStr`,
  `flattenPassTypeArg`, `appendMangledArgsAt` and the hand-rolled `mangleAnnotation` /
  `isUserGenericAnnotation` / `isArrayGeneric` / `normalizeGenericTypeName` bodies (~90
  lines, a verbatim duplicate kept alive only because this file sits on a different include
  chain) are `parseType` + `printSymbol` + shape tests now.
- Two latent bugs died with it. `isBuiltinType` accepted **any** name longer than five
  characters beginning with "Array", so the AST node `ArrayLitNode` was classified as the
  built-in array; a shape test on the parsed type cannot make that mistake.
  `normalizeGenericTypeName` mangled the outer application only, so `Option<Array<i64>>`
  became `Option-Array<i64>` — a name the mono pass never emits; `printSymbol` flattens
  every level.
- File sizes: `passes.suru` 516 → 477, `exprs.suru` 564 → 484, `typeOps.suru` 485 — all back
  under the 500-line limit. Getting `exprs.suru` there moved the pure AST accessor
  `nodeResolvedType` to its own leaf file `semantic/nodeType.suru` (no state, no inference,
  no imports beyond the parser AST).
- The lost design document is reconstructed as `semantic_analysis_architecture_rd.md`,
  which six source files and several tests cite by section number. Sections §0–§3.4 describe
  what landed; §4.B (registries keyed by `Type`) and §4.C (fresh inference variables at call
  sites) are marked open.
- Coverage: `assignability_test.suru` gains the applied↔mangled pairs in both directions
  (`Box<i64>` ↔ `Box-i64`, `Pair<i64, String>` ↔ `Pair-i64-String`, `Option<Array<i64>>` ↔
  `Option-Array-i64`, `Array<i64>` ↔ `Array:i64`) plus the rejections that must survive the
  duality (differing argument, differing argument *order*, differing base name);
  `typeOpsTest.suru` covers `canonType` per shape and pins the `printSuruType`-preservation
  invariant against `makeSuruType`; new `passesTypeNameTest.suru` pins `isBuiltinType`,
  `normalizeGenericTypeName` (including the fixed nesting) and `resolveTypeName`. Full suite
  green (106 passed, 0 failed, 0 memcheck); bootstrap fixed point confirmed (C2 == C3).

### `typeArgs` are the only binding: the syntactic mono fallbacks are gone

Closes the phase the previous entry opened. Monomorphization's two halves — what gets
instantiated and what each call site is renamed to — now read the **same** structured
`typeArgs` the resolver recorded, with no second, syntactic derivation anywhere.

- `monoPass.rewriteCallName` is reduced to `mangleFromTypeArgs`: a call with recorded
  `typeArgs` is renamed to the mangled name built from them, and a call without them keeps
  its name. The arg-binding fallback (`collectArgTypes` / `buildConcreteTypes` / `allBound`)
  is deleted. A missing rename is now, by construction, a resolver bug — the binding belongs
  recorded in `rtCallUnify` / `rtMethodUnify` / `rtRecordArgBindings`, not re-derived here.
- The two annotation-driven zero-arg fallbacks are deleted with it: `rewriteZeroArgCallName`
  (+ the `rewriteCallInExprWithAnnotation` wrapper that threaded the `let` annotation down to
  it) and `rewriteZeroArgFnFromAnnotation` (the registry scan in the `LetNode` arm, added as a
  workaround for a bootstrap binary whose match arm did not fire — no longer needed). The
  `LetNode` arm is now a plain `rewriteCallsInExpr`. `let arr List<i64>: newList()` is renamed
  through the resolver's return-vs-annotation unification like every other call.
- `monoInfer`'s "retained helpers" section goes with them (`extractBaseTypeName`,
  `suruCanonToAnnotation`, `collectArgTypes`, `findBindingForVar`, `buildConcreteTypes`,
  `allBound`, `matchReturnTypeSingle`) — the last string-surgery type matching in the pass.
- Coverage: the pinned assertions in `tests/unit/compiler/monoCallSiteRewriteTest.suru` (both
  the `let`-annotation and object-literal-field binding paths) were written for exactly this
  removal and still assert `newList-i64`; the Phase-0 oracle `monoInstantiationTest.suru` is
  unchanged. Full suite green (106 passed, 0 failed, 0 memcheck) after each step; bootstrap
  fixed point confirmed (C2 == C3), so the emitted IR is byte-identical.

### `instantiateVariantTypes` on the structured `Type` (and a depth-blind comma bug)

- The generic sum type's variant-arm instantiation (`monoInstantiate.suru`) was the last
  hand-rolled `indexOfLT` / `slice` / `splitByComma` re-parse in the mono layer. It now
  substitutes the arm, parses it once (`parseType`), and reads the base name and arguments off
  the resulting `Named` (new `instantiateVariantFromType`) — the same boundary pattern as
  `collectGenericAppFromType`.
- This fixes a real bug: a variant arm whose type argument is itself multi-arg
  (`Some<Pair<i64, String>>`) was split at the **inner** comma, yielding the two bogus
  arguments `"Pair<i64"` / `" String>"` and a mangled variant name no call site could match.
- With that, `subst.indexOfLT` / `subst.splitByComma` / `subst.trimLeft` have no callers left
  anywhere and are deleted: **no string surgery remains in `src/compiler/semantic/mono/`.**
- Coverage: new `tests/unit/compiler/variantInstantiationTest.suru` drives `instantiateAll`
  against a hand-built `Option<T>` / `Some<T>` registry and pins the produced declaration names
  for the single-arg, nested, and multi-arg-argument cases.

### `monoPass.suru` split under the 500-line limit

- The self-contained variant-arm rewriting section moved to
  `src/compiler/semantic/mono/monoVariantArms.suru` (namespace
  `Suru.Compiler.Semantic.Mono.VariantArms`); the three `pipeline.suru` call sites are
  repointed. `monoPass.suru` 663 → 417 lines, `monoInfer.suru` 503 → 407,
  `monoSubst.suru` 469 → 431. No behaviour change.

### The generic call-site rewriter reads `typeArgs` (and `typeArgs` are now substituted)

- `monoPass.rewriteCallName` now prefers the resolver-recorded structured `typeArgs` over the
  syntactic argument-binding helpers, via a new `mangleFromTypeArgs` that mirrors monoInfer's
  `requestFromTypeArgs` exactly (`printAnnotation` → `mangleGenericName`) so the rewritten call
  name is byte-identical to the instantiated definition's name. This closes the second half of
  the object-literal-field bug: the field value `{ items: newList() }` has neither a call
  argument nor a `let` annotation, so every syntactic binding arm returned the name unchanged
  even after monoInfer had correctly instantiated `newList-i64`. The syntactic helpers
  (`collectArgTypes` / `buildConcreteTypes` / `allBound`, and the annotation-driven
  `rewriteZeroArgCallName` / `rewriteZeroArgFnFromAnnotation`) are retained as a fallback for
  calls with no recorded `typeArgs`; removing them is a later phase.
- **`monoSubst` now substitutes `typeArgs` when cloning a generic body** (new `substTypeArgs`,
  wired into the `CallNode` / `MethodCallNode` / `ObjectLitNode` arms). They were previously
  propagated verbatim, which was harmless only while `typeArgs` were advisory. Inside a template
  the resolver records the enclosing type parameter itself — `List<T>.clone()`'s inner
  `let result List<T>: newList()` carries `typeArgs=[T]` — so once the rewriter reads them as
  authoritative, the `List-i64` copy asked for `newList-T`, a name that is never instantiated.
  This broke the `list-basic` / `list-string` / `list-toarray` fixtures. Implemented with the
  same three steps as `substituteType` minus the string boundary (`varizeType` → `substitute`),
  so an already-concrete argument is copied unchanged.
- Coverage: `tests/unit/compiler/monoCallSiteRewriteTest.suru` asserts the post-`mono` AST names
  the mangled callee at both binding sites — the `let`-annotation path (which the syntactic
  fallback already handled, pinned so a later removal cannot silently regress it) and the
  object-literal-field path, which only `typeArgs` can resolve. The existing
  `mono_instantiation_test.suru` oracle pins the complementary half (that `newList-i64` is
  emitted at all). Full suite green (106 passed, 0 failed, 0 memcheck); bootstrap fixed point
  confirmed (C2 == C3).

### Fixed: duplicate `isAssignable` left the compiler unable to build itself

- `semantic/exprs.suru` carried **two** `export fn isAssignable` definitions, so every build
  failed with `error: function 'isAssignable' is already declared`. They were merged into a
  single predicate that is the union of both rule sets — no call site can gain a false error
  relative to either — combining the newer version's integer family (`i64`/`i32`/`char`
  mutually assignable; `f64` still excluded) with the older version's unknown-type guard
  (`""` on either side → assignable), `makeSuruType` canonicalization of both sides, and the
  sum→variant direction that the compiler's own typed-bridge downcast idiom
  (`let n MatchStmtNode: someAstNode`) depends on. The unknown-type guard has to live inside the
  predicate because the field-assignment, object-literal-field and array-element sites call it
  ungated with raw `nodeResolvedType` output.
- Refreshed the two stale `semantic_type_mismatch_test.suru` assertions that still expected the
  pre-rewording let/assign diagnostics (`let binding type mismatch: expected …`); the messages
  are now `type mismatch: variable 'x' declared i64 but initialized with bool` and
  `… has type i64 but assigned bool`, matching the `let-type-mismatch` / `assign-type-mismatch`
  fixtures.

### `monoInfer` is now a mechanical collector (no type inference of its own)

- Monomorphization's instantiation collector (`monoInfer.suru`) no longer performs type-variable
  *inference*. It reads the structured `typeArgs` the resolver records on call / method / object
  nodes, and reads explicit generic annotations (`let` / concrete-fn param/return types) through
  the structured `collectGenericAppFromType` (parse → walk `Type`) instead of ad-hoc string
  surgery. The old unification arms (`inferFromGenericFnCall`, the zero-arg return-vs-annotation
  matchers, `inferFromAnnotation`) are gone.
- The inference they did moved into the resolver. **`rtRecordArgBindings`** adds the §2.2-deferred
  **param-vs-arg** unification: it binds a callee's type params by unifying each declared
  parameter-type template against the argument's resolved `Type`, recording the concrete
  `typeArgs` for monomorphization — *without* changing `resolvedType`, so emitted IR is unchanged
  (this is what covers `elemBytes<T>(dummy T) i64` and `printLn(identity("x"))`).
- `ObjectLitNode` gains a `typeArgs Array<Type>` field. The resolver (**`rtObjectLitRecord`**)
  copies a generic literal's concrete type args from the expected type (e.g. `Box<i64>` → `[i64]`),
  so a direct struct/sum literal with no constructor call is monomorphized from the recorded
  binding rather than the (mangled, previously no-op) `resolvedType` string.
- The syntactic helpers `collectArgTypes` / `buildConcreteTypes` / `allBound` /
  `matchReturnTypeSingle` / `extractBaseTypeName` are retained for now — the `monoPass` call-site
  rewriter still uses them; a later phase migrates that rewriter to `node.typeArgs` and removes
  them. Phase-0 oracle byte-identical; full suite green (104 passed, 0 memcheck); bootstrap fixed
  point confirmed (C2==C3). Coverage: `tests/unit/compiler/objectLitTypeArgsTest.suru` and
  `tests/unit/compiler/monoMechanicalTest.suru`.

### Monomorphization prefers recorded `typeArgs`

- Fixes the long-standing bug where a generic zero-arg constructor used as an object/struct
  literal *field value* (e.g. `type Container: { items List<i64> }` then `{ items: newList() }`)
  was never monomorphized — mono inferred type arguments syntactically from call *arguments*, so
  a no-arg call like `newList()` produced no instantiation and the program failed at compile
  time with `undefined function 'newList'`. `monoInfer` now reads the structured `typeArgs` the
  resolver already records on `CallNode`/`MethodCallNode` (the §2.2 resolver threads each field's
  declared type as the expected type via `rtResolveObjectFields`, binding `T=i64`), falling back
  to the syntactic arm only when no `typeArgs` were recorded. A new
  `requestFromTypeArgs(genericName, typeArgs, registry)` helper builds the `InstantiationRequest`
  directly (concrete types via `printAnnotation`); the existing fixed-point worklist
  (`instantiateAll`) then derives the transitively-needed `List-i64` from `newList-i64`'s return
  type, so no change to `monoInstantiate.suru` was needed. The working `let`-annotation case is
  byte-identical (the recorded request dedups against the annotation-arm request by mangled key);
  the self-hosting bootstrap fixed point is unchanged (C2==C3). Coverage: Phase-0 oracle
  `tests/unit/compiler/mono_instantiation_test.suru` — the pinned object-field assertions flip
  from "absent" to "present".

### Bind generic type args at the MethodCallNode arm 

- The resolver now binds a generic callee's type arguments for namespace-aliased calls written
  with method syntax (e.g. `import { opt: Suru.Stdlib.Option }` then `opt.some(42)`). After
  `importPass` rewrites the alias receiver into a `QualifiedNameNode` (left untyped by
  `resolveExpr`), such a call survives into `rtResolveMethodCallNode` as a `MethodCallNode`
  with an empty receiver type; when `methodName` names an unambiguous generic free function and
  a real expectation is present, the (varized) return template is unified against the expected
  `Type`, recording concrete `typeArgs` and a concrete `resolvedType` (e.g. `Option-i64`) on the
  node — the symmetric counterpart of the `CallNode` binding (§2.2). The §2.2 solver was
  refactored into a node-agnostic core `rtSolveReturnArgs` shared by the new `rtMethodUnify` and
  `rtCallUnify` wrappers (`semantic/rtCallUnify.suru`). Fails closed to the legacy method-table
  resolution, so the mono instantiation set (Phase-0 oracle) and the self-hosting bootstrap fixed
  point are unchanged. Coverage: `tests/unit/compiler/methodCallUnifyTypeArgsTest.suru`.

### `typeArgs` on call nodes

- `CallNode` and `MethodCallNode` (`parser/parserAst.suru`) gain a `typeArgs Array<Type>`
  field — the structured type arguments bound for a call to a generic callee. The parser
  initializes it to `[]`; node-cloning/rewriting passes (`importPass`, `qualNamePass`,
  `monoPass`, `monoSubst`) propagate it unchanged. This is the structural groundwork for
  Phase 2: the resolver's unifier will populate `typeArgs` (tasks 2.2/2.3) so
  monomorphization can read the concrete instantiation directly instead of re-deriving it
  syntactically. No behavior change yet; the Phase-0 oracle and the self-hosting bootstrap
  fixed point are unchanged. Coverage: `tests/unit/compiler/callNodeTypeArgsTest.suru`.

### Transitive instantiation re-parse on the structured `Type`

- `collectGenericAppFromType` (`semantic/mono/monoInstantiate.suru`), the transitive
  follow-up scanner that discovers further generic applications inside a freshly
  instantiated function's substituted annotations, is reimplemented over the structured
  `Type` instead of hand-rolled string surgery. It now parses the annotation once
  (`parseType`) and walks the resulting `Type` (new `collectGenericAppFromTypeVal`):
  it appends an `InstantiationRequest` for every `Named` whose base is a registry struct
  or sum type, recursing into type arguments / `Arr` elements inner-before-outer, with
  each request's `concreteTypes` rendered back to source form via `printAnnotation`. This
  removes the last `indexOfLT`/`slice`/`splitByComma`/`trimLeft` re-parse in the
  transitive path. The Phase-0 oracle and the self-hosting bootstrap fixed point are
  unchanged.
- Side benefit: a nested *multi-arg* type argument (`List<Pair<i64, String>>`) is now
  split at the correct angle-bracket depth — the old depth-blind `splitByComma` mangled
  the inner comma. (`instantiateVariantTypes` still does string surgery on generic
  variant templates; that is outside this change and left for a later phase.)
- Coverage: `tests/unit/compiler/collectGenericAppTest.suru` (the function is exported so
  the test can drive it against a hand-built `GenericRegistry`).

### Monomorphization type substitution + mangling on the structured `Type`

- `substituteType` (`semantic/mono/monoSubst.suru`) is reimplemented over the
  structured `Type` ops instead of hand-rolled string surgery: it now does
  `parseType` → `varizeType` → `substitute` → `printAnnotation`. `varizeType` is a
  new op in `types/typeOps.suru` that promotes type-parameter `Prim` leaves to `Var`
  (since `substitute` only rewrites `Var`). The output is normalized to source form
  (a colon-array input like `Array:i64` reprints as `Array<i64>`, and multi-arg
  generics print with `, `); every downstream consumer re-normalizes, so the mangled
  instantiation names — and the Phase-0 oracle — are byte-identical.
- `mangleGenericName` and `flattenTypeArg` (`semantic/mono/monoInstantiate.suru`) now
  route through `printSymbol` (`mangleGenericName` = `printSymbol(namedType(base, args))`,
  `flattenTypeArg` = `printSymbol ∘ parseType`). The redundant `mangleTypeExpr` helper
  is folded into `flattenTypeArg`.
- Fixed latent memory leaks in the `types/` layer that this change was the first to
  exercise under valgrind (the in-process type-ops unit tests are not leak-checked):
  `cloneType` never dropped its scratch `empty` bindings array (leaked on every `Var`
  substitution / clone), and `parseType`/`parseArgs` never dropped their intermediate
  slice and per-char `at()` temporaries. The test suite reports 0 memcheck failures.
- Coverage: `tests/unit/compiler/substituteTypeTest.suru`; verified by the Phase-0
  `mono-instantiation` oracle and the self-hosting bootstrap fixed point.

### Type-check imported module bodies (not just the root file)

- Pass-3 semantic analysis previously ran only over the **root** compilation
  file's own statements, so a type error inside an *imported* module
  (e.g. an `Array` literal assigned to a non-`Array` struct field) was never
  diagnosed and only surfaced as a runtime segfault. The pipeline now also runs
  the type-mismatch checks over imported module **function bodies**
  (`analyzeImportedFnBodies` in `pipeline.suru`; root-only namespace/export
  validation still runs on the root). Imported modules' top-level constants are
  registered into scope first so their bodies resolve.
- New compileError fixture `tests/fixtures/import-type-mismatch-error`: the
  offending `{ data: [0] }` (an `Array:i64` assigned to a non-`Array` field)
  lives in an imported `lib.suru`; the diagnostic is attributed to that file.
- Extending pass-3 coverage to the compiler's own source surfaced (and this
  change fixes) several latent type-checker gaps, all exercised by the
  self-hosting bootstrap:
  - `isAssignable` now canonicalizes both sides through `makeSuruType`, so
    variant→sum assignability works for **monomorphized generic** sum types
    (`Some<i64>` → `Option<i64>`), and also accepts the **sum→variant downcast**
    idiom the compiler relies on (`let n MatchStmtNode: someAstNode`).
  - A `match` **expression** over a sum type is now valid; its variant-tag arm
    patterns (bare identifiers) are no longer mis-analyzed as undefined
    variables.
  - Cross-module function **name collisions** (same simple name, different
    signatures in two namespaces) no longer produce spurious argument
    type-mismatch errors — ambiguous names skip arg typing.

### Compile error for calls to undefined functions

- A call to a name that is neither a builtin nor defined anywhere is now a
  compile-time error (`undefined function 'name'`) raised in the single semantic
  pass (`analyzeExpr` CallNode arm, `semantic/exprs.suru`). Previously such a
  call slipped through pass-3 silently and only failed later as an unresolved
  LLVM symbol at link time — e.g. a generic call like `newList()` that mono
  could not instantiate (template removed, no concrete copy) produced a confusing
  failure instead of a diagnostic. `FunctionRegistry.isDefined(name)` gates it:
  true on an exact match (normal/extern/imported fn, or a concrete fn called by
  its mangled name) or when a monomorphized instantiation (`name-T`) is
  registered, so generic call sites — which keep their base name through pass-3 —
  are not mis-flagged.
- Imported `extern fn` signatures (e.g. stdlib's `malloc`/`free`/`memcpy`) are
  now visible to the semantic function registry. `resolveIncludes` collects them
  into a **side channel** (`ResolveResult.externFns`) — deliberately *not* into
  the codegen statement list, since each module emits its own externs in its
  `{module}__mono.ll` and a second `declare` in the root IR would redefine them.
  `passes.registerImportedExterns` registers them after the pass-2 prepass so
  calls to them inside imported bodies resolve. A repeated extern declaration
  (several modules each declaring `extern fn malloc`) is now a silent skip rather
  than a duplicate-declaration error.
- Coverage: in-process unit test `undefined-fn` in
  `tests/unit/compiler/semantic_type_mismatch_test.suru`
  (+ fixture `tests/unit/compiler/fixtures/undefined_fn_call.suru`).

### `List<T>.toArray()` — bridge a List back to a built-in Array

- New method on `List<T>` (`src/stdlib/list.suru`): `fn toArray() Array<T>`
  returns a fresh built-in `Array<T>` holding a **deep-copied** snapshot of the
  list's elements. The receiver `List` is unchanged and still owned by the
  caller — each element is cloned individually (no-op value copy for scalars,
  `suru_clone_dyn` for heap `T`), exactly mirroring `clone`/`slice`.
- Purpose: enable an **incremental** migration from the built-in `Array<T>` to
  the stdlib `List<T>`. A function switched to return a `List<T>` can still feed
  call sites that have not yet migrated and expect an `Array<T>`.
- Coverage: new fixture `tests/fixtures/list-toarray` (scalar + heap elements,
  asserts the source list survives the call), registered in the runner.

### Type-mismatch diagnostics for wrong-typed values in typed slots

- The semantic analyzer now rejects, at compile time, several wrong-value-in-a-
  typed-slot cases that previously slipped through to a runtime crash:
  - `let x i64: <non-i64>` → `let binding type mismatch: expected i64, got bool`
  - `x: <wrong type>` → `assignment type mismatch: expected …, got …`
  - `obj.field: <wrong type>` → `field assignment type mismatch: expected …, got …`
  - object literal `{ f: <wrong type> }` → `struct literal field 'f' type mismatch: expected …, got …`
  - array literal element → `array element type mismatch: expected …, got …`
  - plain function call `f(<wrong type>)` → `argument type mismatch: expected …, got …`
    (previously only method-call syntax checked argument types).
- All checks (including the pre-existing argument/return/condition checks) now
  route through one assignability rule, `isAssignable` in `semantic/exprs.suru`:
  identical types, sum-type variant → base, and `String`/`Str` interop are
  accepted; it is the one place to extend for new legitimate subtyping. The
  *actual* type is read from the node's resolved type (`nodeResolvedType`), so
  context narrowing is respected — an integer literal under an `i32`/field/param
  expectation resolves to that type rather than spuriously reading as `i64`.
- Field-type lookup added to `AnalyzerState` (`lookupFieldType`).
- New in-process unit suite `tests/unit/compiler/semantic_type_mismatch_test.suru`
  (with snippet fixtures under `tests/unit/compiler/fixtures/`) drives each case
  through `runPipelineDebug(..., "semantic")` and asserts the exact message and a
  single-error count — covering all ten mismatch diagnostics.
- Fixed a latent typo surfaced by the new struct-literal check: `alloc-stress`
  declared a field as lowercase `string` (it holds a `String`).
### Semantic analyzer now type-checks `let` bindings and assignments

- A `let x T: expr` whose initializer type is not assignable to `T`, and an
  `x: expr` reassignment whose value type is not assignable to `x`'s declared
  type, are now compile errors ("type mismatch: variable 'x' declared T but
  initialized with U" / "… has type T but assigned U"). Previously neither site
  was checked: a `List<Token>` value bound to an `Array<Token>` slot (two
  distinct generic types both erased to `ptr`) compiled clean and **segfaulted**
  at runtime — the compiler failing its job of diagnosing a type-incorrect
  program. This closes that class of silent failure.
- The "can a value of type `actual` be bound to a slot of type `expected`?"
  predicate is now a single shared helper `isAssignable` (in
  `semantic/exprs.suru`): exact match, declared sum-type variant, the String
  family (`Str` ↔ `String`), or the integer family (`i64`/`i32`/`char` — integer
  literals infer as `i64` but validly initialize narrower slots; `f64` is
  excluded, as Suru has no implicit int↔float conversion). The two pre-existing
  checks (return statement, call arguments) were refactored to call it, so all
  value-flow sites share one definition.
- Checks gate on a *reliably* inferred initializer type via
  `inferTypeForMismatchCheck`, which returns "" (skip) for expression kinds whose
  type is a name-based heuristic that is wrong for user types — notably
  `MethodCallNode` (`toString → String` is correct for builtins but
  `StringBuilder.toString()` returns `SuruString`) and `MatchExprNode`. `inferType`
  itself (and the codegen `resolvedType` path) is untouched.
- New `assignability` in-process unit suite (`tests/unit/compiler/assignability_test.suru`)
  and `compileError` fixtures `let-type-mismatch` / `assign-type-mismatch` lock the
  behavior in. Front-end only (no codegen change); fixed point (C2 == C3) re-verified.
- Known remaining gaps (same predicate, not yet wired in): field assignment
  (`obj.f: expr`), object-literal fields (`{ f: expr }`), and match-arm result
  agreement — each needs field-type resolution and is a mechanical follow-up.

### `export` is now a declaration prefix (the `export { }` block is removed)

- A top-level declaration is made part of a module's public surface by writing
  `export` directly in front of it: `export fn parse(...)`, `export type Parser: { ... }`,
  `export let OP_NOT i64: 1`, etc. Applies to `fn`, `type`, `cType`, `let`,
  `extern fn`, and `extern type`. The standalone `export { name1, name2 }` header
  block no longer parses, and the unused aliasing form (`export { public: local }`)
  is gone — a declaration is always exported under its own name.
- The parser folds every `export`-prefixed declaration into a single synthetic
  `ExportNode` (via the new `declName` helper in `parser/parser.suru`), so the
  cross-module visibility machinery (`exportPass.suru`, the `pipeline.suru`
  export pre-scan, `importPass.suru`, codegen alias mapping) is untouched.
- New diagnostic: `export` before a non-declaration token → "expected a
  declaration after 'export'". With the block gone, the module-header ordering
  error simplifies to "import block must appear before declarations".
- All ~120 `.suru` source, stdlib, fixture, and unit-test files migrated to the
  prefix form. Fixtures `import-after-export-error` and `namespace-export-error`
  repurposed for the new diagnostics.
- Front-end only (no codegen change), rolled out across two bootstrap cycles
  (additive accept-both, then migrate-and-remove); fixed point (C2 == C3) re-verified.

### Private object methods (`_ fn`) are now callable via `this.method()`

- A private method (`_ fn name(...) Ret { ... }`) declared in an object literal can
  now be invoked via `this.name(...)` from sibling method bodies. Previously the call
  silently lowered to `suru_unbox_int64(null)` and segfaulted, because private methods
  lived only as literal fields and never entered the method lists that drive the
  lambda-lift + vtable dispatch.
- The fix mirrors the existing private-*field* handling: the semantic layer
  (`rtAugmentPrivateFields`, `src/compiler/semantic/resolveTypes.suru`) now injects each
  private method's signature into `SemTypeEntry.methods`, and the codegen layer
  (`augPrivAddMethod`, `src/compiler/pipeline.suru`) injects it into
  `TypeDeclNode.methods`. So `registerTypes` builds a vtable slot and `methodIndex`
  resolves it. Privacy is unchanged — `privateMethodNames` still rejects external calls.
- Public→private, private→private, and private-method-reads-private-field all work.
- New fixture `tests/fixtures/private-method` covers all three call shapes.
- **Codegen change** — bootstrap fixed point (C2 == C3) re-verified.

### `List<T>` growth: shrinking geometric factor (Go-style)

- `List<T>.add` (`src/stdlib/list.suru`) no longer doubles capacity unconditionally.
  It now doubles while small (`cap < 1024`, where the absolute over-allocation is
  negligible) and grows by 1.5x (`cap + cap/2`) once large. Growth stays **geometric**
  throughout, so append remains amortized O(1); the smaller large-array factor bounds
  wasted memory at ~50% instead of compounding to a full 2x. Mirrors Go's slice-growth
  strategy. (A switch to *linear* growth was considered and rejected — it would make
  filling a list O(n²).)
- The growth math lives in a private helper `_ fn computeGrowth(currentCap)` called
  from `add` (now that private methods are callable via `this.` — see above).
- `tests/fixtures/list-basic` extended to grow a list past 2000 elements, exercising
  the 1.5x branch and verifying (under valgrind) that `realloc` preserves earlier data
  across the threshold.

### Fixed module-header order: `namespace` → `import` → `export` → declarations

- The parser now enforces a single, fixed order for the module header. A module may
  declare **at most one `import` block**, and it must appear **before** the `export`
  block. New compile errors (file:line:col + caret, emitted from `parse()` in
  `src/compiler/parser/parser.suru`):
  - a second `import` block → `a module may declare at most one import block`
  - an `import` after the `export` block → `import block must appear before the export block`
  - an `export` after a declaration → `export block must appear before declarations`
- A single `import` block may list any number of entries (one per line or
  comma-separated), so multi-import modules consolidate into one block.
- All existing `.suru` sources (compiler, stdlib, tests, fixtures) were reordered to
  conform; bootstrap re-reached its C2 == C3 fixed point.
- New `compileError` fixtures: `tests/fixtures/multiple-import-error`,
  `tests/fixtures/import-after-export-error`.

### `Str` borrowed-string type introduced (String-split B1)

- **New builtin type `Str`** — the borrowed, static string-literal type, the first
  step of splitting Suru's single `String` into `Str` (borrowed, `.rodata`, drop is a
  no-op) vs `String` (owned heap). In B1, `Str` is recognized everywhere `String` is and
  shares the `%suru.String` layout and all read-only operations (`len`, `at`, `__at`,
  `slice`, `compare`, `equals`, match scrutinee, `printLn`). A `Str` value is assignable
  to a `String` parameter/return (and vice-versa) via a new string-family interop rule in
  the type checker. String literals are **still typed `String`** in B1 (the literal-typing
  flip + clone-on-transfer materialization is B2); `suru_string_drop`'s tag-8 static-literal
  check stays as a safety net through B1/B2.
- Central helper **`isStringType`** (`codegen/irCodegenTypeHelpers.suru`) and
  `isStringFamilyName` (`semantic/exprs.suru`) consolidate the ~15 previously-scattered
  `.equals("String")` checks so the two string types stay in sync. `suruTypeTag("Str")`=8,
  and `Str` is added to the builtin-type allow-lists (`passes.suru`, `exprs.suru`) and the
  match-scrutinee allow-list.
- New fixture `tests/fixtures/str-type/` exercises `Str` end-to-end (declaration, print,
  `len`/`__at`/`slice`, `Str`→`String` argument interop, `Str` parameter, match scrutinee,
  no-op drop) — valgrind clean. Codegen change — bootstrap fixed point reconfirmed
  (C2 == C3), full suite 99 passed.

### Static clone/drop/print dispatch — the runtime `type_tag` is no longer read (Step 1)

- **All clone/drop/print dispatch is now resolved at compile time from the static
  type, not from the runtime `type_tag` header word.** After monomorphization every
  type is statically known, so the codegen emits direct calls instead of routing
  through the type-erased `suru_clone_dyn`/`suru_drop_dyn`/`suru_println` dispatchers
  (which switched on `type_tag`). This is Step 1 of removing `type_tag` entirely — the
  32-byte header layout is unchanged (the tag is still written, just never read), so
  the change is bootstrap-safe.
- **How each kind dispatches now** (`heapCloneSym`/`heapDropSym` in
  `codegen/irCodegenTypeHelpers.suru`):
  - **struct / variant** → null-safe vtable trampolines `suru_clone_via_vtable` /
    `suru_drop_via_vtable` (`runtime/struct.c`): the struct/variant arm of the old
    dispatchers with the `type_tag` switch removed — they dispatch straight through
    the value's own `clone_fn`/`drop_fn` pointer, which is uniform for plain structs
    and variant arms (the concrete arm is only known per value).
  - **String** → `suru_string_clone` / `suru_string_drop` directly (now null-safe).
  - **Array<T>** → a per-element-type wrapper `@suru_array_clone_<T>` /
    `@suru_array_drop_<T>` (`linkonce_odr`, monomorphized like a C++ template) that
    calls the new `suru_array_clone_scalar`/`_heap` / `suru_array_drop_scalar`/`_heap`
    helpers (`runtime/array.c`). The element size / element clone-drop symbol is
    passed from the static call site, so the runtime no longer reads `elem_tag` to
    decide scalar-vs-heap. Nested `Array<Array<T>>` is handled by recursive wrapper
    emission.
  - **scalars** → bitcopy / no-op (unchanged).
- **`printLn`/`printError`** lower `String` to a direct `data`-buffer `puts` /
  `suru_printerror_lit`, and struct/array aggregates to a fixed `"<struct>"`/`"<array>"`
  placeholder — no `suru_println` (which read the tag). `suru_dyn_len` was already
  unreferenced (`.len()` dispatches statically).
- **Incidental fix:** `Array<char>` now clones correctly as a 1-byte scalar bitcopy
  (previously `char`'s `elem_tag` of 9 mis-routed it through the heap-element path).
  Also fixed `makeSuruType`/`extractElemType` for nested generic arrays
  (`Array<Array<i64>>` now canonicalizes to `Array:Array:i64`, previously the
  malformed `Array:i64>`). The `char` fixture gained `Array<char>` clone/drop coverage.
- The type-erased runtime functions (`suru_clone_dyn`, `suru_drop_dyn`,
  `suru_array_*_dyn`, `suru_println`, `suru_printerror`, `suru_dyn_len`) are kept
  defined for now so the transitional bootstrap binary still links; they are dead from
  the new codegen and will be deleted in Step 1b. Codegen change — bootstrap fixed
  point reconfirmed (C2 == C3), full suite + valgrind clean.

### `match` on a `SuruString` scrutinee

- **`match s { "foo": ... }` now works when `s` is a `SuruString`** (Blocker #5), not
  just a built-in `String`. The semantic scrutinee allow-list (`semantic/exprs.suru`)
  gained `SuruString`, and `emitPatternComparisons` (`codegen/irCodegen.suru`) keeps a
  `SuruString` scrutinee as an object pointer (skipping the named-type i64 unbox) and
  lowers each `"..."` literal arm to a vtable `scrutinee.equals(pattern)` call via the
  new `emitSuruStringEqualsPattern` helper (returns `i1` directly). The literal pattern
  stays a built-in `String`, so there is no per-arm `suruStringFrom`; `String`
  scrutinees still use the existing `@strcmp` path. New fixture
  `tests/fixtures/surustring-match/` covers matched/default arms, statement and
  expression forms, and the empty-string case (valgrind clean). Codegen change —
  bootstrap fixed point reconfirmed (C2 == C3).

### StringBuilder number appenders (`appendI64` / `appendF64`)

- **`StringBuilder` can now format numbers directly into its buffer** so migrated
  code can build strings from numbers without round-tripping through the built-in
  `String`. Two methods were added in `src/stdlib/stringBuilder.suru`:
  - `appendI64(n i64) void` — decimal `i64`. Pure Suru: digits are extracted with
    `split` (sdiv) / `multiply` / `take` (no modulo operator exists) and converted
    to bytes via `"0123456789".__at(d)`. Digit extraction runs in the non-positive
    domain so it is correct at `i64` min (which cannot be negated).
  - `appendF64(n f64) void` — fixed-point decimal with exactly 6 fractional digits
    (e.g. `1.5` → `1.500000`); `NaN` → `"nan"`. The formatting algorithm is pure
    Suru; only the `f64`↔`i64` cast is delegated to FFI.
- **Runtime cast bridge** (`runtime/string.c`): `suru_f64_to_i64` (truncate toward
  zero) and `suru_i64_to_f64` (widen). Suru has no `fptosi`/`sitofp` primitive, so
  `appendF64` splits a float into integer/fractional parts through these. Declared
  as `extern fn` in the stdlib — no codegen change, so no bootstrap is required.
- Tests: `tests/unit/stdlib/string_builder_num_test.suru` (new), covering `0`,
  positives, negatives, `i64` max/min, fixed-fraction floats, and the NaN guard.

### SuruString ordering (`compare` / `compareStr`)

- **`SuruString` now supports three-way ordering** so migrated code can sort and
  compare strings. Two methods were added in `src/stdlib/string.suru`:
  - `compare(other String) i64` — order against a built-in `String`.
  - `compareStr(other SuruString) i64` — order against another `SuruString`.
- Both follow `strcmp` sign semantics: a negative result when the receiver sorts
  first, `0` when equal, positive otherwise (only the sign is meaningful). They
  scan the common prefix (up to the shorter length) byte-by-byte via `ptr.load`,
  then break ties by length. Comparison is over signed byte values, which matches
  `strcmp` for ASCII (the intended use). Stdlib-only — no codegen change.
- Tests: `tests/unit/stdlib/string_test.suru` (new), covering less/equal/greater,
  prefix tie-breaks, and empty strings for both methods.

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
