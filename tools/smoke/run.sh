#!/usr/bin/env bash
#
# Toolchain smoke test — meant to run INSIDE the suru-dev container.
#
# Verifies the dev environment end to end:
#   1. the expected binaries are present and report the pinned versions
#      (dotnet SDK, clang, ld.lld, llvm-config), and the LLVM major matches;
#   2. LLVMSharp restores under the pinned SDK and can load the native libLLVM
#      and call the LLVM-C API (the Smoke project in this directory).
#
# From the host (repo mounted at /workspace by the Dockerfile's WORKDIR):
#   docker run --rm -v "$PWD":/workspace suru-dev bash tools/smoke/run.sh
# Later this is reachable via `docker compose run --rm dev …` once compose exists.
set -euo pipefail

# The LLVM major the whole toolchain is pinned to (see docs/toolchain.md).
LLVM_MAJOR=20

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "== .NET SDK =="
dotnet --info | sed -n '1,6p'

echo "== clang =="
clang --version | head -n1

echo "== lld =="
ld.lld --version

echo "== llvm-config =="
llvm_ver="$(llvm-config --version)"
echo "llvm-config: ${llvm_ver}"
case "${llvm_ver}" in
    ${LLVM_MAJOR}.*) ;;
    *) echo "ERROR: expected LLVM ${LLVM_MAJOR}.x but found ${llvm_ver}" >&2; exit 1;;
esac

echo "== LLVMSharp binding load =="
# Build and run in a throwaway temp dir so we never write bin/obj into the
# mounted host repo. The project is self-contained (just .csproj + Program.cs).
work="$(mktemp -d)"
trap 'rm -rf "${work}"' EXIT
cp "${here}/Smoke.csproj" "${here}/Program.cs" "${work}/"
dotnet run --project "${work}/Smoke.csproj" -c Release

echo "ALL SMOKE CHECKS PASSED"
