# Generics

## The problem

Without generics, a function or struct that should work with *any* type either has to be duplicated per type, or give up on type safety entirely. Generics let you write code once, parameterized over a type that's filled in later.

## Generic functions

A generic function declares its type parameters in angle brackets right after its name:

```tine
fn identity<T>(x: T): T {
    x
}
```

Here, `T` stands for "whatever type is passed in" — calling `identity(5)` makes `T` be `int` for that call, while `identity("hi")` makes `T` be `str`, all from the same declaration.

```tine
let a = identity(5)     // T is int here
let b = identity("hi")  // T is str here
```

### Specifying the type explicitly

Usually, `T` is inferred from the argument, as above. When you need to specify it explicitly instead, use a dot before the angle brackets:

```tine
let a = identity.<int>(5)
```

The `.` isn't just decoration — without it, `identity<int>(5)` would be ambiguous with a chain of comparisons (`identity < int > (5)`), since `<` and `>` are also the less-than and greater-than operators. The `.` tells the parser unambiguously that a type argument is coming.

## Generic structs

Structs can be generic the same way, parameterized over the type of their field(s):

```tine
struct Box<T> {
    value: T
}

let intBox = Box { value: 5 }
let strBox = Box { value: "hi" }
```

`intBox` and `strBox` are both `Box`es, but `Box<int>` and `Box<str>` are different concrete types — `T` is filled in from the value provided, same inference as with generic functions.
This means that, while they could both use methods implemented on `Box`, they cannot be used in place of one another if a concrete type is expected.

## Concrete implementations

Sometimes, you might want to implement (or override) a method for a specific concrete type. This can be done with the `impl` block mentionned in the [structs](./structs.md#method-definitions-outside-of-the-struct) chapter.

```tine
struct MyCollection<T> {
    // ...
}

impl MyCollection<int> {
    fn sum() -> int {
        // this method will be accessible only for MyCollection<int>,
        // not for other concrete types of MyCollection
    }
}
```

## What's not covered here yet

Generics currently have no way to constrain `T` to types that support specific behavior (e.g. "any type with an `area()` method") — that's planned, but not implemented. For now, a generic function's body can only do things that work for *any* possible type, without assuming anything more specific.