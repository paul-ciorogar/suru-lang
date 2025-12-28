# Advanced Topics

> Currying, partial application, string interpolation, and documentation

## Currying and Partial Application

All functions and methods in Suru can be curried. Calling a function with `_` placeholder instead of an argument returns a new function that takes the remaining arguments.

### Function Currying

#### Basic Currying with Placeholders

```suru
// Currying with placeholders
addTwo: add(2, _)           // Partial application
addToFive: add(_, 5)        // Different partial application
increment: add(_, 1)        // Another partial application
```

#### Multiple Placeholders

```suru
// Multiple placeholders
combine: someFunction(_, "default", _)

// First placeholder filled, second remains
partialCombine: combine("value1", _)
```

#### Explicit `partial` Keyword

Use the `partial` keyword when a function has many arguments to avoid writing many underscores:

```suru
// Explicit partial when avoiding: f(_, _, _, _, _, _, _, _, _)
complexCurry: partial functionWithManyArguments(2_283i32)
```

#### Currying with Pipes

```suru
addTwo: add(2, _)

// Works with pipe operations
result: 10 | addTwo    // Same as addTwo(10)
```

### Method Currying

Methods can also be curried:

```suru
type BinaryOperation: (a Number, b Number) Number
type UnaryOperation: (x Number) Number

type Calculator: {
    multiply: (a Number, b Number, c Number) Number
}

calc Calculator: {
    multiply: (a Number, b Number, c Number) Number {
        return a.multiply(b).multiply(c)
    }
}

// Curry the method
double: calc.multiply(2, _, _)        // Type: BinaryOperation
doubleTriple: calc.multiply(2, 3, _)  // Type: UnaryOperation

result: doubleTriple(4)        // 2 * 3 * 4 = 24
```

### Currying Patterns

#### Function Composition

```suru
compose: (f, g) {
    return (x) { return f(g(x)) }
}

addOne: add(_, 1)
double: multiply(_, 2)

addOneThenDouble: compose(double, addOne)
result: addOneThenDouble(5)  // (5 + 1) * 2 = 12
```

#### Partial Configuration

```suru
configureLogger: (level LogLevel, format Format, output Output) Logger {
    // Implementation
}

// Create specialized loggers
debugLogger: configureLogger(Debug, _, StdOut)
productionLogger: configureLogger(Info, Json, File)
```

#### Callback Factories

```suru
createCallback: (prefix String, suffix String) CallbackFunction {
    return (value String) {
        return `{prefix}{value}{suffix}`
    }
}

wrapInParens: createCallback("(", ")")
wrapInBrackets: createCallback("[", "]")

result1: wrapInParens("text")     // "(text)"
result2: wrapInBrackets("text")   // "[text]"
```

## String Interpolation

Suru features advanced string interpolation with multiple nesting levels using backticks.

### Single Backticks (\`)

For simple interpolation:

```suru
name: "Alice"
greeting: `Hello {name}!`
// Result: "Hello Alice!"
```

#### Multi-line Strings

Follow the backticks with a newline for multi-line strings:

```suru
name: "Alice"
greeting: (name) String {
    return `
    Hello {name}!
        How are you?
    `
}

greeting(name) | print  // Result: "Hello Alice!\n\tHow are you?"
```

### Double Backticks (\`\`)

For nesting or more complex interpolation:

```suru
user: getUser()
message: ``
    Welcome {{user.name}}!
    Your account balance is ${{user.balance}}.
    ``
```

### Triple Backticks (\`\`\`)

For even deeper nesting:

````suru
items: getItems()
report: ```
    Processing {{{items.length}}} items:
    {{{formatItemList(items)}}}
    Status: {{{getProcessingStatus()}}}
    ```
````

### Quad Backticks (\`\`\`\`)

For maximum nesting depth:

`````suru
template: getTemplate()
rendered: ````
    Template: {{{{template.name}}}}
    Content: {{{{renderContent(template.data)}}}}
    Metadata: {{{{template.metadata.toString()}}}}
    ````
`````

