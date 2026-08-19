# Reactivity with signals

🚧 Unstable

_`computed`'s exact syntax is still being reconsidered — the expression form shown here may become a callback form instead._

## Creating a signal

A signal is a value that can change over time, and that other parts of your program can react to automatically. Create one with `state`:

```tine
let counter = state(0)
```

## Reading and writing a signal

A signal isn't the value itself — it's a container for one. Reading or writing the value inside goes through `*`, the same dereference operator whether you're getting or setting:

```tine
let counter = state(0)

let current = *counter      // read: 0
*counter = *counter + 1     // write: counter now holds 1
```

This is deliberate: Tine doesn't auto-unwrap signals for you. Every place a signal's value is actually read or written is visible in the code, rather than happening implicitly.

### An exception to value semantics

Recall from previous chapters that assigning a value normally behaves as if it were deeply copied. Signals are the one deliberate exception to that: they use reference semantics, so multiple bindings to the same signal all observe and affect the same underlying state.

```tine
let counter = state(0)
let sameCounter = counter

*sameCounter = 5
*counter            // 5, not 0
```

This is also why a signal can be written to even though it was declared without `mut`. Signals are inherently mutable.

## Derived values with `computed$`

A `computed$` value is derived from one or more signals, and stays up to date automatically whenever those signals change:

```tine
let count = state(0)
let doubled = computed$(*count * 2)

*count = 5
*doubled    // 10 (recalculated automatically)
```

Like a signal, a `computed` value is read with `*`. Unlike a signal, it can't be written to directly — its value only ever comes from re-evaluating its expression.