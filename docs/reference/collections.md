# Collections Reference

> Quick reference for List, Set, and Map

## Overview

Suru provides three built-in collection types, all using the unified `[]` syntax for creation. The type annotation determines which collection is created.

## Lists

Ordered collections that allow duplicates.

### Creation

```suru
// List creation using [] syntax
numbers List<Number>: [1, 2, 3, 4, 5]
names List<String>: ["alice", "bob", "charlie"]
emptyList List<Float64>: []
```

### Common Operations

```suru
// Adding elements
extended: numbers
    .add(6)
    .add([7, 8, 9])

// Accessing elements
first: numbers.get(0)
last: numbers.last()

// Modifying
updated: numbers.set(0, 100)  // Insert at index

// Iteration
numbers.each((num) { print(num) })
numbers.each((num, index) { print(`{index}: {num}`) })

// Transformation
doubled: numbers.map((n) { return n * 2 })
evens: numbers.filter((n) { return n % 2 == 0 })

// Reduction
sum: numbers.reduce(0, (acc, n) { return acc + n })
```

## Sets

Unordered collections with unique elements.

### Creation

```suru
// Set creation - duplicates automatically removed
uniqueNumbers Set<Number>: [1, 2, 3, 2, 1]  // Results in {1, 2, 3}
colors Set<String>: ["red", "green", "blue"]
emptySet Set<Float64>: []
```

### Common Operations

```suru
// Adding elements (duplicates ignored)
extended: colors.add("yellow")

// Checking membership
hasRed: colors.contains("red")

// Set operations
union: set1.union(set2)
intersection: set1.intersect(set2)
difference: set1.difference(set2)

// Iteration
colors.each((color) { print(color) })
```

## Maps

Key-value collections.

### Creation

```suru
// Map creation using key:value syntax
userAges Map<String, Number>: [
    "alice": 25,
    "bob": 30,
    "charlie": 35
]

scores Map<String, Float64>: [
    "math": 95.5,
    "science": 87.2,
    "history": 92.1
]

emptyMap Map<String, Int64>: []
```

### Common Operations

```suru
// Adding/updating entries
updated: userAges.put("david", 28)

// Accessing values
aliceAge: userAges.get("alice")  // Returns Option<Number>

// Checking keys
hasAlice: userAges.containsKey("alice")

// Removing entries
removed: userAges.remove("alice")

// Iteration
userAges.each((key, value) {
    print(`{key}: {value}`)
})

// Get keys/values
keys: userAges.keys()      // List<String>
values: userAges.values()  // List<Number>
```

## Collection Type Inference

The type annotation determines which collection is created:

```suru
// Same syntax, different types based on annotation
numbersList List<Number>: [1, 2, 3]        // Creates List
numbersSet Set<Number>: [1, 2, 3]          // Creates Set

// Maps require key:value syntax
mapping Map<Int64, String>: [1: "one", 2: "two"]  // Creates Map
```

## Method Reference

### List<T> Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `add` | `(item T) List<T>` | Add element to end |
| `get` | `(index Int64) Option<T>` | Get element at index |
| `set` | `(index Int64, value T) List<T>` | Set element at index |
| `remove` | `(index Int64) List<T>` | Remove element at index |
| `first` | `() Option<T>` | Get first element |
| `last` | `() Option<T>` | Get last element |
| `length` | `() Int64` | Get number of elements |
| `isEmpty` | `() Bool` | Check if empty |
| `contains` | `(item T) Bool` | Check if contains element |
| `map` | `<R>(fn Transform<T, R>) List<R>` | Transform each element |
| `filter` | `(fn Predicate<T>) List<T>` | Keep matching elements |
| `reduce` | `<R>(initial R, fn Reducer<T, R>) R` | Reduce to single value |
| `each` | `(fn Consumer<T>)` | Iterate over elements |
| `reverse` | `() List<T>` | Reverse the list |
| `sort` | `() List<T>` | Sort elements (requires Orderable) |

### Set<T> Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `add` | `(item T) Set<T>` | Add element (ignores duplicates) |
| `remove` | `(item T) Set<T>` | Remove element |
| `contains` | `(item T) Bool` | Check if contains element |
| `size` | `() Int64` | Get number of elements |
| `isEmpty` | `() Bool` | Check if empty |
| `union` | `(other Set<T>) Set<T>` | Union of two sets |
| `intersect` | `(other Set<T>) Set<T>` | Intersection of two sets |
| `difference` | `(other Set<T>) Set<T>` | Difference of two sets |
| `isSubsetOf` | `(other Set<T>) Bool` | Check if subset |
| `each` | `(fn Consumer<T>)` | Iterate over elements |

### Map<K, V> Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `put` | `(key K, value V) Map<K, V>` | Insert or update entry |
| `get` | `(key K) Option<V>` | Get value for key |
| `remove` | `(key K) Map<K, V>` | Remove entry |
| `containsKey` | `(key K) Bool` | Check if key exists |
| `size` | `() Int64` | Get number of entries |
| `isEmpty` | `() Bool` | Check if empty |
| `keys` | `() List<K>` | Get all keys |
| `values` | `() List<V>` | Get all values |
| `entries` | `() List<Pair<K, V>>` | Get all key-value pairs |
| `each` | `(fn BiConsumer<K, V>)` | Iterate over entries |

---

**See also:**
- [Control Flow](../language/control-flow.md) - Using `.each()` for iteration
- [Functions](../language/functions.md) - Generic functions with collections
- [Types](../language/types.md) - Generic types
