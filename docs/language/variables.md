# Variables

> Variable declarations and assignments in Suru

## Variable Declarations

A variable declaration declares a new variable for the current scope:

```suru
name: value          # Type is inferred
name Type: value     # Explicit type annotation
```

### Type Inference

Suru can infer types from values:

```suru
x: 42                # Inferred as Number
name: "Alice"        # Inferred as String
flag: true           # Inferred as Bool
list: [1, 2, 3]      # Inferred as List<Number>
```

### Explicit Type Annotations

You can explicitly specify types:

```suru
count Number: 42
username String: "alice"
age Int64: 30
price Float64: 99.99
```

## Scope Rules

### File Scope (Constants)

Declarations at the file scope are **constants**:

```suru
MAX_SIZE: 100        # File-scoped constant
PI: 3.14159          # Cannot be reassigned

main: () {
    # Can use constants
    area: PI * radius * radius
}
```

**Important:** A constant's value cannot be changed after declaration. The constant's value must be able to be evaluated at compile time.

### Function Scope (Mutable)

Variables declared within functions are **mutable**:

```suru
calculate: (x Number) Number {
    result: x * 2     # Mutable variable
    result: result + 10  # Can reassign
    return result
}
```

## Statement Termination

Declarations end with a newline unless the next line has a continuation operator:

```suru
# Single line declarations
x: 42
y: 100

# Multi-line with continuation
result: calculateValue()
    | transform
    | validate
```

## Examples

### Basic Declarations

```suru
# Numbers
count: 42
price: 19.99
hex: 0xFF
binary: 0b1010

# Strings
name: "Alice"
message: 'Hello, World!'
interpolated: `Hello {name}!`

# Booleans
isActive: true
hasPermission: false

# Collections
numbers: [1, 2, 3, 4, 5]
names: ["Alice", "Bob", "Charlie"]
scores: ["math": 95.5, "science": 87.2]
```

### Type Annotations

```suru
# Explicit types
id UserId: 12345
username String: "alice"
age Int64: 30
balance Float64: 1000.50

# Custom types
user User: {
    name: "Alice"
    age: 30
}
```

### Function Results

```suru
# Assign function results
result: add(1, 2)
message: greet("Alice")
user: createUser("alice", 30)

# Chain method calls
uppercased: text.trim().toUpper()
```

### With Operators

```suru
# Arithmetic expressions (when implemented)
sum: x + y
difference: x - y

# Boolean expressions
isValid: x > 0 and y < 100
shouldProcess: isActive or hasPermission
notReady: not isComplete

# Function calls
count: calculate(x, y)
result: process(data)
```

## Reassignment

### In Functions (Mutable)

Variables within functions can be reassigned:

```suru
process: () {
    value: 10        # Initial value
    value: value * 2   # Reassign to 20
    value: value + 5   # Reassign to 25
}
```

### At File Level (Immutable)

File-level declarations are constants and cannot be reassigned:

```suru
CONFIG: "production"  # Constant

main: () {
    # CONFIG: "development"  # ERROR: Cannot reassign constant
    localConfig: "development"  # OK: New local variable
}
```

## Pattern Examples

### Swap Values

```suru
swap: (a, b) {
    temp: a
    a: b
    b: temp
    return (a, b)
}
```

### Accumulator Pattern

```suru
sumList: (numbers List<Number>) Number {
    total: 0
    numbers.each((num) {
        total: total + num
    })
    return total
}
```

### Conditional Assignment

```suru
getValue: (condition Bool) Number {
    result: match condition {
        true: 100
        false: 0
    }
    return result
}
```

---

**See also:**
- [Syntax](syntax.md) - Lexical elements and literals
- [Types](types.md) - Type system and annotations
- [Functions](functions.md) - Function declarations and scope
