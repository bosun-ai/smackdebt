## Context

The seven crates already follow durable product responsibilities, but their
implementations are single files and their seams expose too much assembly
state. The analysis crate has the widest surface: project code assigns report
indexes, mutates scopes, runs aggregation, and clones tables to satisfy those
interfaces. The current API snapshot script only scans declarations in
`lib.rs`, so it will stop seeing exports after modules are introduced.

The change must preserve command behavior, JSON schema version 1, stable serial
and parallel output, single-pass discovery, limited source reads, and fixed Git
process counts.

## Goals / Non-Goals

**Goals:**

- Keep the existing crates and make each entry file mechanical.
- Give each private module one clear responsibility.
- Reduce cross-crate interfaces to behavior needed by direct consumers.
- Make report construction and aggregation owned by analysis.
- Remove repeated report path ownership and avoid full-table clones.
- Enforce entry-file shape and the actual reachable Rust API.

**Non-Goals:**

- Adding or merging crates.
- Adding traits, plugins, async work, or replacement data models.
- Redesigning terminal output or JSON schema version 1.
- Publishing any crate.

## Decisions

### Keep seven responsibility-led crates

Analysis remains the pure domain, languages owns parsing, discovery owns the
filesystem inventory, Git owns repository access, project composes use cases,
output writes borrowed reports, and the CLI owns process policy. Splitting
report or comparison into new crates would widen interfaces; merging crates
would mix policy with infrastructure.

Discovery will use the analysis-owned `PackageId`. This activates the already
allowed discovery-to-analysis edge and removes a duplicate identity without a
mapping layer.

### Make entry files mechanical

`lib.rs` and `mod.rs` may contain attributes, documentation, private module
declarations, imports, and explicit reexports. They may not contain behavior,
types, implementations, values, macros, inline modules, tests, or glob
reexports. `main.rs` remains the executable composition root and contains only
high-level orchestration.

Private modules use ordinary Rust visibility. `pub` is reserved for direct
crate consumers, while `pub(crate)` and `pub(super)` connect implementation
modules.

### Let analysis finish reports

A small public `ReportBuilder` owns report table insertion, links, path
interning, and final aggregation. Project owns I/O, scheduling, health
application, and its shared codebase/diff hierarchy policy, but never edits a
`Report` after construction. Keeping use-case-specific assembly in project
avoids a broad input model that would only mirror project results.

A completed `Report` is read-only and always has a root. Initial selection
belongs to `ProjectReport`. This removes assembly methods from the read model
and avoids cloning file or comparison tables before aggregation.

### Store report paths once

Scopes and files store `PathId`; the report owns strings. Renderers resolve
paths through the report. JSON schema version 1 still emits both its existing
string fields and path indexes, deriving both from the one path table.

### Put completed change merging in Git

Git combines base changes with worktree status and returns sorted, unique
changes. Project consumes source presence and base/current paths rather than
status codes or process plumbing.

### Check reachable APIs

The architecture gate uses `cargo-public-api` 0.52.0 with derived and blanket
noise omitted. Checked snapshots represent what downstream crates can reach
after reexports. The gate also enables `unreachable_pub` so accidental exports
inside private modules fail compilation.

## Risks / Trade-offs

- **Large mechanical move obscures behavior changes** → Move one crate at a
  time, preserve tests, and validate each seam before continuing.
- **A smaller API can expose missing ownership** → Move behavior to the owning
  value instead of adding pass-through helpers.
- **Path interning can change output** → Characterize terminal and JSON output
  and resolve existing fields from the shared table.
- **A new API tool can be unavailable** → Pin and check its version with a clear
  setup error.
- **Report assembly can affect hot paths** → Retain allocation checks and rerun
  the existing performance workloads.

## Migration Plan

1. Add characterization tests and architecture checks without enabling the
   entry-file rule.
2. Refactor analysis and languages, then validate their seam.
3. Refactor discovery and Git, then move project to the new interfaces.
4. Refactor output and the CLI.
5. Enable entry-file and reachable-API checks, update snapshots, docs, and
   OpenSpec tasks.
6. Run complete correctness, architecture, license, and performance gates.

Internal APIs are private and may change together. Rollback is a source revert;
there is no user data migration.

## Open Questions

None.
