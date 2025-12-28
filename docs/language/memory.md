# Memory Model

> No garbage collection with simple ownership and move semantics

## Overview

Suru manages memory **without garbage collection**, using a straightforward ownership model that eliminates entire classes of memory-related bugs while keeping the memory model simple and predictable.

## Ownership and Move Semantics

### Core Rules

1. **Functions take ownership** of all values passed to them
2. **All values are passed by move** by default
3. **Automatic copy before move** when a value would be mutated after being passed
4. **Memory can be shared** as long as no mutations occur

### Example: Basic Move

```suru
process: (data Data) {
    // `data` is moved into this function
    // The caller no longer has access to `data`
    modified: transform(data)
}

main: () {
    myData: createData()
    process(myData)
    // myData is no longer accessible here - it was moved
}
```

### Example: Automatic Copy

```suru
useMultipleTimes: () {
    value: 42

    // First usage: value is moved
    result1: process(value)

    // Second usage: value is automatically copied before move
    result2: process(value)
}
```

**How it works:**
- When a value would be mutated after being passed to a function, Suru automatically creates a copy before the move
- This ensures no shared mutable state

## No Shared Mutable State

### Core Principle

**Mutable state is never shared between scopes.**

When a mutation would cause sharing, Suru duplicates the memory instead. All copies are deep copies with no exceptions.

### Example

```suru
modify: (data Data) Data {
    data.field: newValue  // Mutation
    return data
}

main: () {
    original: createData()

    // `original` is copied before being moved to `modify`
    // because it will be used again
    modified: modify(original)

    // Both `original` and `modified` exist independently
    print(original.field)   // Original value
    print(modified.field)   // New value
}
```

## Function Scope

### Ownership Within Functions

1. **Each function owns all its memory values**
2. **All values within a function scope are mutable**
3. **Once a function receives a value, it has complete ownership**

### Example

```suru
processData: (data Data) {
    // Function owns `data`
    // Can modify it freely
    data.field1: value1
    data.field2: value2
    data.field3: value3

    // All modifications are local to this function
    return data
}
```

## Memory Safety Guarantees

### No Dangling Pointers

Values cannot be accessed after they've been moved:

```suru
useValue: () {
    data: createData()
    process(data)          // `data` is moved
    // print(data)         // ERROR: `data` was moved
}
```

### No Use-After-Free

The compiler ensures values are not used after being freed:

```suru
invalid: () {
    data: createData()
    return data.field      // OK: accessing before return
}
// data is freed after function returns
```

### No Data Races

No shared mutable state means no data races:

```suru
parallel: () {
    data: createData()

    // Each function gets its own copy if needed
    result1: asyncProcess(data)
    result2: asyncProcess(data)

    // No race condition - separate copies
}
```

## Memory Patterns

### Return Ownership

```suru
createAndReturn: () Data {
    data: createData()
    return data  // Ownership transferred to caller
}

caller: () {
    myData: createAndReturn()  // Receives ownership
}
```

### Chain Ownership

```suru
transform1: (data Data) Data { /* ... */ }
transform2: (data Data) Data { /* ... */ }
transform3: (data Data) Data { /* ... */ }

process: (data Data) Data {
    data
        | transform1
        | transform2
        | transform3
}
```

### Explicit Copy (Future Feature)

Planned syntax for explicit copying:

```suru
// Planned
makeCopy: (data Data) {
    copied: data.copy()
    // Now have two independent values
}
```

## Comparison with Other Languages

### vs Rust

- **Simpler**: No lifetime annotations or borrow checker
- **Automatic copying**: Compiler inserts copies when needed
- **No unsafe**: All operations are safe

### vs Go

- **No GC**: Deterministic memory management
- **Move semantics**: Explicit ownership transfer
- **No shared mutable state**: Prevents data races

### vs C++

- **No manual management**: No `new`/`delete`
- **Memory safe**: No dangling pointers or leaks
- **Simpler model**: No smart pointers needed

## Best Practices

1. **Trust the compiler**: It inserts copies when needed
2. **Design for move semantics**: Functions consume their arguments
3. **Return new values**: Don't try to mutate parameters
4. **Use functional style**: Transform data, don't mutate in place
5. **Leverage immutability**: File-scope constants are immutable
6. **Understand ownership**: Values are owned by one scope at a time

## Examples

### Data Transformation

```suru
processUser: (user User) User {
    // Function owns user, can modify freely
    user.name: user.name.trim().toUpper()
    user.email: user.email.toLower()
    user.verified: true
    return user  // Transfer ownership to caller
}
```

### Pipeline with Ownership Transfer

```suru
prepareData: (raw RawData) ProcessedData {
    raw
        | validate
        | normalize
        | enrich
        | transform
}
```

### Builder Pattern

```suru
type Builder: {
    data Data
    withName: (name String) Builder
    withAge: (age Number) Builder
    build: () Result
}

Builder: () Builder {
    return {
        data: emptyData()

        withName: (name String) Builder {
            this.data.name: name
            return this  // Return self for chaining
        }

        withAge: (age Number) Builder {
            this.data.age: age
            return this
        }

        build: () Result {
            return this.data
        }
    }
}

// Usage
result: Builder()
    .withName("Alice")
    .withAge(30)
    .build()
```

---

**See also:**
- [Variables](variables.md) - Variable scope and mutability
- [Functions](functions.md) - Function parameters
- [Types](types.md) - Type ownership
