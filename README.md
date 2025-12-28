# Suru Lang

> A minimalist, library-driven, general-purpose programming language with structural typing and no garbage collection.

## Overview

Suru Lang prioritizes interactive development, transforming editors into REPL-like environments through LSP integration. The language emphasizes minimal syntax with maximum expressiveness, enabling developers to write clear, readable code without unnecessary ceremony.

## Key Features

- **Minimal Syntax** - Maximum expressiveness with minimal punctuation
- **Structural Typing** - Duck typing based on shape, not names
- **No Garbage Collection** - Simple ownership model with move semantics
- **Method-Centric Design** - No loop keywords; iteration through methods like `.times()` and `.each()`
- **Interactive Development** - LSP-first tooling for inline inspection and testing
- **Composition Over Inheritance** - Use `+` operator to compose types and data
- **Error as Values** - No exceptions; errors are values you can't ignore
- **Rich Type System** - Unit types, unions, structs, intersections, generics with structural compatibility

## Quick Start

```bash
# Build the Docker development environment
docker build -t suru-lang:dev .

# Parse a Suru file
docker run --rm -v $(pwd):/workspace suru-lang:dev cargo run -- parse example.suru
```

See [**Getting Started Guide**](GETTING_STARTED.md) for detailed installation and first steps.

## Example

```suru
type Person: {
    name String
    age Number
    greet: () String
}

sayHello: (person Person) {
    greeting: person.greet()
    print(greeting)
}

alice Person: {
    name: "Alice"
    age: 30
    greet: () {
        return `Hello, I'm {this.name}!`
    }
}

sayHello(alice)
```

## Documentation

### For Language Users

- [**Getting Started**](GETTING_STARTED.md) - Installation and your first program
- [**Language Guide**](docs/language/README.md) - Complete language specification
  - [Syntax](docs/language/syntax.md) - Lexical elements and literals
  - [Types](docs/language/types.md) - Type system and structural typing
  - [Functions](docs/language/functions.md) - Functions, parameters, overloading
  - [Control Flow](docs/language/control-flow.md) - Pattern matching and iteration
  - [Error Handling](docs/language/error-handling.md) - Error as values, `try` keyword
  - [More topics...](docs/language/README.md)
- [**Reference**](docs/reference/) - Quick lookup tables

### For Contributors

- [**Contributing Guide**](docs/contributing/README.md) - How to contribute
- [**Architecture**](docs/contributing/architecture.md) - Compiler design and pipeline
- [**Development Workflow**](docs/contributing/development.md) - Docker setup, build commands
- [**Design Decisions**](docs/contributing/design-decisions.md) - Key architectural choices
- [**Roadmap**](docs/contributing/roadmap.md) - Future plans

### For Compiler Developers

- [**dev/progress.md**](dev/progress.md) - Development log and milestones

## Project Status

**Current Version:** 0.12.0 (Parser - Pipe Operator)

**Implemented:**
- Lexer - Complete tokenization
- Parser - All type declarations, functions, expressions, method calls, pipe operator
- AST - First-child/next-sibling representation
- CLI - Parse command with clap

**In Progress:**
- Semantic Analysis - Type checking, symbol tables
- Code Generation - LLVM IR generation

**Planned:**
- LSP Server - Interactive development tooling
- Standard Library - Core language features
- Package Manager - Dependency management

See [**CHANGELOG.md**](CHANGELOG.md) for version history and [**Roadmap**](docs/contributing/roadmap.md) for future plans.

## Building from Source

Suru Lang uses Docker for consistent development environments:

```bash
# Build the Docker image
docker build -t suru-lang:dev .

# Run tests
docker run --rm -v $(pwd):/workspace suru-lang:dev cargo test

# Build the compiler
docker run --rm -v $(pwd):/workspace suru-lang:dev cargo build --release
```

See [Development Workflow](docs/contributing/development.md) for detailed build instructions.

## Community & Support

- **Issues:** [GitHub Issues](https://github.com/yourusername/suru-lang-rs/issues)
- **Discussions:** [GitHub Discussions](https://github.com/yourusername/suru-lang-rs/discussions)

## Acknowledgments

Built with Rust and LLVM 18, leveraging Inkwell for safe LLVM bindings.

---

**Learn more:** [Language Guide](docs/language/README.md) | [Getting Started](GETTING_STARTED.md) | [Contributing](docs/contributing/README.md)
