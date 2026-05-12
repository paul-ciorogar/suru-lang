# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v1.0 — Self-Hosting Achieved

### Added

- The Suru compiler (`src/compiler/build.suru`) is written in Suru and compiles itself.
  The bootstrap binary was seeded from the frozen C# compiler
  ([v0.1.0-bootstrap](https://github.com/paul-ciorogar/suru-lang-bootstrap/releases/tag/v0.1.0)).
- Docker-based build environment (`Dockerfile`, `scripts/build.sh`, `scripts/bootstrap.sh`).
  All build and bootstrap operations run inside the container — no host toolchain required.
- Language features: `Bool`, `Int32`, `Int64`, `Float64`, `String`, `Array<T>`, named struct types,
  sum types, `match` (statement and expression), `while`, `include`, `clone`/`drop`,
  `readFile`/`writeFile`, `exit`, `printLn`, `printError`.
