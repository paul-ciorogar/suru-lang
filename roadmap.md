# Suru Language Compiler Roadmap

## Phase 1: Foundation (v0.1.0 - v0.3.0)
**Goal**: Core language features working end-to-end

### v0.1.0 - Minimal Viable Compiler
- **Lexer**: Tokenize basic syntax
  - Identifiers, keywords, operators
  - Numbers (decimal, binary `0b`, octal `0o`, hex `0x`)
  - Strings (double quotes, basic backtick interpolation)
  - Booleans (`true`, `false`)
- **Parser**: Build parse tree for basic constructs
  - Main function definition
  - Print function call
- **Tooling**:
  - Format tool for code formatting
- **AST Interpreter**: Build AST from parse tree. Execute AST directly
- **Standard Library Basics**: Print function
- **Milestone**: Run "Hello World" and basic logical expressions

### v0.2.0 - Pattern Matching & Control Flow
- **Pattern Matching**: `match` expressions
  - Variable declarations (`name: value`)
  - Simple expressions (logical)
  - Value matching
  - Default case (`_`)
- **Milestone**: Run programs using pattern matching for control flow

### v0.3.0 - Functions & Lexical Scoping
- **Function Definitions**: 
  - Without type annotations
  - Generic functions (simple cases)
- **Function Calls**: Positional arguments
- **Lexical Scoping**: Strict function-level scoping
- **Access Rules**: Parameters, local vars, global constants only
- **Milestone**: Run programs with multiple functions and proper scoping

## Phase 2: Language Server Protocol (LSP) Foundation (v0.4.0 - v0.6.0)
**Goal**: Establish LSP infrastructure to enable interactive development and IDE support

### v0.4.0 - LSP Server Foundation & Syntax Highlighting
- **LSP Server Architecture**:
  - JSON-RPC communication protocol implementation
  - Message parsing and response handling
  - Server initialization and capability negotiation
- **Syntax Highlighting**:
  - TextMate grammar for Suru syntax
  - Token classification (keywords, identifiers, strings, numbers, operators)
  - Support for string interpolation highlighting
- **Document Management**:
  - Track open documents and changes
  - Incremental document sync
  - Version tracking
- **Basic Server Lifecycle**: Initialize, shutdown, exit
- **Milestone**: Suru files display with proper syntax highlighting in LSP-compatible editors

### v0.5.0 - Diagnostics & Error Reporting
- **Real-time Diagnostics**:
  - Lexer error reporting (invalid tokens, malformed strings)
  - Parser error reporting (syntax errors, unexpected tokens)
  - Error location tracking (line, column, range)
- **Diagnostic Publishing**:
  - Push diagnostics to client on file change
  - Error severity levels (error, warning, info, hint)
  - Diagnostic messages with helpful context
- **Multi-file Support**:
  - Track multiple open documents
  - Per-document diagnostic caching
- **Performance Optimization**:
  - Debouncing for frequent edits
  - Background parsing
- **Milestone**: Developers see syntax and parse errors in real-time as they type

### v0.6.0 - Code Completion & Hover Information
- **Symbol Table Integration**:
  - Track function definitions and variables
  - Scope-aware symbol lookup
  - Build symbol index from parsed AST
- **Code Completion**:
  - Complete function names
  - Complete variable names in scope
  - Complete keywords (match, return, true, false)
  - Context-aware suggestions (e.g., only show functions in call position)
- **Hover Information**:
  - Show function signatures on hover
  - Display variable types (when type system is ready)
  - Show documentation comments (when available)
- **Signature Help**:
  - Parameter hints for function calls
  - Highlight current parameter
- **Milestone**: Interactive development with autocomplete and inline information

## Phase 3: Type System & Core Features (v0.7.0 - v0.10.0)
**Goal**: Implement Suru's unique type system and structural typing

### v0.7.0 - Structural Type System
- **Type Definitions**: 
  - Simple types (flags like `type Success`)
  - Type aliases (`type UserId: Number`)
  - Alternative types (`type Status: Success, Error, Loading`)
  - Record types with fields and methods
