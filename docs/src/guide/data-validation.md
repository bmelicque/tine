# Validating data

🔮 Planned — not yet implemented

_This chapter describes a planned feature. Nothing on this page works yet — it's here to give you a sense of where Tine is headed._

## The problem

Data coming from outside your program — a network response, form input, a file on disk — arrives without any guarantee it matches the shape your code expects. Most languages leave you to write that checking by hand, field by field, or reach for a separate validation library that lives outside the type system.

## The plan: derive macros

Tine intends to solve this with `@derive` attributes on structs, generating serialization and validation logic from the struct definition itself — so the shape you already declared *is* the source of truth, rather than something you duplicate in a schema file:

```tine
@derive(Serialize, Deserialize)
struct User {
    name: str,
    age:  int,
}
```

Applying `@derive(Serialize, Deserialize)` would generate the logic needed to convert a `User` to and from an untyped format (e.g. JSON) — checking that incoming data actually has a `name` string and an `age` int before it's ever treated as a trusted `User` value.

## Beyond basic shape-checking

The current thinking goes further than "does this field exist and have the right type" — the goal is to eventually support finer-grained rules (for example, constraining `age` to a sensible range) directly through the same attribute-driven approach, rather than requiring hand-written validation code alongside the struct.

The exact syntax for these finer-grained rules isn't decided yet, so it isn't shown here. This section will be filled in once that design solidifies.

## Why derive macros specifically

This build on the same `@derive` mechanism used elsewhere in Tine (for example, deriving `Eq` or `Hash`) — one general-purpose macro system, rather than a special case just for validation.