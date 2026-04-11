# Memory Model

> No garbage collection with simple ownership and move semantics

## Overview

Suru manages memory **without garbage collection**, using a straightforward ownership model that eliminates entire classes of memory-related bugs while keeping the memory model simple and predictable.

### No Shared Mutable State

**Mutable state is never shared between scopes.**

When a mutation would cause sharing, Suru automatically copies the memory instead. All copies are deep copies with no exceptions.

## Ownership and Move Semantics

### Core Rules

1. **Functions take ownership** of all values passed to them
2. **All values are passed by move** in default cases
3. **Automatic copy before move** when a value would be mutated after being passed
4. **Memory can be shared** as long as no mutations occur
5. **Automatic cleanup** via `drop` when values go out of scope

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
    value: createData()

    // First usage: value is automatically copied because it's used again
    result1: process(value)
    // Compiler replaces with result1: process(copy(value))

    // Second usage: value is moved (last usage)
    result2: process(value)
}
```

**How it works:**
- When a value would be mutated after being passed to a function, Suru automatically copies it before the move
- This ensures no shared mutable state
- All copies are deep copies

## Memory Cleanup with `drop`

When a value goes out of scope and is no longer needed, the compiler automatically inserts a call to `drop` to free the allocated memory. You never need to call `drop` manually.

### Basic Cleanup

```suru
main: () {
    text: "this is a value on the heap"
    // Compiler adds: drop(text)
}
```

The `text` value is allocated on the heap. When it goes out of scope at the end of the function, the compiler ensures it is cleaned up.

### Passing Ownership Transfers Cleanup Responsibility

When you pass a value to a function, ownership transfers and so does the responsibility for cleanup:

```suru
makeSomething: (val String) void {
    // val needs to be cleaned here
    // Compiler adds: drop(val)
}

type Circle: { x Number, y Number, radius Number }

makeCircle: () Circle {
    // Create a Circle on the heap and return it
    // Ownership is returned also - no drop here
    return {
        x: 1
        y: 1
        radius: 15
    }
}

main: () {
    greeting: "hello!"

    // Pass ownership - no drop needed for greeting here
    makeSomething(greeting)

    circle: makeCircle()

    // Compiler adds: drop(circle)
    // No drop(greeting) - ownership was transferred
}
```

## Copying with `copy`

When a value needs to be used after being passed to a mutating function, the compiler automatically inserts a `copy` call to create a deep copy.

### Copying for Mutating Functions

```suru
// Mutating function
changeAndPrint: (circle Circle) void {
    circle.updateRadius(5)
    // pring code here ...
    // Compiler adds: drop(circle)
}

main: () {
    circle: { radius: 3 }

    // Compiler copies circle because it's used later
    changeAndPrint(copy(circle))

    circle.updateRadius(6)

    // No copy - last usage, passes ownership
    changeAndPrint(message)
}
```

### Copying When Adding to Structs

When a value becomes part of a struct while still being needed elsewhere:

```suru
type Person: { name String, age Number }

theName: "Paul"

person Person: {
    name: theName
    // theName will be used agains so we need a copy
    // Compiler will rewrite to: name: copy(theName)
}

secondPerson Person: {
    name: theName  // Ownership passed to the struct
}
```

### Copying Values from Data Structures

Extracting a value from a struct may require copying:

```suru
type Person: {
    name String
    age Number
}

extractName: (person Person) String {
    return person.name  // Copy needed - name leaving struct scope
    // Compiler will rewrite to: return copy(person.name)
}

printName: (person Person) String {
    name: person.name
    io.writeLine(name)  // No copy - no mutation occurs
}
```

## Reference Passing for Performance

For performance, the compiler passes references when possible, avoiding unnecessary copies or moves.

### Non-Mutating Functions Receive References

```suru
// Non-mutating function
printMessage: (val String) void {
    io.writeLine(val)
}

main: () {
    message: "Hello"

    // Pass a reference - message is still used later
    printMessage(message)

    // Pass ownership - last usage of message
    printMessage(message)
}
```

The compiler generates two versions of `printMessage`:
- One that receives a reference (for when the caller still needs the value)
- One that receives ownership (for the final usage)



## Linear Types: Guaranteed Consumption

Suru's affine move semantics ensure a value is used *at most* once. **Linear types** (`type-linear`) strengthen this to *exactly* once: certain values cannot be dropped implicitly — every code path must call a consumer method.

```suru
type-linear FileHandle: {
    read: (n Number) String  // observer — handle remains live
    close: () void           // consumer — obligation satisfied
}

safeRead: (path String) String {
    handle FileHandle: openFile(path)
    content: handle.read(4096)
    handle.close()           // required — compiler error if omitted
    return content
}
```

The compiler tracks the obligation state flow-sensitively. Every match arm, every early return path must satisfy the obligation before it exits scope. This prevents resource leaks at compile time without requiring runtime cleanup or garbage collection. Linear types can only be passed by move they will never be copied. Calling `copy` on a linear type will produce a compilation error.

**See also:** [Linear Types](linear-types.md) for the full specification including typestate patterns, generics, and fallible consumers.

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

### Explicit Copy

You can explicitly copy a value when needed:

```suru
makeCopy: (data Data) {
    copied: copy(data)
    // Now have two independent values
}
```

Note: The compiler automatically inserts `copy` calls where needed, so explicit copy is rarely necessary.

## Comparison with Other Languages

## Best Practices

1. **Trust the compiler**: It inserts `clone` and `drop` calls when needed
2. **Design for move semantics**: Functions consume their arguments
4. **Use functional style**: Transform data, don't mutate in place
6. **Understand ownership**: Values are owned by one scope at a time
7. **Last usage optimization**: Structure code so values are used once when possible to avoid copying

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
    return raw
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
- [Linear Types](linear-types.md) - Must-consume obligations for resource safety
