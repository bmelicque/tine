# Changelog

All meaningful changes to this project will be written here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)

## [0.1.1] - 2026-09-07

### Fixed

- Fixed markup tag parsing
- Fixed crashes when parsing invalid enums
- Fixed crashes when parsing invalid use items
- Fixed crashes when checking some reactive expressions in markup
- Fixed module resolution
- Fixed functions treated as statements where they should be expressions
- Fixed missing return in generated method code
- Fixed generated markup attribute value
- Fixed errors on duplicated method names
- Fixed concrete type resolution for struct fields
- Fixed error locations on variable declarations
- Fixed misleading error on compile
- Fixed function type display
- Fixed syntax coloring for generic args and expressions in markup
- Fixed 'Installing' section from docs

## [0.1.0] - 2026-08-29

### Added

- Added type annotations on declarations
- Added method definitions
- Added exhaustivity checks on pattern matching
- Added visibility markers `pub`
- Added array methods: `length`, `get`, `set`, `push`, `pop`, `map`, `filter`
- Added reactive node attributes
- Added mdbook-format docs
- Added todo-list example
- Added `restart` server command for extension
- Reinforced type inference
- Started implementing traits and `@derive` instructions (like `Eq` and `Hash`)

### Changed

- [BREAKING] Changed `derived$` to `computed$` (eg. `$0` -> `state(0)`; `@(*counter * 2)` -> `derived$(*counter * 2)`)
- [BREAKING] Changed declaration syntax to `let mut?`
- [BREAKING] Added `:` before type annotations and `->` before return types
- [BREAKING] Various other syntax changes

### Removed

- [BREAKING] Removed specific callback syntax

### Fixed

So much!

## [0.0.2] - 2026-03-03

CLI: v0.0.1
Core: v0.0.2
Server: v0.0.2
Transpiler: v0.0.2
VSCode Extension: v0.0.2

### Added

- Added generic function declarations `fn name<...Params>(...params) {...}`
- Added generic function calls `function.<...Args>(...args)`
- Allowed '$' in identifiers for internal macros (reserved for now)
- Made language server update on change instead of on save
- Added initial CLI implementation
- Added initial syntax documentation

### Changed

- [BREAKING] Changed operator syntax to explicit function calls for signals (eg. `$0` -> `state(0)`; `@(*counter * 2)` -> `derived$(*counter * 2)`)
- Updated signatures displayed by server to match the correct syntax
- Moved `transpiler` logic into library, used by the new CLI

### Fixed

- Fixed parser crashes on most syntax errors
- Fixed reactive dom nodes losing reactivity
- Fixed server's signature display & coloring for imported names

## [0.0.1] - 2026-01-29

Core: v0.0.1
Server: v0.0.1
Transpiler: v0.0.1
VSCode Extension: v0.0.1

Initial release