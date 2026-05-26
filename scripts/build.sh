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
docker compose run --rm suru suru build /work/${SOURCE}
printf "=== total: %s ===\n" "$(_fmt_ms "$(( $(date +%s%3N) - $_t0 ))")"
