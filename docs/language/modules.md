# Modules

> Module system, imports, and exports

## Module Organization

Suru programs are organized into modules. A module is a directory of Suru code files, one of which has a module declaration at the top. Execution starts in the main module's `main` function.

## Module Declaration

Module names must start with a letter and can contain numbers, dots, and underscores:

```suru
module Calculator
```

```suru
module math.geometry
```

```suru
module app_v2.handlers
```

## Imports

Suru supports three types of imports:

### Full Module Import

Import an entire module:

```suru
import {
    math
    mathAlias: math
    io
}

// Usage
result: math.sin(3.14)
aliased: mathAlias.cos(1.57)
io.stdout.write("Hello")
```

**Syntax:**
- `moduleName` - Import module with its original name
- `alias: moduleName` - Import module with a custom alias

### Selective Import

Import specific items from a module:

```suru
import {
    {sin, cos, pi}: math
}

// Usage
angle: sin(pi)
result: cos(pi / 2)
```

**Syntax:** `{item1, item2, ...}: moduleName`

### Import All

Import all exported items from a module:

```suru
import {
    *: math
}

// Usage
result: sin(pi)
angle: cos(pi / 2)
area: sqrt(value)
```

**Syntax:** `*: moduleName`

**Warning:** Use sparingly to avoid namespace pollution.

## Exports

Exports specify what your module makes available to other modules.

### Module-Level Exports

If a file starts with a module declaration, exports define the public API:

```suru
module Calculator

export {
    Calculator
    add
    subtract
}

// Only exported items are accessible from other modules
type Calculator: {
    value Number
}

add: (a Number, b Number) Number { return a + b }
subtract: (a Number, b Number) Number { return a - b }
multiply: (a Number, b Number) Number { return a * b }  // Not exported
```

### File-Level Exports

If a file does not have a module declaration, exports are only available to files in the same directory:

```suru
// file: utils.suru (no module declaration)
export {
    formatNumber
    validateInput
}

formatNumber: (n Number) String { /* ... */ }
validateInput: (s String) Bool { /* ... */ }
internalHelper: () { /* ... */ }  // Not exported
```

## Module Structure Example

```
calculator/
├── mod.suru              # Module root with module declaration
├── operations.suru       # Basic operations
├── advanced.suru         # Advanced functions
└── utils.suru            # Internal utilities
```

**mod.suru:**
```suru
module Calculator

import {
    {add, subtract}: operations
    {power, sqrt}: advanced
}

export {
    Calculator
    add
    subtract
    power
    sqrt
}

type Calculator: {
    value Number
    add: (n Number) Calculator
    subtract: (n Number) Calculator
}
```

**operations.suru:**
```suru
export {
    add
    subtract
}

add: (a Number, b Number) Number {
    return a + b
}

subtract: (a Number, b Number) Number {
    return a - b
}
```

## Module Resolution

### Local Modules

```suru
import {
    {helper}: utils  # Looks for utils.suru in same directory
}
```

### Nested Modules

```suru
import {
    math.geometry     # Looks for math/geometry module
}
```

### Standard Library

```suru
import {
    io                # Standard library module
    collections       # Standard library module
}
```

## Best Practices

1. **One module per directory**: Clear organization
2. **Explicit exports**: Only export public API
3. **Selective imports**: Import only what you need
4. **Avoid import all (`*`)**: Prevents namespace pollution
5. **Use aliases for clarity**: Rename imports when needed
6. **Group related functionality**: Keep modules focused
7. **Document public API**: Help module users

## Examples

### Simple Module

```suru
module Math

export {
    add
    subtract
    multiply
    divide
}

add: (a Number, b Number) Number {
    return a + b
}

subtract: (a Number, b Number) Number {
    return a - b
}

multiply: (a Number, b Number) Number {
    return a * b
}

divide: (a Number, b Number) Result<Number, String> {
    return match b.equals(0) {
        true: Error("Division by zero")
        false: Ok(a / b)
    }
}
```

### Using a Module

```suru
module App

import {
    {add, multiply}: Math
}

main: () {
    result: add(multiply(2, 3), 4)  # (2 * 3) + 4 = 10
    print(result.toString())
}
```

### Module with Types

```suru
module User

export {
    User
    createUser
    validateUser
}

type User: {
    id UserId
    name String
    email String
}

createUser: (name String, email String) Result<User, String> {
    validated: validateUser(name, email)
    return match validated {
        Ok: Ok({
            id: generateId()
            name: name
            email: email
        })
        Error: Error(validated.error)
    }
}

validateUser: (name String, email String) Result<Bool, String> {
    // Validation logic
}

# Private helper (not exported)
generateId: () UserId {
    // ID generation logic
}
```

### Multiple Imports

```suru
module App

import {
    {User, createUser}: User
    {save, find}: Database
    {validate}: Validation
    io
}

registerUser: (name String, email String) Result<User, String> {
    user: try createUser(name, email)
    try validate(user)
    saved: try save(user)
    io.stdout.write("User registered successfully")
    return Ok(saved)
}
```

---

**See also:**
- [Syntax](syntax.md) - File structure
- [Types](types.md) - Exporting types
- [Functions](functions.md) - Exporting functions
- [Variables](variables.md) - Module-level constants
