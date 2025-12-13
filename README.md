# Suru Lang

A general-purpose, high-level, minimalist, library-driven programming language.

## Core Philosophy

The language prioritizes interactive development, transforming editors into REPL-like environments through LSP integration. Developers can inspect variables, mock dependencies, and define behavioral expectations directly without separate test files. Use cases inform compiler optimization decisions based on actual usage patterns.

**Key principles include:**
- Minimal syntax with maximum expressiveness
- Library-based extensibility with granular upgrade paths
- Interactive development through LSP-first tooling
- Use cases driving both validation and compilation optimization

## Notable Features

Suru supports clear, readable syntax with minimal punctuation, including:
- Strong type system with generics
- Module-based organization
- Pattern matching for control flow
- Intersection and union types
- Method and function overloading/currying
- Piped values and composition
- Rich documentation support
- Advanced string interpolation

## Type System

Suru uses **structural typing**, meaning types are compatible based on shape rather than explicit declarations. The language supports:
- Unit types (simple flags/states)
- Type aliases
- Union types (alternatives)
- Struct types (records with fields/methods)
- Intersection types (combining types with `+`)
- Function types
- Generic types with constraints

The structural approach enables duck typing—if a type has required methods, it satisfies that interface.

## Memory

Suru manages memory without garbage collection, using a straightforward ownership model:

**Ownership and Move Semantics**
- Functions take ownership of all values passed to them
- All values are passed by move by default
- When a value would be mutated after being passed to a function, the language automatically creates a copy before the move
- Memory can be shared as long as no mutations occur

**No Shared Mutable State**
- Mutable state is never shared between scopes
- When a mutation would cause sharing, Suru duplicates the memory instead
- All copies are deep copies with no exceptions

**Function Scope**
- Each function owns all its memory values
- All values within a function scope are mutable
- Once a function receives a value, it has complete ownership and can modify it freely

This approach eliminates entire classes of memory-related bugs while keeping the memory model simple and predictable.


## Table of Contents

