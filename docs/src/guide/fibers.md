# Colorless async with fibers

🔮 Planned

## The problem with `async` in most languages

In many languages, marking a function `async` is contagious: every caller of that function usually has to become `async` too, and the two flavors of function — sync and async — end up needing separate versions of otherwise-identical code. This is sometimes called "function coloring."

Tine avoids this entirely. There's no `async fn` — a function is just a function, and any call to it can be made blocking or non-blocking independently, at the call site.

## Calls are synchronous by default

Every call in Tine blocks until it completes — including things that would normally require `await` elsewhere, like fetching data over the network:

```tine
let response = fetch("https://example.com/data")
handleResponse(response) // This runs only after the fetch completes
```

This reads exactly like ordinary synchronous code, because it *is* synchronous from the caller's point of view — nothing about `fetch` here suggests it might take time.

## Fibers: blocking without stalling everything

Under the hood, each unit of concurrently-running code is a *fiber* — lightweight enough that blocking one doesn't block the whole program. While the `fetch` call above waits on the network, other fibers keep making progress. This is closer to how goroutines work in Go than to JavaScript's single-threaded event loop with explicit `await` points.

## Making a call asynchronous

To run a call without waiting on it immediately, prefix it with `async`. Instead of blocking, this returns a `Promise` right away:

```tine
let pending = async fetch("https://example.com/data")
// pending is not yet resolved at this point
```

## Getting the result with `await`

`await` blocks the current fiber until a `Promise` resolves, and unwraps its value:

```tine
let pending = async fetch("https://example.com/data")

// do other work here while the fetch is in flight

let response = await pending
```

## Running work concurrently

Because `async` and `await` are separate steps, you can start several calls before waiting on any of them — letting them run concurrently instead of one after another:

```tine
let first = async fetch("https://example.com/users")
let second = async fetch("https://example.com/posts")

// both requests are in flight at this point

let users = await first
let posts = await second
```

Compare this to calling `fetch` twice without `async`: each call would block completely before the next one even starts.