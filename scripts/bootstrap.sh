#!/usr/bin/env bash
set -euo pipefail

docker compose run --rm suru \
  bash << 'DOCKER_EOF'
set -euo pipefail
RUNTIME="/usr/local/lib/suru/runtime"

mkdir -p /tmp/new-compiler
suru-build /work/src/compiler/build.suru /tmp/new-compiler/build.ll
clang-18 /tmp/new-compiler/*.ll "${RUNTIME}"/*.ll -o /tmp/new-suru-build

cp /tmp/new-suru-build /work/bin/suru-build
echo "bin/suru-build updated"
DOCKER_EOF
