#!/usr/bin/env bash
set -euo pipefail

docker run --rm -i \
  -v "$(pwd):/work" \
  suru \
  bash << 'DOCKER_EOF'
set -euo pipefail
PASS=0
FAIL=0
FAIL_NAMES=()
RUNTIME="/usr/local/lib/suru/runtime"

run_test() {
  local name="$1"; shift
  local extra_args=("$@")
  local source="/work/tests/fixtures/${name}/main.suru"
  local expected="/work/tests/fixtures/${name}/expected.txt"
  local ll="/tmp/test_${name}.ll"
  local bin="/tmp/test_${name}"

  if ! suru-build "$source" "$ll" 2>/tmp/err_${name}.txt; then
    echo "FAIL ${name} (compile error)"
    FAIL=$((FAIL + 1)); FAIL_NAMES+=("$name"); return
  fi
  if ! clang-18 "$ll" "${RUNTIME}"/*.ll -o "$bin" 2>/tmp/lnk_${name}.txt; then
    echo "FAIL ${name} (link error)"
    FAIL=$((FAIL + 1)); FAIL_NAMES+=("$name"); return
  fi

  actual=$("$bin" "${extra_args[@]}" 2>/dev/null || true)
  if [ "$actual" = "$(cat "$expected")" ]; then
    echo "PASS ${name}"
    PASS=$((PASS + 1))
  else
    echo "FAIL ${name}"
    diff <(cat "$expected") <(echo "$actual") | head -20
    FAIL=$((FAIL + 1)); FAIL_NAMES+=("$name")
  fi
}

run_test arithmetic
run_test fibonacci
run_test while-loop
run_test strings
run_test sum-types
run_test comparisons
run_test arrays
run_test types
run_test file_io /work/tests/fixtures/file_io/input.txt

echo ""
echo "Results: ${PASS} passed, ${FAIL} failed"
[ "${FAIL}" -eq 0 ]
DOCKER_EOF
