# Introduction

Tine is a statically typed language for building reactive frontend applications without runtime errors.

Frontend frameworks today often rely on large runtimes, complex build pipelines, or implicit reactivity models. TypeScript improves JavaScript with types, but still inherits many of JavaScript's design limitations.

Tine takes a different approach:

- Reactivity, data validation, asynchronous code and other common frontend concerns are built directly into the language rather than layered on top of it.
- Value semantics by default, with signals as an explicit, deliberate exception for shared reactive state.
- Structural traits with no inheritance — behavior is defined by what a type *can do*, not by where it sits in a hierarchy.

Tine compiles to production-ready JavaScript with a single CLI command.

## What Tine is not

Tine is **not**:

- A framework like React or Vue
- A general-purpose language

## A quick look

```tine
use dom.render
use signals.{state, computed$}

let counter = state(0)
let suffix = computed$(if *counter != 1 { "s" } else { "" })

fn increment() {
    *counter = *counter + 1
}

let app = <div id="app">
    <h1>A 'counter' example</h1>
    <button onclick={increment}>Click me!</button>
    <p>You clicked the button {counter} time{suffix}</p>
</div>

render("body", app)
```

This one example already touches several of Tine's core ideas: `state` creates a piece of reactive data, `*` explicitly reads or writes it (no hidden magic), and the markup updates automatically wherever that state is used — down to just the piece of the page that actually changed.

## Project status

Tine is a work in progress. The core language — bindings, control flow, functions, structs, enums, and pattern matching — is usable today. Some chapters in this guide cover features that are still partially implemented or actively being designed; those are marked accordingly as you go.

## Installing

As of today, there are no prebuilt binaries — you'll build Tine yourself.

First, you'll need [Rust](https://rust-lang.org/) installed.

Then, clone Tine's repo:

```sh
git clone https://github.com/bmelicque/tine
```

You can then run `cargo build` to buld binaries, or `cargo run` to use it on the fly.

## Trying it out

From the project root, transpile a `.tine` file to JavaScript:

```sh
cargo run -p tine_cli build <source> <output>
```

The repository includes examples you can try in the `examples` folder.

```sh
cargo run -p tine_cli build ./examples/counter/counter.tine ./examples/counter/output.js
```

For now, the cli only handles Tine-to-JS, but example folders also include ready-to-use HTML files that expect a JS output.

## Editor support

A VS Code extension exists but isn't published on the marketplace yet. To use it locally, see the extension's own setup instructions in the repository.

You can either press `F5` on your keyboard to build a fresh language server and launch a VSCode debug window where you can use the extension.

If you don't plan on working on Tine and just use it, you can also build it then install `.vsix` file.

## Where to go next

The [Guide](./guide/variables.md) walks through Tine's core concepts in order, starting with variables and basic types. Each chapter builds on the ones before it, so reading in order is the easiest path if you're new to the language.