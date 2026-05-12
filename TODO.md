# Suru — Suru-Native Development

C# is frozen as the bootstrap compiler (released as v0.1.0).
All new work happens in this repo. Each stage ends with a testable milestone.

---

## Stage 2 — New Repo Structure (~1-2h) ✓

Create a clean `suru` repository with a proper directory layout.
Move all Suru source files from the archived bootstrap repo and update include paths.

- [x] Copy Suru source files into the directory structure (camelCase, no `suru-` prefix)
- [x] Update all `include` paths to match the new relative layout
- [x] Copy `bin/suru-build` from the v0.1.0-bootstrap release
- [x] Copy runtime `.ll` files into `runtime/`
- [x] Move `compiler/` into `src/` (anticipating future `src/cli/`, `src/lsp/`)
- [x] Verify: `./bin/suru-build src/compiler/build.suru /tmp/test.ll` exits 0
- [x] **Milestone:** new repo is self-contained; can compile the Suru compiler from its own source

---

## Stage 3 — Docker & Build Scripts (~1-2h) ✓

A `Dockerfile` with the full LLVM toolchain and three shell scripts.
All build/test/bootstrap operations run inside the container — no host toolchain needed.

- [x] `Dockerfile`:
  - Base: `ubuntu:24.04`
  - Install: `clang-18 llvm-18 lld-18`, `git`
  - Copy `bin/suru-build` to `/usr/local/bin/`
  - Copy `runtime/` to `/usr/local/lib/suru/runtime/`
  - `WORKDIR /work`
- [x] `scripts/build.sh <source.suru> <out-dir>`:
  runs `docker run --rm -v $(pwd):/work suru suru-build /work/<source> /work/<out-dir>/main.ll`
  then links `/work/<out-dir>/main.ll` + all runtimes → `/work/<out-dir>/main`
- [x] `scripts/test.sh`:
  runs tests inside Docker; see Stage 5
- [x] `scripts/bootstrap.sh`:
  inside Docker, uses current `suru-build` to compile `src/compiler/build.suru`;
  links result; verifies round-trip (new binary produces same output as old binary on arithmetic);
  replaces `bin/suru-build` on success
- [x] `docker build -t suru .` documented in README
- [x] **Milestone:** `./scripts/build.sh tests/fixtures/arithmetic/main.suru build/` → correct output inside Docker

---

## Stage 4 — Clean Documentation (~1h)

Remove all stage-by-stage historical references from code comments.
The new repo has forward-looking docs only.

- [x] `README.md`: fresh write — language overview, getting started with Docker, table of contents,
      link to v0.1.0-bootstrap release for bootstrap context; no stage history
- [x] `CLAUDE.md`: rewrite — current architecture only (lexer → parser → semantic → codegen);
      no per-stage narrative
- [x] `CHANGELOG.md`: starts at `v1.0 — Self-Hosting Achieved`;
      no per-stage entries; only user-visible changes going forward
- [x] **Milestone:** docs describe the current state of the language, not its construction history

---

## Stage 5 — Shell Test Harness (~1h) ✓

`scripts/test.sh` exercises the full fixture corpus inside Docker.

- [x] `scripts/test.sh`: inside Docker, for each (fixture, expected-output) pair:
      compile with `suru-build`, link, run, compare stdout; print `PASS` / `FAIL`;
      exit 1 on any failure
- [x] `scripts/generate-expected.sh`: one-time helper to capture expected outputs into `tests/fixtures/*/expected.txt`
- [x] Corpus: arithmetic, fibonacci, while-loop, strings, sum-types, comparisons, arrays, types, file_io
- [x] **Milestone:** `./scripts/test.sh` → all PASS inside Docker

---

## Stage 6 — `exec(cmd String) Int64` built-in (~1-2h)

First language feature added in pure Suru — no C# changes.
`exec()` runs a shell command and returns its exit code.

- [x] `compiler/codegen/irCodegen.suru`: handle `exec` `CallNode` in `emitValue` —
      extract String data ptr, emit `call i32 @system(ptr %data)`, `sext i32 to i64`;
      add `declare i32 @system(ptr)` to extern decls
- [x] `compiler/semantic/exprs.suru`: whitelist `exec` as a known built-in call
- [x] Rebuild `bin/suru-build` via `./scripts/bootstrap.sh`
- [x] New fixture `tests/fixtures/exec-test/main.suru`:
      calls `exec("echo hello")`, prints the exit code with `printLn`
- [x] Add `exec-test` to `scripts/test.sh`
- [x] **Milestone:** `exec()` works; `./scripts/test.sh` still all PASS; zero C# was modified

---

## Stage 7 — Test runner in Suru (~2h)

A Suru program that compiles and runs test cases using `exec()` + `readFile()`,
replacing `scripts/test.sh` with a program written in the language itself.

- [ ] `tests/runner/main.suru`:
  - `type TestCase: { name String, source String, expected String }`
  - For each test case: `exec("./scripts/build.sh <source> /tmp/<name>")` to compile+link;
    `exec("/tmp/<name>/main > /tmp/<name>.out 2>&1")` to run with output capture;
    `readFile("/tmp/<name>.out")` to read; compare with `expected`;
    print `PASS: <name>` or `FAIL: <name>`; `exit(1)` if any FAIL
- [ ] Compile runner: `./scripts/build.sh tests/runner/main.suru build/`
- [ ] Update `scripts/test.sh` to run `./build/main` (the compiled Suru runner) instead of shell logic
- [ ] **Milestone:** `./scripts/test.sh` runs the corpus from a compiled Suru program → all PASS

---

Each stage builds on the last and ends with a working, testable milestone.
All development is in Suru; the Docker image provides the full toolchain.
