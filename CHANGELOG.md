# Changelog

All meaningful changes to this project will be written here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)

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