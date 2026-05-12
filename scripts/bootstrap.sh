#!/usr/bin/env bash
set -euo pipefail

docker run --rm -i \
  -v "$(pwd):/work" \
  -v "$(pwd)/bin/suru-build:/usr/local/bin/suru-build" \
  suru \
  bash << 'DOCKER_EOF'
set -euo pipefail
RUNTIME="/usr/local/lib/suru/runtime"

# Step 1: compile compiler source; suru-build emits one .ll per module into the
# output directory (same as the output file's parent).
mkdir -p /tmp/new-compiler
suru-build /work/src/compiler/build.suru /tmp/new-compiler/build.ll
clang-18 /tmp/new-compiler/*.ll "${RUNTIME}"/*.ll -o /tmp/new-suru-build

# Step 2: round-trip verification using arithmetic fixture
suru-build /work/tests/fixtures/arithmetic/main.suru /tmp/arith-old.ll
clang-18 /tmp/arith-old.ll "${RUNTIME}"/*.ll -o /tmp/arith-old
/tmp/arith-old > /tmp/arith-old.out

/tmp/new-suru-build /work/tests/fixtures/arithmetic/main.suru /tmp/arith-new.ll
clang-18 /tmp/arith-new.ll "${RUNTIME}"/*.ll -o /tmp/arith-new
/tmp/arith-new > /tmp/arith-new.out

if diff /tmp/arith-old.out /tmp/arith-new.out > /dev/null; then
  echo "Round-trip verified: outputs match"
  cp /tmp/new-suru-build /work/bin/suru-build
  echo "bin/suru-build updated"
else
  echo "ERROR: round-trip FAILED — outputs differ"
  diff /tmp/arith-old.out /tmp/arith-new.out
  exit 1
fi
DOCKER_EOF
