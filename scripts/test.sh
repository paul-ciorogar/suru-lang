#!/usr/bin/env bash
set -euo pipefail

# Build the Suru test runner
./scripts/build.sh tests/runner/main.suru build/runner

# Run the compiled runner inside Docker (uses suru-build + clang-18 directly)
docker run --rm -i \
  -v "$(pwd):/work" \
  -v "$(pwd)/bin/suru-build:/usr/local/bin/suru-build" \
  suru \
  bash -c "cd /work && ./build/runner/main"
