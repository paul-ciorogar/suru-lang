#!/usr/bin/env bash
set -euo pipefail

SOURCE=${1?Usage: build.sh <source.suru>}

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

_t0=$(date +%s%3N)
# Compile the C runtime to objects (what `suru build` links), then build the
# program. Both run in one container so the runtime/*.o files are present at link.
docker compose run --rm suru bash -c '
  set -euo pipefail
  for f in /work/runtime/*.c; do
    clang-18 -c -O2 -Wall -Wextra -o "${f%.c}.o" "$f"
  done
  suru build /work/'"${SOURCE}"'
'
printf "=== total: %s ===\n" "$(_fmt_ms "$(( $(date +%s%3N) - $_t0 ))")"
