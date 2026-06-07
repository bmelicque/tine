# Ownership analysis

## Context

The code generator has to generate JS code that preserves Tine's value-based semantics.

Instead of doing things naively and clone everything every time, the goal is to clone efficiently, only when necessary.

The first big step is to share data between immutable variables. This is always safe.

Then, when crossing the mutable/immutable boundary, or even when mutating data, deep cloning can be reduced by tracking ownership of the data and cloning only when moving is not possible.

## Tracking ownership

Moving is done by transferring ownership from one place to another.

This can be done only if the first place is not live after this.

### Immutable variables

Immutable variables, since they are shared, need to by analysed by alias group.

This analysis will track which objects might share references and will consider the group as a whole.

Objects that escape the current scope (function params, return values, assignment to outer variables) are considered always live.
This also applies to variables that escape *after* the current use site.

This aliasing analysis should also keep track of functions definitions so it can, when possible, know whether call arguments end up being shared with its return value or not.
This also tells whether a function returns a fresh object or not.

### Mutable variables

Mutable variables are much simpler to anaylse, since they are almost never shared. The only place where it happens is when a mutation is captured by a closure (eg `fn update(value: Type) { captured = value }`).
In that case, this mutable borrow lives as long as the closure.

Mutable variables cannot be moved while a mutable borrow is live.

## Analyser structure

The analyser works in 3 passes:

### Liveness analysis (`liveness.rs`)

Here, the analyser tracks all the use sites of every symbol.
It also annotates each use-site with information relevant to the liveness of a variable (is it captured by a closure? is it captured by a loop body? etc.).

### Alias analysis (`alias.rs`)

Here, the analyser tracks which immutable symbols may share the same JS object.

This implies function body analysis and escape analysis.

> This analysis is not field-sensitive: aliasing is tracked at the object level.
> A future improvement could track aliasing field by field to reduce unnecessary clones.

### Ownership analysis (`ownership.rs`)

Here, the analyser combines the result of the two previous steps to determine the most efficient safe action (cloning or not) for every symbol usage.