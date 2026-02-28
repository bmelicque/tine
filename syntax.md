# Introduction

This document describes the core syntax of the language: how programs are structured, expressions built and values written.

This is a **high-level overview**, not a fully-detailed grammar.

Also please note that since Tine is in its early stages, this is syntax might be tweaked in later versions.
(Also note that the parser might break under some circumstances)

# Common programming concepts

This part covers concepts found accross most programming languages.

## Values

Tine has various value types, including the most simple ones:
- **booleans** of type `bool` (`true` or `false`)
- **integers** of type `int` (eg. `0`, `42`)
- **floating point numbers** of type `float` (eg. `2.5`, `1.`)
- **strings** of type `str` (eg. `"hello"`)

> If you come from JavaScript, you might wonder why make integers and floats distinct when it could only be `number`.
> Actually, differentiating the two helps communicating intent, and also avoids shenaningans when handling indices or ids.

> If you come from lower-level programming languages, you might be surprised to see strings being considered a 'simple' type.
> This is actually common in web development (see JavaScript or Go), since text is such a major part the web.
> Thus, strings complexity is abstracted by the language itself.

Tine also support common binary operations:

```tine
// Arithmetic operations
1 + 1
2. * 3.5
4 - 2
4 / 2
5 % 2

// Comparisons
1 == 1
1 != 2
1 < 2
1 <= 2
2 > 1
2 >= 1

// Logical operations
true && false
true || false
!false

// String concatenation
"Hello, " + "World!"
```

> **Note to JavaScript developers:**
> Types cannot be mixed (there is no type coercion).
> Thus, equality is checked with `==` and inequality with `!=`.

If you need to mix `int`s and `float`s, you need to explicitely convert one into another.

```tine
4. * 2 // This is invalid
round(4.) * 2 // This is ok
float(3) / 2. // This is ok
```

## Variable declarations and assignements

Variables can be declared using either `const` or `var`.
Variables declared with `const` are immutable: the cannot be reassigned nor mutated.
Variables declared with `var` can be reassigned and mutated.

```tine
const x = 42
x = 0
// ^ The compiler will report this as an error

var y = 42
y = 0 // This is ok
```

> If you come from JavaScript, you might be reluctant to use the `var` keyword. This is actually a good thing: you should default to using immutable variables, and use mutable ones only when absolutely necessary.

Assignments are **not** expressions. This means that they cannot be used as conditions or arguments.

```tine
// This is invalid:
if value = 42 {} 
```

## Compound types

Tine is a statically-typed language. This means that every variable has a type.
Simple 'scalar' types include `bool`, `int`, `float` and `str`, already detailled in the 'Variables' section.
However, you might sometimes need to use more complex, 'compound' types.

### Tuples

Tuples are a collection of values of different types. Once declared, a tuple cannot change size nor element types.

Tuples are declared using a comma-separated list of values, surrounded by parentheses.
For example:

```tine
const aTuple = (1, "hello", true)
```

Having a fixed size allows you to access elements by index safely:
```tine
aTuple.1 // "hello"
aTuple.3 // <- The compiler will report this as an error
```

You can also pattern-match a tuple to destructure it:

```tine
const (id, text, isDone) = (1, "hello", true)
```

### Arrays

Arrays are collections of values of the same type. They can be resized, but not the element type cannot change.
Arrays are declared using square brackets, with values separated by commas:

```tine
const anArray = [1, 2, 3]
```

Since arrays can be any size, accessing elements cannot be done via a single operator.

Instead, you should use the `get` method, which takes an index as argument and returns an option:

```tine
anArray.get(1) // Some(2)
anArray.get(4) // None
```

This might seem cumbersome at first.
However, in practice, accessing elements by index should be avoided, and using iterators should be the preferred way of doing things.

Also note that arrays are handled **by value**, unlike JavaScript arrays or Rust's vectors.

This has several implications:
- Assigning creates a copy of the array, not a new reference to the same array.
- Changing the length of the array, or mutating one of its inner values is considered mutating the array.

```tine
var a = [1, 2, 3]
var b = a
b.set(0, 42)
a   // [1, 2, 3], left unchanged
b   // [42, 2, 3]

const array = [0]
array.push(1)
// ^ prevented by the compiler
```

> You might be wondering about the performane issues of handling arrays by value instead of reference. 
> Actually, the compiler will do its best to optimize this away and avoid unnecessary copies.
> In practice, this means that you should not worry about performance issues when using arrays. Most of the time.

