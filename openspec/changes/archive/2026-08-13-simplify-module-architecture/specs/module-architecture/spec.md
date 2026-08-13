## ADDED Requirements

### Requirement: Entry modules contain wiring only
Every workspace `lib.rs` and `mod.rs` SHALL contain only crate attributes,
documentation, private module declarations, imports, and explicit reexports.
Executable `main.rs` files SHALL remain high-level composition roots.

#### Scenario: Behavior is added to an entry module
- **WHEN** an entry module defines a function, type, implementation, value,
  macro, inline module, test module, or wildcard reexport
- **THEN** the architecture check fails and identifies the file and declaration

### Requirement: Implementation visibility stays local
Implementation items SHALL use the narrowest Rust visibility that satisfies a
direct consumer, and unreachable public items SHALL fail compilation.

#### Scenario: Private module exposes an unused public item
- **WHEN** an implementation item is declared public but no crate consumer can
  reach it
- **THEN** the workspace lint fails

### Requirement: API snapshots inspect reachable items
The architecture gate SHALL compare the compiler-visible public API of each
workspace library with its checked snapshot using the pinned API inspection
tool.

#### Scenario: Reexport changes after module extraction
- **WHEN** a library adds, removes, or changes a reachable item through an
  explicit reexport
- **THEN** its API snapshot check fails with the affected crate and difference

#### Scenario: API inspection tool is unavailable
- **WHEN** the required tool or version is not installed
- **THEN** the check fails with a concise installation message
