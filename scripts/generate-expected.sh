#!/usr/bin/env bash
set -euo pipefail

docker run --rm -i \
  -v "$(pwd):/work" \
  suru \
  bash << 'DOCKER_EOF'
set -euo pipefail
RUNTIME="/usr/local/lib/suru/runtime"

capture() {
  local name="$1"; shift
  local extra_args=("$@")
  local source="/work/tests/fixtures/${name}/main.suru"
  local out_ll="/tmp/cap_${name}.ll"
  local out_bin="/tmp/cap_${name}"
  local expected="/work/tests/fixtures/${name}/expected.txt"

  echo "capturing ${name}..."
  suru-build "$source" "$out_ll"
  clang-18 "$out_ll" "${RUNTIME}"/*.ll -o "$out_bin"
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
DOCKER_EOF