### Aliasing types

You can create aliases for existing types using the `type` keyword.

```tine
type IntPair = (int, int)
```

Type aliases are just sugar you defined for types that you use often.
They are considered the same as the original type by the compiler (it has structural equality).

## Blocks and control flow

### Blocks

You can group statements together into _blocks_.

Blocks themselves are **expressions**, not statements. This means you can use blocks as values.

A block's value is that of its last statement if it is an expression.

```tine
const sum = {
    const x = 1
    const y = 2
    x + y
}
```

### If expressions

`if` is an expression that allows your code to branch based on a condition.
If the condition is met, the first branch will be executed and its value returned.
Otherwise, if there is a second branch, it will be executed and its value returned.
This second branch itself can also have a condition and so on.

```tine
const result = if x > 0 {
    "positive"
} else {
    "negative"
}
```

All branches must produce values of the same type.
If there is no catch-all branch (a simple `else` branch with no condition), the result type will be an `Option` (see `types.md`).

### Loops

You can repeat blocks of code with loops, using the `for` keyword. There are two main ways to use loops:
- iterating through a collection of values with the `in` keyword
- looping while a given condition holds true

```tine
for user in users {
    greet(user)
}

for count < 5 {
    io.log("count: " + count)
}
```

You might need to break out of a loop, which you can do with the `break` statement.

Loops are expressions, so you can assign them to variables. Loops evaluate their `break` values. They all need to match in type, and the expression type will be an `Option` of those types.

```tine
const result = for count < 5 {
    io.log("count: " + count)
    if count == 3 {
        break "three"
    }
    if count == 4 {
        break "four"
    }
}
```

## Functions

### Declaration

Functions can be declared using the `fn` keyword. Return type has to be explicitely annotated, unless the function returns nothing.

```tine
fn add(x int, y int) int {
    x + y
}
```

Tine also supports generics.

```tine
fn getFirst<T>(array []T) ?T {
    array.get(0)
}
```

If you need to return early, you can use the `return` statement. It will return from the function immediately.

```tine
fn add(x int, y int) int {
    if x < 0 || y < 0 {
        return -1
    }
    x + y
}```

Functions are closures, meaning they capture their environment. This means that they can access the values of their surrounding scope.

```tine
const x = 10
fn getX() int {
    x
}
```

Functions can then be called using parentheses.

```tine
const result = add(1, 2)
```

### Function expressions and callbacks

Functions are first class citizens in Tine, so you can assign them to variables and pass them around.

```tine
const add = fn (x int, y int) int {
    x + y
}
```

Tine also provides a concise syntax for callbacks (functions that are passed as arguments):
- use a fat arraow `=>` instead of the `fn` keyword
- no need to provide types for parameters and return value, since these are inferred from the callback expected type
- the body can be any kind of expression instead of a block

```tine
// With regular function syntax
const result = integers.map(fn (value int) int { value * 2 })

// With callback syntax
const result = integers.map((value) => value * 2)
```

# Structuring related data

## Structs

Structs are a collection of fields.
They are similar to tuples in the sense that they hold multiple values of different types.
However, structs label each piece of data to make clear what value it holds.

```tine
// Depending on your locale, days, months and years are ordered differently,
// so the following is ambiguous
type Date = (int, int, int)
```

They can be defining using the `struct` keyword, a name and a list of field definition (a field name and a type).

```tine
struct Date {
    day   int
    month int
    year  int
}
```

Once declared, you can create an instance using the constructor literal syntax.

```tine
const date = Date {
    day:   01,
    month: 01,
    year:  1970,
}
```

Structs are nominal, meaning that each struct definition will lead to a different type, even if they are structurally the same.

```tine
struct User {
    id int
}
struct Node {
    id int
}
const user = User { id: 1 }
const node = Node { id: 1 }
user == node // actually prevented by the compiler since they don't have the same type
```

Structs are handled by value and not by reference, which means that:
- assigning creates a full copy
- equality checks are deep

## Methods

TO BE DONE

# Enums and patterns

## Enums

Enums are a way of saying that a value is one of a set of possible values. They can be declared using the `enum` keyword.

```tine
enum Color {
    Red,
    Green,
    Blue,
}
```

Just like structs, enums are nominal and handled by value.

If you need your subvalues to contain more data, you can also provide constructors for them.

```tine
enum Color {
    Red,
    Green,
    Blue,
    RGB { r int, g int, b int },
}
```

## Pattern matching

TO BE DONE