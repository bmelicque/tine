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
- `/vscode-extension` - The language extension for VSCode

## Project status

⚠️ Tine is under development and is not yet production-ready

## Getting started

You should have Rust installed.

Then run:

```sh
git clone https://github.com/bmelicque/tine
cd tine/crates/transpiler
cargo run <source-file> <output>
```
