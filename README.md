# Suru Programming Language

Suru is high-level programming language designed for productivity and maintainablility. It features a clean syntax with strong typing, documentation-first design, and powerful pattern matching capabilities.

## Table of Contents

- [Getting Started](#getting-started)
- [Language Basics](#language-basics)
- [Module System](#module-system)
- [Documentation](#documentation)
- [Variables and Types](#variables-and-types)
- [Functions](#functions)
- [Type Definitions](#type-definitions)
- [Control Flow](#control-flow)
- [String Interpolation](#string-interpolation)
- [Examples](#examples)

## Getting Started

A Suru program 

```suru
module main

import {
    * : io
}

main: () {
    printLine("Hello, Suru!")
}
```

This is the simplest program in Suru.
The first line says this is the main module. This is the entry point of your program.
Then we have an import block where we add to the file scope everything from the io module.
`main: () {}` declares a function named main that is called wen you run this program.
and last we have the `printLine` function call from the io module that does exactly what you expect.

```suru
module MyProgram

import {
    utils : standard_lib
    {print, readline} : io
}

export {
    main
}

main: () {
    print("Hello, Suru!")
}
```

## Language Basics

### Syntax Fundamentals

- **Statements end** with newlines or semicolons
- **Blocks** are defined with `{ }`
- **Comments** use `//` for line comments
- **Case sensitive** identifiers
- **Type annotations** are optional but recommended

## Module System

### Module Declaration

Every Suru file can start with a module declaration:

```suru
module Calculator
```

### Imports

Suru supports three types of imports:

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

Specify what your module makes available to other modules:

```suru
export {
    Calculator
    add
    subtract
}
```

## Documentation

Suru has built-in documentation support using `====` delimiters:

```suru
====
This is a documentation block for the following function.
You can use **markdown** formatting here.

@param x The first number to add
@param y The second number to add  
@return The sum of x and y
@example add(5, 3) // returns 8
====
add: (x Number, y Number) Number {
    return x + y
}
```

### Documentation Keywords

- `@param name description` - Parameter documentation
- `@return description` - Return value documentation
- `@example code` - Usage examples
- `@deprecated reason` - Mark as deprecated
- `@experimental note` - Mark as experimental
- `@todo description` - TODO items
- `@see reference` - Cross-references
- `@link url` - External links

## Variables and Types

### Variable Declaration

Variables are declared with the pattern: `name : value` or `name Type : value`

```suru
// Type inferred
name : "Alice"
age : 25
isStudent : true

// Explicit typing
height Number : 5.9
grades List<Number> : [85, 92, 78]
```

### Basic Types

```suru
// Numbers
count : 42
price : 19.99

// Strings  
message : "Hello World"
singleQuoted : 'Also valid'

// Booleans
isReady : true
isComplete : false
```

## Functions

### Function Declaration

Functions use the syntax: `name: (parameters) ReturnType { body }`

```suru
// Simple function types infered
greet: (name) {
    return "Hello, " + name + "!"
}

// Simple function
greet: (name String) String {
    return "Hello, " + name + "!"
}

// Function with multiple parameters
calculateArea: (width Number, height Number) Number {
    return width * height
}

// Function without return type (void)
printMessage: (msg String) {
    print(msg)
}

// Function with no parameters
getCurrentTime: () String {
    return getTime()
}
```

### Function Calls

```suru
result : greet("Alice")
area : calculateArea(10, 5)
printMessage("Debug info")
time : getCurrentTime()
```

### Pipe expressions 

```suru
// Simple piping
data | processData | formatOutput

// Chaining multiple operations
numbers | filter | map | reduce

// Mixed with other expressions
result : getValue() | transform | validate
```

## Type Definitions

### Struct Types

Define custom data structures:

```suru
type Person: {
    name String
    age Number
    email String
    
    // Methods can be declared in type definitions
    getDisplayName: () String
    isAdult: () Boolean
}
```

### Union Types

Use commas for union types (OR relationship):

```suru
type Result: Success, Error, Pending

type StringOrNumber: String, Number
```

### Intersection Types

Use `+` for intersection types (AND relationship):

```suru
type Employee: Person + {
    employeeId String
    department String
}
```

### Generic Types

```suru
type Container<T>: {
    value T
    isEmpty: () Boolean
}

type Pair<T, U>: {
    first T  
    second U
}
```

### Function Types

```suru
type MathOperation: (Number, Number) Number

type EventHandler: (Event) 
```

## Control Flow

### Match Expressions

Suru uses pattern matching for control flow:

```suru
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
handleResponse: (response HttpResponse) {
    match response.status {
        200: processSuccess()
        404: handleNotFound()
        _: handleError()
    }
}
```

## String Interpolation

Suru supports multiple levels of string interpolation:

### Single Backticks
```suru
name : "Alice"
message : `Hello, {name}!`
```

### Double Backticks (Multiline)
```suru
template : ``
Welcome to Suru, {{username}}!
Your account balance is: ${{balance}}
``
```

### Triple Backticks (Complex)
```suru
report : ```
Report for {{{date}}}:
- Total users: {{{userCount}}}
- Revenue: ${{{revenue}}}
```

### Quad Backticks (Maximum Nesting)
```suru
complexTemplate : ````
{{{{deeplyNestedData.process()}}}}
````
```

## Examples

### Basic Calculator

```suru
====
A simple calculator module demonstrating basic Suru features
====
module Calculator

export {
    add
    subtract  
    multiply
    divide
}

====
Adds two numbers together
@param a First number
@param b Second number
@return Sum of a and b
====
add: (a Number, b Number) Number {
    return a + b
}

====
Subtracts second number from first
@param a First number  
@param b Second number to subtract
@return Difference of a and b
====
subtract: (a Number, b Number) Number {
    return a - b  
}

multiply: (a Number, b Number) Number {
    return a * b
}

divide: (a Number, b Number) Number {
    match b {
        0: error("Division by zero")
        _: return a / b
    }
}
```

### User Management System

```suru
module UserSystem

type Role: Admin, User, Guest

type User: {
    id String
    name String  
    email String
    role Role
    isActive Boolean
    
    getDisplayName: () String
    hasPermission: (permission String) Boolean
}

====
Creates a new user with default settings
@param name User's full name
@param email User's email address  
@return New User instance
====
createUser: (name String, email String) User {
    return {
        id: generateId()
        name: name
        email: email  
        role: Guest
        isActive: true
        
        getDisplayName: () String {
            return `{name} ({email})`
        }
        
        hasPermission: (permission String) Boolean {
            match role {
                Admin: true
                User: permission != "admin"
                Guest: permission == "read"
                _: false
            }
        }
    }
}

====
Processes user authentication
@param credentials Login credentials
@return Authentication result
====
authenticateUser: (credentials LoginCredentials) AuthResult {
    user : findUserByEmail(credentials.email)
    
    match user {
        null: return AuthResult.Failed
        _: {
            isValid : validatePassword(credentials.password, user.passwordHash)
            match isValid {
                true: return AuthResult.Success  
                false: return AuthResult.Failed
            }
        }
    }
}
```

### Configuration System

```suru
module Config

type Environment: Development, Staging, Production

type DatabaseConfig: {
    host String
    port Number
    username String
    password String
    database String
}

type AppConfig: {
    environment Environment
    database DatabaseConfig
    apiKey String
    debugMode Boolean
}

====
Loads configuration based on environment
@param env Target environment
@return Complete application configuration
====
loadConfig: (env Environment) AppConfig {
    baseConfig : {
        environment: env
        apiKey: getEnvVar("API_KEY")
        debugMode: env == Development
        
        database: match env {
            Development: {
                host: "localhost"
                port: 5432
                username: "dev_user"
                password: "dev_pass"  
                database: "app_dev"
            }
            
            Staging: {
                host: getEnvVar("STAGING_DB_HOST")
                port: 5432
                username: getEnvVar("STAGING_DB_USER")
                password: getEnvVar("STAGING_DB_PASS")
                database: "app_staging"
            }
            
            Production: {
                host: getEnvVar("PROD_DB_HOST")
                port: 5432  
                username: getEnvVar("PROD_DB_USER")
                password: getEnvVar("PROD_DB_PASS")
                database: "app_prod"
            }
        }
    }
    
    return baseConfig
}
```

## Best Practices

1. **Always document your functions** with the documentation block syntax
2. **Use explicit types** for function parameters and return values
3. **Prefer pattern matching** over complex conditional logic  
4. **Keep functions small** and focused on single responsibilities
5. **Use meaningful variable names** that express intent
6. **Group related functionality** into modules
7. **Export only what's needed** to maintain clean module interfaces

## Language Features Summary

- ✅ **Strong typing** with optional type inference
- ✅ **Documentation-first** design with built-in doc blocks
- ✅ **Pattern matching** for control flow
- ✅ **Module system** with flexible import/export
- ✅ **Generic types** for reusable code  
- ✅ **String interpolation** with multiple nesting levels
- ✅ **Union and intersection types** for flexible type composition
- ✅ **Clean syntax** focused on readability

Suru is designed to make code both powerful and readable, with features that scale from simple scripts to large applications.
