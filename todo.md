## Types

### Parser

- allow type constraints in generics
- define generic type

### Type checker

- function types
- type instantiation
- inferred type instances `#{ key: 42 }`

### Codegen

- type declaration
- struct declaration
- sum type declaration
- type instantiation

### Other

- handle all FIXMEs and TODOs

## Expressions

### From zero

- tuples: `a, b`
- access: `object.key`, `tuple.0`

## Flow

- blocks as expressions
- if - else
- for loops
- handle return in functions (check nested blocks)
- function call

## Other

- adjust reserved names (`function`, `let`, etc.) when transpiling
- fix exponentiation (should be rtl)
