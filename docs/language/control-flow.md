# Control Flow

> Pattern matching and method-based iteration (no loop keywords)

## Match Expressions

Suru uses pattern matching for control flow instead of if-else chains:

### Match on Types

```suru
processResult: (result Result) String {
    return match result {
        Success: "Operation completed successfully"
        Error: "An error occurred"
        Pending: "Operation in progress"
        _: "Unknown status"
    }
}
```

### Match on Values

```suru
checkNumber: (n Number) String {
    return match n {
        0: "zero"
        1: "one"
        _: "other number"
    }
}
```

### Match with function calls

```suru
status: match user {
    equals(admin, _): "admin"
    equals(guest, _): "guest"
    _: "unknown user"
}
```

### Match as statement

```suru
match status {
    Success: print("success")
    Error: exit()
}
```

### Wildcard Pattern

The `_` pattern matches anything:

```suru
classify: (value Number) String {
    match value {
        0: "zero"
        1: "one"
        _: "many"  // Catch-all
    }
}
```

## Method-Based Iteration

**Important:** Suru has **no loop keywords** (no `while`, `for`, `loop`).

Instead, Suru uses method-based iteration, maintaining consistency with the method-centric approach. Control flow is managed through continuation types.

### Continuation Types

Control flow is managed with union types representing continuation decisions:

```suru
// Core continuation types
type Continue
type Break<T>: Some<T>, None
type Produce<T>
type Continuation<T>: Produce<T>, Continue, Break<T>
```

## Number Iteration

Numbers provide iteration methods:

### Basic Repetition

```suru
printHello: (step Number) {
    print(`Hello #{step.toString()}`)
}

5.times(printHello)  // Prints "Hello #1" through "Hello #5"
```

### Early Termination

```suru
countWithBreak: (step Number) Continuation {
    print(`Step: {step.toString()}`)
    return match step {
        .equals(3): Break  // Stop at step 3
        _: Continue
    }
}

10.times(countWithBreak)  // Only prints steps 1, 2, 3
```

### Early Termination with Value

```suru
find3: (step Number) Continuation<Number> {
    return match step {
        .equals(3): Break(step)  // Stop at step 3
        _: Continue
    }
}

result Option<Number>: 10.times(find3) // returns Some(3)
```

### Accumulator Pattern

```suru
appendStars: (_, result String) Continuation<String> {
    return Produce(result + "*")
}

result: 3.times(appendStars, "+") // returns "+***"
```

## Collection Iteration

Collections provide rich iteration methods:

### Basic Iteration

```suru
numbers List<Number>: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]

printNumbers: (num Number) {
    print(`Number: {num.toString()}`)
}

numbers.each(printNumbers)
```

### Iteration with Index

```suru
printNumbersWithIndex: (num Number, index Number) {
    print(`Index {index.toString()}: {num.toString()}`)
}

numbers.each(printNumbersWithIndex)
```

### Map, Filter, Reduce

```suru
// Map - transform each element
doubled: numbers.map((n) { return n.multiply(2) })

// Filter - keep only matching elements
evens: numbers.filter((n) { return n.mod(2) == 0 })

// Reduce - accumulate a result
sum: numbers.reduce(0, (acc, n) { return acc.add(n) })
```

## Infinite Loops

Conditional loops using the `loop` function with continuation types:

```suru
// While-like behavior
while: (current Number) Continuation<Number> {
    print(`Count: {current.toString()}`)
    current: current.subtract(1)

    return match current.equals(3) {
        true: Break    // Early exit
        false: Continue(current)
    }
}

loop(while, 100)
```

## Examples

### Finding an Element

```suru
findFirst: (items List<Number>, target Number) Option<Number> {
    items.each((item, index) {
        return match item.equals(target) {
            true: Break(Some(index))
            false: Continue
        }
    })
    return None
}
```

### Countdown

```suru
countdown: (from Number) {
    stepDown: (from, step) {
        remaining: from.substract(step.add(1))
        print(`{remaining.toString()}...`)
    }
    from.times(setepDown(from, _))
    print("Blast off!")
}

countdown(10)
```

### Processing Until Condition

```suru
processUntil: (items List<Data>, condition Predicate) List<Data> {

    results: items.each(process)

    return results

    process: (item) {
        return match condition(item) {
            true: Break
            false: Produce(item)
        }
    }
}
```

## Pattern Matching Patterns

### Match on Struct Fields

```suru
processUser: (user User) String {
    match user.status {
        Active: "User is active"
        Suspended: "User is suspended"
        Deleted: "User is deleted"
    }
}
```

### Nested Match

```suru
evaluate: (result Result<Data, Error>) String {
    match result {
        Ok: match data.type {
            TypeA: "Process type A"
            TypeB: "Process type B"
            _: "Unknown type"
        }
        Error: "An error occurred"
    }
}
```

## Control Flow Best Practices

1. **Use `.times()` for fixed iterations**: Clearer than manual loops
2. **Use `.each()` for collection traversal**: More declarative
3. **Leverage continuation types**: Explicit control flow
4. **Use `Break` with value for early returns**: Communicate result clearly
5. **Keep iteration functions pure when possible**: Easier to reason about

---

**See also:**
- [Functions](functions.md) - Function declarations
- [Error Handling](error-handling.md) - Using `try` with patterns
- [Collections](../reference/collections.md) - List, Set, Map methods
