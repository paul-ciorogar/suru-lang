# Suru Programming Language

A general-purpose, high-level programming language with a focus on clean syntax, type safety, and modular design.

## Overview

Suru is designed with modern programming principles in mind, featuring:
- Clear, readable syntax with minimal punctuation
- Strong type system with generics support
- Module-based organization
- Pattern matching for control flow
- Intersection and union types
- Method and function overrloading
- Method and function curring 
- Piped values
- Composition
- Rich documentation support
- Advanced string interpolation with multiple nesting levels

## Table of Contents

- [File Structure](#file-structure)
- [Lexical elements and literals](#lexical_elements_and_literals)

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
simple: &#96;Hello {name}!&#96;
```

#### Escape Characters #

- \b - backspace (BS)
- \e - escape (ESC)
- \n - newline
- \r - carriage return
- \t - tab
- \\ - backslash
- \" - double quote (if needed)
- \' - single quote (if needed)
- \` - single tick (if needed)
- \NNN- octal 6 bit character (3 digits)
- \xNN - hexadecimal 8 bit character (2 digits)
- \uNNNN - hexadecimal 16-bit Unicode character UTF-8 encoded (4 digits)
- \UNNNNNNNN - hexadecimal 32-bit Unicode character UTF-8 encoded (8 digits)

### Numbers

Multiple Number Bases:

- Binary: `0b1010, 0b1010_1100`
- Octal: `0o755, 0o77_55`
- Hexadecimal: `0xFF, 0xDEAD_BEEF`
- Decimal: `123`, `1_000_000`

Underscore Separators for Readability:

- `1_000_000` instead of 1000000
- `0xDEAD_BEEF` for hex numbers
- Works in all number bases

Scientific Notation:

- `1e10, 1.5e10, 1e-10`
- Supports underscores: `1_000e5`


Hexadecimal Floats:

- `0x1.Ap+3` 


Type Suffixes:

- Numberegers: `i8, i16, i32, i64, i128, u8, u16, u32, u64, u128`
- Floats: `f16, f32, f64, f128`

Examples

```suru
// Decimal with separators and suffix
count: 1_000_000u64;

// Binary with suffix
flags: 0b1010_1100u8;

// Hex with suffix
address: 0xDEAD_BEEFuintptr;

// Float with scientific notation
pi: 3.14159_26535f64;
large_number: 1.5e10f32;

// Hex float
precise: 0x1.921FB54442D18p+1f64; // π in hex float
```

## Variable declarations 

A variable declaration declares a new variable for the current scope.
```suru
name : value // type is infered
name Type : value
```
Declarations at the file scope are constants.
A constant’s value cannot be changed. The constant’s value must be able to be evaluated at compile time

### Assignment statements
```suru
name : value
```

## Modules

Suru programs are organized into modules. A module is a directory of Suru code files, one of which has module declaration at the top. Execution starts in the main module's main ffunction.

### Module Declaration

Module names have to start with a letter and can contain numbers dots and underscores

```suru
module Calculator
```

### Imports

suru supports three types of imports:

#### Full Module Import
```suru
import {
    math : standard_math
}
// Usage: math.sin(3.14)
```

#### Selective Import
```suru
import {
    {sin, cos, pi} : standard_math
}
// Usage: sin(pi)
```

#### Import All
```suru
import {
    * : standard_math
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
`not` not false = true
`and` true and true = true
`for` true or false = true

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

```
type UserId : Number
type Username : String
```

### Union Types
Alternative types:

```suru
type Status : Success, Error, Loading
type Value : Int, String, Bool
```

### Struct Types
Records with fields and method declarations:

```suru
type Person : {
    name String
    age Number
    
    greet: () String
    birthday: () Person
}
```

### Intersection Types
Combine types using `+`:

```suru
type Employee : Person + {
    salary Int
    department String
}
```

### Function Types
Function signatures must be defined as named types:

```suru
type AddFunction : (a Number, b Number) Number
type Predicate : (value String) Bool
type VoidFunction : ()
type Identity<T> : (value T) T
type UnaryOperator : (x Float) Float
```

### Generic Types
Define types that work with multiple specific types:

```suru
// Single type parameter
type List<T> : {
    items Array<T>
    size Int
    
    add: (item T) List<T>
    get: (index Int) T
    contains: (item T) Bool
    map: <R>(transform (T) R) List<R>
}

// Multiple type parameters  
type Map<K, V> : {
    entries Array<Pair<K, V>>
    
    put: (key K, value V) Map<K, V>
    get: (key K) Option<V>
    containsKey: (key K) Bool
}

// Generic types with constraints
type Comparable<T: Orderable> : {
    value T
    
    compare: (other Comparable<T>) Ordering
    lessThan: (other Comparable<T>) Bool
}
```

## Functions
All function types must be defined as named types first:

```suru
type AddFunction : (x Number, y Number) Number
type UnaryFunction : (x Number) Number
type Transformer : (input String) String
type AnyToString : (input) String

// Function returning a simple type
add: (x Number, y Number) Number {
    return x.add(y)
}

// Function with infered types
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
    temp : fn(value)
    return fn(temp)
}
```

### Generic Functions
Functions that work with multiple types:

```
// Simple generic function
identity<T>: (value T) T {
    return value
}

// Multiple type parameters
map<T, R>: (items List<T>, transform Transform<T, R>) List<R> {
    result : List<R>()
    // Implementation iterates and transforms
    return result
}

// Generic function with constraints
sort<T: Orderable>: (items List<T>) List<T> {
    // Implementation uses T's ordering methods
    return items.quick_sort()
}
```

### Function overloading
```suru
// Function overloading (same name, different signatures)
add: (a i32, b i32) i32 { return a + b }
add: (a f64, b f64) f64 { return a + b }
add: (a i32) i32 { return a }
add: (a string, b string) string { return a + b }
```

### Method overloading

Same as function overloading

```suru
type Adds: {
    add: (a i32, b i32) i32 { return a + b }
    add: (a f64, b f64) f64 { return a + b }
    add: (a i32) i32 { return a }
    add: (a string, b string) string { return a + b }
}
```

### Overloading by Return Type

```suru
// Same function name and parameters, different return types
parse: (input String) Int {
    return input.to_int()
}

parse: (input String) Float {
    return input.to_float()
}

parse: (input String) Bool {
    return input.equals("true")
}

// Usage - type annotation determines which overload
int_value Int : parse("123")      // Calls parse: (String) Int
float_value Float : parse("3.14") // Calls parse: (String) Float
bool_value Bool : parse("true")   // Calls parse: (String) Bool
```

## Pipeline

The `|` (pipe) operator can be used to pipe values to functions

```suru
2_283 | subtract(_, 2) | print // 2281 would be printed

processed : "Hello, world!"
    | trim()
    | to_lower()
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

```
type User: {
    username: String                      // Public field

    authenticate: (password String) Bool  // Public method
}

user: User : {
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
        account_id: id
        transaction_count: 0
        _ balance: initial

        deposit: (amount Float) Float {
            return match this.validate(amount) {  // Call private method
                true : {
                    this.balance : this.balance.add(amount)
                    this.logTransaction("deposit", amount)
                    return this.balance
                },
                false : this.balance
            }
        }

        _ validate: (amount Float) Bool {  // Private method implementation
            return amount.greater_than(0.0)
        }

        _ log_transaction: (type String, amount Float) {
            // Private logging logic
        }

        // ... other methods
    }

    return impl
}

// Usage
account : BankAccount(100.0, "ACC123")
// account.balance        // ❌ Compile error: not in public interface  
// account.validate(50.0) // ❌ Compile error: private method not accessible
balance : account.getBalance()  // ✅ OK: public method
```

## Currying and Partial Application

All functions and methods in Suru can be curried.
Calling a function with `_` placeholder instead of an argument returns a new function that takes the remaining arguments which where given the placeholder.
Explicit `partial` keword when a function has many arguments and adding a lot of `_, _, _, _, _, _, _, _, _,` would look ugly.

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

### Function Currying
```
type BinaryFunction : (x Number, y Number) Number
type UnaryFunction : (x Number) Number

// Multi-parameter function
add: (x Number, y Number, z Number) Number {
    return x.add(y).add(z)
}

// Partial application creates functions of named types
addFive : add(5)           // Type: BinaryFunction (conceptually)
addEight : add(5, 3)  // Type: UnaryFunction

// Usage
result1 : addFive(2, 1)    // Same as add(5, 2, 1) = 8
result2 : addEight(4) // Same as add(5, 3, 4) = 12
```

### Method Currying
Methods can also be curried:

```
type BinaryOperation : (a Int, b Int) Int
type UnaryOperation : (x Int) Int

type Calculator : {
    multiply: (a Int, b Int, c Int) Int
}

calc Calculator : {
    multiply: (a Int, b Int, c Int) Int {
        return a.multiply(b).multiply(c)
    }
}

// Curry the method
double : calc.multiply(2, _, _)        // Type: BinaryOperation (conceptually)
double_triple : calc.multiply(2, 3, _)  // Type: UnaryOperation

result : double_triple(4)        // 2 * 3 * 4 = 24
```

## Lexical Scoping

Functions and methods have strict lexical scoping - they can only access:
1. Their parameters
2. Variables declared within their body
3. Global constants and functions

They **cannot** access variables from outer scopes. This ensures predictable behavior and makes currying safe.

### Correct Scoping
```
constant: 42

outerFunction: (x Number) Number {
    localVar : 10
    
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
```
type NumberFunction : (x Number) Number

// Parameters become part of the curried function's closure
add: (x Number, y Number) Number {
    return x.add(y)
}

addFive : add(5)
result : addFive(3)  // 8
```

## Collections

Suru provides four built-in collection types, all using the unified `[]` syntax for creation. The type annotation determines which collection is created.

### Lists
Ordered collections that allow duplicates:

```
// List creation using [] syntax
numbers List<Number> : [1, 2, 3, 4, 5]
names List<String> : ["alice", "bob", "charlie"]
empty_list List<Float> : []

// List building
extended : numbers
    .add(6)
    .add([7, 8, 9])
    .set(0, 0)                         // Insert at index
```

### Sets
Unordered collections with unique elements:

```suru
// Set creation - duplicates automatically removed
uniqueNumbers Set<Number> : [1, 2, 3, 2, 1]  // Results in {1, 2, 3}
colors Set<String> : ["red", "green", "blue"]
emptySet Set<Float> : []
```

### Maps
Key-value collections:

```suru
// Map creation using key:value syntax
userAges Map<String, Number> : [
    "alice": 25,
    "bob": 30,
    "charlie": 35
]

scores Map<String, Float> : [
    "math": 95.5,
    "science": 87.2,
    "history": 92.1
]

emptyMap Map<String, Int> : []
```

### Collection Type Inference
The type annotation determines which collection is created:

```suru
// Same syntax, different types based on annotation
numbersList List<Number> : [1, 2, 3]        // Creates List
numbersSet Set<Number> : [1, 2, 3]          // Creates Set  

// Maps require key:value syntax
mapping Map<Int, String> : [1: "one", 2: "two"]  // Creates Map
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
status : match user_input {
    .equals("quit") : "exiting",
    .equals("help") : "showing help",
    _ : "unknown command"
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
type Continuation<T> : Produce<T>, Continue, Break<T>
```

#### Number Iteration
Numbers provide iteration methods:

```suru
// Basic repetition
printHello: (step Number) {
    print("Hello #" + step.toString())
}

5.times(printHello)  // Prints "Hello #1" through "Hello #5"

// Early termination
countWithBreak: (step Number) Continuation {
    print("Step: " + step.toString())
    return match step {
        .equals(3) : Break,  // Stop at step 3
        _ : Continue
    }
}

10.times(countWithBreak)  // Only prints steps 1, 2, 3

// Early termination with value
find3: (step Number) Continuation<Number> {
    return match step {
        .equals(3) : Break(step),  // Stop at step 3
        _ : Continue
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
numbers List<Number> : [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]

// Basic iteration
printNumbers: (num Number) {
    print("Number: " + num.toString())
}

numbers.each(printNumbers)

// Iteration with index
printNumbersWithIndex: (num Number, index Number) {
    print("Index " + index.toString() + ": " + num.toString())
}

numbers.each(printNumbersWithIndex)

```

### Infinit loops
Conditional loops using method calls:

```suru
// While-like behavior
while: (current Number) Continuation<Number> {
    print("Count: " + current.toString())
    current : current.subtract(1)
    
    return match current.equals(3) {
        true : Break,    // Early exit
        false : Continue(current)
    }
})

loop(while, 100);
```

## Composition
Code reuse is done by composition
The `+` operator is used for all composition.

### Type composition
Suru composes types from other types similar to interface iheritence.

```
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
   Y: 300
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
aCircle Circle: aPonit + {
    radius: 500
    area: + partial area(this) // "adding" a method to the struct by partially applying a function with the instance itself
}

// Usage
theArea: aCircle.area()
```

Considerations:
- when composing structs if there are name conficts the last overrides the previous

More Examples:

```suru
// Function library
validate_email: (email String) Bool { ... }
format_phone: (phone String) String { ... }
calculate_tax: (amount Number, rate Number) Number { ... }

// Compose methods into structs
type User: {
    email String
    phone String
    
    // Add validation methods via composition
    is_valid_email: + partial validate_email(this.email)
    formatted_phone: + partial format_phone(this.phone)
}

type Invoice: {
    amount Number
    tax_rate Number
    
    // Compose calculation method
    total: + partial calculate_tax(this.amount, this.tax_rate)
}

// Even compose methods from other types
type EnhancedCircle: Circle + {
    // Reuse area function but add logging
    area_with_log: + partial area(this) | + partial log_result(_)
    
    // Compose validation from another type
    validate: + Point.validate_coordinates
}
```

More examples:

```suru
// Base functionality
log_call: (func_name String, result Any) Any { 
    print(`Called {func_name}, result: {result}`)
    return result
}

validate_positive: (value Number) Number { ... }

// Compose a rich Circle type
aCircle Circle: aPoint + {
    radius: 500
    
    // Chain multiple behaviors
    area: + partial area(this) 
          | + partial validate_positive(_) 
          | + partial log_call("area", _)
    
    // Override inherited behavior
    move: + partial move_with_bounds(this, _, _)  // Last one wins over Point.move
    
    // Compose from multiple sources
    describe: + partial format_shape(this) 
              | + partial add_timestamp(_)
              | + partial to_uppercase(_)
}

// Usage
result: aCircle.area()  
// Calls: area(aCircle) -> validate_positive(result) -> log_call("area", result)
```

## Documentation

Suru supports rich documentation using equals sign delimiters with markdown content and special keywords:

```tracelang
==========
# Calculate Circle Area
Calculates the area of a circle given its radius.

@param radius The radius of the circle in meters (must be positive)
@return The area in square meters
@example
&#96;&#96;&#96;suru
area: calculateCircleArea(5.0)
// Returns: 78.54
&#96;&#96;&#96;
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
```

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

### Single Backticks (&#96;)

For simple interpolation:

```suru
name: "Alice"
greeting: &#96;Hello {name}!&#96;
// Result: "Hello Alice!"
```

For multi-line strings follow the backticls with a new line.

```suru
name: "Alice"
greeting: (name) String {
    return &#96;
    Hello {name}!
        How are you?
    &#96;
} 
greeting(name) | print // Result: "Hello Alice!\n\tHow are you?"
```
 

### Double Backticks (&#96;&#96;)

```suru
user: getUser()
message: &#96;&#96;
    Welcome {{user.name}}!
    Your account balance is ${{user.balance}}.
    &#96;&#96;
```

### Triple Backticks (&#96;&#96;&#96;)
```tracelang
items: getItems()
report: &#96;&#96;&#96;
    Processing {{{items.length}}} items:
    {{{formatItemList(items)}}}
    Status: {{{getProcessingStatus()}}}
    &#96;&#96;&#96;
```

### Quad Backticks (&#96;&#96;&#96;&#96;)
```tracelang
template: getTemplate()
rendered: &#96;&#96;&#96;&#96;
    Template: {{{{template.name}}}}
    Content: {{{{renderContent(template.data)}}}}
    Metadata: {{{{template.metadata.toString()}}}}
    &#96;&#96;&#96;&#96;
```

The different backtick levels allow for flexible string templating.

