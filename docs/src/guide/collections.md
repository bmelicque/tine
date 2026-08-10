# Common collections

## Arrays

### Declaring arrays

Arrays are ordered collections of values of the same type. They are expressed with square brackets:

```tine
let ints = [1, 2, 3]
let names = ["Ada", "Grace", "Barbara"]
```

Arrays are homogeneous: every element must share the same type. For a fixed-length collection that mixes types, see tuples.

Arrays are dynamically sized: elements can be added or removed after creation.

### Reading elements

Tine has no `array[index]` syntax. Instead, reading an element goes through `get`, which returns an `Option<T>` rather than panicking on an out-of-bounds index:

```tine
let numbers = [1, 2, 3]

let first = numbers.get(0)  // Some(1)
let last = numbers.get(-1)  // Some(3)
let missing = numbers.get(9) // None
```

This means every read is explicit about the possibility of failure — there's no way to accidentally crash on a bad index.

### Writing elements

Similarly, `set` replaces an element at a given index and reports success as a `bool`, rather than panicking:

```tine
let mut numbers = [1, 2, 3]

let ok = numbers.set(1, 20)   // true — numbers is now [1, 20, 3]
let failed = numbers.set(9, 0) // false — index out of bounds, array unchanged
```

Note that `set` requires a mutable binding (`let mut`), consistent with how mutability works for any other value.

### Growing an array

`push` appends an element to the end:

```tine
let mut numbers = [1, 2, 3]
numbers.push(4) // numbers is now [1, 2, 3, 4]
```

### Length

`length` returns the number of elements:

```tine
let numbers = [1, 2, 3]
numbers.length() // 3
```

## Tuples

Tuples are fixed-length collections of values of different types. They are expressed with parentheses:

```tine
let t = (1, "Ada", true)
```

Tuples can be read and written using a period `.`:

```tine
let t = (1, "Ada", true)

let first = t.0  // 1
let second = t.1 // "Ada"
let third = t.2  // true
```

```tine
let mut t = (1, "Ada", true)

t.0 = 2   // t is now (2, "Ada", true)
t.1 = "Barbara"  // t is now (2, "Barbara", true)
t.2 = false    // t is now (2, "Barbara", false)
```

Since tuple have a fixed length, the compiler can safely infer the type of each element.

## Equality

_🚧 : this will maybe not stay, since it would imply all elements to also be comparable with `==`._

Since Tine uses value-based semantics, collections are equal if they contain the same values, even if they are represented by different objects in-memory.

```tine
let a = [1, 2, 3]
let b = [1, 2, 3]
let areEqual = a == b // true
```