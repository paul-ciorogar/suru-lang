# Suru Lang

A new programming language built with Rust and LLVM.

## Overview

Suru Lang is a programming language compiler project that uses:
- **Rust** (edition 2024) for the compiler implementation
- **LLVM 18** for code generation and optimization

## Prerequisites

- [Docker](https://docs.docker.com/get-docker/) installed on your system
- Basic familiarity with Docker and command-line tools

## Quick Start

### 1. Build the Docker Image

```bash
docker build -t suru-lang:dev .
```

This will create a development environment with:
- Ubuntu 24.04 LTS
- Rust stable toolchain (edition 2024 support)
- LLVM 18 with full development libraries
- All necessary build tools

**Note**: First build takes 5-10 minutes. Subsequent builds are much faster due to Docker layer caching.

### 2. Run Interactive Development Container

```bash
docker run -it --rm \
  -v $(pwd):/workspace \
  suru-lang:dev
```

This command:
- Mounts your project directory to `/workspace` in the container
- Removes the container when you exit (`--rm`)
- Provides an interactive terminal (`-it`)

### 3. Build and Run Inside Container

Once inside the container:

```bash
# Build the project
cargo build

# Run the project
cargo run

# Run tests
cargo test

# Build for release (optimized)
cargo build --release
```

## Development Workflow

### Option 1: Work Inside the Container

```bash
# Start container
docker run -it --rm -v $(pwd):/workspace suru-lang:dev

# Inside container - edit, build, test
cargo build
cargo test
cargo run
```

### Option 2: Run Commands from Host

```bash
# Build from host machine
docker run --rm -v $(pwd):/workspace suru-lang:dev cargo build

# Run tests from host machine
docker run --rm -v $(pwd):/workspace suru-lang:dev cargo test

# Run your compiler from host machine
docker run --rm -v $(pwd):/workspace suru-lang:dev cargo run -- <args>
```

### Option 3: Use Docker Compose (Optional)

Create a `docker-compose.yml` file for easier management:

```yaml
version: '3.8'
services:
  dev:
    build: .
    image: suru-lang:dev
    volumes:
      - .:/workspace
    stdin_open: true
    tty: true
```

Then use:
```bash
docker-compose run --rm dev
```

## LLVM Integration

The Docker environment is pre-configured for LLVM development. The following environment variables are set:

```bash
LLVM_SYS_180_PREFIX=/usr/lib/llvm-18
PATH=/usr/lib/llvm-18/bin:...
LD_LIBRARY_PATH=/usr/lib/llvm-18/lib:...
```

### Adding LLVM Bindings

To use LLVM in your Rust code, add one of these to your `Cargo.toml`:

#### Option 1: Inkwell (Recommended for beginners)

```toml
[dependencies]
inkwell = { version = "0.6", features = ["llvm18-1"] }
```

Inkwell provides a safe, high-level Rust API for LLVM.

#### Option 2: llvm-sys (Low-level bindings)

```toml
[dependencies]
llvm-sys = "180"
```

Provides direct bindings to LLVM C API for maximum control.

### Verifying LLVM Installation

Inside the container:

```bash
# Check LLVM version
llvm-config-18 --version

# Check available LLVM tools
clang-18 --version
llc-18 --version
opt-18 --version

# Check LLVM library path
llvm-config-18 --libdir
```

## Useful Commands

### Inside Container

```bash
# Check Rust version
rustc --version

# Check cargo version
cargo --version

# Format code
cargo fmt

# Run clippy (linter)
cargo clippy

# Generate documentation
cargo doc --open

# Clean build artifacts
cargo clean
```

### LLVM Tools

```bash
# Compile LLVM IR
llc-18 <file.ll> -o <output.o>

# Optimize LLVM IR
opt-18 <file.ll> -o <optimized.ll>

# View LLVM IR
llvm-dis-18 <file.bc>

# Compile C/C++ to LLVM IR
clang-18 -S -emit-llvm <file.c> -o <file.ll>
```

## Project Structure

```
suru-lang/
├── Cargo.toml          # Rust project manifest
├── Cargo.lock          # Dependency lock file
├── Dockerfile          # Development environment definition
├── .dockerignore       # Files excluded from docker build
├── README.md           # This file
└── src/
    └── main.rs         # Main entry point
```

## Building for Production

When you're ready to build an optimized binary:

```bash
# Inside container
cargo build --release

# The binary will be in target/release/suru-lang
./target/release/suru-lang
```

## Troubleshooting

### Docker Build Fails

**Issue**: "Cannot connect to the Docker daemon"
**Solution**: Ensure Docker is running: `sudo systemctl start docker`

### LLVM Not Found

**Issue**: Cargo build fails with "could not find LLVM"
**Solution**: The environment variables should be set automatically. Verify inside container:
```bash
echo $LLVM_SYS_180_PREFIX
which llvm-config-18
```

### Permission Issues

**Issue**: Cannot write to `/workspace` inside container or files created have wrong ownership
**Solution**: The container runs as user `rustuser`. If you encounter permission issues, you can run the container as your host user:
```bash
docker run -it --rm -u $(id -u):$(id -g) \
  -v $(pwd):/workspace \
  suru-lang:dev
```

**Note**: When running with `-u $(id -u):$(id -g)`, cargo may have issues with the home directory. In that case, set `CARGO_HOME`:
```bash
docker run -it --rm -u $(id -u):$(id -g) \
  -e CARGO_HOME=/workspace/.cargo \
  -v $(pwd):/workspace \
  suru-lang:dev
```

### Slow Builds

**Issue**: Cargo builds take a long time
**Solution**: Use Docker volumes to persist cargo cache:
```bash
docker run -it --rm \
  -v $(pwd):/workspace \
  -v suru-cargo-registry:/home/rustuser/.cargo/registry \
  -v suru-cargo-git:/home/rustuser/.cargo/git \
  suru-lang:dev
```

## Advanced Usage

### Persistent Development Container

Instead of recreating the container each time, create a long-running container:

```bash
# Create and start container
docker run -d --name suru-dev \
  -v $(pwd):/workspace \
  suru-lang:dev \
  sleep infinity

# Execute commands in the container
docker exec -it suru-dev cargo build
docker exec -it suru-dev cargo test

# Get a shell in the container
docker exec -it suru-dev /bin/bash

# Stop and remove when done
docker stop suru-dev
docker rm suru-dev
```

### Using BuildKit for Faster Builds

```bash
DOCKER_BUILDKIT=1 docker build \
  --progress=plain \
  -t suru-lang:dev .
```

## Contributing

When contributing to this project:
1. Make sure your code builds: `cargo build`
2. Run tests: `cargo test`
3. Format code: `cargo fmt`
4. Check for lints: `cargo clippy`

All of these can be run inside the Docker container.

## License

[Add your license here]

## Resources

- [Rust Documentation](https://doc.rust-lang.org/)
- [LLVM Documentation](https://llvm.org/docs/)
- [Inkwell Documentation](https://thedan64.github.io/inkwell/)
- [Docker Documentation](https://docs.docker.com/)
