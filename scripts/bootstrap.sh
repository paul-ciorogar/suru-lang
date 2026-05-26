#!/usr/bin/env bash
set -euo pipefail

# Self-hosting bootstrap with full verification.
#
# A 3-stage build proves a true self-hosting fixed point:
#   C1 = current bin/suru-build  builds source
#   C2 = C1                      builds source   (first binary on NEW codegen)
#   C3 = C2                      builds source
# C1 may legitimately differ from C2 when the compiler's own codegen changed;
# the invariant that must hold is C2 == C3 (the new codegen reproduces itself
# byte-for-byte). The full test suite is run with C2.
#
# Only if the suite passes AND C2 == C3 is bin/suru-build replaced (with C2).
# This prevents silently committing a binary that mis-handles its own source —
# the failure mode that let regressions ride forward across multiple commits.

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

# Clean /tmp/ artifacts and stale build outputs from previous runs. Without
# this, a broken candidate can appear to pass tests because leftover binaries
# from earlier runs get re-executed when the new compile silently fails (the
# runner masks compile errors with `2>/dev/null`).
rm -rf /tmp/suru-test-* /tmp/candidate-* /tmp/cstage-* /work/src/cli/build \
       /work/tests/runner/build /work/tests/fixtures/*/build

build_self() {  # $1 = output path
    rm -rf /work/src/cli/build
    suru build /work/src/cli/main.suru
    test -x /work/src/cli/build/main \
        || { echo "bootstrap: build produced no binary" >&2; exit 1; }
    cp /work/src/cli/build/main "$1"
}

# ── Stage 1: current bin/suru-build → C1 ────────────────────────────────────
echo "--- Stage 1: building C1 with current bin/suru-build ---"
build_self /tmp/cstage-c1
_step_done

# ── Stage 2: C1 → C2 (first binary on the candidate's own codegen) ──────────
echo "--- Stage 2: building C2 with C1 ---"
mkdir -p /tmp/candidate-bin
ln -sf /tmp/cstage-c1 /tmp/candidate-bin/suru
ln -sf /tmp/cstage-c1 /tmp/candidate-bin/suru-build
export PATH="/tmp/candidate-bin:${PATH}"
build_self /tmp/cstage-c2
_step_done

# ── Stage 3: C2 → C3, must equal C2 (self-hosting fixed point) ──────────────
echo "--- Stage 3: building C3 with C2 (fixed-point check) ---"
ln -sf /tmp/cstage-c2 /tmp/candidate-bin/suru
ln -sf /tmp/cstage-c2 /tmp/candidate-bin/suru-build
build_self /tmp/cstage-c3
if ! cmp -s /tmp/cstage-c2 /tmp/cstage-c3; then
    echo "bootstrap: NOT a self-hosting fixed point — bin/suru-build NOT replaced" >&2
    echo "  C2 md5: $(md5sum /tmp/cstage-c2 | awk '{print $1}')" >&2
    echo "  C3 md5: $(md5sum /tmp/cstage-c3 | awk '{print $1}')" >&2
    exit 1
fi
echo "fixed point confirmed: C2 == C3"
_step_done

# ── Full test suite with the converged compiler (C2) ───────────────────────
echo "--- Running full test suite with C2 ---"
ln -sf /tmp/cstage-c2 /tmp/candidate-bin/suru
ln -sf /tmp/cstage-c2 /tmp/candidate-bin/suru-build
suru build /work/tests/runner/main.suru
if ! /work/tests/runner/build/main; then
    echo "bootstrap: candidate failed the test suite — bin/suru-build NOT replaced" >&2
    exit 1
fi
_step_done

# ── Replace bin/suru-build with the converged compiler ─────────────────────
cp /tmp/cstage-c2 /work/bin/suru-build
echo "bin/suru-build updated ($(md5sum /work/bin/suru-build | awk '{print $1}'))"
_total
DOCKER_EOF
