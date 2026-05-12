# Suru

A minimalist, library-driven, general-purpose programming language — statically typed, no garbage collection.

## Table of contents

- [Status](#status)
- [Language overview](#language-overview)
- [Language guide](#language-guide)
- [Getting started](#getting-started)
- [Repository layout](#repository-layout)
- [Bootstrap context](#bootstrap-context)

## Status

Suru is self-hosted: the compiler (`src/compiler/build.suru`) is written in Suru and compiles itself.
The bootstrap binary was compiled from the frozen C# compiler released as [v0.1.0-bootstrap](https://github.com/paul-ciorogar/suru-lang-bootstrap/releases/tag/v0.1.0). See [BOOTSTRAP.md](BOOTSTRAP.md) for details.


## Language overview

- Entry point: `fn main(args Array<String>)`
- Variables: `let name Type: value` (type annotation mandatory)
- Types: `Bool`, `Int32`, `Int64`, `Float64`, `String`, `Array<T>`, named types, sum types
- Control flow: `while`, `match` (statement and expression forms)
- No GC: explicit `clone` / `drop` for heap values
- Cross-file includes: `include "path/file.suru" as ns`
- Built-ins: `printLn`, `printError`, `readFile`, `writeFile`, `appendToFile`, `clone`, `drop`, `exit`

## Language guide

### Printing

```suru
printLn(true)
printLn(42)
printLn(3.14)
```

### Variables

Declare with `let`. The type annotation is **mandatory** and appears between the variable name and the `:`:

```suru
let x Int64: 42
let ratio Float64: 1.5
let flag Bool: true
let name String: "suru"
```

Reassign with `name: value` (no `let`):

```suru
flag: false
```

A `let` declared at module level (outside any function) is a **constant** — reassignment is a compile error:

```suru
let MAX_SIZE Int64: 1024

fn main(args Array<String>) {
    MAX_SIZE: 2048  // error: cannot reassign constant 'MAX_SIZE'
}
```

Available types: `Bool`, `Int32`, `Int64`, `Float64`, `String`, `Array<T>`, and any declared named type (e.g. `Point`).

### Comments

Use `//` for line comments. Everything from `//` to the end of the line is ignored:

```suru
// full-line comment
let x Int64: 42  // inline comment
```

### Arithmetic

Arithmetic is expressed as method calls on values:

| Method | Description | Example |
|---|---|---|
| `add(n)` | addition | `3.add(1)` → `4` |
| `take(n)` | subtraction | `5.take(2)` → `3` |
| `multiply(n)` | multiplication | `3.multiply(2)` → `6` |
| `split(n)` | division | `9.split(3)` → `3` |
| `invert()` | negation | `5.invert()` → `-5` |

Methods chain naturally:

```suru
let result Int64: 2.add(3).multiply(4)
printLn(result)
```

### Boolean operators

```suru
printLn(true and false)   // false
printLn(true or false)    // true
printLn(not true)         // false
```

### Comparison methods

| Method | Description | Example |
|---|---|---|
| `equals(n)` | equality | `3.equals(3)` → `true` |
| `lt(n)` | less-than | `5.lt(10)` → `true` |
| `gt(n)` | greater-than | `10.gt(5)` → `true` |
| `lte(n)` | less-than-or-equal | `5.lte(5)` → `true` |
| `gte(n)` | greater-than-or-equal | `5.gte(3)` → `true` |
| `compare(n)` | three-way comparison | `2.compare(5)` → `-1` |

`equals`, `lt`, `gt`, `lte`, `gte` work on `Int64`, `Float64`, and `Bool`; always return `Bool`. `compare` works on `Int64` and `Float64`; returns `Int64` (`-1` = less, `0` = equal, `1` = greater).

### Control flow — while

```suru
let i Int64: 0
while i.lt(5) {
    printLn(i)
    i: i.add(1)
}
```

### Control flow — match

`match` has two forms depending on context.

**Match statement** — used at statement level; each arm has a `{ }` block body that can contain multiple statements, `let` bindings, and early `return`. An empty arm is written `{}`.

```suru
fn classify(n Int64) String {
    match n {
        0: { return "zero" }
        1: {
            let msg String: "one"
            return msg
        }
        _: {}
    }
    return "many"
}
```

A match statement satisfies the non-void return requirement when every arm (including the wildcard) contains a `return` or `exit`.

**Match expression** — used on the right-hand side of `let`, `return`, or any expression position; each arm body is a single expression that produces the result value.

```suru
let y Int64: match x { true: 1, _: 0 }
printLn(y)
```

Both forms use the same arm syntax (`pattern: body`) and support the same pattern types: `Bool` literals, integer/float literals (including negative), string literals, and identifier patterns (variable or constant lookup).

Match on integers (negative literals supported):

```suru
let n Int64: 0.take(1)
match n { -1: printLn("negative"), 0: printLn("zero"), 1: printLn("positive"), _: printLn("other") }
```

Match on strings:

```suru
let day String: "Monday"
match day { "Monday": printLn("start"), "Friday": printLn("end"), _: printLn("middle") }
```

Match on variables or constants:

```suru
let THRESHOLD Int64: 10

fn main(args Array<String>) {
    let score Int64: 10
    match score {
        THRESHOLD: printLn("exact")
        _: printLn("other")
    }
}
```

Arms are separated by `,` or newlines. The condition must be `Bool`, `Int64`, `Float64`, or `String`.

### Functions

Declare with `fn`. Parameters are `name Type` pairs. The return type follows the parameter list:

```suru
fn add(a Int64, b Int64) Int64 {
  return a.add(b)
}

printLn(add(3, 4))
```

Use `void` for functions that return no value:

```suru
fn printDouble(n Int64) void {
  printLn(n.add(n))
}
```

Recursion is supported:

```suru
fn fibonacci(n Int64) Int64 {
  return match n.lt(2) {
    true: n,
    _: fibonacci(n.take(1)).add(fibonacci(n.take(2)))
  }
}

printLn(fibonacci(10))
```

### Named types

Declare a named type with `type Name: { field Type, ... }`. Fields use only a name and a type — no value. Inline and multiline forms are both valid:

```suru
// inline
type Point: { x Int64, y Int64 }

// multiline
type Person: {
    name String
    age  Int64
}
```

Create a value with `{ field: value, ... }`. Fields are separated by `,` or newlines. No per-field type annotations — types come from the `type` declaration:

```suru
type Person: { tall Bool, height Int64 }

let person Person: { tall: true, height: 2283 }
```

Read a field with `.field` (no parentheses):

```suru
printLn(person.tall)    // true
printLn(person.height)  // 2283
```

Write a field with `receiver.field: value`:

```suru
person.tall: false
printLn(person.tall)  // false
```

Deep-copy with `clone`; free memory with `drop`:

```suru
let copy Person: clone(person)
drop(person)
```

Use the type name in `let` declarations, function parameters, and return types:

```suru
fn makePoint(x Int64, y Int64) Point {
    return { x: x, y: y }
}

fn getX(p Point) Int64 {
    return p.x
}
```

### Sum types (discriminated unions)

Declare a sum type with `type Name: Variant1, Variant2, ...`. Each variant name must refer to a declared struct type:

```suru
type Circle: { radius Int64 }
type Square: { side   Int64 }
type Shape: Circle, Square
```

Create a variant value using the struct literal syntax with the variant type as the annotation:

```suru
let c Circle: { radius: 2283 }
let s Square: { side:   100  }
```

Match on a variant-typed value with a **match statement**. The match must be exhaustive — all variants covered or a `_` wildcard present:

```suru
fn describe(shape Shape) String {
    match shape {
        Circle: { return "circle" }
        Square: { return "square" }
    }
}
```

A non-exhaustive match is a compile-time error listing each missing variant. A `_` wildcard covers all remaining variants:

```suru
fn info(shape Shape) String {
    match shape {
        Circle: { return "circle" }
        _: { return "other" }
    }
}
```

### Arrays

Create an array with `[e1, e2, ...]`. The element type is specified with the `Array<T>` generic annotation:

```suru
let nums Array<Int64>: [10, 20, 30]
let words Array<String>: ["hello", "world"]
```

The type parameter `T` can be any Suru type: `Bool`, `Int32`, `Int64`, `Float64`, `String`, or any named type (e.g. `Array<Token>`).

| Method | Description | Example |
|---|---|---|
| `len()` | number of elements | `nums.len()` → `3` |
| `at(i)` | element at index | `nums.at(0)` → `10` |
| `set(val, i)` | update element in-place | `nums.set(99, 1)` |
| `add(v)` | append element | `nums.add(40)` |
| `equals(other)` | element-wise equality | `nums.equals(other)` → `Bool` |
| `slice(from, to)` | new array copy of `[from, to)` | `nums.slice(1, 3)` |

```suru
let nums Array<Int64>: [10, 20, 30]
printLn(nums.len())      // 3
nums.add(40)
printLn(nums.at(3))      // 40
let part Array<Int64>: nums.slice(0, 2)
printLn(part.len())      // 2
```

Use `clone(arr)` to deep-copy and `drop(arr)` to free the array and its data.

### Strings

String literals are written with double quotes. Supported escapes: `\n`, `\t`, `\\`, `\"`.

```suru
let s String: "hello\nworld"
printLn(s)
```

| Method | Description | Example |
|---|---|---|
| `len()` | byte length | `s.len()` → `5` |
| `at(i)` | single-char `String` at index | `s.at(0)` → `"h"` |
| `equals(other)` | string equality | `s.equals("hello")` → `true` |
| `append(other)` | concatenate, new string | `s.append(" world")` |
| `slice(from, to)` | substring copy of `[from, to)` | `s.slice(1, 3)` → `"el"` |
| `ord()` | ASCII code of first byte | `"A".ord()` → `65` |
| `toString()` | identity — returns itself | `s.toString()` |

```suru
let s String: "hello"
printLn(s.len())            // 5
printLn(s.equals("hello"))  // true
let s2 String: s.append(" world")
printLn(s2)                 // hello world
printLn(s.at(0))            // h
```

### Type conversions

Convert a `String` to a number with the static `from` method:

```suru
let n Int64: Int64.from("42")
let f Float64: Float64.from("3.14")
```

Convert any primitive to a `String` with `toString()`:

```suru
let s String: 42.toString()
let b String: true.toString()
printLn(s)  // 42
printLn(b)  // true
```

### File I/O

Read an entire file as a `String`:

```suru
let content String: readFile("input.txt")
printLn(content)
```

Write a `String` to a file (overwrites if the file exists):

```suru
writeFile("output.txt", content)
```

Append a `String` to a file:

```suru
appendToFile("log.txt", "new line\n")
```

### Exit

Terminate the process with a specific exit code. `exit` is a terminal statement — a non-void function does not need an explicit `return` after it:

```suru
fn main(args Array<String>) {
    exit(1)
}
```

### printError

Write to stderr. Accepts the same types as `printLn` (`Bool`, `Int64`, `Float64`, `String`):

```suru
printError("error: file not found")
printError(42)
```

### Include directive

Split a program across multiple `.suru` files using `include`. Functions from the included file are accessible under a namespace alias:

```suru
include "lib.suru" as lib

fn main(args Array<String>) {
    let result Int64: lib.double(21)
    printLn(result)   // 42
}
```

`lib.suru`:
```suru
fn double(n Int64) Int64 {
    return n.multiply(2)
}
```

- The path is relative to the file that contains the `include`.
- All functions from the included file become available as `ns.fn(args)`.
- Named types from the included file are available by name in the importing module — no need to re-declare them. Re-declaring an imported type is a compile error.
- Scalar constants (`let NAME Type: literal`) from the included file are also available in the importing module.
- Include chains are transitive: if `main.suru` includes `a.suru` which includes `b.suru`, all three are compiled to separate objects and linked together. Diamond includes are deduplicated.

### Main function and CLI arguments

Every Suru program defines `fn main(args Array<String>)` as its entry point. `args.at(0)` is the program name; `args.at(1)` is the first user argument, and so on. The process always exits with code `0` unless `exit(code)` is called explicitly.

```suru
fn main(args Array<String>) {
    let path String: args.at(1)
    let content String: readFile(path)
    printLn(content)
}
```

## Getting started

Build the Docker image (requires Docker):

```sh
docker build -t suru .
```

Compile and run a program:

```sh
./scripts/build.sh tests/fixtures/arithmetic/main.suru build/
./build/main
```

Run the test suite (builds a Suru test runner, then runs it inside Docker):

```sh
./scripts/test.sh
```

Bootstrap a new compiler binary from source:

```sh
./scripts/bootstrap.sh
```

## Repository layout

```
src/
  compiler/       Suru compiler source (lexer → parser → semantic → codegen)
    lexer/
    parser/
    semantic/
    codegen/
runtime/          LLVM IR runtime modules (box, string, array, struct, variant)
tests/fixtures/   Corpus programs; each dir contains main.suru + expected.txt
tests/runner/     Suru test runner compiled and invoked by scripts/test.sh
scripts/          build.sh, test.sh, bootstrap.sh, generate-expected.sh
bin/              Bootstrap binary (suru-build)
Dockerfile        Ubuntu 24.04 image with clang-18/llvm-18/lld-18
```

## Bootstrap context

The first Suru compiler was written in C# and released as
[v0.1.0-bootstrap](https://github.com/paul-ciorogar/suru-lang-bootstrap/releases/tag/v0.1.0).
That compiler's output was used as the seed to compile the Suru compiler from its own source.
The C# compiler is frozen and archived in the bootstrap repository.
