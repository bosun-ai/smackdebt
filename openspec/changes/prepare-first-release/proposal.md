## Why

Smackdebt crates are private while the product contract and measured workloads
settle. Publication needs its own review so registry metadata cannot drift from
the tested CLI and JSON behavior.

## What Changes

- Verify command, JSON, license, provenance, and installation behavior for the
  first public release.
- Replace private crate settings only after release evidence passes.
- Publish libraries in dependency order and the CLI last.

## Capabilities

### New Capabilities

- `release-readiness`: Defines evidence and ordering for a future registry
  release.

## Impact

This proposal does not enable publication. All crates remain private until its
implementation tasks are approved and completed.
