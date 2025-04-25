- codegen option literals
  - add `__Option` class
- type check generics
- allow type constraints in generics
- grammar: `identifier` vs. `type_identifier`
  - `type_identifier` = PascalCase | number, etc.
- member expression: `object.key`, `tuple.0`

  - parsing
  - type check
  - type check as sum variant
  - codegen

- tuples expressions: `a, b`

  - parsing

- blocks as expressions
- if - else
- for loops
- handle return in functions (check nested blocks)
- function call

- adjust reserved names (`function`, `let`, etc.) when transpiling
- FIXME: exponentiation (should be rtl)
