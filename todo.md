## Types

### Parser

- allow type constraints in generics

### Type checker

- type instantiation

  - instead parse map_literal as `t#u{}` (no alias)
  - need to rework grammar:
    - `type_name = map_type|result_type|...`
    - `map_type = unary? ~ # ~ unary?` (prevent `a#b#c` which is ambiguous)
    - `result_type = unary? ~ ! ~ unary?` (prevent `a#b#c` which is ambiguous)
    - `unary_type = array_type | option_type`
    - `array_type = [] ~ primary` (primary = generic)
    - `option_type = ? ~ primary`

- inferred type instances `#{ key: 42 }`

### Codegen

- type instantiation

### Other

- handle all FIXMEs and TODOs

## Expressions

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
