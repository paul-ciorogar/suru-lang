#!/usr/bin/env bash
set -euo pipefail

docker compose run --rm suru \
  bash << 'DOCKER_EOF'
set -euo pipefail

# Step 1: compile a fresh CLI compiler from source using the bootstrap binary
echo "--- Compiling fresh compiler from source ---"
suru build /work/src/cli/main.suru

# Step 2: compile the test runner using the stable bootstrap binary
echo "--- Compiling test runner ---"
suru build /work/tests/runner/main.suru

# Step 3: run the test runner with the fresh compiler first on PATH
echo "--- Running tests ---"
mkdir -p /tmp/fresh-bin
ln -sf /work/src/cli/build/main /tmp/fresh-bin/suru
ln -sf /work/src/cli/build/main /tmp/fresh-bin/suru-build
export PATH="/tmp/fresh-bin:${PATH}"
/work/tests/runner/build/main
DOCKER_EOF
