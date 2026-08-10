# Variables & basic types

✅ Stable

_🚧 : behavior when mixing int and float operands is not yet finalized._

## Declaring a variable

In Tine, variables are declared with `let`. Bindings are immutable by default, but can be made mutable with `mut`.

```tine
let name = "Ada"
// name = "Jane" // error because `name` is immutable

let mut count = 0
count = count + 1 // ok
```

There is no explicit type annotation syntax (at least for now). Tine always infers the type from the assigned value.

## Primitive types

Tine has four primitive types:

| Type    | Description        | Example  |
|---------|--------------------|----------|
| `bool`  | Boolean            | `true`   |
| `int`   | Integer            | `42`     |
| `float` | Floating-point     | `3.14`   |
| `str`   | String             | `"hi"`   |

```tine
let isReady = true
let age = 30
let price = 19.99
let greeting = "Hello, Tine!"
```

## Value semantics by default

By default, assigning one variable to another copies the value. There's no implicit references.

This applies to almost all data types, including more complex variable types like tuple, structs and arrays.

## Basic operations

### Numbers (`int`, `float`)

Standard arithmetic operators are available:

```tine
let sum = 2 + 3             // 5
let diff = 5 - 2            // 3
let product = 4 * 2         // 8
let quotient = 9 / 2        // 4 (int division)
let floatQuotient = 9. / 2. // 4.5 (float division)
let remainder = 9 % 2       // 1
```

### Comparisons

Comparison operators work on `int` and `float`, and return a `bool`:

```tine
let isEqual = 2 == 2      // true
let isDifferent = 2 != 3  // true
let isLess = 2 < 3        // true
let isGreater = 3 > 2     // true
let isLessEq = 2 <= 2     // true
let isGreaterEq = 3 >= 2  // true
```

There is no implicit type coercion in Tine, so the `===` and `!==` do not exist.

### Booleans (`bool`)

```tine
let a = true
let b = false

let and = a && b  // false
let or = a || b   // true
let not = !a      // false
```

### Strings (`str`)

Strings are concatenated with `+`:

```tine
let first = "Hello"
let last = ", Tine!"
let fullGreeting = first + last // "Hello, Tine!"
```