## MODIFIED Requirements

### Requirement: Public Rust surfaces stay minimal
Workspace libraries SHALL export only behavior and values required by direct
crate consumers. The private generic language trait, concrete language marker
types, tree-sitter parsers, queries, syntax nodes, semantic traversal events,
algorithm state, Rayon types, Git process handles, status parsing, and output
helpers SHALL remain private.

#### Scenario: Language implementation changes
- **WHEN** a grammar query or semantic mapping is added inside the language crate
- **THEN** compiler-visible API snapshots remain unchanged unless the completed Smackdebt fact seam also changes intentionally

### Requirement: Language syntax and metric policy have separate owners
`smackdebt-languages` SHALL own grammar-specific parsing and syntax meaning.
Shared source measurement algorithms SHALL remain private language-crate
implementation modules, while `smackdebt-analysis` SHALL own completed
measurements, health policy, comparison, aggregation, and report facts.

#### Scenario: One language gains new syntax
- **WHEN** its grammar adds a control-flow construct
- **THEN** only that language implementation and its fixtures change unless the construct requires a deliberate shared metric-rule change
