# Operator Precedence Reference

> Quick reference for operator precedence and associativity

## Precedence Table

Operators are evaluated in the following order (highest to lowest precedence):

| Precedence | Operator | Name | Associativity | Example |
|------------|----------|------|---------------|---------|
| 4 | `.` | Member access / method call | Left-to-right | `obj.method()`, `obj.field` |
| 3 | `not`, `-` | Logical NOT, negation | Right-to-left | `not flag`, `-value` |
| 2 | `and` | Logical AND | Left-to-right | `a and b` |
| 1 | `or` | Logical OR | Left-to-right | `a or b` |

## Associativity

### Left-to-Right

Operators with left-to-right associativity are evaluated from left to right:

```suru
a or b or c  // Evaluates as: (a or b) or c
a and b and c  // Evaluates as: (a and b) and c
obj.method1().method2()  // Evaluates as: (obj.method1()).method2()
```

### Right-to-Left

Operators with right-to-left associativity are evaluated from right to left:

```suru
not not value  // Evaluates as: not (not value)
-(-value)      // Evaluates as: -((-value))
```

## Precedence Examples

### Dot Operator (Highest Precedence)

```suru
result: obj.getValue() and check
// Evaluates as: (obj.getValue()) and check

value: user.profile.name
// Evaluates as: (user.profile).name
```

### NOT Operator

```suru
result: not x and y
// Evaluates as: (not x) and y

value: not a or b
// Evaluates as: (not a) or b
```

### AND Operator

```suru
result: a or b and c
// Evaluates as: a or (b and c)

value: true or false and false
// Evaluates as: true or (false and false)
// Result: true
```

### OR Operator (Lowest Precedence)

```suru
result: a and b or c and d
// Evaluates as: (a and b) or (c and d)
```

## Using Parentheses for Clarity

While precedence rules determine evaluation order, use parentheses for clarity:

```suru
// Without parentheses (relies on precedence)
result: not x and y or z

// With parentheses (explicit and clear)
result: ((not x) and y) or z
```

## Complex Examples

### Boolean Logic

```suru
// Without parentheses
isValid: hasAccount and isVerified and not isBanned or isAdmin
// Evaluates as: ((hasAccount and isVerified) and (not isBanned)) or isAdmin

// With parentheses for clarity
isValid: (hasAccount and isVerified and (not isBanned)) or isAdmin
```

### Method Chaining with Logic

```suru
// Method calls have highest precedence
shouldProcess: data.isValid() and data.isReady() or fallback.check()
// Evaluates as: ((data.isValid()) and (data.isReady())) or (fallback.check())
```

### Negation with Method Calls

```suru
isNotEmpty: not list.isEmpty()
// Evaluates as: not (list.isEmpty())

result: not user.isAuthenticated() and user.isGuest()
// Evaluates as: (not (user.isAuthenticated())) and (user.isGuest())
```

## Special Cases

### Pipe Operator

The pipe operator (`|`) is not in the precedence table because it works at the expression level:

```suru
result: value | transform | validate
// Pipes are evaluated left-to-right, outside normal precedence
```

### Composition Operator

The composition operator (`+`) is used for type and data composition, not expressions:

```suru
type Employee: Person + {
    salary Int64
}
```

## Best Practices

1. **Use parentheses for clarity**: Even when not required
2. **Don't rely on complex precedence**: Make intent explicit
3. **Break complex expressions**: Into multiple lines/variables
4. **Prefer named variables**: Over deeply nested expressions
5. **Use method chaining carefully**: Can reduce readability

## Quick Reference

**Remember:**
- `.` binds tightest (method calls first)
- `not` before `and`
- `and` before `or`
- When in doubt, use parentheses

---

**See also:**
- [Operators](../language/operators.md) - Complete operator guide
- [Syntax](../language/syntax.md) - Language syntax
- [Keywords](keywords.md) - Reserved keywords
