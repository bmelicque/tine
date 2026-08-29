# Tine

This is the main repository for Tine, a statically typed language for building reactive frontend applications without runtime errors.

[![Project Status: WIP – Initial development is in progress, but there has not yet been a stable, usable release suitable for the public.](https://www.repostatus.org/badges/latest/wip.svg)](https://www.repostatus.org/#wip)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

📖 **[Read the guide](https://bmelicque.github.io/tine/)** — the best place to start, covering installation, a first example, and the language itself chapter by chapter.

## Why Tine?

Frontend frameworks today often rely on large runtimes, complex build pipelines, or implicit reactivity models. TypeScript improves JavaScript with types, but still inherits many of JavaScript's design limitations.

Tine builds reactivity, and other common frontend concerns, directly into the language — with value semantics by default and structural traits instead of inheritance. Tine compiles to production-ready JavaScript with a single CLI command.

## A quick look

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

See the [guide](https://bmelicque.github.io/tine/) for a full walkthrough, or jump straight to [installing Tine](https://bmelicque.github.io/tine/introduction.html#installing).

## Repository structure

This repository is a monorepo containing all the core components of Tine:

- `/crates`
  - `/parser` - Parses source files into an AST
  - `/ast` - AST types
  - `/expander` - Expands language macros within the AST
  - `/checker` - Type checker; lowers the AST into IR
  - `/ir` - Lowered representation with resolved types and symbols
  - `/symbols` - Symbols and associated tables
  - `/types` - Resolved type tables
  - `/transpiler` - Transpiles Tine code to JavaScript
  - `/cli` - Command-line interface for the transpiler
  - `/server` - Language server (LSP)
  - `/common` - Data structures and utilities shared across crates, notably `locations` and `diagnostics`
  - `/macros` - Utility macros for the other crates
- `/examples` - Example codebases
- `/docs` - The language guide and documentation
- `/vscode-extension` - VSCode language extension