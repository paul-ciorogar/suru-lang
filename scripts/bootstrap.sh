#!/usr/bin/env bash
set -euo pipefail

docker run --rm \
  -v "$(pwd):/work" \
  suru \
  sh -c '
    set -e

    # Step 1: compile the compiler source with the current binary
    suru-build /work/src/compiler/build.suru /tmp/new-compiler.ll
    clang-18 /tmp/new-compiler.ll /usr/local/lib/suru/runtime/*.ll -o /tmp/new-suru-build

    # Step 2: run arithmetic with old binary
    suru-build /work/tests/fixtures/arithmetic/main.suru /tmp/arith-old.ll
    clang-18 /tmp/arith-old.ll /usr/local/lib/suru/runtime/*.ll -o /tmp/arith-old
    /tmp/arith-old > /tmp/arith-old.out

    # Step 3: run arithmetic with new binary
    /tmp/new-suru-build /work/tests/fixtures/arithmetic/main.suru /tmp/arith-new.ll
    clang-18 /tmp/arith-new.ll /usr/local/lib/suru/runtime/*.ll -o /tmp/arith-new
    /tmp/arith-new > /tmp/arith-new.out

    # Step 4: compare outputs
    if diff /tmp/arith-old.out /tmp/arith-new.out > /dev/null; then
      echo "Round-trip verified: outputs match"
      cp /tmp/new-suru-build /work/bin/suru-build
      echo "bin/suru-build updated"
    else
      echo "ERROR: round-trip FAILED — outputs differ"
      diff /tmp/arith-old.out /tmp/arith-new.out
      exit 1
    fi
  '
