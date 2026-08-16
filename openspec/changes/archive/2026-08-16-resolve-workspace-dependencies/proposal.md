## Why

Running the release binary against smackdebt, swiftide, and fluyt on 2026-08-15
showed that workspace dependency resolution is the largest analysis correctness
gap in the product.

- `use smackdebt_analysis::…` is classified `external` because the declared
  crate name does not match its directory `crates/analysis`. On every Rust or
  JavaScript workspace — the primary audience — the package dependency graph is
  effectively empty, so package cycles, fan-in, fan-out, and instability cannot
  be detected.
- Because no package edge exists, every `HISTORY` coupling finding claims `no
  code dependency` falsely. Smackdebt's own report states `crates/analysis ↔
  crates/project · 82% · no code dependency` while the same report lists
  `project.rs → smackdebt_analysis · external`.
- Unmatched-import warnings drown the report: 29 on smackdebt, 133 on swiftide,
  300 on fluyt.
- `--all` prints the same package pair three times with contradictory numbers
  (`crates/cli ↔ crates/project` at 62%, 60%, and 67%) because role and trust
  variants render indistinguishably, and it couples ancestor with descendant
  scopes (`repository root ↔ crates/project`), which cannot mean anything.
- Rust extraction emits `lib.rs → pub · external` as a dependency row: a
  visibility keyword became a dependency target.

This change supersedes the analysis-correctness parts of the retired
`make-terminal-verdict-clear`, `make-terminal-detail-relevant`, and
`make-diff-output-debt-focused` changes, which were archived without merging
their deltas. Those changes assumed the graph was already correct and only its
presentation was wrong.

## What Changes

- Discovery extracts the declared package name from a recognized manifest —
  `Cargo.toml` `[package] name` with a `[lib] name` override, `package.json`
  `name` including `@scope/name`, `pyproject.toml` `[project] name`, and gemspec
  name — and stores it on the package record.
- Resolution normalizes an unresolved reference's first segment (Rust hyphen and
  underscore equivalence) against the manifest-name index before classifying it
  external. A unique internal match becomes an internal `uses` edge; the
  file-level target is the package entry file when it resolves, otherwise the
  edge is package-scoped.
- The exactly-one-match rule is unchanged: a manifest name that maps to more
  than one internal package stays unresolved or ambiguous, with its diagnostic
  retained.
- Package change coupling emits one row per unordered package pair instead of
  one row per role and trust key, and ancestor-descendant scope pairs never form
  a coupling pair.
- Rust dependency extraction stops emitting visibility keywords as dependency
  targets.
- JSON version 3 shape is unchanged. Values move (fewer external and unmatched
  references, more internal edges), so terminal and JSON evidence regenerates
  with review.

## Capabilities

### Modified Capabilities

- `architecture-analysis`: Adds manifest-name resolution of cross-package
  references and states how ambiguity and shadowing stay diagnostics.
- `evolutionary-analysis`: One coupling row per package pair and no
  ancestor-descendant pairs.
- `language-analysis`: Dependency extraction emits dependency targets only.
- `product-documentation`: README explains workspace package resolution and the
  meaning of `no code dependency`.
- `end-to-end-evidence`: Generated workspace fixtures prove internal resolution
  for every supported manifest kind.
- `release-readiness`: Keeps publication blocked through the five v-next
  changes.

## Dependency Order

The v-next set is implemented, reviewed, and archived in this order:

1. `resolve-workspace-dependencies` (this change)
2. `deepen-debt-signals`
3. `add-verdict-policy`
4. `redesign-terminal-report`
5. `adopt-report-schema-v4`

`prepare-first-release` stays blocked until all five are archived.

## Impact

This changes analysis correctness, dependency and coupling values, warning
counts, and the exact terminal and JSON evidence that depends on them. It
affects discovery manifest recognition, project resolution, change coupling,
Rust dependency syntax, fixtures, and reviewed snapshots. It does not change
JSON version 3 structure, health thresholds, finding rank, terminal section
policy, the verdict, or the report schema; those belong to the four later
changes.
