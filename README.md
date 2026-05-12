# Suru

A minimalist, library-driven, general-purpose programming language — statically typed, no garbage collection, self-hosted.

## Table of contents

- [Status](#status)
- [Getting started](#getting-started)
- [Repository layout](#repository-layout)
- [Language overview](#language-overview)
- [Bootstrap context](#bootstrap-context)

## Status

Suru is self-hosted: the compiler (`src/compiler/build.suru`) is written in Suru and compiles itself.
The bootstrap binary was compiled from the frozen C# compiler released as [v0.1.0-bootstrap](https://github.com/paul-ciorogar/suru-lang-bootstrap/releases/tag/v0.1.0). See [BOOTSTRAP.md](BOOTSTRAP.md) for details.

## Getting started

Build the Docker image (requires Docker):

```sh
docker build -t suru .
```

Compile and run a program:

```sh
./scripts/build.sh tests/fixtures/arithmetic/main.suru build/
./build/main
```

Bootstrap a new compiler binary from source:

```sh
./scripts/bootstrap.sh
```

## Repository layout

```
src/
  compiler/       Suru compiler source (lexer → parser → semantic → codegen)
    lexer/
    parser/
    semantic/
    codegen/
runtime/          LLVM IR runtime modules (box, string, array, struct, variant)
tests/fixtures/   Corpus programs used for validation
scripts/          build.sh, test.sh, bootstrap.sh
bin/              Bootstrap binary (suru-build)
Dockerfile        Ubuntu 24.04 image with clang-18/llvm-18/lld-18
```

## Language overview

- Entry point: `fn main(args Array<String>)`
- Variables: `let name Type: value` (type annotation mandatory)
- Types: `Bool`, `Int32`, `Int64`, `Float64`, `String`, `Array<T>`, named structs, sum types
- Control flow: `while`, `match` (statement and expression forms)
- No GC: explicit `clone` / `drop` for heap values
- Cross-file includes: `include "path/file.suru" as ns`

## Bootstrap context

The first Suru compiler was written in C# and released as
[v0.1.0-bootstrap](https://github.com/paul-ciorogar/suru-lang-bootstrap/releases/tag/v0.1.0).
That compiler's output was used as the seed to compile the Suru compiler from its own source.
The C# compiler is frozen and archived in the bootstrap repository.
