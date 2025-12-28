# Error Handling

> Error as values with the `try` keyword

## Error as Values

The Suru language uses errors as values - you **cannot throw an error**.

You can use any of the built-in types or make your own:

```suru
type Result<T, E>: Ok T, Error E
type Option<T>: Some T, None
type Response<T, E>: Success T, Failure E
type Either<L, R>: Left L, Right R
```

## The `try` Keyword

Use the `try` keyword in front of a call to short-circuit if there is an error and return early.

**`try` works with any union type with exactly two variants.**

### Basic Usage

```suru
processData: (input String) Result<Data, Error> {
    // try unwraps the "success" variant (first one) or short-circuits with the "failure" variant (second one)
    parsed: try parseInput(input)     // parseInput returns Result<ParsedData, ParseError>

    // try unwraps Some or returns None (auto-converted to Err None)
    value: try findValue(parsed)      // findValue returns Option<Value>

    // try unwraps Success or returns Failure
    result: try sendRequest(value)    // sendRequest returns Response<Data, NetworkError>

    return Ok(result)
}
```

### Try Compatibility Rules

1. **Try Compatibility**: A type is try-compatible if it's a union with exactly 2 variants
2. **Success Unwrapping**: `try expr` where `expr: Union<A, B>` produces type `A`
3. **Failure Propagation**: The containing function must return a union where the second variant is compatible with `B`

## Built-In Error Types

### Result Type

```suru
type Result<T, E>: Ok T, Error E

parseNumber: (input String) Result<Number, String> {
    // Implementation
}

process: (input String) Result<Number, String> {
    num: try parseNumber(input)
    return Ok(num * 2)
}
```

### Option Type

```suru
type Option<T>: Some T, None

findUser: (id String) Option<User>
getProfile: (user User) Option<Profile>

getUserProfile: (id String) Option<Profile> {
    user: try findUser(id)        // Unwraps Some or returns None
    profile: try getProfile(user) // Chains naturally
    return Some(profile)
}
```

### Either Type

```suru
type Either<L, R>: Left L, Right R

parseAndValidate: (input String) Either<Data, Error> {
    parsed: try parseJson(input)    // parseJson returns Either<JsonValue, ParseError>
    data: try validateData(parsed)  // validateData returns Either<Data, ValidationError>
    return Left(data)
}
```

### Custom Domain Types

```suru
type AuthResult<T>: Authenticated T, Unauthorized String
type DatabaseResult<T>: Found T, NotFound String

secureGetUser: (token String, id String) AuthResult<User> {
    session: try authenticate(token)  // Returns AuthResult<Session>
    user: try getUser(session, id)    // Returns DatabaseResult<User>
    return Authenticated(user)
}
```

## Pipe Integration

The `try` operator works beautifully with pipes:

### Clean Pipeline with Automatic Unwrapping

```suru
processRequest: (request String) Result<Response, Error> {
    request
        | try parseJson
        | try validateRequest
        | try processBusinessLogic
        | try formatResponse
        | try sendResponse
}
```

### Error Transformation in Pipes

```suru
getData: (id String) Result<Data, Error> {
    id
        | try fetchFromDatabase
        | try transform
        | try validate
        | try enrichWithMetadata
}
```

## Error Handling Patterns

### Early Return on Error

```suru
complexOperation: (input Input) Result<Output, Error> {
    // Each step can fail and return early
    step1: try validateInput(input)
    step2: try processData(step1)
    step3: try transformResult(step2)
    step4: try finalizeOutput(step3)

    return Ok(step4)
}
```

### Error Conversion

```suru
// Convert between error types
apiCall: (data Data) Result<Response, ApiError> {
    validated: try validate(data)  // Returns Result<Data, ValidationError>
        .mapErr((e) { return ApiError.ValidationFailed(e) })

    response: try sendRequest(validated)

    return Ok(response)
}
```

### Fallback Values

```suru
getValueOrDefault: (id String) Value {
    result: findValue(id)  // Returns Option<Value>

    return match result {
        Some: result.value
        None: defaultValue
    }
}
```

### Error Accumulation

```suru
validateAll: (items List<Item>) Result<List<Item>, List<Error>> {
    errors: []
    validated: []

    items.each((item) {
        result: validateItem(item)
        match result {
            Ok: validated.add(result.value)
            Error: errors.add(result.error)
        }
    })

    return match errors.isEmpty() {
        true: Ok(validated)
        false: Error(errors)
    }
}
```

## Best Practices

1. **Use `Result` for operations that can fail**: Makes errors explicit
2. **Use `Option` for optional values**: Clearer than null/nil
3. **Leverage `try` for clean code**: Avoids error-handling boilerplate
4. **Chain operations with pipes**: Readable error propagation
5. **Document error conditions**: Help API users
6. **Use specific error types**: Better than generic errors
7. **Handle errors at appropriate levels**: Don't propagate too far

## Examples

### File Operations

```suru
readConfig: (path String) Result<Config, FileError> {
    contents: try readFile(path)
    parsed: try parseJson(contents)
    config: try validateConfig(parsed)
    return Ok(config)
}
```

### Network Requests

```suru
fetchUserData: (userId String) Result<UserData, NetworkError> {
    url: buildUrl(userId)
    response: try httpGet(url)
    data: try parseResponse(response)
    validated: try validateUserData(data)
    return Ok(validated)
}
```

### Database Operations

```suru
saveUser: (user User) Result<UserId, DatabaseError> {
    validated: try validateUser(user)
    id: try insertIntoDatabase(validated)
    try updateCache(id, validated)
    return Ok(id)
}
```

### Complex Pipeline

```suru
processOrder: (order Order) Result<Receipt, OrderError> {
    order
        | try validateOrder
        | try checkInventory
        | try calculateTotal
        | try processPayment
        | try updateInventory
        | try generateReceipt
}
```

---

**See also:**
- [Types](types.md) - Union types
- [Control Flow](control-flow.md) - Pattern matching
- [Functions](functions.md) - Return types
- [Operators](operators.md) - Pipe operator
