# Enums and pattern matching

## Declaring an enum

An enum defines a type as one of several named possibilities, called *variants*:

```tine
enum TrafficLight {
    Red,
    Yellow,
    Green
}

let light = TrafficLight.Red
```

## Variants with data

Unlike a plain label, a variant can carry its own data — written like a tuple:

```tine
enum Shape {
    Circle(float),
    Rectangle(float, float),
}

let circle = Shape.Circle(2.0)
let rectangle = Shape.Rectangle(3.0, 4.0)
```

Each variant of the same enum can carry completely different data — `Circle` holds one `float` (a radius), `Rectangle` holds two (width and height). This is what makes an enum useful for modeling "one of several distinct shapes of data," rather than just "one of several labels."

## Pattern matching with `match`

`match` compares a value against a series of patterns, running the code for whichever one applies. Variant patterns can bind the data they carry to a name:

```tine
fn area(shape: Shape) -> float {
    match shape {
        Circle(radius) => PI * radius * radius
        Rectangle(width, height) => width * height
    }
}

// or...
impl Shape {
    fn area() -> float {
        match . {
            Circle(radius) => PI * radius * radius
            Rectangle(width, height) => width * height
        }
    }
}
```

Like `if`/`else` and `for`, `match` is an expression — the example above works because each arm's expression becomes the function's return value.

### Exhaustiveness

The compiler checks that a `match` covers every possible variant. Leaving one out is a compile error, not a runtime surprise:

```tine
fn area(shape: Shape) -> float {
    match shape {
        Circle(radius) => PI * radius * radius,
        // compiler error because `Rectangle` is not covered
    }
}
```

This means adding a new variant to an enum later will point you to every `match` that now needs updating, rather than letting a case silently fall through unhandled.

### The wildcard pattern

`_` matches anything, and is commonly used as a catch-all for the cases you don't need to handle individually:

```tine
fn isCircle(shape: Shape): bool {
    match shape {
        Circle(radius) => true,
        _ => false
    }
}
```

## `Option`

`Option` is a built-in generic enum with two variants, `Some(T)` and `None` — used throughout Tine anywhere a value might be absent, like the array `get` method from the [Collections](./collections.md) chapter, or an `if` without a final `else` from [Control flow](./control-flow.md):

```tine
let numbers = [1, 2, 3]

let defaultFirst = match numbers.get(0) {
    Some(value) => value,
    None => 0,
}
```

This is the same pattern-matching machinery as any other enum — `Option` isn't a special case syntactically, just a very commonly used one.
