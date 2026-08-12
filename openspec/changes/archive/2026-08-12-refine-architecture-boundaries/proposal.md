## Why

The product foundation defines Smackdebt's user flows, but it does not define
the crate seams, ownership rules, or measurable performance contract needed to
implement them safely. Those decisions must be explicit before implementation
so broad language support and Git analysis do not become one coupled pipeline.

## What Changes

- Define seven focused crates with dependencies pointing toward pure analysis
  policy and one project crate composing infrastructure.
- Keep CLI behavior and JSON schema as the supported first-release interfaces
  while tracking the smaller Rust APIs used between workspace crates.
- Define a flat report model, package-root identity, single-owner source facts,
  and allocation rules for analysis, aggregation, comparison, and output.
- Place Rayon scheduling only in project orchestration and require deterministic
  serial and parallel results.
- Define the verified upstream language set, owned Ruby and Vue analyzers, and
  usage-led replacement of upstream language implementations without a fork.
- Add evidence-led performance gates for filesystem reads, Git processes,
  allocations, memory growth, and representative repository workloads.
- Keep all crates private until a separate release change authorizes publishing.

## Capabilities

### New Capabilities

- `workspace-architecture`: Defines crate responsibilities, dependency
  direction, composition, and supported interfaces.
- `language-analysis`: Defines compiled language dispatch, verified initial
  language support, Vue and Ruby behavior, and upstream replacement.
- `analysis-performance`: Defines scheduling, allocation, memory, process, and
  benchmark requirements.

### Modified Capabilities

- `architecture-documentation`: Extends the architecture contract with the
  agreed crate map, flat data flow, package identity, and enforcement rules.
- `product-documentation`: Updates support and installation documentation to
  match private development and the verified initial language set.

## Impact

The later implementation will replace the single package with a seven-crate
workspace, pin `rust-code-analysis` behind one private adapter, add Rayon and
tree-sitter language dependencies, and add dependency, API, acceptance, and
performance checks. This change itself only adds OpenSpec planning artifacts;
it does not change Rust code, the README, or the architecture document.
