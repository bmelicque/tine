# Defining shared behavior with traits

🔮 Planned — partially implemented

_This chapter describes a feature that's still taking shape. Some of what's shown here works today; some of it doesn't yet. Treat this as a preview, not a reference._

## Declaring a trait

A trait describes a set of method signatures — a shape of *behavior* rather than data:

```tine
trait Shaped {
    fn area() -> float
}
```

## Implementing a trait — implicitly

Tine traits are satisfied structurally: any type with the right methods already implements the trait, with nothing extra to write.

```tine
struct Rectangle {
    width:  float
    height: float

    fn area() -> float {
        .width * .height
    }
}
```

Because `Rectangle` already has an `area()` method matching `Shaped`'s signature, `Rectangle` satisfies `Shaped` automatically — there's no separate step declaring that relationship.

```tine
struct Circle {
    radius: float

    fn area() -> float {
        PI * .width * .width
    }
}
```

`Rectangle` and `Circle` share no relationship whatsoever, they just happen to both implement the `Shaped` trait.

## Writing a function that accepts any implementer

A trait name can currently be used directly as a parameter type, meaning "anything that implements this trait":

```tine
fn totalArea(shapes: Shaped[]) -> float {
    let mut total = 0.0
    for shape in shapes {
        total = total + shape.area()
    }
    total
}
```

`totalArea` works for an array mixing `Rectangle`s and `Circle`s (and anything else satisfying `Shaped`), without needing to know about either concrete type.

This exact syntax — reusing the trait name in place of a type — is one of the things most likely to change as generics and trait bounds are fleshed out further, so don't be surprised if a future version of Tine asks you to spell this differently.