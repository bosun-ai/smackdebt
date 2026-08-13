## Why

The unified analysis works, but its signal still mixes source with different
roles, treats parser recovery too much like trusted syntax, mistakes Rust module
ownership for dependency use, and promotes weak history observations. Package
identity and root labels also need stable user and machine contracts.

This change sharpens the answer to whether code and architecture need attention
without adding another analysis path.

## What Changes

- Classify every selected file as primary, test, example, benchmark, fixture,
  or generated with explicit precedence and conflict behavior.
- Keep recovered measurements and dependency context as advisory evidence in
  JSON and `--all`, while excluding them from health, default output,
  architecture verdicts, and diff verdicts.
- Give every package a stable table row and ID, including empty and base-only
  packages, and display machine path `.` as `repository root` in terminal output.
- Model static relations as `uses` or `module_ownership`, with trust and source
  role as separate evidence fields.
- Create coupling findings only at three shared commits and 20% Jaccard or
  higher; retain weaker observations in JSON and `--all`.
- Publish JSON schema version 3 with roles, trust, package identity, static
  relations, history operands, and advisory facts.
- Apply one exact finding rank and remove arbitrary dependency-edge rows from
  default architecture output.
- Prove the decisions with public fixtures, three reviewed workload families,
  and clean evidence recorded after the reviewed implementation commit.

## Capabilities

### New Capabilities

- `source-signal-quality`: Defines source roles, precedence, conflicts, and
  recovery trust.
- `report-schema-v3`: Defines the revised machine report contract.

### Modified Capabilities

- `workspace-architecture`: Stabilizes package IDs and package path ownership.
- `architecture-analysis`: Separates relation kind from role and trust.
- `evolutionary-analysis`: Defines exact history fields, visibility, and the
  stronger coupling rule.
- `progressive-exploration`: Defines ranking, root labels, architecture detail,
  and default de-duplication.
- `product-documentation`: Explains the revised signals and JSON version 3.
- `architecture-documentation`: Records ownership at the actual crate seams.
- `end-to-end-evidence`: Extends exact public and reviewed-workload proof.
- `analysis-performance`: Measures the complete revised flow.
- `release-readiness`: Requires post-commit clean release evidence.

## Impact

Default verdict counts and ordering can change. Fixture and generated source no
longer affect a default verdict, weak coupling no longer appears by default,
and module ownership no longer creates dependency cycles. Advisory facts remain
available for inspection. JSON moves from version 2 to version 3 before the
first release, with no compatibility selector. All crates remain private.
