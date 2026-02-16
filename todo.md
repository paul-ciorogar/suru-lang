# Semantic Analysis Implementation Roadmap

## Phase 1: Foundation - Symbol Tables and Scopes

### 1.1 Basic Symbol Table Infrastructure
- [x] Create `src/semantic/mod.rs` module
- [x] Define `Symbol` struct (name, type, kind: variable/function/type)
- [x] Define `SymbolTable` struct with HashMap storage
- [x] Implement basic insert/lookup methods
- [x] Write tests for symbol insertion and retrieval

### 1.2 Scope Management
- [x] Define `Scope` struct (parent scope, symbol table, scope kind)
- [x] Define `ScopeKind` enum (Global, Module, Function, Block)
- [x] Implement `ScopeStack` for managing nested scopes
- [x] Add `enter_scope()` and `exit_scope()` methods
- [x] Write tests for scope nesting and variable shadowing

### 1.3 Semantic Analyzer Skeleton
- [x] Create `SemanticAnalyzer` struct with AST and scope stack
- [x] Implement `analyze()` entry point that traverses AST
- [x] Add helper methods for visiting different node types
- [x] Implement error collection (Vec<SemanticError>)
- [x] Write basic integration test (empty program)

## Phase 2: Name Resolution

### 2.1 Variable Declaration Resolution
- [x] Implement resolution for variable declarations
- [x] Check for duplicate declarations in same scope (NOTE: Variables allow redeclaration)
- [x] Add variable to current scope's symbol table
- [x] Write tests for valid/invalid variable declarations

### 2.2 Variable Reference Resolution
- [x] Implement identifier lookup in scope chain
- [x] Report error for undefined variables
- [x] Write tests for variable references (valid/undefined)
- [x] Test variable shadowing across scopes

### 2.3 Function Declaration Resolution
- [x] Implement function declaration registration
- [x] Check for duplicate function names
- [x] Store function signature in symbol table
- [x] Write tests for function declarations

### 2.4 Function Call Resolution
- [x] Implement function name lookup for calls
- [x] Report error for calls to undefined functions
- [x] Write tests for valid/invalid function calls

## Phase 3: Type System Foundation

### 3.1 Internal Type Representation
- [x] Create `src/semantic/types.rs` module
- [x] Define `Type` enum (Unit, Number, String, Bool, Function, Struct, etc.)
- [x] Define `TypeId` for efficient type comparisons
- [x] Implement type interning/caching system
- [x] Write tests for type creation and equality

### 3.2 Type Declaration Processing
- [x] Implement type alias resolution
- [x] Implement unit type registration
- [x] Implement union type registration
- [x] Implement struct type registration
- [x] Write tests for each type declaration form

### 3.3 Built-in Types
- [x] Register built-in types (Number, String, Bool, Int8-Int64, UInt8-UInt64, Float32, Float64)
- [x] Create type registry for built-in types
- [x] Write tests for built-in type lookup

## Phase 4: Basic Type Checking

### 4.1 Literal Type Inference (Phase 4.1a - Hindley-Milner Foundation)
- [x] Implement type inference for number literals
- [x] Implement type inference for string literals
- [x] Implement type inference for boolean literals
- [x] Implement type inference for list literals (empty lists; non-empty deferred to 4.1b)
- [x] Write tests for literal type inference
- [x] Implement Hindley-Milner type system infrastructure
  - [x] Type variables (`Type::Var(TypeVarId)`) for unknowns
  - [x] Constraint system for collecting type equalities
  - [x] Unification algorithm with occurs check
  - [x] Substitution mechanism for type variable bindings
- [x] Three-phase analysis algorithm
  - [x] Phase 1: Constraint collection via AST traversal
  - [x] Phase 2: Constraint solving via unification
  - [x] Phase 3: Substitution application to all nodes

### 4.2 Expression Type Checking
- [x] Implement type checking for binary operators (and, or)
- [x] Implement type checking for unary operators (not, negate)
- [x] Report type errors for incompatible operations
- [x] Write tests for expression type checking

