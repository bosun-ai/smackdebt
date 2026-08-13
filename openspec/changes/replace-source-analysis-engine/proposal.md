## Why

Smackdebt currently obtains the same three measurements through three different
engines: `rust-code-analysis`, a Ruby text scanner, and a Vue text scanner. The
results are not defined consistently enough to compare languages, and the
upstream engine computes data that Smackdebt immediately discards.

The first public release should define metric meaning before its output becomes
a compatibility promise. Language syntax must remain local to each language,
while measurement policy must have one implementation per algorithm.

## What Changes

- Replace every current source engine with one private, statically dispatched
  tree-sitter engine.
- Add a private generic language trait whose implementations own all grammar,
  query, naming, unit, control-flow, dependency-syntax, and injection details.
- Put cognitive complexity, cyclomatic complexity, and logical-line behavior in
  one focused module per algorithm.
- Define exact cross-language metric semantics and accept documented corrections
  to current values.
- Parse each source region once, reuse worker-local parser state, and construct
  Smackdebt analysis facts without an intermediate copy of the same model.
- Remove `rust-code-analysis` after every supported language moves to the new
  engine.
- **BREAKING**: Replace JSON schema version 1 with schema version 2 before the
  first public release.

## Capabilities

### New Capabilities

- `metric-semantics`: Defines language-independent measurement behavior and its
  language-specific syntax input.
- `report-schema-v2`: Defines the next machine-readable report contract.

### Modified Capabilities

- `language-analysis`: Replaces mixed analyzers with a private generic
  tree-sitter engine and defined language implementations.
- `analysis-performance`: Requires one parse per source region and worker-local
  parser, query, and scratch reuse.
- `progressive-exploration`: Moves complete machine output to JSON schema
  version 2.
- `workspace-architecture`: Keeps parser details and the language trait private
  to the language crate.
- `architecture-documentation`: Documents the generic algorithm and language
  boundary.
- `product-documentation`: Explains defined metric behavior and JSON version 2.

## Impact

The language and analysis crates, language dependencies, metric fixtures,
terminal and JSON snapshots, API snapshots, performance baselines, README, and
architecture guide change. CLI commands, exit-code meanings, privacy behavior,
and unsupported-file coverage remain compatible.

This change precedes `add-static-architecture-analysis`,
`add-evolutionary-architecture-analysis`, and `prove-unified-analysis`.
