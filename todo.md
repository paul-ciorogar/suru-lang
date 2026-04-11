# Code Generation Preparations - Implementation Roadmap

This pass sits between semantic analysis and LLVM IR generation.
It produces a **lowered AST** with:
- Generic functions replaced by concrete type specializations (monomorphization)
- Functions that take heap params split into ref and owned specializations
- Explicit `drop()` calls inserted at ownership end-of-life
- Explicit `copy()` calls inserted where sharing requires copying

Module location: `src/lower/`

---

## Phase 0: Mutation Analysis — Semantic Annotation (1-2 hours)

**Prerequisite semantic analysis step.** While traversing function bodies, annotate each
function symbol with which of its parameters it mutates. The lowering pass reads these
flags instead of re-traversing bodies.

- [ ] Add `mutates_params: Vec<bool>` field to `FunctionSymbol` in `src/semantic/` (alternative consider using bit flags and limit the parameters to 64)
  - One `bool` per parameter, in declaration order
  - Default: all `false` until body is analyzed
- [ ] In `visit_function_decl`, after visiting the body, compute mutation per param:
  - A param is mutated if the body contains a field assignment on it (`param.field: value`)
  - A param is mutated if the body calls a method on it that is itself marked as mutating
  - Resolve transitive calls via the symbol table (already-analyzed callees)
- [ ] Store the result in the function's `FunctionSymbol` entry in the symbol table
- [ ] Write tests:
  - Function that only reads param fields → all `false`
  - Function with `param.name: newName` → that param `true`
  - Function that calls a mutating method on param → that param `true`
  - Transitive: function calls another function that mutates the param → `true`

---

## Phase 1: Lowered AST Infrastructure (1-2 hours)

Define the data structures the lowering pass produces.

- [ ] Create `src/lower/mod.rs` with public API skeleton
- [ ] Define `LoweredProgram` struct (top-level container)
- [ ] Define `LoweredFunction` (mangled name, params with `PassMode`, return type, body)
- [ ] Define `LoweredStatement` enum:
  - `VarDecl(name, expr, is_heap: bool)`
  - `Drop(name)` — inserted by compiler
  - `Assign(name, expr)`
  - `ExprStmt(expr)`
  - `Return(Option<expr>)`
- [ ] Define `LoweredExpr` enum:
  - `Literal`, `Identifier`, `Call`, `MethodCall`, `FieldAccess`
  - `Copy(Box<LoweredExpr>)` — inserted by compiler
  - `BoolOp`, `Not`
- [ ] Define `PassMode` enum: `ByRef`, `ByOwnership`
- [ ] Define `LoweredParam(name, type, pass_mode: PassMode)`
- [ ] Write unit tests for constructing a minimal `LoweredProgram`

---

## Phase 2: Specialization Key Design (1-2 hours)

Both generic instantiation and ref/own variants are specializations of the same function.
Unify them under a single key so Phase 3 can handle both in one pass.

