# Tine

This is the main repository for Tine, a statically typed language for building reactive frontend applications without runtime errors.

[![Project Status: WIP – Initial development is in progress, but there has not yet been a stable, usable release suitable for the public.](https://www.repostatus.org/badges/latest/wip.svg)](https://www.repostatus.org/#wip)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Why Tine?

Frontend frameworks today often rely on large runtimes, complex build pipelines, or implicit reactivity models.
TypeScript improves JavaScript with types, but still inherits many of JavaScript’s design limitations.

Tine solves this by:

- Building common frontend concerns such as reactivity, asynchronous code, and data validation directly into the language
- Value semantics by default, with explicit references for shared mutable state.

Tine code compiles to production-ready JavaScript with a single CLI command.

## What Tine is not

Tine is **not**:

- A framework like React or Vue
- A general purpose language

## Example

```tine
use dom.render
use signals.state

const counter = state(0)

fn increment() {
    // deref a signal to get/set its value
    *counter = *counter + 1
}

const app = <div id="app">
    <button onclick={increment}>Click me!</button>
    // signals automatically update the DOM
    <p>You clicked the button {counter} time(s)</p>
</div>

render("body", app)
```

## Syntax

Discover the syntax using the [syntax guide](./syntax.md)

## Getting started

### Installing

As of today, there are no binaries to download, but you can build them yourself or just run as dev.

First, make sure you have [Rust](https://rust-lang.org/) installed.

Then just run the following command to clone the project.

```sh
git clone https://github.com/bmelicque/tine
```

### Transpiling a project

Once you have everything installed, you can transpile your files using this command (at project's root):

```sh
cargo run -p tine_cli build <source> <output>
```

You can use the 'counter' example to test it:

```sh
cargo run -p tine_cli build ./examples/counter/counter.tine ./examples/counter/output.js
```

The `index.html` file in that folder already expects an `output.js` file to run.
You can run it using any web server.

### Using the VSCode extension

The VSCode extension is not yet published on the marketplace.

You can still use it in one of two ways:

#### Development mode

Open `/vscode-extension/src/extension.ts` in VSCode and press `F5`.  
This will compile the language server and launch a new VSCode window with the extension enabled.

#### Manual installation

Build the extension and install the generated `.vsix` file in VSCode.

## Repository structure

This repository is a monorepo containing all the core components of Tine:

- `/crates`
  - `/cli` - Command-line interface for the transpiler
  - `/core` - Language parser, types and core semantics
  - `/enum_from_derive` - A utility macro for all the `enum`s in `core`
  - `/server` - Language server (LSP)
  - `/transpiler` - Transpiles Tine code to JavaScript
- `/examples` - Contains example codebases
- `/vscode-extension` - The language extension for VSCode