- **Structural Typing**: Type compatibility by shape, not name
- **Type Checking**: Structural equivalence checking
- **Better Error Messages**: Type mismatch explanations
- **Method Calls**: Dot notation for methods
- **Built-in Methods**: Basic string and number methods
- **Milestone**: Run programs using structural typing with multiple record types

### v0.8.0 - Advanced Types & Composition
- **Type Composition**: Using `+` operator
  - Struct composition
  - Type extension
  - Method composition with `+` prefix
- **Generic Types**: 
  - Single type parameters (`List<T>`)
  - Multiple type parameters (`Map<K, V>`)
  - Basic constraints
- **Union Types**: Multi-variant types
- **Milestone**: Run programs with generic collections and type composition working

### v0.9.0 - Collections & Iteration
- **Collection Types**: `List<T>`, `Set<T>`, `Map<K,V>`
- **Unified Syntax**: `[]` for all collections, type-driven interpretation
- **Collection Methods**: 
  - `.each()`, `.map()`, `.filter()`
  - Iteration with continuation types
- **Number Iteration**: `.times()` method
- **Continuation Types**: `Continue`, `Break<T>`, `Produce<T>`
- **Milestone**: Run programs with complex data processing with collections

### v0.10.0 - Privacy & Encapsulation
- **Private Members**: `_` prefix for private fields and methods
- **Public Interface**: Type declarations define public API
- **Constructor Functions**: Type-named constructors
- **Instance Methods**: Per-instance implementations
- **`this` Reference**: Self-reference in methods
- **Milestone**: Properly encapsulated types with clean public/private separation

## Phase 4: Advanced Language Features (v0.11.0 - v0.13.0)
**Goal**: Currying, pipelines, and error handling

### v0.11.0 - Currying & Partial Application
- **Placeholder Currying**: Using `_` for partial application
- **Explicit Partial**: `partial` keyword for many-argument functions
- **Method Currying**: Currying on type methods
- **Function Composition**: Curried functions in pipelines
- **Milestone**: Complex functional programming patterns working

### v0.12.0 - Pipeline Operator & Error Handling
- **Pipeline Operator**: `|` for chaining operations
- **Result Types**: `Result<T, E>`, `Option<T>`, `Response<T, E>`, `Either<L, R>`
- **Try Operator**: `try` keyword for error short-circuiting
- **Try Compatibility**: Works with any 2-variant union type
- **Pipeline + Try**: Combining operators for clean error handling
- **Milestone**: Complex data pipelines with proper error propagation

### v0.13.0 - Modules & Organization
- **Module System**: 
  - Module declarations
  - Import statements (aliased, selective, wildcard)
  - Export declarations
- **File Structure**: `.suru` files, module directories
- **Module Resolution**: Finding and loading modules
- **Main Function**: Entry point in main module
- **Milestone**: Multi-file projects with proper module structure

## Phase 5: Polish & Advanced Features (v0.14.0 - v0.16.0)
**Goal**: String interpolation, overloading, and documentation

### v0.14.0 - String Interpolation
- **Basic Interpolation**: Single backtick with `{expr}`
- **Multi-line Strings**: Backtick followed by newline
- **Nested Interpolation**: Multiple backtick levels
  - `{{}}` for double backticks
  - `{{{}}}` for triple backticks
  - `{{{{}}}}` for quadruple backticks
- **Escape Sequences**: `\n`, `\t`, `\xNN`, `\uNNNN`, `\UNNNNNNNN`
- **Milestone**: Complex string templating and formatting

### v0.15.0 - Function & Method Overloading
- **Function Overloading**: Same name, different parameter types
- **Return Type Overloading**: Same signature, different return types
- **Method Overloading**: Overloaded methods in types
- **Overload Resolution**: Choose correct overload at compile time
- **Milestone**: Type-safe overloading across the language

### v0.16.0 - Documentation System
- **Doc Comments**: Markdown between `====` delimiters
- **Doc Annotations**: 
  - `@param`, `@return`, `@example`
  - `@deprecated`, `@experimental`, `@todo`
  - `@see`, `@link`, `@author`, `@since`
