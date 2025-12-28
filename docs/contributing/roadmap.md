# Development Roadmap

> Future plans and milestones for Suru Lang

## Project Status

**Current Version:** v0.11.0

**Status:** Early development - Parser and type system complete, semantic analysis in progress

## Completed Milestones

### v0.11.0 - Method Calls & Property Access (Current)
- Method call parsing with dot notation
- Property access support
- Chained method calls
- Integration with expression parser

### v0.10.0 - Parser Module Refactoring
- Modular parser structure (expressions, types, statements)
- 200+ tests across modules
- Improved maintainability

### v0.9.0 - Type Declarations
- All 7 type forms implemented
- Unit, alias, union, struct, intersection types
- Function types and generic types
- Comprehensive type parsing

### v0.8.0 - Function Declarations
- Function declarations with parameters
- Type annotations for parameters and return types
- Return statements
- Nested function support

### v0.7.0 - Expression Parsing
- Boolean operators (and, or, not)
- Precedence climbing algorithm
- Function calls in expressions
- Literal expressions

### v0.6.0 - Compiler Limits
- Configurable safety limits
- TOML-based configuration
- Resource exhaustion prevention
- 8 comprehensive tests

### v0.5.0 - Basic Parser
- Variable declarations
- Statement parsing
- AST construction
- First-child/next-sibling tree structure

### v0.4.0 - CLI Foundation
- Command-line interface using clap
- `suru parse` command
- Type-safe argument handling

### v0.3.0 - AST Design
- Index-based tree structure
- Uniform node size
- Single vector storage

### v0.2.0 - Lexer
- Complete tokenization
- 14 keywords
- Number literals (all bases and type suffixes)
- String literals (standard and interpolated)
- Zero-copy design

### v0.1.0 - Project Setup
- Rust project structure
- Docker development environment
- LLVM 18 integration with Inkwell
- Basic project scaffolding

## Immediate Goals (v0.12.0 - v0.14.0)

### v0.12.0 - Control Flow Parsing
**Timeline:** Next milestone

**Objectives:**
- Parse match expressions
- Pattern matching syntax
- Guard expressions
- Wildcard patterns

**Tasks:**
- [ ] Add match expression parsing to parser
- [ ] Implement pattern AST nodes
- [ ] Add comprehensive tests for all pattern types
- [ ] Update example.suru with match examples

### v0.13.0 - Pipe Operator
**Timeline:** Following v0.12.0

**Objectives:**
- Parse pipe operator `|`
- Left-to-right evaluation
- Integration with method calls

**Tasks:**
- [ ] Implement pipe parsing
- [ ] Add pipe AST nodes
- [ ] Test pipe with function calls and methods
- [ ] Document pipe semantics

### v0.14.0 - Module System Parsing
**Timeline:** Following v0.13.0

**Objectives:**
- Parse module declarations
- Import statements (full, selective, import all)
- Export statements (module-level and file-level)

**Tasks:**
- [ ] Add module parsing
- [ ] Implement import/export syntax
- [ ] Create module AST nodes
- [ ] Test module resolution logic

## Short-term Goals (v0.15.0 - v0.20.0)

### v0.15.0 - Symbol Table
**Objectives:**
- Build symbol table during parsing
- Track declarations and scopes
- Resolve identifiers

**Tasks:**
- [ ] Design symbol table structure
- [ ] Implement scope tracking
- [ ] Add identifier resolution
- [ ] Test with nested scopes

### v0.16.0 - Type Checking Foundation
**Objectives:**
- Basic type checking pass
- Type annotation validation
- Simple type inference

**Tasks:**
- [ ] Create type checker module
- [ ] Implement type representation
- [ ] Add type compatibility checking
- [ ] Test type errors

### v0.17.0 - Generic Type Constraint Inference
**Objectives:**
- Infer generic constraints from method usage
- Duck typing based on inferred constraints
- Type compatibility checking

