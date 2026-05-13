#!/usr/bin/env bash
set -euo pipefail

docker compose run --rm suru \
  bash << 'DOCKER_EOF'
set -euo pipefail
RUNTIME="/usr/local/lib/suru/runtime"

# Step 1: compile a fresh compiler from source using the bootstrap binary
echo "--- Compiling fresh compiler from source ---"
mkdir -p /tmp/new-compiler
suru-build /work/src/compiler/build.suru /tmp/new-compiler/build.ll
clang-18 /tmp/new-compiler/*.ll "${RUNTIME}"/*.ll -o /tmp/new-suru-build

# Step 2: compile the test runner using the stable bootstrap binary
echo "--- Compiling test runner ---"
mkdir -p /tmp/runner
suru-build /work/tests/runner/main.suru /tmp/runner/main.ll
clang-18 /tmp/runner/main.ll "${RUNTIME}"/*.ll -o /tmp/runner/main

# Step 3: run the test runner with the fresh compiler first on PATH
echo "--- Running tests ---"
mkdir -p /tmp/fresh-bin
ln -sf /tmp/new-suru-build /tmp/fresh-bin/suru-build
export PATH="/tmp/fresh-bin:${PATH}"
/tmp/runner/main
DOCKER_EOF
