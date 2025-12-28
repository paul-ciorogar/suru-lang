# Keywords Reference

> Complete list of Suru's 14 reserved keywords

## Keywords

Suru has 14 reserved keywords that cannot be used as identifiers.

### module

Declares a module.

```suru
module Calculator
```

**See:** [Modules](../language/modules.md)

### import

Imports items from other modules.

```suru
import {
    math
    {sin, cos}: trigonometry
}
```

**See:** [Modules](../language/modules.md#imports)

### export

Exports items from the current module.

```suru
export {
    Calculator
    add
    subtract
}
```

**See:** [Modules](../language/modules.md#exports)

### type

Declares a type.

```suru
type Person: {
    name String
    age Number
}
```

**See:** [Types](../language/types.md)

### return

Returns a value from a function.

```suru
getValue: () Number {
    return 42
}
```

**See:** [Functions](../language/functions.md)

### match

Pattern matching for control flow.

```suru
result: match value {
    Success: "OK"
    Error: "Failed"
    _: "Unknown"
}
```

**See:** [Control Flow](../language/control-flow.md#match-expressions)

### try

Short-circuit error handling.

```suru
processData: (input String) Result<Data, Error> {
    parsed: try parseInput(input)
    validated: try validateData(parsed)
    return Ok(validated)
}
```

**See:** [Error Handling](../language/error-handling.md)

### and

Logical AND operator.

```suru
result: true and false  // false
isValid: hasPermission and isActive
```

**See:** [Operators](../language/operators.md#logical-operators)

### or

Logical OR operator.

```suru
result: true or false  // true
canAccess: isAdmin or isOwner
```

**See:** [Operators](../language/operators.md#logical-operators)

### not

Logical NOT operator.

```suru
result: not true  // false
shouldSkip: not isReady
```

**See:** [Operators](../language/operators.md#unary-operators)

### true

Boolean literal for true.

```suru
flag: true
```

**See:** [Syntax](../language/syntax.md#booleans)

### false

Boolean literal for false.

```suru
flag: false
```

**See:** [Syntax](../language/syntax.md#booleans)

### this

Reference to the current instance within methods.

```suru
type Counter: {
    value Number
    increment: () {
        this.value: this.value + 1
    }
}
```

**See:** [Types](../language/types.md#the-this-keyword)

### partial

Explicit partial application.

```suru
// When avoiding many underscores
complexCurry: partial functionWithManyArguments(2_283i32)
```

**See:** [Advanced Topics](../language/advanced.md#currying-and-partial-application)

## Reserved for Future Use

The following are not currently keywords but may be reserved in the future:

- `if`, `else` (use `match` instead)
- `while`, `for`, `loop` (use method-based iteration)
- `class`, `interface`, `extends`, `implements` (use types and composition)
- `throw`, `catch`, `finally` (errors are values, use `try`)
- `async`, `await` (planned for future async support)
- `yield` (planned for generators)
- `const`, `let`, `var` (use `:` for declarations)

---

**See also:**
- [Syntax](../language/syntax.md) - Complete syntax guide
- [Operators](../language/operators.md) - Operator reference
- [Operator Precedence](operator-precedence.md) - Precedence table
