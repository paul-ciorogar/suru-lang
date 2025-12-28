# Functions

> Function declarations, parameters, generics, and overloading

## Function Declarations

### Basic Functions

Functions are declared with parameters and return type:

```suru
// Function returning a simple type
add: (x Number, y Number) Number {
    return x.add(y)
}

// Function with inferred types
add: (x, y) {
    return x.add(y)
}
```

### Higher-Order Functions

Functions can return and accept other functions:

```suru
type UnaryFunction: (x Number) Number

// Function returning a function type
createAdder: (base Number) UnaryFunction {

    identity: (x Number) Number {
        return x.add(base)
    }

    return identity
}

// Function taking a function
applyTwice: (fn UnaryFunction, value Number) Number {
    temp: fn(value)
    return fn(temp)
}
```

## Generic Functions

Functions that work with multiple types:

### Simple Generic Function

```suru
identity<T>: (value T) T {
    return value
}

// Usage
num: identity<Number>(42)
str: identity<String>("hello")
```

### Multiple Type Parameters

```suru
map<T, R>: (items List<T>, transform Transform<T, R>) List<R> {
    result: List<R>()
    // Implementation iterates and transforms
    return result
}
```

### Generic Functions with Constraints

```suru
sort<T: Orderable>: (items List<T>) List<T> {
    // Implementation uses T's ordering methods
    return items.quickSort()
}
```

## Function Overloading

Suru supports function overloading with multiple signatures:

### Overloading by Parameter Types

```suru
// Function overloading (same name, different signatures)
add: (a Int64, b Int64) Int64 { return a + b }
add: (a Float64, b Float64) Float64 { return a + b }
add: (a Int64) Int64 { return a }
add: (a String, b String) String { return a + b }
```

### Method Overloading

Methods can also be overloaded:

```suru
type Calculator: {
    add: (a Int64, b Int64) Int64
    add: (a Float64, b Float64) Float64
    add: (a Int64) Int64
    add: (a String, b String) String
}

calc Calculator: {
    add: (a Int64, b Int64) Int64 { return a + b }
    add: (a Float64, b Float64) Float64 { return a + b }
    add: (a Int64) Int64 { return a }
    add: (a String, b String) String { return a + b }
}
```

### Overloading by Return Type

Functions can be overloaded by return type alone:

```suru
// Same function name and parameters, different return types
parse: (input String) Int64 {
    return input.toInt64()
}

parse: (input String) Float64 {
    return input.toFloat64()
}

parse: (input String) Bool {
    return input.equals("true")
}

// Usage - type annotation determines which overload
intValue Int64: parse("123")      // Calls parse: (String) Int64
floatValue Float64: parse("3.14") // Calls parse: (String) Float64
boolValue Bool: parse("true")     // Calls parse: (String) Bool
```

## Lexical Scoping

Functions and methods have strict lexical scoping - they can only access:

1. Their parameters
2. Variables declared within their body
3. Global constants and functions

They **cannot** access variables from outer scopes. This ensures predictable behavior and makes currying safe.

### Scoping Rules

```suru
constant: 42

outerFunction: (x Number) Number {
    localVar: 10

    innerFunction: (y Number) Number {
        // Can access: y (parameter), constant
        // Can access other functions: outerFunction
        return y.add(constant)
    }

    // Cannot access: localVar from outer scope
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

See [Advanced Topics](advanced.md#currying-and-partial-application) for detailed currying information.

## Examples

### Recursive Functions

```suru
factorial: (n Number) Number {
    return match n {
        .equals(0): 1
        .equals(1): 1
        _: n.multiply(factorial(n - 1))
    }
}
```

### Function Composition

```suru
compose<A, B, C>: (f Transform<B, C>, g Transform<A, B>) Transform<A, C> {

    impl: (x A) C {
        return f(g(x))
    }

    return impl
}

// Usage
addOne: (x Number) Number { return x.add(1) }
double: (x Number) Number { return x.add(2) }

addOneThenDouble: compose(double, addOne)
result: addOneThenDouble(5)  // (5 + 1) * 2 = 12
```

## Parameters

### Typed Parameters

Explicitly specify parameter types:

```suru
greet: (name String, age Number) String {
    return `Hello {name}, you are {age} years old`
}
```

### Inferred Parameters

Let the compiler infer parameter types:

```suru
add: (x, y) {
    return x + y
}
```

### Mixed Parameters

Mix typed and inferred parameters:

```suru
process: (id String, data) {
    // id is String, data is inferred
    return transform(id, data)
}
```

## Return Types

### Explicit Return Type

```suru
calculate: (x Number, y Number) Number {
    return x + y
}
```

### Inferred Return Type

```suru
calculate: (x Number, y Number) {
    return x + y  // Type inferred from return value
}
```

### Multiple Return Types (Union)

```suru
divide: (x Number, y Number) Result<Number, String> {
    return match y.equals(0) {
        true: Error("Division by zero")
        false: Ok(x / y)
    }
}
```

## Best Practices

1. **Use type annotations for public APIs**: Makes intent clear
2. **Prefer type inference for internal functions**: Less verbose
3. **Use descriptive parameter names**: Improves readability

---

**See also:**
- [Types](types.md) - Function types
- [Advanced Topics](advanced.md) - Currying and partial application
- [Control Flow](control-flow.md) - Pattern matching and iteration
- [Error Handling](error-handling.md) - Result types and try