**Tasks:**
- [ ] Design constraint inference algorithm
- [ ] Implement method signature tracking
- [ ] Add constraint solving
- [ ] Test duck typing scenarios

### v0.18.0 - Semantic Analysis
**Objectives:**
- Complete semantic analysis pass
- Variable usage validation
- Return statement checking
- Unreachable code detection

**Tasks:**
- [ ] Implement control flow analysis
- [ ] Add definite assignment checking
- [ ] Validate return statements
- [ ] Detect unreachable code

### v0.19.0 - Error Diagnostics
**Objectives:**
- Improve error messages
- Add error recovery
- Multiple error reporting

**Tasks:**
- [ ] Design error recovery strategy
- [ ] Implement synchronization points
- [ ] Add helpful error messages with suggestions
- [ ] Test error scenarios

### v0.20.0 - Parser Complete
**Milestone:** Parser and semantic analysis complete

**Deliverables:**
- Full Suru syntax parsing
- Complete type checking
- Comprehensive error reporting
- 300+ tests passing

## Medium-term Goals (v0.21.0 - v0.30.0)

### v0.21.0 - LLVM IR Generation Foundation
**Objectives:**
- Basic LLVM IR generation
- Function compilation
- Simple expressions

**Tasks:**
- [ ] Design code generation architecture
- [ ] Implement function lowering
- [ ] Generate IR for basic expressions
- [ ] Test with simple programs

### v0.22.0 - Type Lowering
**Objectives:**
- Lower Suru types to LLVM types
- Struct layout
- Union representation

**Tasks:**
- [ ] Implement type lowering
- [ ] Design struct memory layout
- [ ] Add union tag generation
- [ ] Test type representations

### v0.23.0 - Control Flow Code Generation
**Objectives:**
- Generate match expressions
- Continuation types (Continue, Break, Produce)
- Method-based iteration

**Tasks:**
- [ ] Implement match lowering
- [ ] Generate continuation type code
- [ ] Add iteration code generation
- [ ] Test control flow

### v0.24.0 - Memory Management
**Objectives:**
- Move semantics implementation
- Automatic copying
- Memory safety guarantees

**Tasks:**
- [ ] Implement move tracking
- [ ] Add copy insertion
- [ ] Generate drop code
- [ ] Test memory safety

### v0.25.0 - Module Compilation
**Objectives:**
- Compile multiple modules
- Module linking
- Export/import resolution

**Tasks:**
- [ ] Implement module compilation
- [ ] Add linking support
- [ ] Generate module metadata
- [ ] Test multi-module projects

### v0.26.0 - Standard Library Foundation
**Objectives:**
- Core types (Int, Float, String, Bool)
- Basic collections (List, Set, Map)
- Essential functions

**Tasks:**
- [ ] Design standard library structure
- [ ] Implement core types
- [ ] Add collection types
- [ ] Write documentation

### v0.27.0 - Error Handling Code Generation
**Objectives:**
- Result and Option types
- Try keyword implementation
- Error propagation

**Tasks:**
- [ ] Implement Result/Option lowering
- [ ] Generate try code
- [ ] Add error propagation
- [ ] Test error scenarios

### v0.28.0 - Optimization
**Objectives:**
- Basic LLVM optimization passes
- Dead code elimination
- Constant folding

**Tasks:**
- [ ] Configure LLVM optimization passes
- [ ] Add custom optimization passes
- [ ] Benchmark performance
- [ ] Test optimized output

### v0.29.0 - Debugging Support
**Objectives:**
- DWARF debug information
- Source mapping
- Variable inspection

**Tasks:**
- [ ] Generate debug info
- [ ] Add line number mapping
- [ ] Test with debuggers (gdb, lldb)
- [ ] Document debugging workflow

### v0.30.0 - Compiler v1.0 Alpha
**Milestone:** Working compiler with basic features

