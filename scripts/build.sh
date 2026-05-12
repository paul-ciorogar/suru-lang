#!/usr/bin/env bash
set -euo pipefail

SOURCE=${1?Usage: build.sh <source.suru> <out-dir>}
OUT_DIR=${2?Usage: build.sh <source.suru> <out-dir>}

mkdir -p "$OUT_DIR"

docker run --rm \
  -v "$(pwd):/work" \
  suru \
  sh -c "suru-build /work/${SOURCE} /work/${OUT_DIR}/main.ll && \
         clang-18 /work/${OUT_DIR}/main.ll /usr/local/lib/suru/runtime/*.ll \
                  -o /work/${OUT_DIR}/main"
