# Contributing to Suru Lang

> How to contribute to the Suru programming language compiler

## Welcome!

Thank you for your interest in contributing to Suru Lang! This guide will help you get started with contributing to the compiler implementation.

## Ways to Contribute

- **Report bugs**: File issues for bugs you find
- **Suggest features**: Propose new language features or improvements
- **Write code**: Implement new features or fix bugs
- **Improve documentation**: Help make docs clearer and more complete
- **Write tests**: Add test coverage for existing features
- **Review pull requests**: Help review other contributors' code

## Getting Started

### 1. Set Up Development Environment

Follow the [Development Workflow](development.md) guide to set up your local environment using Docker.

### 2. Understand the Architecture

Read the [Architecture](architecture.md) document to understand how the compiler is structured.

### 3. Find an Issue

- Check [GitHub Issues](https://github.com/yourusername/suru-lang-rs/issues)
- Look for issues labeled `good first issue` or `help wanted`
- Or work on something from the [Roadmap](roadmap.md)

### 4. Make Your Changes

1. Fork the repository
2. Create a new branch: `git checkout -b feature/your-feature-name`
3. Make your changes
4. Write tests for your changes
5. Ensure all tests pass: `cargo test`
6. Format your code: `cargo fmt`
7. Run the linter: `cargo clippy`

### 5. Submit a Pull Request

1. Push your branch to your fork
2. Create a pull request against the `main` branch
3. Describe your changes clearly
4. Reference any related issues
5. Wait for review

## Development Workflow

### Running Tests

```bash
# Run all tests
docker run --rm -v $(pwd):/workspace suru-lang:dev cargo test

# Run specific test
docker run --rm -v $(pwd):/workspace suru-lang:dev cargo test test_name

# Run tests with output
docker run --rm -v $(pwd):/workspace suru-lang:dev cargo test -- --nocapture
```

### Building the Compiler

```bash
# Debug build
docker run --rm -v $(pwd):/workspace suru-lang:dev cargo build

# Release build (optimized)
docker run --rm -v $(pwd):/workspace suru-lang:dev cargo build --release
```

### Code Quality

```bash
# Format code
docker run --rm -v $(pwd):/workspace suru-lang:dev cargo fmt

# Check formatting
docker run --rm -v $(pwd):/workspace suru-lang:dev cargo fmt -- --check

# Run linter
docker run --rm -v $(pwd):/workspace suru-lang:dev cargo clippy

# Run linter with pedantic checks
docker run --rm -v $(pwd):/workspace suru-lang:dev cargo clippy -- -W clippy::pedantic
```

## Code Guidelines

### Rust Style

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` to format code
- Address `cargo clippy` warnings
- Write documentation comments for public APIs

### Testing

- Add tests for all new features
- Maintain or improve test coverage
- Test edge cases and error conditions
- Use descriptive test names

### Commits

- Write clear, descriptive commit messages
- Reference issues in commits (e.g., "Fixes #123")
- Keep commits focused and atomic
- Use present tense ("Add feature" not "Added feature")

### Pull Requests

- Keep PRs focused on a single concern
- Write a clear description
- Include examples if adding features
- Update documentation as needed
- Ensure CI passes before requesting review

## Project Structure

```
suru-lang-rs/
├── src/
│   ├── main.rs           # CLI entry point
│   ├── cli.rs            # Command-line interface
│   ├── lexer.rs          # Tokenization
│   ├── parser/           # Parsing (modular)
│   ├── ast.rs            # AST data structures
│   ├── limits.rs         # Compiler safety limits
│   └── codegen.rs        # LLVM code generation
├── docs/                 # User documentation
├── dev/                  # Developer documentation
└── tests/                # Integration tests
```

See [Architecture](architecture.md) for detailed structure.

## Communication

- **Issues**: Use GitHub Issues for bugs and feature requests
- **Pull Requests**: Use GitHub PRs for code contributions
- **Discussions**: Use GitHub Discussions for questions and ideas

## Code of Conduct

- Be respectful and inclusive
- Welcome newcomers
- Focus on constructive feedback
- Help create a positive environment

## Recognition

All contributors will be recognized in:
- The project's contributors list
- Release notes for their contributions
- The project README

## Questions?

- Check the [Development Workflow](development.md) guide
- Review the [Architecture](architecture.md) documentation
- Ask in GitHub Discussions
- File an issue if you find bugs in the docs

---

**Thank you for contributing to Suru Lang!**
