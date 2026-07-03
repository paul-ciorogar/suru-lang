# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

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