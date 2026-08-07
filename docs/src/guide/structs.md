# Structuring related data

## Structs

Structs are a collection of fields. In that way, they are similar to [tuples](./collections.md#tuples).

However, structs label each piece of data to make clear what value it holds.

```tine
// Depending on your locale, days, months and years are ordered differently,
// so the following is ambiguous
let date = (3, 4, 8)
```

To structure data, you first have to declare a `struct`, defining what should all structs of this type contain.

```tine
struct Date {
    day:   int
    month: int
    year:  int
}
```

Once declared, you can create an instance using the constructor literal syntax.

```tine
let unixStart = Date {
    day:   1,
    month: 1,
    year:  1970,
}
```

You can access inner data with the `.` notation:

```tine
let unixStartYear = unixStart.year
```

Structs are **nominal**. This means that different structs lead to different types that are not compatible, even though they could be the same structurally.

```tine
struct User {
    id:   int
    name: str
}

fn greetUser(user: User) {
    // ...
}

struct Node {
    id:   int
    name: str
}
let node = Node { id: 1, name "Ada" }

greetUser(node) // caught by the compiler
```

Also, don't forget that Tine uses value semantics!
This means assigning a struct behaves as if the struct was deeply cloned on copy:

```tine
let mut firstUser = User { id: 1, name "John" }
let secondUser = firstUser // behaves as a clone
firstUser.id = 0 // `secondUser`'s id has not changed
```

> Please note the 'as if' in the sentence above. Obviously, the Tine compiler avoids deep cloning as much as possible, to improve performance. You can however reason about the code as if it were deep-cloned.

## Defining behavior with methods

🚧 _unstable: still wondering whether `impl` blocks should have `fn` before methods_

Methods are just like functions, except they are defined in the context of some struct (or [enum], see later). This is a nice way to group data and the associated behavior.

Let's take a look at the following example:

```tine
struct Rectangle {
    width:  int
    height: int
}

impl Rectangle {
    new(width: int, height: int): Rectangle {
        Rectangle { width, height }
    }

    (r) area(): int {
        r.width * r.height
    }
}

let rect = Rectangle.new(25, 15)
let area = rect.area()
```

### Mutating methods

A receiver marked `mut` allows the method to modify the struct in place:

```tine
impl Rectangle {
    (mut r) grow(amount: int) {
        r.width = r.width + amount
        r.height = r.height + amount
    }
}

let mut rect = Rectangle.new(25, 15)
rect.grow(5)
```