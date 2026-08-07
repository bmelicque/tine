# Functions

🚧 Unstable

_Still pondering over `:` vs `->` to express return type_

## Declaration & call

New functions are declared using the `fn` keyword.

Functions can have _parameters_, which are part of the function signature. Each parameter's type must be explicitely annotated.

The same goes for the return type, if the function returns any value.

Note that function bodies, just like [blocks](./control-flow.md#blocks), evaluate to their last statement (if it is an expression).

```tine
fn add(x: int, y: int): int {
    x + y
}
```

Of course, you can also return early, using `return` statements.

```tine
fn addNatural(x: int, y: int): int {
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
let adder = fn (x: int, y: int): int {
    x + y
}
```

Since they are first class citizens, functions can take other functions as params, or return other functions:

```tine
fn mapInt(array: int[], transformer: fn(int): int): int[] {
    // ...
}
```

Since the `fn` notation can become cumbersome for callbacks, Tine also provides a concise syntax for this exact case:
- use a fat arrow `=>` instead of the `fn` keyword
- no need to provide types for parameters and return value, since these are inferred from the callback expected type
- the body can be any kind of expression not necessarily a block

```tine
// With regular function syntax
let doubled = integers.map(fn (value: int): int { value * 2 })

// With callback syntax
let doubled = integers.map((value) => value * 2)
```