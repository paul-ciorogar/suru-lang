# Suru

A minimalist, library-driven, general-purpose programming language — statically typed, no garbage collection.

## Table of contents

- [Status](#status)
- [Language overview](#language-overview)
- [Language guide](#language-guide)
  - [Printing](#printing)
  - [Variables](#variables)
  - [Comments](#comments)
  - [Arithmetic](#arithmetic)
  - [Boolean operators](#boolean-operators)
  - [Comparison methods](#comparison-methods)
  - [Control flow — while](#control-flow--while)
  - [Control flow — if / else](#control-flow--if--else)
  - [Control flow — match](#control-flow--match)
  - [Functions](#functions)
  - [Named types](#named-types)
  - [Sum types](#sum-types-discriminated-unions)
  - [Arrays](#arrays)
  - [Strings](#strings)
  - [Characters](#characters)
  - [Type conversions](#type-conversions)
  - [File I/O](#file-io)
  - [Exit](#exit)
  - [printError](#printerror)
  - [Built-in functions](#built-in-functions)
  - [Namespaces and imports](#namespaces-and-imports)
  - [Main function and CLI arguments](#main-function-and-cli-arguments)
- [Getting started](#getting-started)
- [CLI](#cli)
  - [Building the CLI](#building-the-cli)
  - [compile](#compile)
  - [build](#build)
  - [lex](#lex)
  - [parse](#parse)
  - [ir](#ir)
- [Repository layout](#repository-layout)
- [Bootstrap context](#bootstrap-context)

## Status

Suru is self-hosted: the compiler (`src/compiler/build.suru`) is written in Suru and compiles itself.
The bootstrap binary was compiled from the frozen C# compiler released as [v0.1.0-bootstrap](https://github.com/paul-ciorogar/suru-lang-bootstrap/releases/tag/v0.1.0). See [BOOTSTRAP.md](BOOTSTRAP.md) for details.


## Language overview

- Entry point: `fn main(args Array<String>)`
- Variables: `let name Type: value` (type annotation mandatory)
- Types: `bool`, `i32`, `i64`, `f64`, `char`, `String`, `Array<T>`, named types, sum types, generic types, object interface types with private members
- Control flow: `while` (with `break` and `continue`), `if` / `else if` / `else`, `match` (statement and expression forms)
- No GC: explicit `clone` / `drop` for heap values
- Namespace + import system: every `.suru` file declares `namespace A.B.C`; cross-file dependencies use `import { ... }`
- Built-ins: `printLn`, `printError`, `readFile`, `writeFile`, `appendToFile`, `clone`, `drop`, `move`, `exit`, `exec`
- Type expectations flow inward: struct and array literals take their type
  from the surrounding context (the `let` annotation, the enclosing function's
  return type, the parent struct's field type, or the surrounding array's
  element type). Literals never need an inline type tag — the compiler resolves
  them via a dedicated semantic pass that annotates every expression with its
  `resolvedType`.

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
let x i64: 42
let ratio f64: 1.5
let flag bool: true
let name String: "suru"
```

Reassign with `name: value` (no `let`):

```suru
flag: false
```

A `let` declared at module level (outside any function) is a **constant** — reassignment is a compile error:

```suru
let MAX_SIZE i64: 1024

fn main(args Array<String>) {
    MAX_SIZE: 2048  // error: cannot reassign constant 'MAX_SIZE'
}
```

Available types: `bool`, `i32`, `i64`, `f64`, `String`, `Array<T>`, and any declared named type (e.g. `Point`).

### Comments

Use `//` for line comments. Everything from `//` to the end of the line is ignored:

```suru
// full-line comment
let x i64: 42  // inline comment
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
let result i64: 2.add(3).multiply(4)
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

`equals`, `lt`, `gt`, `lte`, `gte` work on `i64`, `f64`, and `bool`; always return `bool`. `compare` works on `i64` and `f64`; returns `i64` (`-1` = less, `0` = equal, `1` = greater).

### Control flow — while

```suru
let i i64: 0
while i.lt(5) {
    printLn(i)
    i: i.add(1)
}
```

Use `break` to exit the loop early and `continue` to skip to the next iteration:

```suru
// Print 0, 1, 2 — stop when i reaches 3
let i i64: 0
while i.lt(10) {
    if i.equals(3) { break }
    printLn(i)
    i: i.add(1)
}

// Print 1, 2, 4, 5 — skip 3
let j i64: 0
while j.lt(5) {
    j: j.add(1)
    if j.equals(3) { continue }
    printLn(j)
}
```

`break` and `continue` always refer to the **innermost** enclosing `while` loop. Using either outside a loop is a compile error.

### Control flow — if / else

`if` evaluates a `bool` condition and runs the matching brace block. The `else` clause is optional, and `else if` chains arbitrarily many conditions. Each branch is its own scope — `let` bindings inside a branch are not visible after the `if`.

```suru
fn classify(n i64) String {
    if n.lt(0) {
        return "negative"
    } else if n.equals(0) {
        return "zero"
    } else {
        return "positive"
    }
}

let n i64: 10
if n.gt(0) {
    printLn("positive")
}
```

The condition must be `bool` — passing an `i64` or other type is a compile-time error (`if condition must be bool, got i64`).

When used inside a non-void function, an `if` only counts as "all paths return" when it has an `else` branch **and** both branches transitively return. An `if` with no `else`, or with one branch falling through, still requires a trailing `return` after the statement:

```suru
fn absVal(n i64) i64 {
    if n.lt(0) {
        return n.invert()
    }
    return n   // required: the false path of the `if` falls through
}
```

### Control flow — match

`match` has two forms depending on context.

**Match statement** — used at statement level; each arm has a `{ }` block body that can contain multiple statements, `let` bindings, and early `return`. An empty arm is written `{}`.

```suru
fn classify(n i64) String {
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
let y i64: match x { true: 1, _: 0 }
printLn(y)
```

Both forms use the same arm syntax (`pattern: body`) and support the same pattern types: `bool` literals, integer/float literals (including negative), string literals, and identifier patterns (variable or constant lookup).

Match on integers (negative literals supported):

```suru
let n i64: 0.take(1)
match n { -1: printLn("negative"), 0: printLn("zero"), 1: printLn("positive"), _: printLn("other") }
```

Match on strings:

```suru
let day String: "Monday"
match day { "Monday": printLn("start"), "Friday": printLn("end"), _: printLn("middle") }
```

Match on variables or constants:

```suru
let THRESHOLD i64: 10

fn main(args Array<String>) {
    let score i64: 10
    match score {
        THRESHOLD: printLn("exact")
        _: printLn("other")
    }
}
```

Arms are separated by `,` or newlines. The condition must be `bool`, `i64`, `f64`, or `String`.

### Functions

Declare with `fn`. Parameters are `name Type` pairs. The return type follows the parameter list:

```suru
fn add(a i64, b i64) i64 {
  return a.add(b)
}

printLn(add(3, 4))
```

Use `void` for functions that return no value:

```suru
fn printDouble(n i64) void {
  printLn(n.add(n))
}
```

Recursion is supported:

```suru
fn fibonacci(n i64) i64 {
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
type Point: { x i64, y i64 }

// multiline
type Person: {
    name String
    age  i64
}
```

Create a value with `{ field: value, ... }`. Fields are separated by `,` or newlines. No per-field type annotations — types come from the `type` declaration:

```suru
type Person: { tall bool, height i64 }

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
fn makePoint(x i64, y i64) Point {
    return { x: x, y: y }
}

fn getX(p Point) i64 {
    return p.x
}
```

### Object interface types

A `type` declaration can include method signatures (name, params, return type — no body). An object literal that carries the method bodies satisfies the interface and is stored as that type. Each literal instance has its own vtable, so two literals of the same type can have different method implementations.

```suru
type Greeter: {
    name String
    fn greet(prefix String) String
}

let g Greeter: {
    name: "World",
    fn greet(prefix String) String { return prefix.append(this.name) }
}
printLn(g.greet("Hello "))   // Hello World
```

Inside a method body the receiver is accessed as `this`. Field reads (`this.name`) and field writes (`this.field: value`) both work.

#### Private members

Object literals support private fields and private methods using a standalone `_` modifier. Private members are inaccessible from outside the object; only `this.name` inside a sibling method is allowed. A read, call, or write from outside is a compile-time error.

```suru
type Counter: {
    fn increment()
    fn get() i64
}

let c Counter: {
    _ count i64: 0,                                       // private data field
    fn increment() { this.count: this.count.add(1) }
    fn get() i64 { return this.count }
}
c.increment()
printLn(c.get().toString())   // 1
// c.count  ← compile error: cannot access private field 'count' from outside its object
```

Private methods follow the same `_ fn name(params) Ret { ... }` syntax and are accessible only via `this.name(...)` inside sibling methods.

**Design rule**: private members appear only on the object literal, never in the `type` declaration. The `type` declaration is the public interface.

### Sum types (discriminated unions)

Declare a sum type with `type Name: Variant1, Variant2, ...`. Each variant name must refer to a declared struct type:

```suru
type Circle: { radius i64 }
type Square: { side   i64 }
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

### Generics

Types and functions can be parameterized with one or more type parameters. The compiler monomorphizes each concrete instantiation at compile time.

```suru
type Box<T>: { value T }

fn identity<T>(x T) T {
    return x
}

fn main(args Array<String>) {
    let b Box<i64>: { value: 42 }
    printLn(b.value)    // 42
    drop(b)

    let r i64: identity(42)
    printLn(r)          // 42
}
```

Type parameters are declared in `<...>` after the name. Concrete type arguments are inferred at each usage site — no explicit instantiation syntax is needed. Each unique combination of type arguments produces a separate concrete copy (e.g. `Box<i64>` and `Box<String>` are distinct types). Because Suru has no GC, heap-allocated generic structs must be explicitly `drop`ped like any other struct.

Multiple type parameters are supported:

```suru
type Pair<T, U>: { first T, second U }

fn main(args Array<String>) {
    let p Pair<i64, String>: { first: 42, second: "hello" }
    printLn(p.first)    // 42
    printLn(p.second)   // hello
    drop(p)
}
```

Generic sum types are also supported. Variant entries may themselves be generic structs:

```suru
type Some<T>: { value T }
type None: {}
type Option<T>: Some<T>, None

fn describeOpt(x Option<i64>) {
    match x {
        Some: { printLn(x.value) }   // arm pattern "Some" resolved automatically
        None: { printLn("none") }
    }
}
```

The stdlib provides `Option<T>` and `Result<T, E>` in `src/stdlib/`:

```suru
namespace My.App
import { [some, none]: Suru.Stdlib.Option }
import { [ok, err]:    Suru.Stdlib.Result }

fn main(args Array<String>) {
    let s Option<i64>: some(42)
    let okVal Result<i64, String>: ok(99)
    let errVal Result<i64, String>: err("oops")
    drop(s)
    drop(okVal)
    drop(errVal)
}
```

### Arrays

Create an array with `[e1, e2, ...]`. The element type is specified with the `Array<T>` generic annotation:

```suru
let nums Array<i64>: [10, 20, 30]
let words Array<String>: ["hello", "world"]
```

The type parameter `T` can be any Suru type: `bool`, `i32`, `i64`, `f64`, `String`, or any named type (e.g. `Array<Token>`).

| Method | Description | Example |
|---|---|---|
| `len()` | number of elements | `nums.len()` → `3` |
| `at(i)` | element at index | `nums.at(0)` → `10` |
| `set(val, i)` | update element in-place | `nums.set(99, 1)` |
| `add(v)` | append element | `nums.add(40)` |
| `equals(other)` | element-wise equality | `nums.equals(other)` → `bool` |
| `slice(from, to)` | new array copy of `[from, to)` | `nums.slice(1, 3)` |

```suru
let nums Array<i64>: [10, 20, 30]
printLn(nums.len())      // 3
nums.add(40)
printLn(nums.at(3))      // 40
let part Array<i64>: nums.slice(0, 2)
printLn(part.len())      // 2
```

Use `clone(arr)` to deep-copy and `drop(arr)` to free the array and its data.

### Strings

String literals are written with double quotes. Supported escapes: `\n`, `\t`, `\\`, `\"`.

```suru
let s String: "hello\nworld"
printLn(s)
```

| Method | Description | Returns | Example |
|---|---|---|---|
| `len()` | byte length | value | `s.len()` → `5` |
| `at(i)` | single-char `String` at index (allocates) | new instance | `s.at(0)` → `"h"` |
| `__at(i)` | byte at index as a `char` (no allocation) | value | `s.__at(0)` → `'h'` |
| `equals(other)` | string equality | value | `s.equals("hello")` → `true` |
| `append(other)` | concatenate `String` or `char`, new string | new instance | `s.append(" world")`, `s.append('!')` |
| `__append(other)` | append in place, returns self | rebound self | `s.__append("!")` mutates `s` |
| `slice(from, to)` | substring copy of `[from, to)` | new instance | `s.slice(1, 3)` → `"el"` |
| `ord()` | ASCII code of first byte | value | `"A".ord()` → `65` |
| `toString()` | identity — returns itself | reference (self) | `s.toString()` |

```suru
let s String: "hello"
printLn(s.len())            // 5
printLn(s.equals("hello"))  // true
let s2 String: s.append(" world")
printLn(s2)                 // hello world
printLn(s.at(0))            // h
```

`__append` is the in-place counterpart of `append`: it builds the full result
in a fresh allocation, **drops the old receiver string**, and rebinds the
receiver variable so the mutation is observable without an explicit reassign.
It also returns the new string, so calls chain
(`s.__append("y").__append("z")`). Because the previous value is freed, any
other binding still pointing at the old string is left dangling — treat
`__append` as taking ownership of the receiver variable.

### Characters

`char` is a value type — a single byte. Unlike `String`, a `char` is never
heap-allocated and never needs `drop`. char literals use single quotes;
supported escapes are `\n`, `\t`, `\\`, `\'`, `\0`.

```suru
let c char: 'h'
let nl char: '\n'
```

Read a character out of a `String` with `__at(i)`, which returns a `char`
without allocating (contrast `at(i)`, which mallocs a one-character
`String`):

```suru
let s String: "hello"
let first char: s.__at(0)
printLn(first.equals('h'))   // true
printLn(s.__at(1).ord())     // 101  (ASCII 'e')
```

| Method | Description | Returns | Example |
|---|---|---|---|
| `equals(other)` | char equality | value | `c.equals('h')` → `true` |
| `ord()` | byte value as `i64` | value | `'A'.ord()` → `65` |
| `toString()` | owned one-char `String` | new instance | `'h'.toString()` → `"h"` |

A `char` can be appended to a `String` with `append`:

```suru
let s String: "ab".append('c')   // "abc"
```

> Migration note: `String.__at` is an interim accessor. `String.at` still
> returns a `String` today; a later step replaces it with the `char`-returning
> form and retires `__at`.

### Type conversions

Convert a `String` to a number with the static `from` method:

```suru
let n i64: i64.from("42")
let f f64: f64.from("3.14")
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

Write to stderr. Accepts the same types as `printLn` (`bool`, `i64`, `f64`, `String`):

```suru
printError("error: file not found")
printError(42)
```

### Built-in functions

| Function | Signature | Description |
|---|---|---|
| `printLn` | `printLn(value)` | Prints `value` to stdout followed by a newline. Accepts `bool`, `i32`, `i64`, `f64`, and `String`. |
| `printError` | `printError(value)` | Same as `printLn` but writes to stderr. |
| `readFile` | `readFile(path) → String` | Reads the entire contents of the file at `path` and returns it as a `String`. Aborts if the file cannot be opened. |
| `writeFile` | `writeFile(path, content)` | Writes `content` to the file at `path`, creating it if it does not exist and truncating it if it does. |
| `appendToFile` | `appendToFile(path, content)` | Appends `content` to the file at `path`. Creates the file if it does not exist. |
| `clone` | `clone(x) → T` | Returns a deep copy of `x`. Required before storing a heap value (String, Array, or struct) in a second variable, passing it somewhere that takes ownership, or outliving the original. |
| `drop` | `drop(x)` | Frees the heap memory owned by `x` (String, Array, or struct). After `drop`, `x` must not be used. |
| `move` | `move(x) → T` | Transfers ownership of `x` without cloning. After `move(x)`, any use of `x` is a compile-time error. Re-assigning `x` clears the moved state. The argument must be a variable, not an expression. |
| `exit` | `exit(code)` | Terminates the process immediately with the given exit code (`i64`). Counts as a terminal statement — no `return` is needed after it in a non-void function. |
| `exec` | `exec(cmd) → i64` | Runs `cmd` as a shell command via `system()` and returns the exit code as `i64`. Stdout and stderr pass through to the process. |

```suru
// print
printLn("hello")           // stdout
printError("bad input")    // stderr

// file I/O
let src String: readFile("input.txt")
writeFile("out.txt", src)
appendToFile("log.txt", "done\n")

// memory
let copy String: clone(src)
drop(src)
let moved String: move(copy)   // copy is now invalid; moved owns the value

// process
let code i64: exec("ls -la")
exit(code)
```

### Namespaces and imports

Every `.suru` file must declare a namespace as its first statement. This declaration is required — omitting it is a compile error:

```suru
namespace My.App.Util
```

Split a program across multiple `.suru` files by writing the full qualified name inline — no import statement required. The compiler auto-discovers source files by scanning the project for matching namespace declarations (configured via `.suruproject`):

`lib.suru`:
```suru
namespace My.Lib

export {
    double
}

fn double(n i64) i64 {
    return n.multiply(2)
}
```

`main.suru`:
```suru
namespace My.App

fn main(args Array<String>) {
    let result i64: My.Lib.double(21)
    printLn(result)   // 42
}
```

`import` is a convenience that lets you use a shorter local name instead of the full qualified path. Four import forms are supported:

```suru
import { My.Lib }                              // FullNs — injects all exports: double() works directly
import { lib: My.Lib }                         // AliasNs — qualify calls as lib.double()
import { [double, triple]: My.Lib }            // SelectiveNames — call double() unqualified
import { [dbl]: double }: My.Lib }             // SelectiveAliased — call as dbl()
```

Exported names are declared with `export { name1, name2 }`. Only exported names are visible to importers; unexported names are private to their module.

A `.suruproject` file at the project root lists source root directories (one per line, relative paths, `#` comments allowed). The compiler scans those directories to build the namespace registry so imports resolve without explicit file paths.

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
./scripts/build.sh tests/fixtures/arithmetic/main.suru
./tests/fixtures/arithmetic/build/main
```

Run the test suite (builds a Suru test runner, then runs it inside Docker):

```sh
./scripts/test.sh
```

Bootstrap a new compiler binary from source:

```sh
./scripts/bootstrap.sh
```

## CLI

The Suru CLI (`src/cli/main.suru`) is the compiler. It wraps the compiler pipeline and exposes six commands: `compile`, `build`, `lex`, `parse`, `ir`, and `debug`.

### Building the CLI

```sh
./scripts/build.sh src/cli/main.suru build/cli
```

The binary is written to `build/cli/main`. You can invoke it directly:

```sh
build/cli/main <command> <file.suru>
```

Or copy/symlink it somewhere on your `$PATH` as `suru`.

### compile

Compile a `.suru` source file to LLVM IR files. This is the low-level interface used by build scripts and the test runner — it mirrors the old `suru-build <source> <output.ll>` interface:

```sh
build/cli/main compile src/myprogram/main.suru /tmp/out/main.ll
# writes main.ll and one .ll per imported file into /tmp/out/
clang-18 /tmp/out/*.ll /usr/local/lib/suru/runtime/*.ll -o myprogram
```

### build

Compile a `.suru` source file to a native executable. The output binary is placed in a `build/` directory next to the source file:

```sh
build/cli/main build src/myprogram/main.suru
# → src/myprogram/build/main
```

### lex

Print every token produced by the lexer, one per line, in the format `KIND text line:col`. Useful for debugging tokenisation:

```sh
build/cli/main lex src/myprogram/main.suru
# KEYWORD fn 1:1
# IDENT   main 1:4
# ...
```

Only the tokens of the given file are printed; imported files are not expanded.

### parse

Parse the source file (resolving all `import` dependencies transitively) and print the AST as an indented tree:

```sh
build/cli/main parse src/myprogram/main.suru
```

Useful for verifying that the parser sees the structure you expect and for inspecting how imports are resolved.

### ir

Run the full compilation pipeline and print the generated LLVM IR to stdout, without invoking `clang`:

```sh
build/cli/main ir src/myprogram/main.suru
```

Useful for inspecting codegen output or diffing IR across changes without producing a binary.

### debug

Stop the pipeline after a named pass and print the full AST and symbol table (functions, types, sum types, scopes, errors) to stdout:

```sh
build/cli/main debug <pass> src/myprogram/main.suru
```

Valid pass names:

| Pass | Stops after |
|---|---|
| `resolve1` | First `resolveModuleTypes` — before monomorphization; generic templates still present |
| `mono` | Monomorphization complete — concrete nodes injected, call-sites and variant arms rewritten |
| `resolve2` | Second `resolveModuleTypes` — types resolved on concrete code |
| `semantic` | `analyzeModuleStatements` — full symbol table visible, all semantic errors collected |

Example output:
```
=== AST after mono ===
Module [mono]
  TypeDecl Box__i64 { value i64 }
  FnDecl main(args Array<String>) ...

=== Symbol Table after mono ===
Functions (3):
  main(Array<String>) → void
  ...
Types (1):
  Box__i64: { value i64 }
...
```

## Repository layout

```
src/
  compiler/       Suru compiler source (lexer → parser → semantic → codegen)
    lexer/
    parser/
    semantic/
      mono/       Monomorphization pass (monoCollect, monoInfer, monoInstantiate, monoSubst, monoPass)
    codegen/
    debug/        Per-pass debug printer (debugPrint.suru)
  stdlib/         Generic stdlib types (option.suru, result.suru)
  cli/            User-facing CLI entry point (main.suru)
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
