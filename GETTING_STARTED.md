# Getting Started with Suru Lang

Welcome to Suru Lang! This guide will walk you through setting up your development environment and writing your first Suru program.

## Prerequisites

Before you begin, make sure you have:

- [Docker](https://docs.docker.com/get-docker/) installed on your system
- Basic familiarity with command-line tools
- A text editor (VS Code, Vim, Emacs, etc.)

## Installation

### Step 1: Clone the Repository

```bash
git clone https://github.com/yourusername/suru-lang-rs.git
cd suru-lang-rs
```

### Step 2: Build the Docker Image

```bash
docker build -t suru-lang:dev .
```

This creates a development environment with:
- Ubuntu 24.04 LTS
- Rust stable toolchain (edition 2024 support)
- LLVM 18 with full development libraries
- All necessary build tools

**Note:** First build takes 5-10 minutes. Subsequent builds are much faster due to Docker layer caching.

### Step 3: Verify Installation

```bash
# Build the compiler
docker run --rm -v $(pwd):/workspace suru-lang:dev cargo build

# Run tests to ensure everything works
docker run --rm -v $(pwd):/workspace suru-lang:dev cargo test
```

If all tests pass, you're ready to start!

## Your First Suru Program

Let's write a simple "Hello, World!" program to get familiar with Suru syntax.

### Create a New File

Create a file called `hello.suru`:

```suru
main: () {
    print("Hello, Suru!")
}
```

### Parse Your Program

Currently, Suru can parse and display the AST (Abstract Syntax Tree) of your program:

```bash
docker run --rm -v $(pwd):/workspace suru-lang:dev cargo run -- parse hello.suru
```

You should see output showing the parsed structure:
```
Program
  FunctionDecl
    Identifier "main"
    ParamList
    Block
      ExprStmt
        FunctionCall
          Identifier "print"
          LiteralString ""Hello, Suru!""
```

## A More Realistic Example

Let's write a program that demonstrates more Suru features. Create `example.suru`:

```suru
// Define a Person type with fields and methods
type Person: {
    name String
    age Number
    greet: () String
}

// Create a Person constructor
Person: (name String, age Number) Person {
    return {
        name: name
        age: age
        greet: () String {
            return `Hello, I'm {this.name} and I'm {this.age} years old!`
        }
    }
}

// Create a function that uses Person
introducePerson: (person Person) {
    message: person.greet()
    print(message)
}

// Main entry point
main: () {
    // Create a person
    alice: Person("Alice", 30)

    // Call the introduction function
    introducePerson(alice)
}
```

Parse this example:
```bash
docker run --rm -v $(pwd):/workspace suru-lang:dev cargo run -- parse example.suru
```

## Development Workflows

### Option 1: Work Inside the Container

This is the recommended approach for longer coding sessions:

```bash
# Start an interactive container
docker run -it --rm -v $(pwd):/workspace suru-lang:dev

# Inside the container, you can run commands directly
cargo build
cargo test
cargo run -- parse myfile.suru

# Exit when done
exit
```

### Option 2: Run Commands from Host

For quick commands without entering the container:

```bash
# Build from host machine
docker run --rm -v $(pwd):/workspace suru-lang:dev cargo build

# Run tests
docker run --rm -v $(pwd):/workspace suru-lang:dev cargo test

# Parse a file
docker run --rm -v $(pwd):/workspace suru-lang:dev cargo run -- parse myfile.suru
```

## Using the CLI

The Suru compiler currently provides a `parse` command:

```bash
# Parse a single file
suru parse myfile.suru

# Get help
suru --help
suru parse --help
```

More commands (compile, run, lsp) are planned for future releases.

## Next Steps

Now that you have Suru set up, here's what to explore next:

1. **Learn the Language** - Read the [Language Guide](docs/language/README.md) to understand Suru's features
   - [Syntax](docs/language/syntax.md) - Literals, comments, operators
   - [Types](docs/language/types.md) - The type system and structural typing
   - [Functions](docs/language/functions.md) - Function declarations and overloading
   - [Control Flow](docs/language/control-flow.md) - Pattern matching and iteration

2. **Try Examples** - Look at example files in the repository (e.g., `example.suru`)

3. **Explore Guides** - Check out [Guides](docs/guides/README.md) for tutorials on specific topics

4. **Join Development** - Read [Contributing Guide](docs/contributing/README.md) to help build Suru

## Common Commands Reference

### Inside Container

```bash
# Build the project
cargo build

# Build for release (optimized)
cargo build --release

# Run tests
cargo test

# Format code
cargo fmt

# Run clippy (linter)
cargo clippy

# Generate documentation
cargo doc --open

# Clean build artifacts
cargo clean

# Check Rust version
rustc --version

# Check cargo version
cargo --version
```

## Resources

- [Language Guide](docs/language/README.md) - Complete language specification
- [Contributing Guide](docs/contributing/README.md) - How to contribute
- [Rust Documentation](https://doc.rust-lang.org/)
- [LLVM Documentation](https://llvm.org/docs/)
- [Inkwell Documentation](https://thedan64.github.io/inkwell/)
- [Docker Documentation](https://docs.docker.com/)

## Getting Help

- **Issues:** Report bugs or request features at [GitHub Issues](https://github.com/yourusername/suru-lang-rs/issues)
- **Discussions:** Ask questions at [GitHub Discussions](https://github.com/yourusername/suru-lang-rs/discussions)

---

**Ready to learn more?** Continue to the [Language Guide](docs/language/README.md) to dive deep into Suru's features.