- **Doc Generation**: Extract documentation to readable format
- **Milestone**: Syntax is final

## Phase 6: Advanced LSP Features (v0.17.0)
**Goal**: Add mocking and simulation using LSP commands

### v0.17.0 - LSP Commands & Interactive Development
- **Create LSP specific syntax**:
  - crete usecases
  - navigate or select usecases
  - crete mock data
  - create asserts
  - inspect values
  - navigate through the callstack
- **Run usecases**: run code with mocked values

## Phase 7: Code Generation (v0.18.0)
**Goal**: Native compilation target

### v0.18.0 - x86-64 Linux Code Generation
- **Assembly Generation**: Emit x86-64 assembly code
- **Calling Conventions**: System V AMD64 ABI
- **Register Allocation**: Basic register allocation strategy
- **System Calls**: Linux syscalls for I/O
- **Linking**: Generate object files and link with system linker
- **Binary Output**: Produce standalone native executables
- **Performance Baseline**: Establish benchmarks vs interpreter
- **Milestone**: Native executables running on Linux x86-64

## Phase 8: Standard Library & Tooling (v0.19.0 - v0.21.0)
**Goal**: Implement and document a minimal viable standard library

### v0.19.0 - Standard Library Implementation
- **Standard Library**: 
  - Complete collection implementations
  - String manipulation functions
  - Math library
  - File I/O basics
  - Async/concurrency primitives
  - FFI (Foreign Function Interface) for C interop
- **Documentation**: Complete language guide and API docs
- **Examples**: Real-world example programs
- **Milestone**: Well-documented standard library and examples

### v0.20.0 - Optimization Pass
- **Constant Folding**: Compile-time expression evaluation
- **Dead Code Elimination**: Remove unused functions and types
- **Type Specialization**: Monomorphization of generics
- **Method Inlining**: Inline small methods for performance
- **Pipeline Optimization**: Optimize chained operations
- **Milestone**: Significant performance improvements on benchmarks

### v0.21.0 - Refinement

- **Documentation**: 
  - Complete language reference
  - Tutorial series
  - API documentation


---

## Progress Log

### 2025-10-20 - Variable Declarations Implemented
**Status**: v0.1.0 milestone completed

Implemented variable declarations in the compiler with full parser, AST, and interpreter support.

**Features Added:**
- **Syntax**: Variables declared with `name: value` (no type annotations yet)
- **Parser Enhancement**:
  - Added `PARSE_STATEMENT` state that unifies handling of:
    - Function declarations (`identifier : (params) block`)
    - Variable declarations (`identifier : value`)
    - Call expressions (`identifier(args)`)
  - Proper lookahead to distinguish statement types
- **AST Nodes**: Added `AST_VAR_DECL` and `NODE_VAR_DECL` node types
- **Interpreter**:
  - Simple array-based variable storage with linear lookup
  - Variables store String* pointers (all strings interned in string storage)
  - Supports string values (literals and variable references)
  - Variables are mutable by default
  - Function-local scope only
- **Variable References**: Can use variables in expressions (e.g., `print(message)`)
- **Test Coverage**: Added integration test `var_decl`

**Example:**
```suru
main: () {
    message: "Hello from a variable!\n"
    print(message)
}
```

**Not Included (Future Work):**
- Type annotations (explicit types)
- Global/file-scope constants
- Nested scopes/shadowing
- Numbers, booleans, or other value types

---

### 2025-10-21 - Boolean Expressions with Operators (v0.1.0 Progress)

**Status**: Boolean expression support added

Implemented boolean literals and logical operators with full expression tree parsing using the Shunting Yard algorithm.

**Features Added:**
- **Boolean Literals**: `true` and `false` keywords
- **Logical Operators**:
  - `not` (unary negation)
  - `and` (binary logical AND)
  - `or` (binary logical OR)
- **Expression Parsing**:
  - Implemented Shunting Yard algorithm for infix to postfix conversion
  - Stack-based expression tree building from postfix notation
  - Operator precedence: `not` (unary) > `and` > `or`
  - Handles complex nested expressions
