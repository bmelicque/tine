# Control flow

## Blocks

You can group statements together into _blocks_.

Blocks themselves are **expressions**, not statements. A block evaluates to its last statement if it is an expression.

```tine
let sum = {
    let x = 1
    let y = 2
    x + y
}
```

## `if`/`else`

`if` takes a condition, followed by a block.
An alternate body can be provided with the `else` keyword.
`if` expressions can be chained.

```tine
if temperature < 0 {
    // frozen
} else if temperature < 100 {
    // liquid
} else {
    // vapor
}
```

### What an `if`/`else` evaluates to

When every possible branch is covered — that is, there's a final `else` — the whole expression evaluates directly to whichever branch ran. This means that the type of both branches must be the same.

```tine
let status = if temperature < 0 { "freezing" } else { "not freezing" }
// here `status` is a plain `str`
```

Without a final `else` (an `else if` chain doesn't count as one), there's no guaranteed value if no branch matches — so the expression evaluates to an `Option` instead, `None` when nothing matched:

```tine
let status = if temperature < 0 { "freezing" }
// here `status` is an `Option<str>`
```

## `for`

Tine has a single looping construct, `for`, covering both what other languages split into `while` and `for`.

### Condition-only loops

With just a condition, `for` behaves like a `while` loop, running as long as the condition holds:

```tine
let mut count = 3

for count > 0 {
    count = count - 1
}
```

### Iterating over a collection

`for` also iterates over an array by value, binding each element in turn:

```tine
let names = ["Ada", "Barbara", "Grace"]

for name in names {
    // log name
}
```

### `break` and `continue`

`continue` skips to the next iteration; `break` exits the loop early:

```tine
let names = ["Ada", "Barbara", "Grace"]

for name in names {
    if name == "Ada" { continue }
    if name == "Barbara" { break }
    // log name
}
```

### What a `for` loop evaluates to

Like a function's `return`, `break` can carry a value out of the loop — `for` is itself an expression:

```tine
let names = ["Ada", "Barbara", "Grace"]

let found = for name in names {
    if name == "Barbara" { break name }
    // log name
}
// found is an Option<str>: Some("Barbara") here
```

If the loop finishes without hitting a `break` that carries a value, the expression evaluates to `None`. We'll put this to more practical use once pattern matching is covered.