# Functions

## Declaration & call

New functions are declared using the `fn` keyword.

Functions can have _parameters_, which are part of the function signature. Each parameter's type must be explicitely annotated.

The same goes for the return type, if the function returns any value.

Note that function bodies, just like [blocks](./control-flow.md#blocks), evaluate to their last statement (if it is an expression).

```tine
fn add(x: int, y: int) -> int {
    x + y
}
```

Of course, you can also return early, using `return` statements.

```tine
fn addNatural(x: int, y: int) -> int {
    if x < 0 || y < 0 {
        return -1
    }
    x + y
}
```

Functions are closures, meaning they capture their environment. This means that they can access the values of their surrounding scope.

```tine
let mut count = 0

fn increment() {
    count = count + 1
}
```

Functions can then be called using parentheses.

```tine
let sum = add(1, 2)
```

## Function expressions and callbacks

Functions are first class citizens in Tine, so you can assign them to variables and pass them around. In that case, you don't need to name the function.

```tine
let adder = fn (x: int, y: int) -> int {
    x + y
}
```

Since they are first class citizens, functions can take other functions as params, or return other functions:

```tine
fn mapInt(array: int[], transformer: fn(int) -> int) -> int[] {
    // ...
}
```

When functions are used as arguments, the compiler can usually infer the full type from the caller's definition.
Thus, type annotations can be omitted and, in this case, braces can be omitted for the returned expression.

```tine
// With regular function syntax
let doubled = integers.map(fn (value: int) -> int { value * 2 })

// Can be simplified to:
let doubled = integers.map(fn (value) value * 2)
```