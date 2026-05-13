#!/usr/bin/env bash
set -euo pipefail

docker run --rm -i \
  -v "$(pwd):/work" \
  suru \
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
DOCKER_EOF
