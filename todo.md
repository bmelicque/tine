- accept dynamic in some expressions? `if some { [""] } else { [] }`

- match numbers
- match strings
- match arrays
- function expressions
- handle return in functions (check nested blocks)
- function call

- else after any option?
  - (if expression) else
  - (for expression) else
  - `x := maybeGet() else ...`
  - else type must be either:
    - left's some type
    - left's type (=> can chain elses)
