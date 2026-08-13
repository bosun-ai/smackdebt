## Why

Smackdebt's crate boundaries are sound, but each crate keeps almost all behavior
in its entry file and several cross-crate interfaces expose construction and
process details. This makes changes harder to place, lets public surfaces grow,
and creates avoidable report cloning.

## What Changes

- Keep the existing seven crates and responsibility-led dependency graph.
- Restrict `lib.rs` and `mod.rs` files to module wiring, imports, and explicit
  reexports, with an automated architecture check.
- Split each crate into private responsibility-led modules.
- **BREAKING**: Replace wide internal Rust APIs with small interfaces owned by
  report construction, language analysis, discovery, Git access, and project
  requests.
- Store each report path once while preserving command behavior and JSON schema
  version 1.
- Check the actual reachable Rust API instead of scanning declarations in entry
  files.

## Capabilities

### New Capabilities

- `module-architecture`: Defines entry-file rules and automated enforcement for
  private implementation modules.

### Modified Capabilities

- `workspace-architecture`: Keeps the seven crates while tightening their
  responsibilities, interfaces, dependency graph, and report ownership.
- `architecture-documentation`: Documents module placement and the smaller
  cross-crate seams.

## Impact

All workspace crates, architecture checks, API snapshots, and architecture
documentation change. Internal Rust interfaces may break before release, while
the CLI, exit codes, standard streams, terminal facts, and JSON schema version
1 remain compatible.