- [File Structure](#file-structure)
- [Lexical Elements and Literals](#lexical-elements-and-literals)
  - [Booleans](#booleans)
  - [Comments](#comments)
  - [String and Character Literals](#string-and-character-literals)
    - [Escape Characters](#escape-characters)
  - [Numbers](#numbers)
- [Variable Declarations](#variable-declarations)
  - [Assignment Statements](#assignment-statements)
- [Modules](#modules)
  - [Module Declaration](#module-declaration)
  - [Imports](#imports)
    - [Full Module Import](#full-module-import)
    - [Selective Import](#selective-import)
    - [Import All](#import-all)
  - [Exports](#exports)
- [Operators](#operators)
  - [Unary](#unary)
  - [Logical](#logical)
  - [Compositional](#compositional)
- [Types](#types)
  - [Unit Types](#unit-types)
  - [Type Aliases](#type-aliases)
  - [Union Types](#union-types)
  - [Struct Types](#struct-types)
  - [Intersection Types](#intersection-types)
  - [Function Types](#function-types)
  - [Generic Types](#generic-types)
  - [Structural Typing](#structural-typing)
  - [Basic Structural Compatibility](#basic-structural-compatibility)
  - [Structural Compatibility with Functions](#structural-compatibility-with-functions)
  - [Duck Typing with Methods](#duck-typing-with-methods)
  - [Structural Subtyping](#structural-subtyping)
  - [Generic Type Structural Compatibility](#generic-type-structural-compatibility)
- [Functions](#functions)
  - [Generic Functions](#generic-functions)
  - [Function Overloading](#function-overloading)
  - [Method Overloading](#method-overloading)
  - [Overloading by Return Type](#overloading-by-return-type)
- [Pipeline](#pipeline)
- [Type Implementation](#type-implementation)
  - [Constructors](#constructors)
  - [Custom Instance](#custom-instance)
- [Privacy and Encapsulation](#privacy-and-encapsulation)
  - [Private Members](#private-members)
  - [The `this` Keyword](#the-this-keyword)
- [Currying and Partial Application](#currying-and-partial-application)
  - [Function Currying](#function-currying)
  - [Method Currying](#method-currying)
- [Lexical Scoping](#lexical-scoping)
  - [Correct Scoping](#correct-scoping)
  - [Currying with Proper Scoping](#currying-with-proper-scoping)
- [Collections](#collections)
  - [Lists](#lists)
  - [Sets](#sets)
  - [Maps](#maps)
  - [Collection Type Inference](#collection-type-inference)
- [Control Flow Statements](#control-flow-statements)
  - [Match Expressions](#match-expressions)
  - [Loops](#loops)
    - [Continuation Types](#continuation-types)
    - [Number Iteration](#number-iteration)
    - [Collection Iteration](#collection-iteration)
  - [Infinite Loops](#infinite-loops)
- [Error Handling](#error-handling)
  - [Short Circuiting](#short-circuiting)
  - [Pipe Integration](#pipe-integration)
- [Composition](#composition)
  - [Type Composition](#type-composition)
  - [Data Composition](#data-composition)
  - [Method Composition](#method-composition)
- [Documentation](#documentation)
  - [Documentation Keywords](#documentation-keywords)
- [String Interpolation](#string-interpolation)
  - [Single Backticks (`)](#single-backticks-)
  - [Double Backticks (``)](#double-backticks-)
  - [Triple Backticks (```)](#triple-backticks-)
  - [Quad Backticks (````)](#quad-backticks-)

## File Structure

A Suru source file has `.suru` extension and follows this structure:

1. **Module Declaration** (optional)
2. **Import Block** (optional)  
3. **Export Block** (optional)
4. **Declarations** (types, functions, variables, expressions)

## Lexical elements and literals

### Booleans

```suru
isTrue: true
isFalse: false
```

### Comments

```suru
// This is a line comment
```

### String and character literals

```suru
doubleQuoted: "Hello, World!"
singleQuoted: 'Hello, World!'

// String interpolation with backticks
simple: `Hello {name}!`
```

#### Escape Characters #

- \b - backspace (BS)
- \e - escape (ESC)
- \n - newline
- \r - carriage return
- \t - tab
- \\\\ - backslash
- \\" - double quote (if needed)
- \\\' - single quote (if needed)
- \\\` - single tick (if needed)
- \NNN- octal 6 bit character (3 digits)
- \xNN - hexadecimal 8 bit character (2 digits)
- \uNNNN - hexadecimal 16-bit Unicode character UTF-8 encoded (4 digits)
- \UNNNNNNNN - hexadecimal 32-bit Unicode character UTF-8 encoded (8 digits)

### Numbers

Multiple Number Bases:

- Binary: `0b1010`
- Octal: `0o755`
- Hexadecimal: `0xFF`
- Decimal: `123`

Underscore Separators for Readability:

- `1_000_000` instead of 1000000
- `0xDEAD_BEEF` for hex numbers
- Works in all number bases

Type Suffixes:

- Integers: `i8, i16, i32, i64, i128, ut8, u16, u32, u64, u128`
- floats: `f16, f32, f64, f128`

Examples

```suru
// Decimal with separators and suffix
count: 1_000_000u64;

// Binary with suffix
flags: 0b1010_1100u8;

// Hex with suffix
address: 0xDEAD_BEEFu16;
```

## Variable declarations 

A variable declaration declares a new variable for the current scope.
Declarations end with a new line unless on the next line there is a continuation like `| , . + and or`
```suru
name: value // type is inferred
name Type: value
```
Declarations at the file scope are constants.
A constant's value cannot be changed. The constant's value must be able to be evaluated at compile time

### Assignment statements
```suru
name: value
```

## Modules

Suru programs are organized into modules. A module is a directory of Suru code files, one of which has module declaration at the top. Execution starts in the main module's main function.

### Module Declaration

Module names have to start with a letter and can contain numbers dots and underscores

```suru
module Calculator
```

### Imports

suru supports the following types of imports:

#### Full Module Import
```suru
import {
    math
    mathAlias: math
    io
}
// Usage: math.sin(3.14)
// Usage: mathAlias.sin(3.14)
// Usage: io.stdout.write('')
```

#### Selective Import
```suru
import {
    {sin, cos, pi}: math
}
// Usage: sin(pi)
```

#### Import All
```suru
import {
    *: math
}
// Usage: sin(pi), cos(pi), etc.
```

### Exports

If a file starts with a module declaration then exports specify what your module makes available to other modules.
If a file does not have a module declaration then the exports are only available to the module in the same directory.

```suru
export {
    Calculator
    add
    subtract
}
```

## Operators

### Unary
`-` negation `-2_283i64`

### Logical

- `not` not false = true
- `and` true and true = true
- `or` true or false = true

### Compositional

`+` used for composing types and structs

## Types

### Unit Types
Simple types with no definition, perfect for flags and states:

```suru
type Success
type Error
type Loading
```

### Type Aliases
Simple renames:

```suru
type UserId: Number
type Username: String
```

### Union Types
Alternative types:

```suru
type Status: Success, Error, Loading
type Value: Int, String, Bool
```

### Struct Types
Records with fields and method declarations:

```suru
type Person: {
    name String
    age Number
    
    greet: () String
    birthday: () Person
}
```

### Intersection Types
Combine types using `+`:

```suru
type Employee: Person + {
    salary Int
    department String
}
```

### Function Types

Function signatures must be defined as named types:

- `void` can be used to tell that a function returns nothing.

```suru
type AddFunction: (a Number, b Number) Number
type Predicate: (value String) Bool
type VoidFunction: () void
type Identity<T>: (value T) T
type UnaryOperator: (x Float) Float
```

### Generic Types
Define types that work with multiple specific types:

```suru
// Single type parameter
type List<T>: {
    items Array<T>
    size Int
    
    add: (item T) List<T>
    get: (index Int) T
    contains: (item T) Bool
    map<R>: (transform R) List<R>
}

// Multiple type parameters  
type Map<K, V>: {
    entries Array<Pair<K, V>>
    
    put: (key K, value V) Map<K, V>
    get: (key K) Option<V>
    containsKey: (key K) Bool
}

// Generic types with constraints
type Comparable<T: Orderable>: {
    value T
    
    compare: (other Comparable<T>) Ordering
    lessThan: (other Comparable<T>) Bool
}
```

### Structural Typing
Suru uses structural typing, meaning types are compatible based on their structure rather than explicit declarations. Two types are considered equivalent if they have the same shape, regardless of their names.

### Basic Structural Compatibility

```suru
// Two different type declarations with same structure
type Person: {
    name String
    age Number
}

type Employee: {
    name String
    age Number
}

// These are structurally equivalent
checkAge: (p Person) Bool {
    return p.age.greaterThan(18)
}

emp Employee: {
    name: "Alice"
    age: 25
}

// This works because Employee has same structure as Person
isAdult: checkAge(emp)  // Valid - structural compatibility
```

### Structural Compatibility with Functions

Function types are also structurally typed:

```suru
type PersonProcessor: (p Person) String
type EmployeeHandler: (e Employee) String

// These function types are structurally equivalent
formatPerson: (person Person) String {
    return `{person.name} is {person.age} years old`
}

// Can assign to either function type
processor PersonProcessor: formatPerson
handler EmployeeHandler: formatPerson  // Same structure
```

### Duck Typing with Methods

If a type has the required methods, it can be used wherever that interface is expected:

```suru
type Drawable: {
    draw: () String
}

type Circle: {
    radius Number
    draw: () String
    area: () Number
}

type Rectangle: {
    width Number
    height Number
    draw: () String
}

// Function expecting Drawable interface
render: (shape Drawable) String {
    return shape.draw()
}

circle Circle: {
    radius: 5.0
    draw: () { return "Drawing circle" }
    area: () { return 3.14159 * this.radius * this.radius }
}

rectangle Rectangle: {
    width: 10.0
    height: 5.0
    draw: () { return "Drawing rectangle" }
}

// Both work because they have draw() method
circleOutput: render(circle)     // Valid
rectangleOutput: render(rectangle) // Valid
```

### Structural Subtyping

Types with additional fields are compatible with types that have fewer fields:

```suru
type BasicInfo: {
    name String
}

type DetailedInfo: {
    name String
    age Number
    email String
}

getName: (info BasicInfo) String {
    return info.name
}

detailed DetailedInfo: {
    name: "Bob"
    age: 30
    email: "bob@example.com"
}

// Works because DetailedInfo contains all fields of BasicInfo
name: getName(detailed)  // Valid - structural subtyping
```

### Generic Type Structural Compatibility

Generic types follow structural rules:

```suru
type Container<T>: {
    value T
    getValue: () T
}

type Box<T>: {
    value T
    getValue: () T
}

// Structurally equivalent generic types
stringContainer Container<String>: {
    value: "hello"
    getValue: () { return this.value }
}

// Can be used as Box<String> due to structural compatibility
useBox: (box Box<String>) String {
    return box.getValue()
}

result: useBox(stringContainer)  // Valid
```


## Functions

```suru
type UnaryFunction: (x Number) Number

// Function returning a simple type
add: (x Number, y Number) Number {
    return x.add(y)
}

// Function with inferred types
add: (x, y) {
    return x.add(y)
}

// Function returning a function type 
createAdder: (base Number) UnaryFunction {
    return (x Number) Number {
        return x.add(base)
    }
}

// Function taking a function 
applyTwice: (fn UnaryFunction, value Number) Number {
    temp: fn(value)
    return fn(temp)
}
```

### Generic Functions
Functions that work with multiple types:

```suru
// Simple generic function
identity<T>: (value T) T {
    return value
}

// Multiple type parameters
map<T, R>: (items List<T>, transform Transform<T, R>) List<R> {
    result: List<R>()
    // Implementation iterates and transforms
    return result
}

// Generic function with constraints
sort<T: Orderable>: (items List<T>) List<T> {
    // Implementation uses T's ordering methods
    return items.quickSort()
}
```

### Function overloading
```suru
// Function overloading (same name, different signatures)
add: (a Int, b Int) Int { return a + b }
add: (a Float, b Float) Float { return a + b }
add: (a Int) Int { return a }
add: (a String, b String) String { return a + b }
```

### Method overloading

Same as function overloading

```suru
type Adds: {
    add: (a Int, b Int) Int { return a + b }
    add: (a Float, b Float) Float { return a + b }
    add: (a Int) Int { return a }
    add: (a String, b String) String { return a + b }
}
```

### Overloading by Return Type

```suru
// Same function name and parameters, different return types
parse: (input String) Int {
    return input.toInt()
}

parse: (input String) Float {
    return input.toFloat()
}

parse: (input String) Bool {
    return input.equals("true")
}

// Usage - type annotation determines which overload
intValue Int: parse("123")      // Calls parse: (String) Int
floatValue Float: parse("3.14") // Calls parse: (String) Float
boolValue Bool: parse("true")   // Calls parse: (String) Bool
```

## Pipeline

The `|` (pipe) operator can be used to pipe values to functions

```suru
2_283 | subtract(_, 2) | print // 2281 would be printed

processed: "Hello, world!"
    | trim()
    | toLower()
    | replace(_, "world", "you")
    | capitalize()
```

## Type implementation

Simple struct implementation
```suru
type User: {
    username: String
    authenticate: (password String) Bool 
}

// implementation
user User: {
    username: "Paul"
    authenticate: (password) {
        return true;
    } 
}
```

### Constructors
Any struct type can define a constructor function
The constructor function can have the same name as the type
```suru
type User: {
    username: String
    authenticate: (password String) Bool 
}

// constructor function
User: (name String) User {
    return {
        username: name
        authenticate: (password) {
            return true;
        } 
    }
}
user: User("Paul")
```

### Custom Instance
Each instance can have unique method implementations:
```suru
// we can create instances without using the constructor
user User: {
    username: "Paul"
    authenticate: (password) {
        return true;
    } 
}

// or this way
newUser: (name String) User {
    return {
        username: name
        authenticate: (password) {
            return true;
        } 
    }
}
user: newUser("Paul")
```

## Privacy and Encapsulation

Suru uses private member declarations for encapsulation.

### Private Members
Use `_` prefix in declarations to mark fields and methods as private:

```suru
type User: {
    username: String                      // Public field

    authenticate: (password String) Bool  // Public method
}

user: User: {
    username: "Paul"        // Public field
    _ passwordHash: "2283"  // Private field
    _ salt: "qwerty"        // Private field

    authenticate: (password String) Bool { // Public method
        // implementation
    }
    _ hashPassword: (password String) String {  // Private method
        // implementation
    }
}
```

### The `this` Keyword
Within method implementations, `this` refers to the current instance:

```suru
// Public interface - what consumers see
type BankAccount: {
    accountId String
    deposit: (amount Float) Float
    withdraw: (amount Float) Float
    getBalance: () Float
}

// Constructor
BankAccount: (initial Float, id String) BankAccount {
    impl BankAccount: {
        accountId: id
        transactionCount: 0
        _ balance: initial

        deposit: (amount Float) Float {
            return match this.validate(amount) {  // Call private method
                true: {
                    this.balance: this.balance.add(amount)
                    this.logTransaction("deposit", amount)
                    return this.balance
                }
                false: this.balance
            }
        }

        _ validate: (amount Float) Bool {  // Private method implementation
            return amount.greaterThan(0.0)
        }

        _ logTransaction: (type String, amount Float) {
            // Private logging logic
        }

        // ... other methods
    }

    return impl
}

// Usage
account: BankAccount(100.0, "ACC123")
// account.balance        // ❌ Compile error: not in public interface
// account.validate(50.0) // ❌ Compile error: private method not accessible
balance: account.getBalance()  // ✅ OK: public method
```

## Currying and Partial Application

All functions and methods in Suru can be curried.
Calling a function with `_` placeholder instead of an argument returns a new function that takes the remaining arguments which were given the placeholder.
Explicit `partial` keyword when a function has many arguments and adding a lot of `_, _, _, _, _, _, _, _, _,` would look ugly.

### Function Currying

```suru
// Currying with placeholders
addTwo: add(2, _)           // Partial application
addToFive: add(_, 5)        // Different partial application
increment: add(_, 1)        // Another partial application

// Explicit partial when a function has many arguments and adding a lot of `_, _, _, _, _, _, _, _, _,` would look ugly.
complexCurry: partial functionWithManyArguments(2_283i32)

// Multiple placeholders
combine: someFunction(_, "default", _)

// Works with pipe operations
result: 10 | addTwo    // Same as addTwo(10)
```

### Method Currying
Methods can also be curried:

```suru
type BinaryOperation: (a Int, b Int) Int
type UnaryOperation: (x Int) Int

type Calculator: {
    multiply: (a Int, b Int, c Int) Int
}

calc Calculator: {
    multiply: (a Int, b Int, c Int) Int {
        return a.multiply(b).multiply(c)
    }
}

// Curry the method
double: calc.multiply(2, _, _)        // Type: BinaryOperation (conceptually)
doubleTriple: calc.multiply(2, 3, _)  // Type: UnaryOperation

result: doubleTriple(4)        // 2 * 3 * 4 = 24
```

## Lexical Scoping

Functions and methods have strict lexical scoping - they can only access:
1. Their parameters
2. Variables declared within their body
3. Global constants and functions

They **cannot** access variables from outer scopes. This ensures predictable behavior and makes currying safe.

### Correct Scoping
```suru
constant: 42

outerFunction: (x Number) Number {
    localVar: 10

    innerFunction: (y Number) Number {
        // ✅ Can access: y (parameter), constant
        // ✅ Can access other functions: outerFunction
        return y.add(constant)
    }

    // ❌ Cannot access: localVar from outer scope
    // innerFunction: (y Number) Number {
    //     return y.add(localVar)  // ERROR: localVar not in scope
    // }

    return innerFunction(x)
}
```

### Currying with Proper Scoping
```suru
type NumberFunction: (x Number) Number

// Parameters become part of the curried function's closure
add: (x Number, y Number) Number {
    return x.add(y)
}

addFive: add(5, _)
result: addFive(3)  // 8
```

## Collections

Suru provides four built-in collection types, all using the unified `[]` syntax for creation. The type annotation determines which collection is created.

### Lists
Ordered collections that allow duplicates:

```suru
// List creation using [] syntax
numbers List<Number>: [1, 2, 3, 4, 5]
names List<String>: ["alice", "bob", "charlie"]
emptyList List<Float>: []

// List building
extended: numbers
    .add(6)
    .add([7, 8, 9])
    .set(0, 0)                         // Insert at index
```

### Sets
Unordered collections with unique elements:

```suru
// Set creation - duplicates automatically removed
uniqueNumbers Set<Number>: [1, 2, 3, 2, 1]  // Results in {1, 2, 3}
colors Set<String>: ["red", "green", "blue"]
emptySet Set<Float>: []
```

### Maps
Key-value collections:

```suru
// Map creation using key:value syntax
userAges Map<String, Number>: [
    "alice": 25,
    "bob": 30,
    "charlie": 35
]

scores Map<String, Float>: [
    "math": 95.5,
    "science": 87.2,
    "history": 92.1
]

emptyMap Map<String, Int>: []
```

### Collection Type Inference
The type annotation determines which collection is created:

```suru
// Same syntax, different types based on annotation
numbersList List<Number>: [1, 2, 3]        // Creates List
numbersSet Set<Number>: [1, 2, 3]          // Creates Set  

// Maps require key:value syntax
mapping Map<Int, String>: [1: "one", 2: "two"]  // Creates Map
```

## Control flow statements

### Match Expressions

Suru uses pattern matching for control flow:

```suru
// Match on types
processResult: (result Result) String {
    match result {
        Success: "Operation completed successfully"
        Error: "An error occurred"
        Pending: "Operation in progress"
        _: "Unknown status"
    }
}

// Match with values
checkNumber: (n Number) String {
    match n {
        0: "zero"
        1: "one"
        _: "other number"
    }
}

// Match with member access
status: match user {
    .equals(admin): "admin"
    .equals(guest): "guest"
    _: "unknown user"
}
```

### Loops

Suru has no syntax for looping. 
Suru uses method-based iteration instead of loop keywords, maintaining consistency with our method-centric approach. Control flow is managed through continuation types.

#### Continuation Types
Control flow is managed with union types representing continuation decisions:

```suru
// Core continuation types
type Continue
type Break<T>: Some<T>, None
type Produce<T>
type Continuation<T>: Produce<T>, Continue, Break<T>
```

#### Number Iteration
Numbers provide iteration methods:

```suru
// Basic repetition
printHello: (step Number) {
    print(`Hello #{step.toString()}`)
}

5.times(printHello)  // Prints "Hello #1" through "Hello #5"

// Early termination
countWithBreak: (step Number) Continuation {
    print(`Step: {step.toString()}`)
    return match step {
        .equals(3): Break  // Stop at step 3
        _: Continue
    }
}

10.times(countWithBreak)  // Only prints steps 1, 2, 3

// Early termination with value
find3: (step Number) Continuation<Number> {
    return match step {
        .equals(3): Break(step)  // Stop at step 3
        _: Continue
    }
}

result Option<Number>: 10.times(find3) // returns Some(3)

// Accumulator
appendStars: (_, result String) Continuation<String> {
    return Produce(result + "*")
}

result: 3.times(appendStars, "+") // returns "+***"
```

#### Collection Iteration
Collections provide rich iteration methods:

```suru
numbers List<Number>: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]

// Basic iteration
printNumbers: (num Number) {
    print(`Number: {num.toString()}`)
}

numbers.each(printNumbers)

// Iteration with index
printNumbersWithIndex: (num Number, index Number) {
    print(`Index {index.toString()} {num.toString()}`)
}

numbers.each(printNumbersWithIndex)

```

### Infinite loops
Conditional loops using method calls:

```suru
// While-like behavior
while: (current Number) Continuation<Number> {
    print(`Count: {current.toString()}`)
    current: current.subtract(1)
    
    return match current.equals(3) {
        true: Break    // Early exit
        false: Continue(current)
    }
})

loop(while, 100);
```

## Error handling
The Suru language uses errors as values you can't throw an error.
You can use any of the built-in types or make your own.

```suru
type Result<T, E>: Ok T, Error E
type Option<T>: Some T, None // generally used when there is no value to return.
type Response<T, E>: Success T, Failure E
type Either<L, R>: Left L, Right R
```

### Short circuiting
Use the `try` keyword in front of a call to short-circuit if there is an error and return early.  `try` works with any union type with exactly two variants.

```suru
// Try unwraps the "success" variant (first one) or short-circuits with the "failure" variant (second one)
processData: (input String) Result<Data, Error> {
    // try unwraps Ok or returns Error
    parsed: try parseInput(input)     // parseInput returns Result<ParsedData, ParseError>

    // try unwraps Some or returns None (auto-converted to Err None)
    value: try findValue(parsed)      // findValue returns Option<Value>

    // try unwraps Success or returns Failure
    result: try sendRequest(value)    // sendRequest returns Response<Data, NetworkError>

    return Ok(result)
}
```
1. **Try Compatibility**: A type is try-compatible if it's a union with exactly 2 variants
2. **Success Unwrapping**: `try expr` where `expr Union<A, B>` produces type `A`
3. **Failure Propagation**: The containing function must return a union where the second variant is compatible with `B`

```suru
// Option type
type Option<T>: Some T, None

findUser: (id String) Option<User>
getProfile: (user User) Option<Profile>

getUserProfile: (id String) Option<Profile> {
    user: try findUser(id)        // Unwraps Some or returns None
    profile: try getProfile(user) // Chains naturally
    return Some(profile)
}

// Either type
type Either<L, R>: Left L, Right R

parseAndValidate: (input String) Either<Data, Error> {
    parsed: try parseJson(input)    // parseJson returns Either<JsonValue, ParseError>
    data: try validateData(parsed)  // validateData returns Either<Data, ValidationError>
    return Left(data)
}

// Custom domain types
type AuthResult<T>: Authenticated T, Unauthorized String
type DatabaseResult<T>: Found T, NotFound String

secureGetUser: (token String, id String) AuthResult<User> {
    session: try authenticate(token)  // Returns AuthResult<Session>
    user: try getUser(session, id)    // Returns DatabaseResult<User> - needs conversion
    return Authenticated(user)
}
```
### Pipe Integration

The try operator works beautifully with pipes:

```suru
// Clean pipeline with automatic unwrapping
processRequest: (request String) Result<Response, Error> {
    request
        | try parseJson
        | try validateRequest  
        | try processBusinessLogic
        | try formatResponse
        | try sendResponse
}
```


## Composition
Code reuse is done by composition
The `+` operator is used for all composition.

### Type composition
Suru composes types from other types similar to interface inheritance.

```suru
type Point: {
    x Number
    y Number
}

type Circle: Point + {
    radius Number
}
```

### Data composition

Building on the previous type declarations we can have:
```suru
aPoint Point: {
   x: 100
   y: 300
}

aCircle Circle: aPoint + {
    radius: 500
}
```

### Method composition

```suru
type Shape: Circle, Square

type AreaFunction: (shape Shape) Number

// Implementation of an area function
area AreaFunction: (shape){
    // some implementation
}

// function reuse with partial application
aCircle Circle: aPoint + {
    radius: 500

    // "adding" a method to the struct by partially applying a function with the instance itself
    area: + partial area(this) 
}

// Usage
theArea: aCircle.area()
```

Considerations:
- when composing structs if there are name conflicts the last overrides the previous

More Examples:

```suru
// Function library
validateEmail: (email String) Bool { ... }
formatPhone: (phone String) String { ... }
calculateTax: (amount Number, rate Number) Number { ... }

// Compose methods into structs
type User: {
    email String
    phone String

    // Add validation methods via composition
    isValidEmail: + partial validateEmail(this.email)
    formattedPhone: + partial formatPhone(this.phone)
}

type Invoice: {
    amount Number
    taxRate Number

    // Compose calculation method
    total: + partial calculateTax(this.amount, this.taxRate)
}

// Even compose methods from other types
type EnhancedCircle: Circle + {
    // Reuse area function but add logging
    areaWithLog: + partial area(this) | + partial logResult(_)

    // Compose validation from another type
    validate: + Point.validateCoordinates
}
```

More examples:

```suru
// Base functionality
logCall: (funcName String, result Any) Any {
    print(`Called {funcName}, result: {result}`)
    return result
}

validatePositive: (value Number) Number { ... }

// Compose a rich Circle type
aCircle Circle: aPoint + {
    radius: 500

    // Chain multiple behaviors
    area: + partial area(this)
          | + partial validatePositive(_)
          | + partial logCall("area", _)

    // Override inherited behavior
    move: + partial moveWithBounds(this, _, _)  // Last one wins over Point.move

    // Compose from multiple sources
    describe: + partial formatShape(this)
              | + partial addTimestamp(_)
              | + partial toUppercase(_)
}

// Usage
result: aCircle.area()
// Calls: area(aCircle) -> validatePositive(result) -> logCall("area", result)
```

## Documentation

Suru supports rich documentation using equals sign delimiters with markdown content and special keywords:

````suru
==========
# Calculate Circle Area
Calculates the area of a circle given its radius.

@param radius The radius of the circle in meters (must be positive)
@return The area in square meters
@example
```suru
area: calculateCircleArea(5.0)
// Returns: 78.54
```
@since 1.0.0
==========
calculateCircleArea: (radius Float) Float {
    return 3.14159 * radius * radius
}

============
# User Account Type
Represents a user account in the system with authentication capabilities.

@field id Unique identifier for the user
@field name Full name of the user  
@field email Contact email address
@deprecated Use UserV2 instead
@author Security Team
============
type User: {
    id UserId
    name String
    email String

    getName: () String
}
````

Documentation blocks must:
- Start and end with at least 4 equals signs (`====`)
- Contain valid markdown between the delimiters
- Support special `@keyword` annotations for structured metadata
- Can be placed before any top-level declaration

### Documentation Keywords

- `@param name description` - Parameter documentation
- `@return description` - Return value documentation
- `@example code` - Usage examples
- `@deprecated reason` - Mark as deprecated
- `@experimental note` - Mark as experimental
- `@todo description` - TODO items
- `@see reference` - Cross-references
- `@link url` - External links

## String interpolation

Suru features advanced string interpolation with multiple nesting levels using backticks:

### Single Backticks (\`)

For simple interpolation:

```suru
name: "Alice"
greeting: `Hello {name}!`
// Result: "Hello Alice!"
```

For multi-line strings follow the backticks with a new line.

```suru
name: "Alice"
greeting: (name) String {
    return `
    Hello {name}!
        How are you?
    `
} 
greeting(name) | print // Result: "Hello Alice!\n\tHow are you?"
```
 

### Double Backticks (\`\`)

```suru
user: getUser()
message: ``
    Welcome {{user.name}}!
    Your account balance is ${{user.balance}}.
    ``
```

### Triple Backticks (\`\`\`)
````suru
items: getItems()
report: ```
    Processing {{{items.length}}} items:
    {{{formatItemList(items)}}}
    Status: {{{getProcessingStatus()}}}
    ```
````

### Quad Backticks (\`\`\`\`)
`````suru
template: getTemplate()
rendered: ````
    Template: {{{{template.name}}}}
    Content: {{{{renderContent(template.data)}}}}
    Metadata: {{{{template.metadata.toString()}}}}
    ````
`````

The different backtick levels allow for flexible string templating.


## Getting Started

### Prerequisites

- [Docker](https://docs.docker.com/get-docker/) installed on your system
- Basic familiarity with Docker and command-line tools


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

## Resources

- [Rust Documentation](https://doc.rust-lang.org/)
- [LLVM Documentation](https://llvm.org/docs/)
- [Inkwell Documentation](https://thedan64.github.io/inkwell/)
- [Docker Documentation](https://docs.docker.com/)
