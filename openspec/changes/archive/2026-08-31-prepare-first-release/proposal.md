## Why

Smackdebt crates are private while the product contract and measured workloads
settle. Publication needs its own review so registry metadata cannot drift from
the tested CLI and JSON behavior.

This change stays in flight and blocked. The three terminal changes that
previously blocked it (`make-terminal-verdict-clear`,
`make-terminal-detail-relevant`, `make-diff-output-debt-focused`) were archived
without merging their deltas and are superseded by the v-next set below.

## What Changes

- Verify command, JSON, license, provenance, and installation behavior for the
  first public release.
- Replace private crate settings only after release evidence passes.
- Publish libraries in dependency order and the CLI last.

## Capabilities

### New Capabilities

- `release-readiness`: Defines evidence and ordering for a future registry
  release.

## Dependency Order

This change is blocked until every v-next change is implemented, reviewed, and
archived in this order:

1. `resolve-workspace-dependencies`
2. `deepen-debt-signals`
3. `add-verdict-policy`
4. `redesign-terminal-report`
5. `adopt-report-schema-v4`

Publication evidence is recorded only after the fifth change is archived, which
also satisfies the version-3 retirement requirement through schema version 4.

## Impact

This proposal does not enable publication. All crates remain private until its
implementation tasks are approved and completed.
