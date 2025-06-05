- accept dynamic in some expressions? `if some { [""] } else { [] }`

- fix codegen for anonymous structs in arrays `[]User((name, role))`

- match numbers
- match strings
- match arrays

- annotated AST (check FIXMEs)
- function call
- HOF

  - missing type annotation in predicates (e.g. `map(array, (el, i) => {})`)
  - infer types

- else after any option?
  - (if expression) else
  - (for expression) else
  - `x := maybeGet() else ...`
  - else type must be either:
    - left's some type
    - left's type (=> can chain elses)
