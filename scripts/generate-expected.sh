#!/usr/bin/env bash
set -euo pipefail

docker compose run --rm suru \
  bash << 'DOCKER_EOF'
set -euo pipefail

capture() {
  local name="$1"; shift
  local extra_args=("$@")
  local source="/work/tests/fixtures/${name}/main.suru"
  local out_bin="/work/tests/fixtures/${name}/build/main"
  local expected="/work/tests/fixtures/${name}/expected.txt"

  echo "capturing ${name}..."
  suru build "$source"
  "$out_bin" "${extra_args[@]}" > "$expected"
  echo "  wrote ${expected}"
}

capture arithmetic
capture fibonacci
capture while-loop
capture strings
capture sum-types
capture comparisons
capture arrays
capture types
capture file_io /work/tests/fixtures/file_io/input.txt

# New fixtures
capture char
capture bool-ops
capture sum-types-dispatch
capture generics-advanced
capture array-advanced
capture string-advanced
capture float-match
capture file-io-write

# builtins-misc: capture stdout+stderr combined (runner uses 2>&1)
capture_combined() {
  local name="$1"; shift
  local extra_args=("$@")
  local source="/work/tests/fixtures/${name}/main.suru"
  local out_bin="/work/tests/fixtures/${name}/build/main"
  local expected="/work/tests/fixtures/${name}/expected.txt"

  echo "capturing ${name} (combined stdout+stderr)..."
  suru build "$source"
  "$out_bin" "${extra_args[@]}" > "$expected" 2>&1 || true
  echo "  wrote ${expected}"
}

capture_combined builtins-misc
DOCKER_EOF
