## MODIFIED Requirements

### Requirement: Languages own dependency syntax
Each supported language SHALL translate its grammar-specific dependency forms
into private dependency syntax values during the existing source parse. The
translation SHALL include reference kind, target text, source span, and ordered
resolution candidates, and SHALL perform no filesystem or Git access.

A grammar form that imports several items in one declaration — such as a Rust
`use` declaration with a brace list — SHALL emit one reference per imported
item with that item's full reconstructed path, so item-level dependency
fidelity is preserved. A grouped declaration SHALL NOT collapse to one
reference naming only its shared prefix.

#### Scenario: Two languages import local files differently
- **WHEN** their grammar nodes express equivalent local dependencies
- **THEN** each language implementation emits its own candidates and graph policy receives the same dependency value shape

#### Scenario: Dependency target is dynamic
- **WHEN** source syntax does not identify a safe fixed target
- **THEN** the implementation emits an unresolved reason instead of manufacturing a candidate

#### Scenario: One declaration imports three items
- **WHEN** a Rust file contains `use crate::{a, b, c};`
- **THEN** extraction emits three references, one per item, and no single reference for the bare prefix
