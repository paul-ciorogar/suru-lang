# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Suru is a general-purpose, high-level, minimalist programming language written in C. The project is in early development (targeting v0.1.0 - v0.3.0), implementing a compiler with lexer, parser, and code generation stages. The language emphasizes minimal syntax, library-based extensibility, and LSP-first interactive development.

## Build System

The project uses a custom C-based build system (`builder.c`) that auto-discovers source files:

```bash
# Compile the build system first (only needed once)
gcc -o builder builder.c

# Build the compiler (development mode with debug symbols)
./builder

# Build for production (optimized, no debug info)
./builder build-prod

# Clean build artifacts
./builder clean

# Rebuild from scratch
./builder rebuild
./builder rebuild-prod
```

**Build output**: Executable is placed in `tmpbuild/suru`

**Test execution**: The builder automatically runs integration tests after building successfully.

## Running the Compiler

```bash
# Lex a source file (tokenization only)
./tmpbuild/suru lex <file.suru>

# Compile and run a source file
./tmpbuild/suru run <file.suru>
```

## Testing

### Unit Tests

The project has a custom test runner (`test_runner.c`) that compiles and runs C test files:

```bash
# Compile test runner (only needed when test_runner.c changes)
gcc -o test test_runner.c -Wall -Wextra -std=c99

# Run unit tests
./test
```

Tests are located in `tests/` directory. The runner automatically:
- Compiles each test file
- Executes compiled tests
- Reports results with timing information
- Aborts on first failure

### Integration Tests

Integration tests are in `integration_tests/` directory. Each test is a subdirectory containing:
- `command.txt` - Command arguments to pass to the compiler
- `expected.txt` - Expected output
- `output.txt` - Actual output (generated during test run)

The builder runs these tests automatically after successful compilation.

## Architecture

### Memory Management

**Arena Allocator** (`src/arena.h`, `src/arena.c`):
- Custom memory allocator for fast, region-based allocation
- Used throughout the compiler to avoid manual free() calls
- Two main arenas: one for string storage, one for general compiler data structures
- Call `arena_create()` to initialize, `arena_destroy()` to clean up

**String Storage** (`src/string_storage.h`, `src/string_storage.c`):
- Dedicated string interning system for lexer tokens
- Deduplicates strings automatically
- All strings are stored with length prefix (not just null-terminated)
- Use `store_from_buffer()` for tokenized strings, `store_cstring()` for C strings

### Compiler Pipeline

**Main Entry** (`src/main.c`):
- Orchestrates the compilation pipeline
- Handles command-line interface (`lex` and `run` commands)
- Creates arenas and initializes subsystems

**Lexer** (`src/lexer.h`, `src/lexer.c`):
- Tokenizes Suru source code
- Tracks line/column positions for error reporting
- Handles string interpolation with multiple backtick nesting levels
- Manages state for multiline strings and brace depth tracking
- Key function: `next_token(Lexer*)` - advances and returns next token

**Parser** (`src/parser.h`, `src/parser.c`):
- Builds Abstract Syntax Tree (AST) from tokens
- Currently skeletal - under active development
- Error collection system tracks multiple parse errors
- Key function: `parse_statement(Parser*)` - parses top-level statements

**Code Generation** (`src/code_generation.h`, `src/code_generation.c`):
- Transforms AST into executable code
- Currently skeletal - targeting x86-64 assembly (planned for v0.14.0)
- Key function: `generate_code(ASTNode*)` - produces machine code buffer

**I/O Utilities** (`src/io.h`, `src/io.c`):
- File reading/writing utilities
- Buffer management for source code and generated output

## Development Roadmap

Reference `roadmap.md` for detailed implementation phases. Current focus:

**Phase 1 (v0.1.0 - v0.3.0)**: Foundation
- **v0.1.0**: Lexer + basic parser + interpreter (currently in progress)
- **v0.2.0**: Pattern matching and control flow
- **v0.3.0**: Functions and lexical scoping

Key upcoming milestones:
- v0.4.0-v0.7.0: Type system and structural typing
- v0.8.0-v0.10.0: Currying, pipelines, error handling, modules
- v0.14.0: Native x86-64 code generation

## Language Features

See `README.md` for complete language specification. Key characteristics:

- **No loop keywords**: Uses method-based iteration (`.times()`, `.each()`)
- **Structural typing**: Types are compatible by shape, not name
- **String interpolation**: Multiple nesting levels with backticks (\`, \`\`, \`\`\`)
- **Error handling**: Errors as values with `try` keyword for short-circuiting
- **Privacy**: `_` prefix for private members
- **Pipeline operator**: `|` for function chaining
- **Type composition**: `+` operator composes types and structs

## File Organization

```
src/           - Compiler source code (C files)
tests/         - Unit tests (C files)
integration_tests/ - End-to-end compiler tests
tmpbuild/      - Build output directory
rd/            - Research & development / experimental code
```

## Compiler Development Notes

- All compiler phases use arena allocation - avoid `malloc`/`free` directly
- String interning is essential for performance - never duplicate identifier strings
- Token line/column tracking is critical for quality error messages
- The lexer handles complex string interpolation state - be careful modifying `in_string_interpolation` and `brace_depth`
- Parser error recovery is not yet implemented - currently fails on first error

### Debugging the Parser

The parser includes optional debug logging to help diagnose infinite loops or understand parser flow:

1. Edit `src/parser.c` and uncomment the `#define DEBUG_PARSER_LOOP` line at the top
2. Rebuild: `./builder`
3. Run any parse command to see detailed loop iteration output

Debug output shows:
- Iteration count
- Parser state ID
- Current step within state
- Stack depth
- Current token type

The debug mode also includes an iteration limit (1000) that breaks the loop if exceeded, helping catch infinite loops early.

## Common Tasks

**Adding a new token type**:
1. Add enum to `TokenType` in `src/lexer.h`
2. Implement recognition in `src/lexer.c`
3. Update `print_tokens()` for debugging output

**Adding a new AST node**:
1. Define structure in `src/parser.h`
2. Implement parsing logic in `src/parser.c`
3. Add code generation in `src/code_generation.c`

**Adding a unit test**:
1. Create test file in `tests/` directory
2. Add test to `test_runner.c` in the `add_test()` calls
3. Recompile and run: `gcc -o test test_runner.c && ./test`

**Adding an integration test**:
1. Create subdirectory in `integration_tests/`
2. Add `command.txt` with compiler arguments
3. Add `expected.txt` with expected output
4. Run builder to execute: `./builder`
