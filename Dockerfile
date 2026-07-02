# syntax=docker/dockerfile:1
#
# Suru development image.
#
# A single, self-contained build/dev environment for the compiler: the .NET SDK
# plus a matching native LLVM toolchain. Everything the compiler needs to build,
# emit IR, and link a binary lives here, so the only host dependency is Docker.
#
# Versions are pinned in docs/toolchain.md — keep this file in sync with it:
#   .NET SDK   10.0 (LTS)            -> base image tag below
#   LLVMSharp  20.1.2 (managed)      -> referenced from the .csproj (added later)
#   LLVM       20.1.x (native)       -> apt packages below; major MUST equal the
#                                       LLVMSharp major or the binding fails to load
#
# The base image mcr.microsoft.com/dotnet/sdk:10.0 is Ubuntu 24.04 LTS, whose
# stock apt repos already ship LLVM 20.1.2 — so no third-party apt.llvm.org repo
# is needed (see docs/toolchain.md, "Base image + LLVM availability").

FROM mcr.microsoft.com/dotnet/sdk:10.0

# The LLVM major we depend on. Bump this together with the LLVMSharp version in
# the .csproj and the table in docs/toolchain.md — never independently.
ARG LLVM_MAJOR=20

# Install the native LLVM 20 toolchain from Ubuntu's stock repos:
#   libllvm20  - the native libLLVM shared object that LLVMSharp dlopen's
#   clang-20   - link driver: turns emitted object files into an executable
#   lld-20     - the linker (ld.lld) that clang drives
#   llvm-20    - tools (llc/opt/llvm-config) used for IR inspection and `ir`/`run`
# --no-install-recommends keeps the image lean; the apt lists are removed after
# install so they don't bloat the layer.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        "libllvm${LLVM_MAJOR}" \
        "clang-${LLVM_MAJOR}" \
        "lld-${LLVM_MAJOR}" \
        "llvm-${LLVM_MAJOR}" \
    && rm -rf /var/lib/apt/lists/*

# Make the LLVM *tools* resolvable without the version suffix. The llvm-20
# package installs unversioned entry points under /usr/lib/llvm-20/bin
# (clang, ld.lld, llc, opt, llvm-config, ...), so put that dir on PATH — scripts
# and the compiler can then invoke `clang`/`llvm-config` without the `-20`
# suffix. PATH is inherited from the base image, so prepend to it.
ENV PATH=/usr/lib/llvm-${LLVM_MAJOR}/bin:${PATH}

# Make the native *library* loadable by LLVMSharp without LD_LIBRARY_PATH.
#
# LLVMSharp's P/Invoke is DllImport("libLLVM") -> dlopen("libLLVM.so"), so it
# needs a plain, UNVERSIONED libLLVM.so. The runtime libllvm20 package, with
# --no-install-recommends, ships only VERSIONED names on the default multiarch
# path (libLLVM-${LLVM_MAJOR}.so and libLLVM.so.${ver}); the unversioned symlink
# otherwise ships only in the heavy llvm-${LLVM_MAJOR}-dev package, which we do
# not want. So create the symlink ourselves, right in the multiarch dir that the
# dynamic loader already searches, and refresh the ld cache. No LD_LIBRARY_PATH
# is then required — dlopen("libLLVM.so") resolves from the standard path.
# (Verified during development with a real dlopen + dlsym("LLVMContextCreate").)
RUN set -eux; \
    libdir="$(dirname "$(find /usr/lib -name "libLLVM-${LLVM_MAJOR}.so" | head -1)")"; \
    ln -sf "libLLVM-${LLVM_MAJOR}.so" "${libdir}/libLLVM.so"; \
    ldconfig

# Fail the build loudly if the base image ever drifts off the pinned LLVM major,
# rather than discovering a binding load failure at runtime. Also surface the
# concrete versions in the build log for traceability.
RUN set -eux; \
    clang --version; \
    ld.lld --version; \
    llvm_ver="$(llvm-config --version)"; \
    echo "llvm-config: ${llvm_ver}"; \
    case "${llvm_ver}" in \
        ${LLVM_MAJOR}.*) echo "LLVM major matches pin (${LLVM_MAJOR})";; \
        *) echo "ERROR: expected LLVM ${LLVM_MAJOR}.x but found ${llvm_ver}" >&2; exit 1;; \
    esac

# Repo is mounted here by docker-compose (added in a later task); host edits then
# compile inside the container.
WORKDIR /workspace
