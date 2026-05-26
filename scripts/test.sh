#!/usr/bin/env bash
set -euo pipefail

docker compose run --rm suru \
  bash << 'DOCKER_EOF'
set -euo pipefail

_t0=$(date +%s%3N)
_step_t=$_t0

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
_step_done() { printf "  [elapsed: %s]\n" "$(_elapsed $_step_t)"; _step_t=$(date +%s%3N); }
_total()     { printf "=== total: %s ===\n" "$(_elapsed $_t0)"; }

# Clean any /tmp/ artifacts from previous runs. Stale test binaries there can
# produce false-positive PASSes when a fresh compile silently fails (e.g. when
# the runner's compile step is masked by `2>/dev/null`).
rm -rf /tmp/suru-test-* /tmp/fresh-bin

# Also wipe stale build outputs in the workspace — same hazard.
rm -rf /work/src/cli/build /work/tests/runner/build /work/tests/fixtures/*/build

# Step 1: compile a fresh CLI compiler from source using the bootstrap binary
echo "--- Compiling fresh compiler from source ---"
suru build /work/src/cli/main.suru
_step_done

# Step 2: compile the test runner using the stable bootstrap binary
echo "--- Compiling test runner ---"
suru build /work/tests/runner/main.suru
_step_done

# Step 3: run the test runner with the fresh compiler first on PATH
echo "--- Running tests ---"
mkdir -p /tmp/fresh-bin
ln -sf /work/src/cli/build/main /tmp/fresh-bin/suru
ln -sf /work/src/cli/build/main /tmp/fresh-bin/suru-build
export PATH="/tmp/fresh-bin:${PATH}"
/work/tests/runner/build/main
_step_done
_total
DOCKER_EOF