### Interpolation Rules

- **Single `{}`:** One backtick (`` ` ``)
- **Double `{{}}`:** Two backticks (``` `` ```)
- **Triple `{{{}}}`:** Three backticks (```` ``` ````)
- **Quad `{{{{}}}}`:** Four backticks (````` ```` `````)

Different backtick levels allow for flexible string templating and metaprogramming.

## Documentation

Suru supports rich documentation using equals sign delimiters with markdown content and special keywords.

### Documentation Blocks

Documentation blocks:
- Start and end with at least 4 equals signs (`====`)
- Contain valid markdown between the delimiters
- Support special `@keyword` annotations for structured metadata
- Can be placed before any top-level declaration

### Function Documentation

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
calculateCircleArea: (radius Float64) Float64 {
    return 3.14159 * radius * radius
}
````

### Type Documentation

````suru
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

### Documentation Keywords

**Standard Keywords:**
- `@param name description` - Parameter documentation
- `@return description` - Return value documentation
- `@example code` - Usage examples
- `@deprecated reason` - Mark as deprecated
- `@experimental note` - Mark as experimental
- `@todo description` - TODO items
- `@see reference` - Cross-references
- `@link url` - External links

**Type-Specific Keywords:**
- `@field name description` - Document struct fields
- `@author name` - Document author
- `@since version` - Since which version

### Best Practices

1. **Document public APIs**: All exported functions/types should have docs
2. **Include examples**: Help users understand usage
3. **Document parameters**: Explain constraints and expectations
4. **Mark deprecations**: Warn users about outdated APIs
5. **Cross-reference related items**: Use `@see` for discoverability
6. **Use markdown formatting**: Make docs readable
7. **Keep docs up to date**: Update when code changes

## Examples

### Currying with Method Composition

```suru
// Library of transformers
uppercase: (s String) String { return s.toUpper() }
lowercase: (s String) String { return s.toLower() }
trim: (s String) String { return s.trim() }

type TextProcessor: {
    value String
    transform: (fn Transform) TextProcessor
    get: () String
}

TextProcessor: (initial String) TextProcessor {
    return {
        value: initial

        transform: (fn Transform) TextProcessor {
            this.value: fn(this.value)
            return this
        }

        get: () String {
            return this.value
        }
    }
}

// Usage with curried functions
result: TextProcessor("  Hello World  ")
    .transform(trim)
    .transform(lowercase)
    .get()  // "hello world"
```

### Complex String Interpolation

```suru
generateReport: (data ReportData) String {
    header: `Report: {data.title}`

    body: ``
        Summary: {{data.summary}}
        Details:
        {{formatDetails(data.details)}}
        ``

    footer: `Generated on {data.date}`

    return ```
        {{{header}}}
        {{{body}}}
        {{{footer}}}
        ```
}
```

### Comprehensive Documentation

````suru
======================
# User Registration Service

Handles user registration with validation and persistence.

This service validates user input, creates new user accounts,
and stores them in the database.

@param name User's full name (2-100 characters)
@param email Valid email address
@param password Strong password (min 8 characters)
@return Result with User or Error
@example
```suru
result: registerUser("Alice Smith", "alice@example.com", "SecurePass123")
match result {
    Ok: print("User registered successfully")
    Error: print("Registration failed: {result.error}")
}
```
@throws ValidationError If input is invalid
@throws DatabaseError If storage fails
@since 2.0.0
@author User Management Team
@see UserValidator for validation rules
@link https://docs.example.com/user-registration
======================
registerUser: (name String, email String, password String) Result<User, Error> {
    validated: try validateInput(name, email, password)
    user: try createUser(validated)
    saved: try saveToDatabase(user)
    return Ok(saved)
}
````

---

**See also:**
- [Functions](functions.md) - Function declarations and scoping
- [Syntax](syntax.md) - String literals
- [Composition](composition.md) - Method composition with partial application
