# Rendering to the DOM

## A minimal example

Bringing together functions, signals, and a JSX-like markup syntax, here's a small counter:

```tine
use dom.render
use signals.{state, computed$}

let counter = state(0)
let suffix = computed$(if *counter != 1 { "s" } else { "" })

fn increment() {
    *counter = *counter + 1
}

let app = <div>
    <button onclick={increment}>Click me!</button>
    <p>Clicked {counter} time{suffix}</p>
</div>

render("body", app)
```

## Markup

Tine markup is written inline, JSX-style — tags, attributes, and nested children read like HTML, but the whole expression produces a value you can assign, pass around, or return, just like any other expression:

```tine
let greeting = <p>Hello, Tine!</p>
```

## Interpolating values

Curly braces embed an expression's value into the markup:

```tine
let name = "Tine"
let greeting = <p>Hello, {name}!</p>
```

Signals are read the same explicit way inside markup as anywhere else — with `*`.
However, passing a full signal (and not just its value) will create a _reactive node_.

```tine
let counter = state(0)
let initalDisplay = <p>Initial: {*count}</p> // This will never ve updated
let reactiveDisplay = <p>Current: {count}</p>
```

Because `{counter}` reads a signal, that spot in the DOM is tied directly to `counter` — when the signal's value changes, only that piece of the page updates. The rest of the markup around it is untouched, rather than the whole component re-rendering from scratch.


Every time `counter` changes, only the text inside `<p>` changes — the surrounding app is never touched again after the initial render.

## Event handlers

Attributes prefixed with `on` (like `onclick`) take a function value, called when that event fires:

```tine
fn increment() {
    *counter = *counter + 1
}

let button = <button onclick={increment}>Click me!</button>
```

## Mounting to the page

`render` takes a CSS selector for where to mount, and the markup to mount there:

```tine
use dom.render

let app = <div id="app">Hello, Tine!</div>

render("body", app)
```