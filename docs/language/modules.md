# Modules

> Module system, imports, and exports

## Module Organization

Suru programs are organized into modules. A module is a directory of Suru code files, one of which has a module declaration at the top. Execution starts in the main module's `main` function.

## Module Declaration

All Suru source files must declare a module. There are two types of module declarations:

### Main Modules

Main module names must start with a letter and can contain numbers, and underscores:

```suru
module Calculator
```

```suru
module math.geometry
```

```suru
module app_v2.handlers
```

### Submodules

Submodules are internal modules within a module's directory. They are declared with a leading dot:

```suru
module .utils
```

```suru
module .helpers
```

```suru
module .validation
```

**Key characteristics:**
- **File naming**: File names are standard (e.g., `utils.suru`), but the module declaration uses the dot (e.g., `module .utils`)
- **Visibility**: Submodules are only visible within their parent module's directory hierarchy
- **Not directly accessible**: External modules cannot import submodules directly
- **Can be re-exported**: Main modules can selectively import and re-export items from submodules

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

Exports specify what your module makes available to other modules. All modules (both main modules and submodules) use the same export syntax:

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

**Submodule exports:**
```suru
module .utils

export {
    formatNumber
    validateInput
}

formatNumber: (n Number) String { /* ... */ }
validateInput: (s String) Bool { /* ... */ }
internalHelper: () { /* ... */ }  // Not exported
```

Submodule exports are visible to:
- The main module in the same directory
- Other submodules in the same directory
- Submodules in nested subdirectories

## Module Structure Example

```
calculator/
├── mod.suru              // Main module: "module Calculator"
├── operations.suru       // Submodule: "module .operations"
├── advanced.suru         // Submodule: "module .advanced"
└── helpers/
    └── validation.suru   // Nested submodule: "module .validation"
```

**mod.suru** (Main module):
```suru
module Calculator

import {
    {add, subtract}: operations    // Import from operations submodule
    {power, sqrt}: advanced         // Import from advanced submodule
}

export {
    Calculator
    add           // Re-exported from operations submodule
    subtract      // Re-exported from operations submodule
    power         // Re-exported from advanced submodule
    sqrt          // Re-exported from advanced submodule
}

type Calculator: {
    value Number
    add: (n Number) Calculator
    subtract: (n Number) Calculator
}
```

**operations.suru** (Submodule):
```suru
module .operations

import {
    {isValid}: helpers.validation  // Submodules can import from nested submodules
}

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

**advanced.suru** (Submodule):
```suru
module .advanced

import {
    {multiply}: operations  // Sibling submodule import
}

export {
    power
    sqrt
}

power: (base Number, exp Number) Number {
    // Implementation using multiply from operations
}

sqrt: (n Number) Number {
    // Implementation
}
```

## Module Resolution

### Submodules (Same Directory)

Import from submodules in the same directory by referencing them by name (without the dot):

```suru
import {
    {helper}: utils  // Looks for utils.suru containing "module .utils"
}
```

```suru
import {
    {add, subtract}: operations  // Import specific items from .operations submodule
}
```

### Nested Submodules

Submodules can import from submodules in nested directories:

```suru
module .operations

import {
    {isValid}: helpers.validation  // Imports from helpers/validation.suru
}
```

### External Modules

External modules (outside the directory hierarchy) use standard module paths:

```suru
import {
    math.geometry     // Looks for math/geometry module
}
```

### Standard Library

```suru
import {
    io                // Standard library module
    collections       // Standard library module
}
```

## Submodule Visibility

Submodules are **internal to their parent module** and have restricted visibility:

### Can Access Submodules:
1. **Main module in same folder**: The module with a standard declaration (e.g., `module Calculator`) can import from submodules
2. **Sibling submodules**: Submodules in the same directory can import from each other
3. **Nested submodules**: Submodules in subdirectories can import from parent directory submodules

### Cannot Access Submodules:
- **External modules**: Modules outside the directory hierarchy cannot directly import submodules
- To expose submodule functionality externally, the main module must import and re-export items

### Example: Re-exporting Submodule Items

```suru
// mod.suru (Main module)
module Calculator

import {
    {formatNumber, parseInput}: utils  // Import from .utils submodule
}

export {
    calculate
    formatNumber  // Re-export from submodule
    parseInput    // Re-export from submodule
}

calculate: (expr String) Number {
    input: parseInput(expr)
    result: evaluate(input)
    return result
}
```

External modules can now use `formatNumber` and `parseInput` through the `Calculator` module:

```suru
// app.suru (External module)
module App

import {
    {formatNumber}: Calculator  // Access re-exported item
}

main: () {
    formatted: formatNumber(42.5)
    print(formatted)
}
```

## Best Practices

1. **All files must have module declarations**: Use `module Name` for main modules or `module .name` for submodules
2. **One module per directory**: Clear organization with a main module and optional submodules
3. **Use submodules for internal organization**: Break complex modules into focused submodules
4. **Explicit exports**: Only export public API from both main modules and submodules
5. **Selective imports**: Import only what you need
6. **Avoid import all (`*`)**: Prevents namespace pollution
7. **Re-export thoughtfully**: Use the main module to curate the public API by selectively re-exporting submodule items
8. **Use aliases for clarity**: Rename imports when needed
9. **Group related functionality**: Keep modules and submodules focused
10. **Document public API**: Help module users understand what's available

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
    result: add(multiply(2, 3), 4)  // (2 * 3) + 4 = 10
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

// Private helper (not exported)
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
