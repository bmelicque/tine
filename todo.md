- member expression: `tuple.index`

  - parse `camelCase.number`
  - type check
  - codegen

- member expression: `object.key`

  - parse `camelCase.camelCase` or `camelCase.number`
  - type check
  - codegen

- variants

  - parse `PascalCase.PascalCase` as sum variant
  - type check
  - codegen

- tuples expressions: `a, b`

  - parsing

- blocks as expressions
- if - else
- for loops
- handle return in functions (check nested blocks)
- function call
