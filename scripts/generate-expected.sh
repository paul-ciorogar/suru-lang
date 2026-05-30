#!/usr/bin/env bash
set -euo pipefail

docker compose run --rm suru \
  bash << 'DOCKER_EOF'
set -euo pipefail

_t0=$(date +%s%3N)

_fmt_ms() {
    local ms=$1
    if (( ms < 1000 )); then
        printf "%dms" "$ms"
    elif (( ms < 60000 )); then
        printf "%d:%03d" "$(( ms / 1000 ))" "$(( ms % 1000 ))"
    else
        printf "%d:%02d:%03d" "$(( ms / 60000 ))" "$(( (ms % 60000) / 1000 ))" "$(( ms % 1000 ))"
    fi
}
_elapsed() { _fmt_ms "$(( $(date +%s%3N) - $1 ))"; }
_total()    { printf "=== total: %s ===\n" "$(_elapsed $_t0)"; }

capture() {
  local name="$1"; shift
  local extra_args=("$@")
  local source="/work/tests/fixtures/${name}/main.suru"
  local out_bin="/work/tests/fixtures/${name}/build/main"
  local expected="/work/tests/fixtures/${name}/expected.txt"
  local _ft; _ft=$(date +%s%3N)

  echo "capturing ${name}..."
  suru build "$source"
  "$out_bin" "${extra_args[@]}" > "$expected"
  printf "  wrote %s  [%s]\n" "$expected" "$(_elapsed $_ft)"
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
capture generics-box-basic
capture generics-fn-basic
capture generics-type-basic
capture generics-multi-param
capture generics-nested
capture generics-cross-module
capture generics-advanced
capture generics-monoinfer-unit
capture generics-monoinstantiate-unit
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
  local _ft; _ft=$(date +%s%3N)

  echo "capturing ${name} (combined stdout+stderr)..."
  suru build "$source"
  "$out_bin" "${extra_args[@]}" > "$expected" 2>&1 || true
  printf "  wrote %s  [%s]\n" "$expected" "$(_elapsed $_ft)"
}

capture_combined builtins-misc

# compileError fixtures: capture the compiler's diagnostic output
capture_compile_error() {
  local name="$1"
  local source="/work/tests/fixtures/${name}/main.suru"
  local ll="/tmp/suru-gen-${name}.ll"
  local expected="/work/tests/fixtures/${name}/expected.txt"
  local _ft; _ft=$(date +%s%3N)

  echo "capturing ${name} (compile error)..."
  suru compile "$source" "$ll" > "$expected" 2>&1 || true
  printf "  wrote %s  [%s]\n" "$expected" "$(_elapsed $_ft)"
}

capture_compile_error lex-error
capture_compile_error parse-error
capture_compile_error dup-type-error
capture_compile_error struct-missing-field-error
capture_compile_error namespace-export-error
capture_compile_error namespace-import-error
capture_compile_error namespace-error-missing
_total
DOCKER_EOF
