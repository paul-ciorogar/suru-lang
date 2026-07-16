# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Typed `extern fn` signatures.** Extern declarations carry a full C-FFI signature —
  `extern fn name(type, type) return` — instead of a bare name. The parameter list is a
  comma-separated list of types (possibly empty) and the return type is optional (a missing
  one means `void`); e.g. `extern fn exit(i32)`, `extern fn add(i32, i32) i32`,
  `extern fn flush()`. `i8 i16 i32 i64 u8 u16 u32 u64 void` are now reserved type keywords
  (lexed as a single `TypeName` token) and can no longer be identifiers, so a misspelled
  type is a parse error. `SuruType` grows a singleton per integer width plus `Void` and a
  `TryResolve(name)`; the AST gains a `TypeReference` node, and `ExternDeclaration` gains
  `Parameters`/`ReturnType` while `CallExpression` now holds an `Arguments` list. The
  semantic pass resolves each signature's types (rejecting `void` in parameter position),
  checks call **arity** against the declared parameter count, and stamps each call's
  `ResolvedType` with the extern's return type. New token kinds `Comma`/`TypeName`; covered
  by new `ParserTests`/`SemanticTests` cases (signature parsing, empty/multi params, zero-
  and multi-argument calls, arity mismatches, void-in-param, reserved-keyword enforcement,
  and return-type stamping). Per-argument value-type checking is deferred until a second
  value type exists.
- **Parser.** `Parser.Parse(IReadOnlyList<Token>)` turns the lexer's token stream into a
  `SuruProgram` AST, returning a `ParseResult` (the parsed program — `null` when none
  could be recovered — plus every diagnostic). A hand-written recursive-descent parser
  over the Stage 1 grammar: `extern` declarations, `fn IDENT() { … }`, a bare call, and
  flat left-to-right arithmetic (no operator precedence, so `1 + 2 * 3` nests as
  `((1 + 2) * 3)`; parentheses are the only regrouping). A program may define any number
  of functions; the one named `main` is its entry point (`SuruProgram.Main`), and a
  missing or duplicated `main` is a located diagnostic. Malformed input is recovered
  from, not fatal: an unexpected token records a located diagnostic and unwinds (via an
  internal `ParseError`) to the nearest recovery boundary — the declaration loop and the
  statement loop — where `Synchronize` resynchronises and parsing continues, so one pass
  reports every independent mistake. New files: `Parsing/Parser.cs`, with `ParserTests.cs`
  covering the example program, left-to-right nesting, parenthesised regrouping, extern
  collection, and multi-error recovery.
- **AST node types.** A mutable-class node hierarchy in `Parsing/Ast.cs`
  (`Suru.Compiler.Parsing`) models the full grammar: an `AstNode` base carrying
  a `SourceSpan` and a settable `ResolvedType` annotation slot, a value-node base
  `Expression`, and the leaf/structural nodes `IntLiteral`, `IdentifierExpression`,
  `BinaryExpression` (with a `BinaryOperator` enum), `CallExpression`, `Block`,
  `ExternDeclaration`, `FunctionDefinition`, and the root `ProgramNode`. Nodes are
  mutable classes (not immutable records) so later passes annotate them in place:
  structural fields are get-only, the `ResolvedType` slot is settable. An empty
  placeholder `SuruType` (root `Suru.Compiler` namespace, `SuruType.cs`) types that slot
  until the type system lands. The parser's tests will exercise these nodes.
- **Lexer.** `Lexer.Tokenize(string)` scans Suru source into a
  `LexResult` — a token stream terminated by an end-of-file token, plus a list of
  structured `Diagnostic`s. It handles integer literals, the `+ - * /` operators,
  `( ) { }` brackets, identifiers, and the `extern`/`fn` keywords (`main` stays an
  ordinary identifier). Unexpected characters are reported as located diagnostics and
  skipped rather than thrown, so lexing continues. New files: `Lexer.cs`, `Token.cs`
  (`TokenKind`/`Token`), and `Diagnostics.cs` (reusable `SourceSpan` with
  line+column+offset and `Diagnostic`), with `LexerTests.cs` covering the example
  program, keyword classification, position tracking, and error recovery.
- **Token text interning.** A `Tokens` collection (`Tokens.cs`) is the lexer's output
  buffer. `AddToken` interns the variable-spelling lexemes — identifiers and int
  literals — through an internal registry so all tokens with the same text share one
  `string` instance (distinct-string allocations are bounded by the number of distinct
  lexemes, not tokens); the lexer feeds it source `ReadOnlySpan<char>` slices, so a
  repeated identifier allocates no string at all after its first occurrence.
  Fixed-spelling tokens (operators, brackets, the `extern`/`fn` keywords, end-of-file)
  go through `AddFixedToken` with the kind's compile-time constant text (`Token.FixedText`)
  and never touch the registry. `Tokens` implements `IReadOnlyList<Token>`, so it doubles
  as the `LexResult` token stream. Covered by `TokensTests.cs` / `TokenTests.cs`.

### Added Development environment

- **Development environment.** Everything builds and runs inside a
  container; the only host dependency is Docker.
  - Dockerized toolchain: a `Dockerfile` (image `suru-dev`) on
    `mcr.microsoft.com/dotnet/sdk:10.0` with native LLVM 20 (`libllvm20`,
    `clang-20`, `lld-20`, `llvm-20`), and a `docker-compose.yml` `dev` service
    that bind-mounts the repo at `/workspace` and runs as the host user. Versions
    pinned in `docs/toolchain.md` (.NET SDK 10.0 LTS, LLVMSharp 20.1.2 ↔ LLVM
    20.1.x). Toolchain smoke test under `tools/smoke/`.
  - Compiler skeleton split across two projects tied together by `Suru.slnx`:
    `src/Suru.Compiler` (class library; placeholder `Compiler.Compile(...)`) and
    `src/Suru.Cli` (executable, `AssemblyName=suru`) dispatching the `run`, `ir`,
    `build`, and `test` subcommands. xUnit tests in `src/Suru.Compiler.Tests`.
  - `scripts/` wrappers (`build`, `test`, `run`, `ir`, `fmt`, `lint`) that drive
    the CLI inside the container, and a one-command quickstart in `README.md`.
  - `.gitignore` and `.dockerignore` covering `bin/`, `obj/`, and the host-owned
    dev cache `.cache/`.