- **AST Nodes**:
  - Added `AST_BOOLEAN_LITERAL`, `AST_NOT_EXPR`, `AST_AND_EXPR`, `AST_OR_EXPR`
  - Also added placeholders for future operators: `AST_PLUS_EXPR`, `AST_PIPE_EXPR`, `AST_NEGATE_EXPR`
- **Type System Foundation**:
  - Introduced `ValueType` enum to distinguish string and boolean values
  - Modified `Variable` struct to use tagged union for typed values
  - Updated variable storage/lookup to handle multiple types
- **Interpreter**:
  - Added `evaluate_expression()` function for recursive expression evaluation
  - Supports boolean literals, variable references, and logical operators
  - Extended `print()` to handle boolean values
- **Test Coverage**: Added integration tests `boolean_test` and `boolean_expr`

**Example:**
```suru
main: () {
    a: true
    b: false

    notA: not a              // false
    aAndB: a and b           // false
    aOrB: a or b             // true
    complex: not a or b and true  // false

    print(complex)
}
```

**Technical Details:**
- Shunting Yard algorithm converts infix expressions to postfix for easier tree construction
- Expression tree built using stack-based postfix evaluation
- All operators stored as specific node types (not generic operator nodes)

---

### 2025-10-23 - Pattern Matching Implementation (v0.2.0 Progress)

**Implemented**: Pattern matching for boolean and string values with wildcard support.

**Changes**:
- **AST** (src/ast.h, src/ast_builder.c):
  - Added `AST_MATCH_EXPR`, `AST_MATCH_ARM`, and `AST_MATCH_WILDCARD` node types
  - Updated AST builder to map new parse tree nodes to AST
  - Modified terminal node detection to include wildcard pattern

- **Parse Tree** (src/parse_tree.h):
  - Added `NODE_MATCH_EXPR`, `NODE_MATCH_ARM`, and `NODE_MATCH_WILDCARD` types

- **Parser** (src/parser.h, src/parser.c):
  - Added `PARSE_MATCH_EXPR` parser state
  - Modified `PARSE_EXPRESSION` to detect `match` keyword and delegate to match parser
  - Implemented match expression parsing:
    - Subject expression (identifier or literal)
    - Multiple match arms with patterns and expressions
    - Boolean patterns (`true`, `false`), string literal patterns, wildcard (`_`)
    - Reuses existing expression parsing for arm expressions

- **Interpreter** (src/interpreter.c):
  - Added `AST_MATCH_EXPR` case in `evaluate_expression()`
  - Pattern matching logic:
    - Evaluates subject expression
    - Iterates through arms to find first matching pattern
    - Supports boolean, string, and wildcard matching
    - Returns matched arm's expression value

**Integration Tests**:
- `integration_tests/match_bool/`: Tests boolean pattern matching
- `integration_tests/match_string/`: Tests string pattern matching with wildcard fallback and variable reassignment

**Test Results**: ✅ All 9 integration tests passing

**Examples**:
```suru
// Boolean match
isTrue: true
result: match isTrue {
    true: "Yes\n"
    false: "No\n"
}
print(result)  // Output: Yes

// String match with wildcard
day: "Monday"
result: match day {
    "Monday": "Yes, it's Monday\n"
    _: "No, maybe tomorrow\n"
}
print(result)  // Output: Yes, it's Monday
```

**Limitations**:
- Match expressions only supported in variable declarations (not as function arguments)
- Only boolean and string literal patterns supported (no type matching, destructuring, or guards yet)
- No nested match expressions

---

### 2025-01-24 - Match Statements Implementation

**Implemented**: Match statements for side-effect execution (complementing match expressions)

**Changes**:

- **AST & Parse Tree** (src/ast.h, src/parse_tree.h):
  - Added `AST_MATCH_STMT` and `NODE_MATCH_STMT` types
  - Match statements use same `NODE_MATCH_ARM` structure as expressions

