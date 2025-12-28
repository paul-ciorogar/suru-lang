# Composition

> Type, data, and method composition with the `+` operator

## Overview

Code reuse in Suru is done through **composition**, not inheritance. The `+` operator is used for all forms of composition.

**Key principle:** Composition over inheritance

## Type Composition

Suru composes types from other types, similar to interface inheritance:

```suru
type Point: {
    x Number
    y Number
}

type Circle: Point + {
    radius Number
}
```

The `Circle` type includes all fields from `Point` plus its own fields.

### Multiple Type Composition

```suru
type Named: {
    name String
}

type Positioned: {
    x Number
    y Number
}

type Sized: {
    width Number
    height Number
}

type NamedRectangle: Named + Positioned + Sized
// Has: name, x, y, width, height
```

## Data Composition

Building on type declarations, you can compose data instances:

```suru
aPoint Point: {
   x: 100
   y: 300
}

aCircle Circle: aPoint + {
    radius: 500
}
```

**Result:**
```suru
// aCircle has:
// x: 100
// y: 300
// radius: 500
```

### Name Conflicts

When composing structs, if there are name conflicts, **the last value overrides previous ones**:

```suru
base: {
    x: 1
    y: 2
}

override: base + {
    x: 100  // Overrides base.x
}

// override has:
// x: 100 (overridden)
// y: 2   (from base)
```

## Method Composition

Compose methods into structs using partial application:

### Basic Method Composition

```suru
type Shape: Circle, Square

type AreaFunction: (shape Shape) Number

// Implementation of an area function
area AreaFunction: (shape){
    // some implementation
}

// Function reuse with partial application
aCircle Circle: aPoint + {
    radius: 500

    // "Adding" a method by partially applying a function
    area: + partial area(this)
}

// Usage
theArea: aCircle.area()
```

### Composing from Function Libraries

```suru
// Function library
validateEmail: (email String) Bool { /* ... */ }
formatPhone: (phone String) String { /* ... */ }
calculateTax: (amount Number, rate Number) Number { /* ... */ }

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
```

### Composing Methods from Other Types

```suru
type EnhancedCircle: Circle + {
    // Reuse area function but add logging
    areaWithLog: + partial area(this) | + partial logResult(_)

    // Compose validation from another type
    validate: + Point.validateCoordinates
}
```

### Chaining Multiple Behaviors

```suru
// Base functionality
logCall<T>: (funcName String, result T) T {
    print(`Called {funcName}, result: {result}`)
    return result
}

validatePositive: (value Number) Number { /* ... */ }

// Compose a rich Circle type
aCircle Circle: aPoint + {
    radius: 500

    // Chain multiple behaviors
    area: + partial area(this)
          | + partial validatePositive(_)
          | + partial logCall("area", _)

    // Override inherited behavior
    move: + partial moveWithBounds(this, _, _)  // Last one wins

    // Compose from multiple sources
    describe: + partial formatShape(this)
              | + partial addTimestamp(_)
              | + partial toUppercase(_)
}

// Usage
result: aCircle.area()
// Calls: area(aCircle) -> validatePositive(result) -> logCall("area", result)
```

## Composition Patterns

### Mixin Pattern

```suru
type Timestamped: {
    createdAt DateTime
    updatedAt DateTime
}

type Versioned: {
    version Number
}

type Document: {
    title String
    content String
}

type VersionedDocument: Document + Versioned + Timestamped
```

### Trait-like Composition

```suru
// Define reusable traits as types
type Serializable: {
    toJson: () String
    fromJson: (json String) Self
}

type Comparable<T>: {
    equals: (other T) Bool
    lessThan: (other T) Bool
}

// Compose traits into types
type User: Serializable + Comparable<User> + {
    id UserId
    name String
}
```

## Best Practices

1. **Be aware of name conflicts**: Last value wins
2. **Use partial application for methods**: Enables powerful composition
3. **Combine with pipes**: Create transformation chains

## Examples

### Building a Rich Domain Model

```suru
type Entity: {
    id Id
    createdAt DateTime
}

type Auditable: {
    createdBy UserId
    updatedBy UserId
    updatedAt DateTime
}

type Deletable: {
    deletedAt Option<DateTime>
    deletedBy Option<UserId>
}

type Product: Entity + Auditable + Deletable + {
    name String
    price Money
    description String
}
```

### Composing Behaviors

```suru
// Behavior functions
logAccess: (entity Entity) {
    print(`Accessed: {entity.id}`)
}

validateNotDeleted: (entity Deletable) Result<Bool, String> {
    return match entity.deletedAt {
        Some: Error("Entity is deleted")
        None: Ok(true)
    }
}

// Compose into a service
type ProductService: {
    find: (id Id) Result<Product, String>
    save: (product Product) Result<Product, String>
}

createProductService: () ProductService {
    return {
        find: (id) {
            product: try findInDatabase(id)
            try validateNotDeleted(product)
            logAccess(product)
            return Ok(product)
        }
        save: (product) {
            try validateNotDeleted(product)
            saved: try saveToDatabase(product)
            logAccess(saved)
            return Ok(saved)
        }
    }
}
```

### Data Transformation Pipeline

```suru
// Base transformers
lowercase: (s String) String { return s.toLower() }
trim: (s String) String { return s.trim() }
removeSpecial: (s String) String { /* ... */ }

// Compose transformers
normalizeText: + partial lowercase
               | + partial trim
               | + partial removeSpecial

// Use composed function
input: "  Hello WORLD!  "
output: normalizeText(input)  // "hello world"
```

---

**See also:**
- [Types](types.md) - Intersection types
- [Functions](functions.md) - Partial application
- [Advanced Topics](advanced.md) - Currying
- [Operators](operators.md) - The `+` operator
