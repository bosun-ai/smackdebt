# Change: Prove unified analysis end to end

## Why

The new source, static architecture, and evolutionary contracts cross every
crate and alter both public output formats. Unit and crate tests cannot prove
that the installed CLI composes those parts correctly, preserves stream and
exit behavior, or produces deterministic reports. The first release needs
strong black-box evidence tied to documented user flows.

## What Changes

- Add generated repository fixtures for all supported languages, static
  architecture, history, worktree comparisons, and recoverable failures.
- Run the built CLI as a real child process and capture status, stdout, and
  stderr.
- Commit exact terminal and JSON version-2 results and validate JSON against its
  schema before snapshot comparison.
- Prove identical serial and parallel bytes, stable narrow output, and color
  parity.
- Prove default, detailed, path-selected, codebase, diff, diagnostic, and exit
  behavior.
- Install the CLI into a temporary prefix and run it outside the workspace.
- Execute README command examples against public generated fixtures.
- Make the unified acceptance matrix required release evidence.

## Capabilities

### New Capabilities

- `end-to-end-evidence`: Black-box repository fixtures, exact output evidence,
  failure contracts, and installed-command proof.

### Modified Capabilities

- `analysis-performance`: Require serial and parallel output identity in the
  complete CLI flow.
- `report-schema-v2`: Require schema validation and committed JSON examples.
- `product-documentation`: Keep executable README examples aligned with CLI
  behavior.
- `architecture-documentation`: Document the acceptance seam and evidence
  ownership.
- `release-readiness`: Require the unified analysis evidence before publication.

## Impact

- Depends on `replace-source-analysis-engine`,
  `add-static-architecture-analysis`, and
  `add-evolutionary-architecture-analysis`.
- Primarily changes black-box tests, generated fixtures, snapshots, validation
  commands, README examples, and release checks.
- May expose integration defects in any crate, but fixes remain owned by the
  crate responsible for the broken rule.
- Does not publish a crate, change a remote system, or add private source to
  fixtures.

