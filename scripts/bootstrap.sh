#!/usr/bin/env bash
set -euo pipefail

docker compose run --rm suru \
  bash << 'DOCKER_EOF'
set -euo pipefail

# Compile the CLI from source and replace bin/suru-build.
suru build /work/src/cli/main.suru
cp /work/src/cli/build/main /work/bin/suru-build
echo "bin/suru-build updated"
DOCKER_EOF