- [ ] Create `src/lower/specialization.rs`
- [ ] Define `SpecKey` struct:
  - `base_name: String` — original function name
  - `type_args: Vec<Type>` — empty for non-generic functions
  - `pass_modes: Vec<PassMode>` — one entry per heap parameter (question? using a bit flag where 1: reference and 0: owned and limit parameters to 64. Would this be a better solution?
- [ ] Implement `mangled_name(key: &SpecKey) -> String`:
  - Example: `adder<I32>` → `adder__I32`
  - Example: `printMessage(ByRef)` → `printMessage__ref` (if the bit flag is implemented then this could be `printMessage__1`)
  - Example: `printMessage(ByOwnership)` → `printMessage__own` (if the bit flag is implemented then this could be `printMessage__0`)
  - Example: combined: `process<String>(ByRef, ByOwnership)` → `process__String__ref_own` (if the bit flag is implemented then this could be `process__String__10`)
- [ ] Implement `SpecKey` equality and hashing (for deduplication)
- [ ] Write tests:
  - Same types + same modes → one key
  - Different type args → different keys
  - Ref vs. owned variant → different keys

---

## Phase 3: Specialization Collection (2-3 hours)

Walk the semantic AST and collect every distinct `SpecKey` needed.

- [ ] Create `src/lower/collect.rs`
- [ ] Implement `collect_specializations(ast, type_info) -> HashSet<SpecKey>`
  - Visit every `FunctionCall` and `MethodCall`
  - For generic callees: read resolved type arguments from `type_info`
  - For each heap parameter: determine whether the argument is its last use in scope
    - Last use → `ByOwnership`; still live after → `ByRef`
  - For non-generic, non-heap-param functions: emit a single key with empty vecs (consider not generating a key if we don't have to. Only have Specialization where specialization is needed)
- [ ] Write tests:
  - `adder(3i32, 7i32)` + `adder(3i64, 7i64)` → two `SpecKey`s
  - `printMessage(message)` used twice → `ByRef` key; last use → `ByOwnership` key
  - Non-generic, stack-only function → single key (no modes)

---

## Phase 4: Function Specialization (2-4 hours)

For each collected `SpecKey`, produce a concrete `LoweredFunction`.

- [ ] Implement `specialize(func_decl, key, type_info) -> LoweredFunction` in `src/lower/specialization.rs`
  - Clone the function's statement list
  - Substitute type parameters using `key.type_args`
  - Annotate each heap param's `PassMode` from `key.pass_modes`
  - Set `mangled_name` from `mangled_name(&key)`
- [ ] Rewrite call sites: replace original function name with mangled name, passing the right `SpecKey`
- [ ] Exclude original generic / unspecialized definitions from the lowered output
- [ ] Write tests:
  - Specialized function has correct concrete param types
  - `__ref` variant has `PassMode::ByRef`; `__own` has `PassMode::ByOwnership`
  - Call site references mangled name
  - Original definition absent from output

---

## Phase 5: Heap vs. Stack Classification (1-2 hours)

Determine which values live on the heap; needed by phases 3, 6, and 7.

- [ ] Create `src/lower/heap_analysis.rs`
- [ ] Implement `is_heap_type(ty: &Type) -> bool`
  - Stack: primitive scalars (Number, Bool, Int8–UInt64, Float32, Float64, unit types)
  - Heap: String, Struct, intersection types, union types containing any heap member
- [ ] Annotate each `LoweredParam` and `VarDecl` with `is_heap`
- [ ] Write tests:
  - `Number` → stack; `String` → heap; custom struct → heap
  - Union of stack types → stack; union containing `String` → heap

---

## Phase 6: Liveness Analysis (2-4 hours)

Find the last-use point of each variable within its scope.
Required for deciding ownership transfer, drop placement, and clone insertion.

- [ ] Create `src/lower/liveness.rs`
- [ ] Define `LivenessMap { last_use: HashMap<String, StatementIndex> }`
- [ ] Implement `compute_liveness(stmts) -> LivenessMap`
  - Forward scan: each read of a variable updates `last_use`
  - A call that takes ownership counts as a use and ends liveness
- [ ] Implement `is_last_use(var, stmt_idx, liveness) -> bool`
- [ ] Write tests:
  - Variable used once → last_use at that statement
  - Variable used twice → last_use at second use
  - Variable passed to function and never read again → last_use is that call

---

## Phase 7: Drop Insertion (2-3 hours)

Insert `LoweredStatement::Drop(name)` so no heap value leaks.

- [ ] Create `src/lower/drop_insertion.rs`
- [ ] Implement `insert_drops(block, liveness, heap_info) -> Vec<LoweredStatement>`
  - At end of scope insert `Drop` for every owned heap variable NOT moved
  - After a `ByOwnership` call, mark the argument as moved — no `Drop` in caller
  - Return values transfer ownership — no `Drop` on returned value
- [ ] Handle function params: heap param with `ByOwnership` and not returned → `Drop` at end
- [ ] Write tests:
  - `text: "hello"` unused after decl → `Drop(text)` at end of block
  - `makeSomething(greeting)` passes ownership → no `Drop(greeting)` in caller
  - `circle: makeCircle()` not passed anywhere → `Drop(circle)` at end
  - Returned value → no `Drop`

---

## Phase 8: Copy Insertion (2-3 hours)

Insert `LoweredExpr::Copy(...)` when a value must be copied before passing.

- [ ] Create `src/lower/copy.rs`
- [ ] Implement `insert_copies(block, liveness, type_info) -> Vec<LoweredStatement>`
  - At each call site: if argument is heap, call takes `ByOwnership`, AND variable is still live → wrap arg in `Copy(...)`
  - If it is the last use → no clone, ownership transferred
  - Struct field init: if source variable is still live after this field assignment → `Copy`
- [ ] Write tests:
  - First `changeAndPrint(message)` when `message` used again → `Copy(message)` inserted
  - Second `changeAndPrint(message)` at last use → no clone
  - `name: theName` twice in two structs → second gets `Copy(theName)`
  - `extractName(person)` returning `person.name` → copy the field value

---

## Phase 9: Lowering Pipeline Integration (2-3 hours)

Wire all passes together and expose through the compiler.

- [ ] Implement `lower(ast, semantic_info) -> Result<LoweredProgram, LoweringError>` in `src/lower/mod.rs`
  - Step 1: classify heap vs. stack (Phase 5)
  - Step 2: collect specialization keys (Phase 3)
  - Step 3: specialize functions — both generics and ref/own variants (Phase 4)
  - Step 4: compute liveness per function (Phase 6)
  - Step 5: insert drops (Phase 7)
  - Step 6: insert clones (Phase 8)
- [ ] Define `LoweringError` with `Display`
- [ ] Add `suru lower <file>` CLI subcommand that prints a debug dump of the lowered AST
- [ ] Wire into existing pipeline: `lex → parse → semantic check → lower`
- [ ] Write integration tests using programs from `code_generation_preparations.md`

---

## Notes

- Stack values (primitives) never get `Drop` or `Copy` — they copy freely
- `ByRef` variant: LLVM pointer arg (`*T`), no drop of parameter at end
- `ByOwnership` variant: LLVM value arg (`T`), drop parameter at end if not returned/moved
- Liveness is per-scope; variables declared in inner blocks are dropped at the inner block's end
- Phases 5–8 can be developed and tested independently before integration in Phase 9
