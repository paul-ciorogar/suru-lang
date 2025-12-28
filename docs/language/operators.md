# Operators

> Operators and operator precedence in Suru

## Operator Types

### Unary Operators

**Negation (`-`)**

Negates a number:

```suru
x: -42
y: -2_283i64
negative: -value
```

**Logical NOT (`not`)**

Inverts a boolean:

```suru
isNotActive: not isActive
isFalse: not true
shouldSkip: not shouldProcess
```

### Logical Operators

**AND (`and`)**

Returns true if both operands are true:

```suru
isValid: hasPermission and isActive
result: true and true    # true
result: true and false   # false
```

**OR (`or`)**

Returns true if either operand is true:

```suru
canAccess: isAdmin or isOwner
result: true or false    # true
result: false or false   # false
```

### Compositional Operator

**Composition (`+`)**

Used for composing types and structs:

```suru
# Type composition
type Employee: Person + {
    salary Int64
    department String
}

# Data composition
aCircle Circle: aPoint + {
    radius: 500
}

# Method composition
type Shape: {
    area: + partial calculateArea(this)
}
```

See [Composition](composition.md) for detailed usage.

### Pipe Operator

**Pipe (`|`)**

Pipes values from left to right, enabling chained transformations:

```suru
# Basic pipe
result: value | transform

# Multiple pipes
processed: "Hello, world!"
    | trim()
    | toLower()
    | replace(_, "world", "you")
    | capitalize()

# Pipe with arithmetic
output: 2_283 | subtract(_, 2) | print  # 2281 would be printed
```

The `_` placeholder indicates partial application

**Detailed usage:** [Pipeline](#pipeline-usage)

## Operator Precedence

Operators are evaluated in the following order (highest to lowest precedence):

| Precedence | Operator | Description | Associativity |
|------------|----------|-------------|---------------|
| 4 | `.` | Member access / method call | Left-to-right |
| 3 | `not`, `-` | Logical NOT, negation | Right-to-left |
| 2 | `and` | Logical AND | Left-to-right |
| 1 | `or` | Logical OR | Left-to-right |

### Precedence Examples

```suru
# NOT has higher precedence than AND
result: not false and true      # (not false) and true = true

# AND has higher precedence than OR
result: true or false and false  # true or (false and false) = true

# Dot (member access) has highest precedence
value: obj.method() and check    # (obj.method()) and check
```

## Pipeline Usage

The pipe operator (`|`) enables functional-style composition:

### Basic Piping

```suru
# Pipe to function
result: value | process

# Equivalent to
result: process(value)
```

### Placeholder Usage

Use `_` indicates partial application:

```suru
# Pipe as second argument
result: 10 | subtract(20, _)  # subtract(20, 10) = 10

# Pipe as first argument (default)
result: 10 | subtract(_, 5)   # subtract(10, 5) = 5
```

### Chaining

Chain multiple transformations:

```suru
# Text processing
processed: "  Hello World  "
    | trim()
    | toLower()
    | replace(_, "world", "Suru")
    | capitalize()
# Result: "Hello suru"

# Number processing
final: 100
    | multiply(_, 2)
    | add(_, 50)
    | divide(_, 10)
# Result: 25
```

### With Methods

Pipe works seamlessly with method calls:

```suru
result: data
    | process()
    | validate()
    | user.transform(_)
    | save()
```

### Error Handling with Pipes

Combine pipes with `try` for clean error handling:

```suru
processRequest: (request String) Result<Response, Error> {
    request
        | try parseJson
        | try validateRequest
        | try processBusinessLogic
        | try formatResponse
        | try sendResponse
}
```

See [Error Handling](error-handling.md#pipe-integration) for details.

## Examples

### Boolean Logic

```suru
# Complex boolean expressions
isEligible: hasAccount and isVerified and not isBanned
```

### Type Composition

```suru
type Point: {
    x Number
    y Number
}

type Circle: Point + {
    radius Number
    area: () Number
}

type LabeledCircle: Circle + {
    label String
}
```

### Pipeline Patterns

```suru
# Data transformation pipeline
users
    | filterActive()
    | sortByName()
    | take(_, 10)
    | mapToDisplay()

# Validation pipeline
input
    | try validateFormat
    | try checkLength
    | try sanitize
    | try store
```

---

**See also:**
- [Syntax](syntax.md) - Basic syntax rules
- [Composition](composition.md) - Using the `+` operator
- [Error Handling](error-handling.md) - Using `try` with pipes
- [Reference: Operator Precedence](../reference/operator-precedence.md) - Quick reference table
