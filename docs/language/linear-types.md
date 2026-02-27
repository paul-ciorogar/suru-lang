# Linear Types

> `type-linear` — values that must be consumed exactly once

## Table of Contents

- [Motivation](#motivation)
- [Declaration Syntax](#declaration-syntax)
- [Consumer vs Observer Methods](#consumer-vs-observer-methods)
- [Compiler Enforcement Rules](#compiler-enforcement-rules)
- [Basic Usage Patterns](#basic-usage-patterns)
- [Typestate: Transitional Consumers](#typestate-transitional-consumers)
- [Control Flow Integration](#control-flow-integration)
- [Linear Types in Function Signatures](#linear-types-in-function-signatures)
- [Fallible Consumers](#fallible-consumers)
- [Structural Typing](#structural-typing)
- [Generics and Virality](#generics-and-virality)
- [Comparison with Regular Types](#comparison-with-regular-types)

---

## Motivation

Suru already has **affine move semantics**: a value can be used *at most* once — if you pass it to a function, the caller can no longer access it. This eliminates use-after-free bugs and shared mutable state.

Linear types add a strictly stronger guarantee: a value must be used **exactly once**. You cannot simply let it go out of scope. This solves a different class of bugs:

- **Resource leaks** — file handles, network sockets, database connections that are opened but never closed
- **Protocol violations** — a database transaction that is neither committed nor rolled back
- **State machine errors** — a handshake that never reaches its final state

Suru's existing type system gives you `type` for regular structs. Linear types use `type-linear`, which signals to the compiler: *a value of this type carries an obligation that must be discharged.*

---

## Declaration Syntax

```suru
type-linear TypeName: {
    // fields and methods
}
```

The `type-linear` keyword replaces `type`. The body is identical in form to a regular struct: fields, methods, private members.

**Constraint at definition:** a `type-linear` body must contain at least one consumer method (defined below). A linear type with no way to be consumed is a compile-time error.

```suru
// Valid: has a terminal consumer
type-linear FileHandle: {
    path: () String          // observer method
    read: (n Number) String  // observer method
    close: () void           // consumer — terminal
}

// Valid: has transitional consumers and a terminal consumer
type-linear DbConnection: {
    query: (sql String) DbConnection  // consumer — transitional (same type)
    commit: () void                   // consumer — terminal
    rollback: () void                 // consumer — terminal
}

// ERROR: no consumer methods — a BadType could never be discharged
type-linear BadType: {
    name: () String
    data: () Number
}
```

---

## Consumer vs Observer Methods

### Consumer Methods

A method is a **consumer** if its return type is:

| Return type | Kind | Effect |
|---|---|---|
| `void` | Terminal consumer | Ends the lifecycle, obligation fully satisfied |
| A linear type (same or different) | Transitional consumer | Moves the value into a new linear state |

When you call a consumer, the current binding is gone. If transitional, the returned linear value carries a new (or continuing) obligation.

```suru
type-linear FileHandle: {
    close: () void                 // terminal consumer
}

type-linear TcpConnecting: {
    connect: () TcpConnected       // transitional consumer — different linear type
    cancel: () void                // terminal consumer
}

type-linear TcpConnected: {
    send: (data String) TcpConnected  // transitional consumer — same type
    close: () void                    // terminal consumer
}
```

### Observer Methods

A method is an **observer** if its return type is a non-linear type (e.g. `String`, `Number`, `Bool`, any regular struct).

Calling an observer does **not** satisfy the linear obligation. The compiler treats observer calls using implicit linear borrowing — the value is temporarily lent to the method and is still live and obligated after the call returns.

```suru
type-linear FileHandle: {
    path: () String          // observer — FileHandle remains live
    size: () Number          // observer — FileHandle remains live
    read: (n Number) String  // observer — FileHandle remains live
    close: () void           // consumer — FileHandle is gone
}

main: () {
    handle: openFile("/etc/hosts")
    p: handle.path()          // observer — handle still live
    n: handle.size()          // observer — handle still live
    content: handle.read(512) // observer — handle still live
    handle.close()            // consumer — obligation satisfied
}
```

> **Implementation note:** Observer calls on linear types are the one place in Suru where implicit borrowing occurs. The compiler enforces that the linear value is not moved during an observer call and that it remains in scope afterward.

### Method Visibility

Private methods (`_` prefix) follow the same consumer/observer classification. A private consumer can satisfy the obligation just as a public one can — as long as it is reachable from the type's implementation.

---

## Compiler Enforcement Rules

The compiler tracks the **linear obligation state** of every binding whose type is `type-linear`. The obligation begins when a value is bound and ends when a consumer is called. The following rules are enforced:

### Rule 1 — No implicit drop

A linear binding that goes out of scope without having a consumer called is a compile-time error.

```suru
bad: () {
    handle: openFile("data.txt")
    // ERROR: `handle` of type FileHandle went out of scope without being consumed
}
```

### Rule 2 — Observers do not satisfy the obligation

Observer calls leave the obligation alive.

```suru
alsobad: () {
    handle: openFile("data.txt")
    content: handle.read(512)
    // ERROR: `handle` still obligated — read() is an observer, not a consumer
}
```

### Rule 3 — All code paths must satisfy the obligation

The compiler performs flow-sensitive checking. Every path through a match, every branch of a conditional, every early return — all must satisfy the linear obligation.

```suru
// Both match arms must consume `conn`
process: (conn DbConnection) {
    match someCondition() {
        true: {
            conn: conn.query("INSERT ...")
            conn.commit()    // satisfied in true branch
        }
        false: {
            conn.rollback()  // satisfied in false branch
        }
    }
}
```

### Rule 4 — Transitional consumers transfer the obligation

The result of a transitional consumer is itself a linear value. The new binding carries the obligation.

```suru
// Both `conn` bindings carry the obligation; the last one must be consumed
tx: (conn DbConnection) {
    conn: conn.query("DELETE FROM tmp")  // old conn consumed, new conn bound
    conn: conn.query("INSERT INTO log")  // old conn consumed, new conn bound
    conn.commit()                        // final obligation satisfied
}
```

### Rule 5 — Passing to a function transfers the obligation

Passing a linear value to a function moves the obligation into that function. The caller's binding is gone. The function parameter must be consumed (Rule 1 applies inside the callee).

```suru
processFile: (handle FileHandle) String {
    content: handle.read(512)
    handle.close()              // callee satisfies obligation
    return content
}

main: () {
    handle: openFile("data.txt")
    result: processFile(handle)  // handle transferred — caller no longer has it
    // handle is gone here
}
```

### Rule 6 — Returning a linear value transfers the obligation to the caller

A function that returns a `type-linear` value creates an obligation for its caller.

```suru
openConn: () DbConnection {
    return db.connect()  // caller receives the obligation
}

main: () {
    conn: openConn()     // conn is now obligated
    conn.rollback()      // satisfy it
}
```

---

## Basic Usage Patterns

### Resource with guaranteed cleanup

```suru
type-linear FileHandle: {
    read: (n Number) String
    write: (data String) void  // terminal consumer
    close: () void
}

readAll: (path String) String {
    handle: openFile(path)
    content: handle.read(4096)
    handle.close()
    return content
}
```

---

## Typestate: Transitional Consumers

Linear types shine when modeling protocols with multiple states. Each state is its own `type-linear` declaration. Transitional consumers move the value from one state to another.

### Example: TCP lifecycle

```suru
type-linear TcpConnecting: {
    timeout: (ms Number) TcpConnecting  // stay in this state
    connect: () TcpConnected            // move to connected state
    cancel: () void                     // terminal — abandon connection
}

type-linear TcpConnected: {
    send: (data String) TcpConnected    // stay connected
    flush: () TcpConnected              // stay connected
    close: () void                      // terminal — done
}

performRequest: (host String) String {
    conn: tcpConnect(host, 443)
    conn: conn.timeout(5000)              // same state
    live: conn.connect()                  // transition to TcpConnected
    live: live.send("GET / HTTP/1.1\r\n")
    live: live.flush()
    live.close()
    return "done"
}
```

### Example: Database transaction lifecycle

```suru
type-linear Transaction: {
    execute: (sql String) Transaction    // transitional — returns same type
    savepoint: (name String) Transaction // transitional — returns same type
    commit: () void                      // terminal
    rollback: () void                    // terminal
}

transfer: (from String, to String, amount Number) {
    tx: db.begin()
    tx: tx.execute(`UPDATE accounts SET balance = balance - {amount} WHERE id = {from}`)
    tx: tx.execute(`UPDATE accounts SET balance = balance + {amount} WHERE id = {to}`)
    tx.commit()
}
```

---

## Control Flow Integration

### Match expressions

Every arm of a match that involves a linear value must consume it. The compiler checks all arms.

```suru
type Status: Ok, Fail

handleConn: (conn DbConnection, status Status) {
    match status {
        Ok:   { conn.commit()   }
        Fail: { conn.rollback() }
    }
    // Both arms consume conn — OK
}
```

A wildcard `_` arm must also consume the value:

```suru
handleConn: (conn DbConnection, status Status) {
    match status {
        Ok: { conn.commit() }
        _:  { conn.rollback() }  // still must consume
    }
}
```

### Early return

If you return early, the linear value must be consumed before the return:

```suru
process: (conn DbConnection) String {
    ok: validate()
    match ok {
        false: {
            conn.rollback()    // consume before early return
            return "invalid"
        }
        true: {}
    }
    conn: conn.execute("INSERT ...")
    conn.commit()
    return "ok"
}
```

---

## Linear Types in Function Signatures

### Parameters

A function parameter of a linear type **must** consume the value before returning. This is checked the same way as any local binding.

```suru
// Function declares it takes ownership of (and responsibility for) a FileHandle
writeAll: (handle FileHandle, data String) {
    handle.write(data)
    handle.close() // terminal consumer
}
```

### Return types

A function can return a linear type, transferring the obligation to the caller.

```suru
openTransaction: () Transaction {
    return db.begin()
}

// Caller must consume it
run: () {
    tx: openTransaction()
    tx.rollback()
}
```

### Both: consuming and producing

A function can take a linear value, do some work, and return a (potentially different) linear value:

```suru
withRetry: (conn DbConnection, sql String) DbConnection {
    return conn.execute(sql)  // returns same linear type, transferring obligation
}

run: () {
    conn: db.begin()
    conn: withRetry(conn, "INSERT ...")
    conn.commit()
}
```

---

## Fallible Consumers

A consumer method can fail. Two signature patterns are available, each with different tradeoffs around error handling and resource cleanup.

### Pattern A — `Result<T, Error>` (type handles cleanup on failure)

The method internally cleans up the linear resource if it fails. `T` appears only in the success branch; the error branch carries no linear value. The method is responsible for discharge on the failure path.

```suru
execute: (sql String) Result<Transaction, DbError>

tx: try tx.execute("INSERT ...")
```

Use Pattern A when the type itself owns the cleanup-on-failure logic.

### Pattern B — `Result<T, ErrorResult<T, Error>>` (caller handles cleanup)

The error branch bundles the linear value back alongside the error, giving the caller the resource for cleanup. `try` is allowed because the error tuple is itself linear (it carries `T`), so short-circuiting propagates the obligation to the caller.

```suru
type ErrorResult<T, V>: {
    data T
    error V
}

execute: (sql String) Result<Transaction, ErrorResult<Transaction, DbError>>

// try is allowed — propagates ErrorResult<Transaction, DbError> to caller
// caller must eventually consume the Transaction
tx: try tx.execute("INSERT ...")

// or match immediately:
result: tx.execute("INSERT ...")
match result {
    Ok:          { result.commit()        }
    ErrorResult: { result.data.rollback() }
}
```

Use Pattern B when the caller must decide how to finalize the resource on failure.

---

## Structural Typing

**A `type-linear` value cannot satisfy a non-linear constraint** — the compiler infers the linearity requirement from the type and will flag any attempt to pass a linear type where a regular type is expected, because the consumer obligation would be silently dropped.

**For generics,** linearity works through inference. When a linear type is passed to a generic parameter, the compiler infers the linearity constraint and checks that all linear obligations are satisfied within the scope of that instantiation. You do not need to annotate generic parameters as linear; the constraint propagates automatically.

```suru
// A generic function that operates on any T
process<T>: (value T) {
    // If T is FileHandle, the compiler infers the linear constraint
    // and enforces that `value` is consumed before this function returns
}
```

Two `type-linear` declarations with the same structural shape are interchangeable — the linearity requirement is part of what makes their shapes identical for structural purposes.

---

## Generics and Virality

If a generic type `Container<T>` is instantiated with a linear type `T`, the resulting `Container<LinearType>` is **automatically linear**. The compiler propagates the must-consume obligation. This is called *virality*.

All methods that expose `T` (e.g. `get: () T`) become transitional consumers of the container when `T` is linear — because retrieving `T` moves the linear value out, which consumes the container.

```suru
type Box<T>: {
    value Option<T>
    get: () T
}

handle: openFile("data.txt")
box: Box { value: Some(handle) }    // Box<FileHandle> is now implicitly linear
retrieved: box.get()                // get() is now a transitional consumer — box is consumed

match retrieved {
    Some: retrieved.close()         // must still consume the FileHandle
    _:
}
```

You do not need to declare `type-linear Box<T>` — the compiler infers linearity from the instantiation. A `Box<Number>` remains a regular, non-linear type.

**Consequence for error types:** `Result<T, Error>` becomes linear when `T` is linear, and `Result<T, ErrorResult<T, Error>>` is linear because its error branch carries a `T`.

---

## Comparison with Regular Types

| Feature | `type` (regular) | `type-linear` |
|---|---|---|
| Can be dropped (out of scope) | Yes | **No — compile error** |
| Can be passed to functions | Yes (move) | Yes (moves the obligation) |
| Can be returned | Yes | Yes (transfers obligation to caller) |
| Methods can observe without consuming | Yes (always) | Yes, via implicit borrow (observer methods) |
| Methods can transition state | Only if you rebind | Transitional consumers (return linear type) |
| Works in match arms | Yes | Yes, but all arms must consume |
| Works with pipe `\|` | Yes | Yes, each stage must be a transitional or terminal consumer; observers banned |
| Structural typing | Full | Linearity is part of the structural signature; cannot downcast to non-linear |
| Generic support | Full | Viral: instantiation with a linear type argument becomes implicitly linear |

---

**See also:**
- [Types](types.md) - All type forms including regular structs
- [Memory Model](memory.md) - Ownership and move semantics
- [Error Handling](error-handling.md) - `try` keyword and Result types