### 4.3 Variable Declaration Type Checking
- [x] Implement type annotation validation
- [x] Check initializer expression matches declared type
- [x] Infer type from initializer if not annotated
- [x] Write tests for variable type checking

### 4.4 Assignment Type Checking
- [x] Check assigned value matches variable type
- [x] Report type mismatch errors
- [x] Write tests for assignment type checking

## Phase 5: Function Type Checking

### 5.1 Function Signature Analysis
- [x] Build function type from parameters and return type
- [x] Handle inferred parameter types (mark as Unknown initially)
- [x] Store function type in symbol table
- [x] Write tests for function signature construction

### 5.2 Function Body Analysis
- [x] Analyze function body in new scope
- [x] Add parameters to function scope
- [x] Track return statement types
- [x] Write tests for function body analysis

### 5.3 Return Type Validation
- [x] Check all return statements match declared return type
- [x] Infer return type if not declared
- [x] Check all paths return a value (if return type specified)
- [x] Write tests for return type checking

### 5.4 Function Call Type Checking
- [x] Check argument count matches parameter count
- [x] Check argument types match parameter types
- [x] Determine call expression result type
- [x] Write tests for function call type checking


## Phase 6: Struct Types

### 6.1 Struct Type Definition
- [x] Parse struct field types from type declarations
- [x] Parse struct method signatures
- [x] Build internal struct type representation
- [x] Write tests for struct type construction

### 6.2 Struct Initialization Type Checking
- [x] Check struct literal field types
- [x] Check struct literal method signatures
- [x] Validate required fields are present
- [x] Write tests for struct initialization

### 6.3 Struct Privacy Enforcement
- [x] Track private fields/methods (using NodeFlags)
- [x] Enforce privacy rules for field access
- [x] Enforce privacy rules for method calls
- [x] Write tests for privacy enforcement

### 6.4 Property Access Type Checking
- [x] Check field exists on struct type
- [x] Determine property access result type
- [x] Check privacy rules for property access
- [x] Write tests for property access

### 6.5 Method Call Type Checking
- [x] Check method exists on struct type
- [x] Validate method call arguments
- [x] Determine method call result type
- [x] Handle `this` keyword in method bodies
- [x] Write tests for method calls

## Phase 7: Advanced Type Features

### 7.1 Union Type Support
- [x] Implement union type checking
- [x] Check value matches one of union alternatives
- [x] Write tests for union types

### 7.2 Intersection Type Support (Composition)
- [x] Implement intersection type construction
- [x] Merge struct fields/methods for intersections
- [x] Check composition operator type compatibility
- [x] Check privacy overwriting (public not overwritten by private and private not overwritten by public)
- [x] Write tests for intersection types

### 7.3 Function Type Checking
- [x] Validate function type declarations
- [x] Check function values match function types
- [x] Write tests for function types

### 7.4 Generic Type Parameters
- [x] Implement generic type parameter tracking
- [x] Implement type parameter substitution
- [x] Implement generic constraints checking
- [x] Write tests for generic types

### 7.5 Structural Type Compatibility
- [x] Implement structural subtyping rules
- [x] Check struct compatibility based on fields/methods
- [x] Write tests for structural typing

## Phase 8: Control Flow and Pattern Matching

### 8.1 Match Expression Type Checking
- [ ] Check match subject type
- [ ] Check all arms return compatible types
- [ ] Determine match expression result type
- [ ] Write tests for match expressions

### 8.2 Match Pattern Validation
- [ ] Validate patterns against subject type
- [ ] Check pattern exhaustiveness
- [ ] Report unreachable patterns
- [ ] Write tests for pattern matching

### 8.3 Match Arm Type Checking
- [ ] Check each arm body type
- [ ] Ensure all arms have compatible types
- [ ] Write tests for match arm types

## Phase 9: Advanced Features

### 9.1 Pipe Operator Type Checking
- [ ] Check left side produces value
- [ ] Check right side accepts value
- [ ] Chain types through pipe sequence
- [ ] Write tests for pipe operator

