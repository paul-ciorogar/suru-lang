#!/usr/bin/env bash
set -euo pipefail

SOURCE=${1?Usage: build.sh <source.suru>}

docker compose run --rm suru suru build /work/${SOURCE}
