## Why

The 2026-08-17 field evaluation on five real repositories — tokio,
scikit-learn, opencode, kwaak, and fluyt — found that tokio's entire
ARCHITECTURE section is an artifact of test code and of idiomatic Rust module
wiring. None of it is architecture debt, and it is printed with the same
authority as the findings that are.

- **Dev-dependency imports manufacture architecture verdicts.** tokio reports a
  High `package dependency cycle` between `tokio` and `tokio-stream` and four
  stable-dependency findings. Every edge behind them comes from
  `[dev-dependencies]` imports — files under `tests/` and `#[cfg(test)]` modules
  inside `src/`. A test importing a sibling crate is exactly what a test is for;
  it says nothing about how the shipped crates depend on each other.
- **The `#[cfg(test)]` half is invisible to role classification.** SourceRole is
  assigned from the file's path, so an inline `#[cfg(test)] mod tests` inside a
  primary `src/` file contributes primary-role edges. Restricting verdict graphs
  by role alone would not remove those edges.
- **`mod.rs ↔ child` file cycles are idiomatic Rust flagged as debt.** tokio's
  `fs/` and `io/` report FileCycle findings built from `pub use child::X` going
  down and `use super::*` coming back up, across files the parent already owns
  through a `mod` declaration. The declaration and the imports that accompany it
  are one wiring relationship, not a cycle.

The underlying relations are correctly extracted in every case. What is wrong is
that relations which describe how a repository is tested and how its modules are
wired are allowed to produce verdicts about how its production code is
structured.

## What Changes

- Architecture verdict graphs — package dependency edges, the package and file
  dependency cycle graphs, the package graph's fan-in, fan-out and instability
  measurements, and the stable-dependency comparison built on them — use trusted
  parsed `uses` relations whose source role is primary. Trusted `uses` from test,
  example, and benchmark source stay complete in the machine report as context.
- Rust references declared under a `#[cfg(test)]` scope carry the test role, even
  when their file is primary. The matching rule is stated exactly, so
  `cfg(all(test, not(loom)))` matches while `cfg(not(test))` and
  `cfg(feature = "test")` do not. Scope stays evidence: no new relation kind, and
  one file pair may now carry both a primary and a test relation.
- The file dependency cycle graph excludes a `uses` edge between two files when a
  `module_ownership` relation exists between the same unordered pair in either
  direction. The exclusion is pairwise, so sibling-module cycles and mutual uses
  between unrelated files survive, and cycle witnesses use the identical
  predicate.
- Change coupling keeps explaining pairs with trusted eligible uses of any role.
  A dev-dependency test import still means the two packages have a code
  dependency, so it must not become an unexplained-coupling Watch finding.
- Orphan facts keep using trusted eligible uses for fan-in: a primary file
  imported only by tests is used, and is not an orphan.
- The README explains primary-only verdict graphs, `cfg(test)` demotion, and
  module-wiring cycle suppression.

## Capabilities

### Modified Capabilities

- `architecture-analysis`: verdict graphs are primary-role only, the file cycle
  graph excludes owning-pair edges, Rust `cfg(test)` scope assigns the test role,
  and orphan fan-in is stated as trusted eligible uses.
- `evolutionary-analysis`: a test-role dependency still explains package change
  coupling.
- `product-documentation`: the README explains what does and does not enter an
  architecture verdict.

## Impact

This changes which architecture findings exist on repositories whose test code
crosses package boundaries or whose Rust modules import their own children:
package cycles, file cycles, and stable-dependency findings disappear where the
only supporting edges are test-role or owning-pair edges, and the architecture
verdict tier moves with them. Package instability, fan-in, and fan-out values
change on those repositories. It affects `crates/languages` dependency syntax,
`crates/analysis` source and architecture types, the `crates/project` graph
pipeline and its coupling-explanation inputs on both diff sides, and the README.

The report schema needs **no delta**: the change is value-level only. Edge rows
keep their shape; their `role` values change, and one file pair may emit a
primary row and a test row where it previously emitted one row. Ratings,
signals, and unit measurements are untouched. Coverage partitioning, external
dependency counts, and resolution diagnostics are unchanged.
`prepare-first-release` stays blocked.