**Deliverables:**
- Compile simple Suru programs to executables
- Standard library with core types
- Basic optimization
- Debug information

## Long-term Goals (v0.31.0+)

### LSP Server (v0.31.0 - v0.35.0)
**Objectives:**
- Language Server Protocol implementation
- IDE integration (VS Code, Neovim, etc.)
- Real-time error checking
- Code completion
- Hover information
- Go to definition

**Key Features:**
- Incremental parsing
- Fast re-checking
- Semantic highlighting
- Refactoring support

### REPL (v0.36.0)
**Objectives:**
- Interactive Read-Eval-Print Loop
- Immediate feedback
- Expression evaluation
- Statement execution

**Features:**
- Multi-line editing
- History support
- Tab completion
- Inline documentation

### Package Manager (v0.37.0 - v0.40.0)
**Objectives:**
- Package registry
- Dependency management
- Version resolution
- Build system integration

**Features:**
- `suru install <package>`
- `suru publish`
- Semantic versioning
- Lock file generation

### Standard Library Expansion (v0.41.0+)
**Objectives:**
- Comprehensive standard library
- Network I/O
- File system operations
- Concurrency primitives
- Data structures

**Modules:**
- `std.io` - Input/output
- `std.fs` - File system
- `std.net` - Networking
- `std.concurrent` - Concurrency
- `std.collections` - Advanced collections
- `std.math` - Mathematics
- `std.text` - Text processing
- `std.time` - Date and time

### Performance (Ongoing)
**Objectives:**
- Compile-time optimization
- Runtime performance
- Memory efficiency

**Areas:**
- Faster parsing
- Incremental compilation
- Parallel compilation
- LLVM optimization tuning

### Tooling (Ongoing)
**Objectives:**
- Developer tools
- Profiler
- Formatter
- Linter

**Tools:**
- `suru fmt` - Code formatter
- `suru lint` - Linter
- `suru prof` - Profiler
- `suru doc` - Documentation generator

## Experimental Features

### Async/Await
**Status:** Research phase

**Concept:** Zero-cost async/await for concurrent programming

**Considerations:**
- Integration with type system
- Memory model compatibility
- Runtime requirements

### Generators
**Status:** Research phase

**Concept:** Yield-based generators for iteration

**Considerations:**
- Continuation types integration
- Memory safety guarantees

### Macros
**Status:** Research phase

**Concept:** Hygenic macros for metaprogramming

**Considerations:**
- Syntax design
- Type safety
- Compile-time evaluation

## Community Goals

### Documentation
- [ ] Complete language reference
- [ ] Tutorial series
- [ ] Example programs
- [ ] Video tutorials
- [ ] Interactive playground

### Community Building
- [ ] Discord/Slack server
- [ ] Monthly blog posts
- [ ] Conference talks
- [ ] Open source governance

### Ecosystem
- [ ] Editor plugins
- [ ] Build tool integrations
- [ ] CI/CD support
- [ ] Docker images

## Success Criteria

### v1.0 Release Criteria
- [ ] Compile all example programs successfully
- [ ] 1000+ tests passing
- [ ] Complete language specification
- [ ] Standard library with 50+ types
- [ ] LSP server with VS Code extension
- [ ] Documentation complete
- [ ] At least 3 non-trivial programs written in Suru

### Long-term Success
- [ ] 100+ contributors
- [ ] 1000+ GitHub stars
- [ ] Production use in at least 5 projects
- [ ] Active community (Discord, forums)
- [ ] Package registry with 50+ packages

## How to Contribute

See [Contributing Guide](README.md) for information on how to help with any of these milestones.

**Priority Areas:**
1. Parser completion (match, pipes, modules)
2. Type system implementation
3. Code generation
4. Standard library
5. Documentation

---

**See also:**
- [Architecture](architecture.md) - Compiler structure
- [Design Decisions](design-decisions.md) - Key architectural choices
- [Development Workflow](development.md) - Build and test instructions
