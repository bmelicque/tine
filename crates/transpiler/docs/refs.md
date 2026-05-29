# Handling references in the transpiler

Tine uses value-based semantics, but you can also take references using the `&` operator.

Since JS has strict semantics (value-based for primitives, reference-based for objects), the transpiler needs to find ways to wrap references and clone objects efficiently.

## Data structures

### Unified API

- `ref.$get()` for deref and getting the inner value
- `ref.$set()` for indirection (`*ref = ...`)

Both methods use the `$` character, which points to internal-only identifiers (this character is forbidden in user-defined names).

### Objects

Objects are already handled by reference in JS, which means we need to implement value-based semantics.

Classes will implement the `$get` and `$set` methods:
- `$get` will basically be a `clone` function.
- `$set` should assign all fields recursively.
  
For example:

```tine
struct User {
    name        str
    age         int
    permissions []Permission
}
```

might become:

```js
class User {
    constructor(name, age, permissions) {
        // implementation
    }

    $get() {
        const permissions = new Array(this.permissions.length)
        for (let i = 0; i < this.permissions.length; i++) 
            permissions[i] = this.permissions[i].$get()
        return new User(this.name, this.age, permissions)
    }

    $set(other) {
        this.name = other.name
        this.age = other.age
        this.permissions.length = other.permissions.length
        for (let i = 0; i < this.permissions.length; i++) 
            this.permissions[i].$set(other.permissions[i])
    }
}
```

Since JS objects already have stable identity, recursively applying $set ensures that nested objects also preserve their identity under mutation.

> ⚠️ Any mutation through a reference must go through `$set`, otherwise reference aliasing is broken.
> For example if `object.inner` is itself an object, `const ref = object.inner` would break if assigned directly (`ref = ...` or `object.inner =`) instead of using `$set`.

### Primitives

Primitives are trickier to handle, since they are handled by value in JS.

The transpiler needs a stable way to create references, that can also be compared (something like pointer equality).
This comparison constraint prevents us from using getter and setter functions, like:
```js
let primitive = 0

// The following cannot be compared with another ref using `===`
let ref = {
    get() { return primitive },
    set($) { primitive = $ },
}
```
This approach breaks pointer equality (`===`), because each reference is a distinct object even if it targets the same value.

The transpiler handles this in 2 ways:

#### 1. Members

Expressions like `&object.inner`, where the member is a primitive.

When encountering this, the transpiler will use the following internal class:

```js
class MemberRef {
    constructor(obj, prop) {
        this.obj = obj
        this.prop = prop
    }

    $get() { return this.obj[this.prop] }

    $set($) { this.obj[this.prop] = $ }
}
```

#### 2. The rest

The previous method doesn't work when using raw primitives (or primitives returned by functions).

To use reference, the primitive needs to be wrapped in an object, using another internal class:

```js
class Cell {
    constructor($) {
        this.$ = $
    }

    $get() { return this.$ }
    $set($) { this.$ = $ }
}
```

This `Cell` internal class is the **canonical storage location**. All refs will point to the same instance.

To avoid the overhead as much as possible, the transpiler will check if a primitive variable is referenced at some point in the code. If it's the case, it is declared as a Cell. Else, it will just use the variable normally.

## Assigning a value

In the following contexts:
- assignments
- variable declarations
- function call argument
- constructor literals (struct, enum, map)

the code generator should check whether the underlying value is handled by value or by reference at the JS level. If a value is known to be reference-backed (i.e. implements $get/$set), then, call `$get` in value context, and `$set` to assign a value. Else, just use the value normally, and assign normally.

⚠️ The compiler should try its best to avoid unnecessary cloning (strategies still have to be defined).

## Example

From:

```tine
let x = 1
let ref1 = &x
*ref1 = 2

// Object should be defined earlier
let object = Object { value: x }
object.value = 1
let ref2 = &object.value
*ref2 = *ref1
```

to:

```js
let x = new Cell(1)
let ref1 = x
ref1.$set(2)

let object = new Object(x)
object.x = 1
let ref2 = new MemberRef(object, "value")
ref2.$set(ref1.$get())
```