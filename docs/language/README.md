# Suru Language Guide

> A comprehensive guide to the Suru programming language

## Table of Contents

- [Core Philosophy](#core-philosophy)
- [Key Features](#key-features)
- [Type System Overview](#type-system-overview)
- [Memory Model](#memory-model)
- [Language Topics](#language-topics)

## Core Philosophy

The Suru language prioritizes **interactive development**, transforming editors into REPL-like environments through LSP integration. Developers can inspect variables, mock dependencies, and define behavioral expectations directly without separate test files. Use cases inform compiler optimization decisions based on actual usage patterns.

**Key principles:**
- **Minimal syntax** with maximum expressiveness
- **Library-based extensibility** with granular upgrade paths
- **Interactive development** through LSP-first tooling
- **Use cases driving validation** and compilation optimization

## Key Features

Suru supports clear, readable syntax with minimal punctuation:

- **Strong type system** with generics and constraints
- **Module-based organization** with submodules for internal code structure
- **Pattern matching** for control flow
- **Union and intersection types** for flexible type composition
- **Method and function overloading** with return type differentiation
- **Currying and partial application** with `_` placeholder
- **Pipe operator** for value chaining
- **Rich documentation** with markdown support
- **Advanced string interpolation** with multiple nesting levels
- **No garbage collection** - simple ownership model
- **Method-centric iteration** - no loop keywords

## Type System Overview

### Supported Type Forms

1. **Unit types** - Simple flags/states (`type Success`)
2. **Type aliases** - Renaming existing types (`type UserId: Number`)
3. **Union types** - Alternatives (`type Status: Success, Error, Loading`)
4. **Struct types** - Records with fields and methods
5. **Intersection types** - Combining types with `+`
6. **Function types** - Function signatures
7. **Generic types** - Parameterized types with constraints

**Learn more:** [Type System](types.md)

## Memory Model

Suru manages memory **without garbage collection**, using a straightforward ownership model:

### Ownership and Move Semantics

- Functions take ownership of all values passed to them
- All values are passed by move by default
- When a value would be mutated after being passed to a function, the language automatically creates a copy before the move
- Memory can be shared as long as no mutations occur

### No Shared Mutable State

- Mutable state is never shared between scopes
- When a mutation would cause sharing, Suru duplicates the memory instead
- All copies are deep copies with no exceptions

### Function Scope

- Each function owns all its memory values
- All values within a function scope are mutable
- Once a function receives a value, it has complete ownership and can modify it freely

This approach **eliminates entire classes of memory-related bugs** while keeping the memory model simple and predictable.

**Learn more:** [Memory Model](memory.md)

## Language Topics

### Fundamentals

- [**Syntax**](syntax.md) - Lexical elements, literals, comments, numbers, strings
- [**Variables**](variables.md) - Variable declarations and assignments
- [**Operators**](operators.md) - All operators with precedence rules

### Type System

- [**Types**](types.md) - Complete type system guide
  - Unit types, type aliases, union types
  - Struct types with fields and methods
  - Intersection types and composition
  - Function types and generics
  - Generic type constraint and inference

### Functions & Control Flow

- [**Functions**](functions.md) - Function declarations, parameters, overloading, generics
- [**Control Flow**](control-flow.md) - Pattern matching and method-based iteration
- [**Error Handling**](error-handling.md) - Error as values, `try` keyword, Result types

### Advanced Features

- [**Modules**](modules.md) - Module system, imports, exports, submodules
- [**Composition**](composition.md) - Type/data/method composition with `+`
- [**Advanced Topics**](advanced.md) - String interpolation, documentation syntax, currying

## Quick Examples

### Hello World

```suru
main: () {
    print("Hello, Suru!")
}
```

### Type Definition

```suru
type Person: {
    name String
    age Number
    greet: () String
}
```

### Function with Type Inference

```suru
add: (x, y) {
    return x + y
}
```

### Method-Based Iteration

```suru
# No loop keywords - use methods
printStep: (step) {
    print(`Step {step}`)
}

10.times(printStep)

numbers: [1, 2, 3, 4, 5]
printNumbers: (num) {
    print(num)
}
numbers.each(printNumbers)
```

### Error Handling

```suru
processData: (input String) Result<Data, Error> {
    parsed: try parseInput(input)
    validated: try validateData(parsed)
    return Ok(validated)
}
```

### Composition

```suru
type Point: {
    x Number
    y Number
}

type Circle: Point + {
    radius Number
}
```

## Design Principles

### 1. No Garbage Collection

Move semantics by default with automatic copy when mutation would share memory. All values within a function are mutable, but no shared mutable state between scopes.

### 2. Type Inference and Generic Type Constraint Inference

Generic Types compatible based on inferred constraints from structure. No explicit interface declarations needed.

### 3. Method-Centric Design

No loop keywords (while, for). Iteration through methods: `.times()`, `.each()`. Control flow via continuation types (Continue, Break, Produce).

### 4. Composition Over Inheritance

Use `+` operator to compose types and data. Partial application for method composition. No class hierarchy.

### 5. Interactive Development

LSP-first approach. Inline variable inspection. Use cases as tests and optimization hints.

## Next Steps

- **New to Suru?** Start with [Syntax](syntax.md) to learn the basics
- **Coming from another language?** Check out [Types](types.md) to understand generic type constraint inference
- **Ready to write code?** Read [Functions](functions.md) and [Control Flow](control-flow.md)
- **Advanced user?** Explore [Composition](composition.md) and [Advanced Topics](advanced.md)

---

**See also:**
- [Getting Started Guide](../../GETTING_STARTED.md) - Installation and setup
- [Reference](../reference/) - Quick lookup tables
- [Guides](../guides/) - Tutorials and patterns
