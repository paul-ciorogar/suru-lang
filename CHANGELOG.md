# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
- `Bool` elements use 1-byte `i8[]` buffers; `Int32` uses 4-byte `i32[]`; `Int64`, `Float64`,
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
- Language features: `Bool`, `Int32`, `Int64`, `Float64`, `String`, `Array<T>`, named struct types,
  sum types, `match` (statement and expression), `while`, `include`, `clone`/`drop`,
  `readFile`/`writeFile`/`appendToFile`, `exit`, `printLn`, `printError`, `exec`.
- `exec(cmd String) Int64` — runs a shell command via `system()` and returns its exit code.
  First language feature added purely in Suru, with no C# changes.
- Fixed `scripts/bootstrap.sh`: the C# bootstrap binary emits one `.ll` per module; the
  script now links all generated modules rather than only the entry-point file.
- All Docker scripts mount `bin/suru-build` as a volume override so bootstrapping no
  longer requires rebuilding the Docker image.