### 9.2 Try Operator Type Checking
- [ ] Implement error handling type checking
- [ ] Check try operator on appropriate types
- [ ] Write tests for try operator

### 9.3 Partial Application
- [ ] Validate placeholder usage
- [ ] Construct partial function types
- [ ] Write tests for partial application

### 9.4 This Keyword Validation
- [ ] Check `this` only used in method context
- [ ] Resolve `this` to correct struct type
- [ ] Write tests for `this` keyword

## Phase 10: Module System

### 10.1 Module Declaration Processing
- [x] Register module declarations
- [x] Support main modules (`module Name`)
- [x] Support submodules (`module .name`)
- [x] Create module symbol tables
- [x] Write tests for module registration

### 10.2 Import Statement Resolution
- [ ] Implement full module import resolution
- [ ] Implement selective import resolution
- [ ] Implement star import resolution
- [ ] Implement import alias handling
- [ ] Write tests for import resolution

### 10.3 Export Statement Validation
- [ ] Validate exported symbols exist
- [ ] Build export lists for modules
- [ ] Check for duplicate exports
- [ ] Write tests for export validation

### 10.4 Submodule Visibility Rules
- [ ] Implement submodule scoping rules
- [ ] Restrict submodule visibility to parent hierarchy
- [ ] Check submodule access permissions
- [ ] Write tests for submodule visibility

### 10.5 Module Path Resolution
- [ ] Implement dotted module path resolution
- [ ] Handle nested module lookups
- [ ] Report errors for undefined modules
- [ ] Write tests for module path resolution

## Phase 11: Error Reporting

### 11.1 Semantic Error Types
- [ ] Define comprehensive SemanticError enum
- [ ] Add error for undefined symbols
- [ ] Add error for type mismatches
- [ ] Add error for duplicate declarations
- [ ] Add error for privacy violations

### 11.2 Error Messages
- [ ] Implement Display for SemanticError
- [ ] Add source location to errors
- [ ] Add helpful error messages with suggestions
- [ ] Write tests for error formatting

### 11.3 Multiple Error Collection
- [ ] Continue analysis after errors
- [ ] Collect all errors before reporting
- [ ] Sort errors by source location
- [ ] Write tests for error collection

## Phase 12: Integration and Testing

### 12.1 Full Pipeline Integration
- [ ] Add semantic analysis to main.rs
- [ ] Create `suru check <file>` CLI command
- [ ] Wire up lexer -> parser -> semantic analysis
- [ ] Write integration tests

### 12.2 Comprehensive Test Suite
- [ ] Test all example.suru constructs
- [ ] Test error cases for each feature
- [ ] Test complex nested scenarios
- [ ] Achieve >90% test coverage

### 12.3 Performance Optimization
- [ ] Profile semantic analysis performance
- [ ] Optimize symbol table lookups
- [ ] Cache type compatibility checks
- [ ] Write performance benchmarks

---

## Notes

- Each checkbox represents a small, focused task (~1-4 hours of work)
- Tasks should be completed in order within each phase
- Write tests for each task before moving to the next
- Keep commits small and focused on individual tasks
- Update progress.md after completing each phase

### Phase 4.1a Implementation Notes (2026-01-13)

Implemented Hindley-Milner type inference with:
- **Incremental approach**: Split into 4.1a (foundation), 4.1b (expressions), 4.1c (functions)
- **Type representation**: Using `Type::Var(TypeVarId)` instead of string-based type variables
- **Function signatures**: Currently stored as strings; will transition to `FunctionType` struct in Phase 4.1c when implementing function inference
- **Testing**: All 144 semantic tests passing, including 18 new tests for unification and type inference

### Future Considerations

- Function signature structure: Consider migrating from string representation to structured `FunctionType` for better type inference and comparison (Phase 5)
- Variable signature struct: May be beneficial for complex type tracking (Phase 8+)

### Test Notes

- test_intersection_invalid_left_type is valid