- **Parser Framework** (src/parser.h):
  - Added `step` field to `ParserStackFrame` for improved state management
  - Updated `push_new_frame()` to accept step parameter for multi-stage parsing
  - All existing parser states updated to use step-based approach

- **Parser** (src/parser.c):
  - Added `PARSE_MATCH_STMT` parser state
  - Modified `PARSE_BLOCK` to detect `match` keyword at statement position
  - Implemented match statement parsing with step-based state machine:
    - Step 0: Create node and parse subject expression
    - Step 1: Expect opening brace `{`
    - Step 2: Parse match arms (pattern `:` statement)
  - Refactored `PARSE_MATCH_EXPR` to use consistent step-based approach
  - Pattern support: boolean, string literals, identifiers, wildcard (`_`)

- **AST Builder** (src/ast_builder.c):
  - Added `NODE_MATCH_STMT → AST_MATCH_STMT` mapping

- **Interpreter** (src/interpreter.c):
  - Implemented `execute_match_stmt()` function
  - Evaluates subject expression and matches patterns
  - Executes matching arm's statement (supports `AST_CALL_EXPR` and `AST_VAR_DECL`)
  - Returns 0 on success (no value propagation, unlike match expressions)
  - Updated `execute_block()` to handle `AST_MATCH_STMT`

- **Formatter** (src/formatter.c):
  - Added `NODE_MATCH_STMT` and related expression nodes to formatting cases

**Integration Tests**:
- `integration_tests/match_stmt/`: Tests boolean pattern matching with side effects

**Test Results**: ✅ All 10 integration tests passing

**Examples**:
```suru
// Match statement (executes for side effects, no return value)
main: () {
    someVal: true

    match someVal {
        true: print("Yes\n")
        false: print("No\n")
    }
    // Output: Yes
}

// String patterns with wildcard
main: () {
    day: "Monday"

    match day {
        "Monday": print("It's Monday!\n")
        "Friday": print("It's Friday!\n")
        _: print("It's another day\n")
    }
    // Output: It's Monday!
}
```

**Key Differences from Match Expressions**:
- Match statements execute for side effects (e.g., function calls)
- No return value (cannot be assigned to variables)
- Used at statement position in blocks
- Arms contain statements, not expressions
- Otherwise identical pattern matching semantics

**Architecture Improvements**:
- Step-based parser state machine provides clearer multi-stage parsing
- Consistent pattern between match expressions and statements
- Improved code maintainability and extensibility

---

### 2025-11-21 - Functions with Parameters and Return Values (v0.3.0 Milestone)

**Implemented**: Full function support with parameters, return statements, nested declarations, and symbol table management.

**Changes**:

- **Symbol Table** (NEW: src/symbol_table.h, src/symbol_table.c):
  - Added symbol table for tracking function definitions and scoping
  - Supports function parameter tracking and lookup
  - Foundation for lexical scoping implementation

- **AST** (src/ast.h, src/ast.c):
  - Added `AST_RETURN_STMT` node type for return statements
  - Extended function-related AST node structures

- **Parser** (src/parser.c, src/parser.h):
  - Function parameter parsing with parameter lists
  - Return statement parsing
  - Nested function declaration support

- **Interpreter** (src/interpreter.c, src/interpreter.h):
  - Function call implementation with argument passing
  - Return value propagation
  - Nested function scope management
  - Match expressions can now return values from function calls

- **Integration Tests**:
  - Reorganized all tests into `parse_*` and `run_*` variants
  - Added `parse_func_basic` / `run_func_basic`: Functions without parameters
  - Added `parse_func_params` / `run_func_params`: Functions with parameters and return values
  - Added `parse_func_nested` / `run_func_nested`: Nested function declarations with closures

**Examples**:
```suru
// Basic function
greet: () {
    print("Hello\n")
}

// Function with parameters and return value
isMonday: (val) {
    return match val {
        "Monday": true
        _: false
    }
}

// Nested functions
main: () {
    isItTrue: (val) {
        return match val {
            true: "it is true\n"
            false: "it is false\n"
        }
    }

    print(isItTrue(true))  // Output: it is true
}
```

