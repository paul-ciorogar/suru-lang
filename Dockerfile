# Dockerfile for Rust + LLVM 18 Development Environment
# Purpose: Development environment for suru-lang-rs compiler project

FROM ubuntu:24.04

# =============================================================================
# Section A: Base Image and System Dependencies
# =============================================================================

# Prevent interactive prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive
ENV TZ=UTC

# Install essential build tools and prerequisites
RUN apt-get update && apt-get install -y \
    # Core utilities
    curl \
    wget \
    git \
    # Build essentials
    build-essential \
    pkg-config \
    libssl-dev \
    # LLVM repository prerequisites
    ca-certificates \
    gnupg \
    lsb-release \
    software-properties-common \
    && rm -rf /var/lib/apt/lists/*

# =============================================================================
# Section B: Install LLVM 18
# =============================================================================

# Add official LLVM repository for Ubuntu 24.04 (Noble)
RUN wget -O - https://apt.llvm.org/llvm-snapshot.gpg.key | apt-key add - && \
    echo "deb http://apt.llvm.org/noble/ llvm-toolchain-noble-18 main" > /etc/apt/sources.list.d/llvm.list && \
    echo "deb-src http://apt.llvm.org/noble/ llvm-toolchain-noble-18 main" >> /etc/apt/sources.list.d/llvm.list

# Install LLVM 18 packages
RUN apt-get update && apt-get install -y \
    llvm-18 \
    llvm-18-dev \
    llvm-18-runtime \
    llvm-18-tools \
    clang-18 \
    libclang-18-dev \
    liblld-18-dev \
    libpolly-18-dev \
    && rm -rf /var/lib/apt/lists/*

# Set LLVM environment variables
# These ensure Rust LLVM bindings (inkwell, llvm-sys) can find LLVM
ENV LLVM_SYS_180_PREFIX=/usr/lib/llvm-18
ENV PATH="/usr/lib/llvm-18/bin:${PATH}"
ENV LD_LIBRARY_PATH="/usr/lib/llvm-18/lib:${LD_LIBRARY_PATH}"

# =============================================================================
# Section C: Install Rust Stable Toolchain
# =============================================================================

# Install Rust using rustup (official installer)
# Stable toolchain supports edition 2024 as of Rust 1.85+
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    --default-toolchain stable \
    --profile default

# Add Rust to PATH
ENV PATH="/root/.cargo/bin:${PATH}"
ENV CARGO_HOME=/root/.cargo
ENV RUSTUP_HOME=/root/.rustup

# Verify installations
RUN rustc --version && \
    cargo --version && \
    llvm-config-18 --version

# =============================================================================
# Section D: Workspace Setup
# =============================================================================

# Create workspace directory
RUN mkdir -p /workspace

# Run as root to avoid permission issues with mounted volumes
WORKDIR /workspace

# =============================================================================
# Section E: Cache Cargo Dependencies
# =============================================================================

# Copy only Cargo manifest files first to cache dependencies layer
COPY Cargo.toml Cargo.lock* ./

# Create a dummy source file to build dependencies
RUN mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    echo "pub mod codegen;" >> src/main.rs && \
    echo "pub mod lexer;" >> src/main.rs && \
    mkdir -p src && \
    echo "pub fn generate_hello_world() -> Result<(), Box<dyn std::error::Error>> { Ok(()) }" > src/codegen.rs && \
    echo "// lexer module" > src/lexer.rs

# Build dependencies (this layer will be cached unless Cargo.toml changes)
RUN cargo build --release && \
    cargo build && \
    rm -rf src target

# Copy actual source code
COPY . .

# Default command: interactive bash shell for development
CMD ["/bin/bash"]
