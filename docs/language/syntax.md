# Syntax

> Lexical elements, literals, and basic syntax rules

## File Structure

A Suru source file has `.suru` extension and follows this structure:

1. **Module Declaration** (optional)
2. **Import Block** (optional)
3. **Export Block** (optional)
4. **Declarations** (types, functions, variables, expressions)

## Booleans

Suru has two boolean literals:

```suru
isTrue: true
isFalse: false
```

## Comments

Single-line comments start with `//`:

```suru
// This is a line comment

x: 42  // Comments can appear after code
```

## String Literals

Suru supports standard string literals with both single and double quotes:

```suru
doubleQuoted: "Hello, World!"
singleQuoted: 'Hello, World!'
```

For string interpolation, use backticks:

```suru
name: "Alice"
simple: `Hello {name}!`
```

See [Advanced Topics](advanced.md#string-interpolation) for multi-level interpolation with double/triple/quad backticks.

### Escape Characters

Suru supports standard escape sequences:

- `\b` - backspace (BS)
- `\e` - escape (ESC)
- `\n` - newline
- `\r` - carriage return
- `\t` - tab
- `\\` - backslash
- `\"` - double quote
- `\'` - single quote
- `` \` `` - backtick
- `\NNN` - octal 6-bit character (3 digits)
- `\xNN` - hexadecimal 8-bit character (2 digits)
- `\uNNNN` - hexadecimal 16-bit Unicode character UTF-8 encoded (4 digits)
- `\UNNNNNNNN` - hexadecimal 32-bit Unicode character UTF-8 encoded (8 digits)

Examples:

```suru
newline: "Hello\nWorld"
tab: "Name:\tAlice"
unicode: "Smile: \u263A"  // ☺
```

## Numbers

### Number Bases

Suru supports multiple number bases:

```suru
binary: 0b1010       # Binary (10 in decimal)
octal: 0o755         # Octal (493 in decimal)
hex: 0xFF            # Hexadecimal (255 in decimal)
decimal: 123         # Decimal
float: 3.14159       # Floating point
```

### Underscore Separators

Use underscores for readability in large numbers:

```suru
million: 1_000_000
hex: 0xDEAD_BEEF
binary: 0b1010_1100_1111_0000
```

Underscores work in all number bases.

### Type Suffixes

Specify exact numeric types with suffixes:

**Integers:**
- Signed: `i8`, `i16`, `i32`, `i64`, `i128`
- Unsigned: `u8`, `u16`, `u32`, `u64`, `u128`

**Floats:**
- `f16`, `f32`, `f64`, `f128`

Examples:

```suru
// Decimal with separators and suffix
count: 1_000_000u64

// Binary with suffix
flags: 0b1010_1100u8

// Hex with suffix
address: 0xDEAD_BEEFu16

// Float with suffix
pi: 3.14159f64
```

## Identifiers

Identifiers must start with a letter and can contain letters, numbers, dots, and underscores:

```suru
name: value
userId: 123
user.name: "Alice"
_private: 42
```

## Statement Termination

Statements are terminated by newlines. Multi-line statements are supported when the next line starts with a continuation character:

```suru
# Single line
x: 42

# Multi-line (continuation with pipe)
result: value
    | transform
    | process

# Multi-line (continuation with comma)
add: (x Number,
      y Number,
      z Number) {
    return x + y + z
}
```

## Keywords

Suru has 14 reserved keywords:

- `module` - Module declaration
- `import` - Import statement
- `export` - Export statement
- `return` - Return from function
- `match` - Pattern matching
- `type` - Type declaration
- `try` - Error handling short-circuit
- `and` - Logical AND
- `or` - Logical OR
- `not` - Logical NOT
- `true` - Boolean literal
- `false` - Boolean literal
- `this` - Current instance reference
- `partial` - Partial application

See [Reference: Keywords](../reference/keywords.md) for detailed descriptions.

## Operators and Punctuation

- `:` - Declaration and type annotation
- `;` - Statement separator (rarely used, newlines preferred)
- `,` - Separator in lists
- `.` - Member access and method calls
- `|` - Pipe operator
- `*` - (Reserved for future use)
- `+` - Composition operator
- `-` - Negation operator
- `( )` - Grouping and function parameters
- `{ }` - Blocks and struct bodies
- `[ ]` - Collections (lists, sets, maps)
- `< >` - Generic type parameters

See [Operators](operators.md) for detailed precedence and usage.

## Whitespace

Suru is whitespace-sensitive for readability:

```suru
# Newlines are significant
x: 42
y: 100

# Indentation is recommended but not enforced
type Person: {
    name String
    age Number
}
```

## Example: Complete Syntax

```suru
// Module declaration
module example

// Import block
import {
    {print}: io
}

// Type declaration
type User: {
    id Number
    name String
}

// Function declaration
createUser: (id Number, name String) User {
    return {
        id: id
        name: name
    }
}

// Variable declaration
alice: createUser(1, "Alice")

// Function call
print(`User: {alice.name}`)
```

---

**See also:**
- [Variables](variables.md) - Variable declarations and assignments
- [Operators](operators.md) - Operator precedence and usage
- [Advanced Topics](advanced.md) - String interpolation, documentation
