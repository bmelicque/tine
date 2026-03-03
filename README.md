# Tine

Tine is statically typed language designed to build modern frontend applications simply and safely.

Tine aims to provide:

- strong type-safety
- no function coloring
- first-class signals and dom rendering

Tine code is transpiled into optimized JavaScript code ready to be shipped.

[![Project Status: WIP – Initial development is in progress, but there has not yet been a stable, usable release suitable for the public.](https://www.repostatus.org/badges/latest/wip.svg)](https://www.repostatus.org/#wip)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Repository structure

This repository is a monorepo containing all the core components of Tine:

- `/crates/core` - Language parser, types and core semantics
- `/crates/server` - Language server (LSP)
- `/crates/transpiler` - Leverages `core` to produce JS output
- `/examples` - Contains example codebases
- `/vscode-extension` - The language extension for VSCode

## Project status

⚠️ Tine is under active development and is not yet production-ready

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
You just have to run it using your favorite server!
