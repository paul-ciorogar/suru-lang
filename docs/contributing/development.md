# Development Workflow

> Docker-based development environment and build instructions

## Development Environment

All development happens in a Docker containerized environment to ensure consistency.

**Environment:**
- Ubuntu 24.04 LTS
- Rust stable (edition 2024)
- LLVM 18 with full dev libraries
- Inkwell 0.6 (Rust bindings for LLVM)

## Initial Setup

### 1. Install Prerequisites

- [Docker](https://docs.docker.com/get-docker/)
- Git

### 2. Clone Repository

```bash
git clone https://github.com/yourusername/suru-lang-rs.git
cd suru-lang-rs
```

### 3. Build Docker Image

```bash
docker build -t suru-lang:dev .
```

**Note:** First build takes 5-10 minutes. Subsequent builds are faster due to layer caching.

## Development Workflows

### Option 1: Work Inside Container (Recommended)

```bash
# Start interactive container
docker run -it --rm -v $(pwd):/workspace suru-lang:dev

# Inside container
cargo build
cargo test
cargo run -- parse example.suru

# Exit when done
exit
```

### Option 2: Run Commands from Host

```bash
# Build
docker run --rm -v $(pwd):/workspace suru-lang:dev cargo build

# Test
docker run --rm -v $(pwd):/workspace suru-lang:dev cargo test

# Parse file
docker run --rm -v $(pwd):/workspace suru-lang:dev cargo run -- parse file.suru
```

### Option 3: Docker Compose (Optional)

Create `docker-compose.yml`:

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

Usage:

```bash
docker-compose run --rm dev
```

## Common Commands

### Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Clean build artifacts
cargo clean
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_function_declarations

# Run tests with output
cargo test -- --nocapture

# Run tests in specific file
cargo test --test integration_tests
```

### Code Quality

```bash
# Format code
cargo fmt

# Check formatting without changing files
cargo fmt -- --check

# Run linter
cargo clippy

# Run linter with all warnings
cargo clippy -- -W clippy::pedantic
```

### Running

```bash
# Parse a file
cargo run -- parse example.suru

# Show help
cargo run -- --help

# Show parse command help
cargo run -- parse --help
```

### Documentation

```bash
# Generate and open documentation
cargo doc --open

# Generate documentation without opening
cargo doc
```

## LLVM Integration

The Docker environment is pre-configured for LLVM development.

### Environment Variables

```bash
LLVM_SYS_180_PREFIX=/usr/lib/llvm-18
PATH=/usr/lib/llvm-18/bin:...
LD_LIBRARY_PATH=/usr/lib/llvm-18/lib:...
```

### Verifying LLVM Installation

```bash
# Check LLVM version
llvm-config-18 --version

# Check available tools
clang-18 --version
llc-18 --version
opt-18 --version

# Check library path
llvm-config-18 --libdir
```

### LLVM Tools

```bash
# Compile LLVM IR
llc-18 file.ll -o output.o

# Optimize LLVM IR
opt-18 file.ll -o optimized.ll

# View LLVM IR
llvm-dis-18 file.bc

# Compile C/C++ to LLVM IR
clang-18 -S -emit-llvm file.c -o file.ll
```

## Troubleshooting

### Docker Build Fails

**Issue:** "Cannot connect to the Docker daemon"

**Solution:**
```bash
# On Linux
sudo systemctl start docker

# On macOS/Windows
# Start Docker Desktop application
```

### LLVM Not Found

**Issue:** Cargo build fails with "could not find LLVM"

**Solution:** Verify environment variables inside container:
```bash
echo $LLVM_SYS_180_PREFIX
which llvm-config-18
llvm-config-18 --version
```

### Permission Issues

**Issue:** Cannot write to `/workspace` or wrong file ownership

**Solution:** Run container as your host user:
```bash
docker run -it --rm -u $(id -u):$(id -g) \
  -v $(pwd):/workspace \
  suru-lang:dev
```

If cargo has issues:
```bash
docker run -it --rm -u $(id -u):$(id -g) \
  -e CARGO_HOME=/workspace/.cargo \
  -v $(pwd):/workspace \
  suru-lang:dev
```

### Slow Builds

**Issue:** Cargo builds take too long

**Solution:** Use Docker volumes to persist cargo cache:
```bash
docker run -it --rm \
  -v $(pwd):/workspace \
  -v suru-cargo-registry:/home/rustuser/.cargo/registry \
  -v suru-cargo-git:/home/rustuser/.cargo/git \
  suru-lang:dev
```

## Advanced Usage

### Persistent Development Container

Create a long-running container:

```bash
# Create and start
docker run -d --name suru-dev \
  -v $(pwd):/workspace \
  suru-lang:dev \
  sleep infinity

# Execute commands
docker exec -it suru-dev cargo build
docker exec -it suru-dev cargo test

# Get a shell
docker exec -it suru-dev /bin/bash

# Stop and remove
docker stop suru-dev
docker rm suru-dev
```

### Using BuildKit for Faster Builds

```bash
DOCKER_BUILDKIT=1 docker build \
  --progress=plain \
  -t suru-lang:dev .
```

## Development Best Practices

### Before Committing

```bash
# 1. Format code
cargo fmt

# 2. Run linter
cargo clippy

# 3. Run all tests
cargo test

# 4. Build in release mode
cargo build --release
```

### Running Specific Tests

```bash
# Run tests matching pattern
cargo test parse_

# Run tests in a specific module
cargo test lexer::tests::

# Run a single test
cargo test test_parse_function_declaration
```

### Debugging

```bash
# Run with debug output
RUST_LOG=debug cargo run -- parse file.suru

# Run tests with debug output
RUST_LOG=debug cargo test -- --nocapture
```

## Project Structure for Development

```
suru-lang-rs/
├── Cargo.toml           # Dependencies and project config
├── Cargo.lock           # Locked dependency versions
├── Dockerfile           # Development environment
├── .dockerignore        # Docker build exclusions
├── .gitignore           # Git exclusions
├── src/                 # Source code
│   ├── main.rs
│   ├── cli.rs
│   ├── lexer.rs
│   ├── parser/
│   ├── ast.rs
│   ├── limits.rs
│   └── codegen.rs
├── tests/               # Integration tests
├── examples/            # Example Suru programs
├── docs/                # User documentation
└── dev/                 # Developer documentation
```

## Continuous Integration

**TODO:** CI/CD pipeline will run:
- `cargo fmt -- --check`
- `cargo clippy`
- `cargo test`
- `cargo build --release`

---

**See also:**
- [Contributing Guide](README.md) - How to contribute
- [Architecture](architecture.md) - Compiler structure
- [Design Decisions](design-decisions.md) - Key choices
