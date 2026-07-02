# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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