# Types

> Complete guide to Suru's type system with generic type constraint inference

## Table of Contents

- [Type Forms](#type-forms)
- [Generic Type Constraint Inference](#generic-type-constraint-inference)
- [Type Implementation](#type-implementation)
- [Privacy and Encapsulation](#privacy-and-encapsulation)

## Type Forms

Suru supports seven distinct type forms:

### 1. Unit Types

Simple types with no definition, perfect for flags and states:

```suru
type Success
type Error
type Loading
```

Unit types are useful for:
- State machines
- Result indicators
- Event types

### 2. Type Aliases

Simple renames for existing types:

```suru
type UserId: Number
type Username: String
type Age: Int64
type Price: Float64
```

Type aliases improve code readability without creating new types.

### 3. Union Types

Alternative types (sum types):

```suru
type Status: Success, Error, Loading
type Value: Int64, String, Bool
type Result: Ok, Error
```

Union types represent values that can be one of several alternatives.

### 4. Struct Types

Records with fields and method declarations:

```suru
type Person: {
    name String
    age Number

    greet: () String
    birthday: () Person
}
```

Structs combine data (fields) and behavior (methods).

### 5. Intersection Types

Combine types using `+`:

```suru
type Employee: Person + {
    salary Int64
    department String
}
```

Intersection types compose existing types with additional fields/methods.

### 6. Function Types

Function signatures must be defined as named types:

```suru
type AddFunction: (a Number, b Number) Number
type Predicate: (value String) Bool
type VoidFunction: () void
type Identity<T>: (value T) T
type UnaryOperator: (x Float64) Float64
```

**Note:** `void` must be used to indicate a function returns nothing.

### 7. Generic Types

Define types that work with multiple specific types:

```suru
// Single type parameter
type List<T>: {
    items Array<T>
    size Int64

    add: (item T) List<T>
    get: (index Int64) T
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

### Basic Compatibility

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
checkAge: (p) Bool {
    return p.age.greaterThan(18)
}

emp Employee: {
    name: "Alice"
    age: 25
}

// This works because Employee has same structure as Person
isAdult: checkAge(emp)  // Valid - inferred constraint compatibility
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
    area: () { return 3.14159.multiply(this.radius.squared()) }
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

### Generic Type Compatibility

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

## Type Implementation

### Simple Struct Implementation

Create instances by providing all fields and methods:

```suru
type User: {
    username String
    authenticate: (password String) Bool
}

// Direct implementation
user User: {
    username: "Paul"
    authenticate: (password) {
        return true
    }
}
```

### Constructors

Any struct type can define a constructor function with the same name as the type:

```suru
type User: {
    username String
    authenticate: (password String) Bool
}

// Constructor function
User: (name String) User {
    return {
        username: name
        authenticate: (password) {
            return true
        }
    }
}

// Use constructor
user: User("Paul")
```

### Custom Instances

Each instance can have unique method implementations:

```suru
// Create instances without using constructor
user User: {
    username: "Paul"
    authenticate: (password) {
        return true
    }
}

// Or define a factory function
newUser: (name String) User {
    return {
        username: name
        authenticate: (password) {
            return true
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
    username String                      // Public field
    authenticate: (password String) Bool  // Public method
}

user User: {
    username: "Paul"        // Public field
    _ passwordHash: "2283"  // Private field
    _ salt: "qwerty"        // Private field

    authenticate: (password String) Bool { // Public method
        // Can access private members
        hash: this.hashPassword(password)
        return hash.equals(this.passwordHash)
    }

    _ hashPassword: (password String) String {  // Private method
        // implementation
    }
}
```

**Rules:**
- Private members can only be accessed within the same instance
- External code cannot access private fields or methods
- Compiler enforces privacy at compile time

### The `this` Keyword

Within method implementations, `this` refers to the current instance:

```suru
// Public interface - what consumers see
type BankAccount: {
    accountId String
    deposit: (amount Float64) Float64
    withdraw: (amount Float64) Float64
    getBalance: () Float64
}

// Constructor
BankAccount: (initial Float64, id String) BankAccount {
    impl BankAccount: {
        accountId: id
        transactionCount: 0
        _ balance: initial

        deposit: (amount Float64) Float64 {
            return match this.validate(amount) {  // Call private method
                true: {
                    this.balance: this.balance.add(amount)
                    this.logTransaction("deposit", amount)
                    return this.balance
                }
                false: this.balance
            }
        }

        _ validate: (amount Float64) Bool {  // Private method implementation
            return amount.greaterThan(0.0)
        }

        _ logTransaction: (type String, amount Float64) {
            // Private logging logic
        }

        // ... other methods
    }

    return impl
}

// Usage
account: BankAccount(100.0, "ACC123")
// account.balance        // Compile error: not in public interface
// account.validate(50.0) // Compile error: private method not accessible
balance: account.getBalance()  // OK: public method
```

**`this` Usage:**
- Access instance fields: `this.fieldName`
- Call instance methods: `this.methodName(args)`
- Access private members: `this.privateField`, `this.privateMethod()`
- Modify instance state: `this.field: newValue`

## Examples

### Complete Type Example

```suru
// Define type
type Person: {
    name String
    age Number
    greet: () String
}

// Constructor
Person: (name String, age Number) Person {
    return {
        name: name
        age: age
        greet: () String {
            return `Hello, I'm {this.name}, age {this.age}`
        }
    }
}

// Create instance
alice: Person("Alice", 30)

// Use instance
greeting: alice.greet()
print(greeting)  // "Hello, I'm Alice, age 30"
```

### Intersection Type Example

```suru
type Point: {
    x Number
    y Number
}

type Circle: Point + {
    radius Number
    area: () Number
}

// Implement
circle Circle: {
    x: 0
    y: 0
    radius: 5
    area: () Number {
        return 3.14159 * this.radius * this.radius
    }
}
```

### Generic Type Example

```suru
type Stack<T>: {
    items List<T>
    push: (item T) Stack<T>
    pop: () Option<T>
    peek: () Option<T>
}

Stack<T>: () Stack<T> {
    return {
        items: []
        push: (item T) Stack<T> {
            this.items.add(item)
            return this
        }
        pop: () Option<T> {
            return this.items.removeLast()
        }
        peek: () Option<T> {
            return this.items.last()
        }
    }
}

// Use
numberStack: Stack<Number>()
numberStack.push(1).push(2).push(3)
top: numberStack.peek()  // Some(3)
```

---

**See also:**
- [Functions](functions.md) - Function types and declarations
- [Composition](composition.md) - Type composition with `+`
- [Variables](variables.md) - Variable type annotations
