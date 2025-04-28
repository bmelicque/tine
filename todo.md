- codegen option literals

  - add `__Option` class
    - add flags to codegen
    - push `__Option` definition the first time (will probably need to do better with bundling later)

- infer arg types in options, arrays, maps (if expected is not a trait)

  - `?User{{name: "John"}}`
  - `[]User{{name: "John"}, {name: "Jane"}}`
  - `string#User{"john": {name: "John"}, "jane": {name: "Jane"}}`

- member expression: `object.key`, `tuple.0`

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
