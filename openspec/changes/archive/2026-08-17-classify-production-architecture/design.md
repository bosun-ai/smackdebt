## Context

Architecture verdicts are built in `crates/project` from the relations that
`crates/languages` extracts and `crates/analysis` types. Today a single
predicate, `DependencyEdge::affects_verdict()`
(`crates/analysis/src/architecture.rs`), decides both what is *evidence*
and what may *produce a verdict*: it accepts trusted parsed `uses` from primary,
test, example, and benchmark source. Every architecture graph is built from that
one set.

Two facts break that arrangement on real Rust repositories.

1. A test is allowed to depend on things production code does not. tokio's
   `tokio ↔ tokio-stream` High cycle and its four stable-dependency findings are
   built entirely from `[dev-dependencies]` imports.
2. SourceRole is assigned from the file's path, so `#[cfg(test)] mod tests`
   inside `tokio/src/**.rs` is primary role. Role alone therefore does not
   separate the code that ships from the code that tests it.

Separately, Rust's module system makes a parent and its child import each other
by construction — `pub use child::Item;` down, `use super::*;` up — over a pair
the parent already owns through `mod child;`. tokio's `fs/` and `io/` report
FileCycle findings for exactly this.

## Goals / Non-Goals

**Goals:**

- An architecture verdict describes the structure of code that ships.
- Non-production dependencies stay complete and inspectable as context, in JSON
  and in `--all`.
- Idiomatic Rust module wiring stops producing file-cycle findings, without
  hiding real cycles between modules.

**Non-Goals:**

- Reading `Cargo.toml` `[dev-dependencies]`, lockfiles, build configuration, or
  any resolver behavior.
- Changing ratings, signals, unit measurements, coverage partitioning, external
  dependency counts, or resolution diagnostics.
- Changing the report schema's shape.
- Hiding non-primary relations. They remain complete in the machine report.

## Decisions

### Predicate split: eligibility for evidence, and eligibility for a verdict

A new predicate `DependencyEdge::enters_verdict_graph()` is added, defined as
`Uses && Trusted && Primary`. The existing `affects_verdict()`
(`DependencyEdge::affects_verdict` in `crates/analysis/src/architecture.rs`) is **kept unchanged** as the
evidence-eligibility predicate — trusted parsed `uses` from primary, test,
example, and benchmark source — because two different questions are being asked
and one predicate cannot answer both.

Call-site disposition of the existing `affects_verdict()` uses:

| Call site | Predicate after this change | Why |
| --- | --- | --- |
| Package dependency edges, package cycle graph, file cycle graph, package-graph fan-in/fan-out/instability, stable-dependency comparison | `enters_verdict_graph()` | These are the verdicts. |
| Coverage partitioning (`DependencyPartitionCounts::record` in `crates/project/src/project.rs`) | `affects_verdict()`, unchanged | Coverage counts extracted references, not verdicts; a test import is still resolved-internal. |
| Coupling explanation | `affects_verdict()`, unchanged | See below. |
| Orphan fan-in | `affects_verdict()`, unchanged | See below. |
| Worst-offender eligibility filter (`crates/analysis/src/report.rs`) | unchanged | Unit findings, not architecture edges. |

Coupling explanation additionally needs the file-edge pairs it is allowed to
consult, which are no longer derivable from the verdict graph. A new
`explanation_pairs` set — file-edge pairs whose edge satisfies `affects_verdict()`
and whose endpoints are in different packages, unioned with the manifest-name
explanation pairs — is threaded through `ArchitectureBuild` into
`evolution::finish` / `unexplained_coupling` / `compare_evolution`. It must be
supplied at **both** call sites, including the diff's **before** side
(`analyze_diff` and the codebase path in `crates/project/src/project.rs`); omitting the before
side silently produces spurious Worse `FindingIntroduced` rows on
dev-dependency repositories.

### Rust `cfg(test)` scope, stated as an exact matching rule

A new `DependencyScope { Default, Test }` field is added to `DependencySyntax`
(`crates/analysis/src/source.rs`). Detection lives in
`crates/languages/src/dependency.rs::rust`: for the item declaring the reference
and for each ancestor `mod_item`, scan the immediately preceding
`attribute_item` siblings (outer attributes), and match when

- the attribute path is exactly `cfg`, and
- its token tree contains the identifier `test` at any depth that is **not**
  inside a token tree immediately following the identifier `not`.

The six decided example forms:

| Form | Matches |
| --- | --- |
| `#[cfg(test)]` | yes |
| `#[cfg(all(test, not(loom)))]` | yes |
| `#[cfg(any(test, fuzzing))]` | yes |
| `#[cfg(not(test))]` | no |
| `#[cfg(feature = "test")]` | no |
| `#[cfg_attr(test, ...)]` | no — the attribute path is `cfg_attr`, not `cfg` |

The rule is deliberately syntactic. It is not a `cfg` evaluator, it does not
know which features are enabled, and it does not need to: an item reachable only
when `test` is set is test code regardless of what else the predicate says.

**Trap:** `offset_dependency` in `crates/languages/src/engine.rs` rebuilds the
`DependencySyntax` struct. It must carry the scope through, or the scope is
silently dropped for every reference that is offset — this compiles and passes
type checks.

### Scope demotes a role through `max()`, and role stays evidence

The reference's role is `evidence_role(reference, deps) = deps.role.max(SourceRole::Test)`
when the reference is test-scoped, and `deps.role` otherwise. `SourceRole`'s
`Ord` places `Primary` before `Test`, so this maps `Primary → Test` and leaves
every non-primary role — test, example, benchmark, fixture, generated —
unchanged. A fixture file does not become test code because it contains a
`cfg(test)` module.

The rule is applied wherever a reference is recorded — `record_internal`,
`record_external`, `record_package`, and `record_diagnostic` in
`crates/project/src/project.rs` — so no path into the graph can bypass it.

Role remains a separate evidence field, as
`architecture-analysis` "Static relation kind is independent from evidence"
in `openspec/specs/architecture-analysis/spec.md` requires. Scope adds
**no** relation kind and no new column: it only decides which role an already
existing relation carries. A consequence is that one file pair may now emit two
edge rows, one Primary and one Test, where a primary file both imports a module
normally and imports it again inside a `cfg(test)` module. That is a value-level
change to `dependency_edges`, not a shape change, so report schema v4 needs no
delta — but every consumer that assumes `(source, target)` uniqueness must be
audited, `crates/analysis/src/architecture_comparison.rs` first.

### A Rust module file owns a directory, and resolution has to know it

The rule below cannot fire without this one. `tokio/src/fs/file.rs` declares
`#[cfg(test)] mod tests;` and the declared file is `tokio/src/fs/file/tests.rs`,
but resolution read `./tests.rs` against the declaring file's *directory* and
looked for `tokio/src/fs/tests.rs`. The declaration resolved to nothing, so no
`module_ownership` relation existed to carry the scope.

Resolution now reads a relative Rust candidate against the module directory the
declaring file owns — its own directory for `mod.rs`, `lib.rs`, and `main.rs`,
and a directory named after the file otherwise — and keeps the sibling reading
where that does not match. The same correction fixes `super::` inside a plain
module file, which previously climbed one directory too far. Field effect on the
five repositories: kwaak's unfollowed imports fall from 31 to 24, tokio's orphan
list loses 15 files that something does import, and fluyt gains one genuine file
cycle that was invisible while its `super::` references went unresolved.

### A file declared only under `#[cfg(test)]` is test source

Scope detection reads the item that declares a reference, so `#[cfg(test)] mod
tests;` makes the *declaration* test-scoped — but the declared file is a separate
file whose role comes from its path, and `tokio/src/fs/file/tests.rs` is not on
any test path. Field verification of the predicate split found tokio's package
cycle standing for exactly that reason: `tokio/src/fs/file.rs` declares
`#[cfg(test)] mod tests;`, and the declared file's `tokio-test` imports were
primary relations.

The rule closes it at classification rather than at the graph: a file is `test`
when it has at least one module declaration and every one of them is
test-scoped, where a declaration is test-scoped if its reference is, or if the
declaring file is itself test source by this rule. The recursion is a
monotone fixpoint over ordered sets, so it is deterministic and terminates.

Three fences keep it narrow.

- It only ever replaces the **primary fallback**. Configuration, language-owned
  generated markers, and generic path rules all keep precedence, so
  `source-signal-quality`'s precedence requirement stays true and gains one
  explicit step.
- **Any** non-test declaration keeps the file primary. A file that `main.rs`
  declares plainly and `lib.rs` declares under `#[cfg(test)]` ships.
- A file no one declares is untouched, so the rule cannot reach a language
  without module declarations.

It runs before the report builder reads a role, so one file carries one role in
findings, ratings, coverage, history evidence, and the graphs alike. The
alternative — filtering the declared file's edges inside `build_architecture` —
was rejected because it would leave the file labelled `primary` in the report
while its relations behaved like test relations.

### Why not parse `Cargo.toml` `[dev-dependencies]`

The obvious alternative is to read the manifest's `[dev-dependencies]` section
and exclude those package edges. It is rejected on two grounds.

- It is language-specific. Role-based classification already exists for every
  supported language, and the same defect (a test importing across a boundary)
  appears in Python, JavaScript, and Ruby repositories with no `Cargo.toml` to
  read.
- It solves only half of tokio. The `#[cfg(test)] mod tests` blocks inside
  `src/**.rs` are not covered by any manifest section; only scope detection
  catches them. Manifest parsing would leave the second half of the false
  findings standing while adding a build-configuration reader the product has
  explicitly refused to own.

### Ownership-pair exclusion is pairwise, not 2-cycle suppression

The file cycle graph excludes a `uses` edge between files A and B whenever a
`module_ownership` relation exists between the same unordered pair, in either
direction. The graph build and the witness lookup in `build_architecture`
(`crates/project/src/project.rs`) share one `enters_cycle_graph` predicate, so
reported witnesses and detected cycles cannot disagree.

The rejected alternative is to suppress two-node cycles between an owning pair.
It does not work: tokio's `fs/` is a single large strongly connected component
containing `mod.rs` and many children, so a 2-cycle rule leaves the finding
standing. The pairwise rule removes exactly the wiring edges and lets the
remaining graph speak.

**False negatives this accepts.** A genuine cycle between a parent module and
its own child — one that is not wiring but a real mutual dependency — is
suppressed. This is accepted: in Rust the parent already owns the child, the two
files are compiled as one module tree, and a reader cannot act on "this parent
depends on its child" as a structural finding. The exclusion is scoped as
narrowly as it can be:

- It applies only to the file cycle graph. Orphan fan-in is untouched, so a file
  that is only ever imported by its own parent is still not an orphan.
- Only the A–B edges are removed. Every other edge of the same component
  survives, so a cycle that merely *passes through* an owning pair still
  reports.

Field-verified survivors: opencode's three dialog cycles, scikit-learn's
`_config ↔ _array_api`, smackdebt's own `change_coupling ↔ evolution`, and
sibling-module cycles (siblings own no one, so no exclusion applies).

### Orphans keep the eligible-role graph

Orphan fan-in stays on trusted eligible uses of any role — primary, test,
example, benchmark. A primary file imported only by its tests is *used*; calling
it an orphan would be a fresh false positive introduced by fixing another one.
The accepted requirement's wording moves from "the verdict graph" to "trusted
eligible uses", which is what it always meant and what the code will now have to
say explicitly.

### Coupling explanation keeps non-primary edges

`no code dependency` is a claim about the repository, not about production code.
Two packages linked only by a dev-dependency test import do have a code
dependency, and reporting their co-change as *unexplained* coupling would be
false. Coupling explanation therefore stays on `affects_verdict()` via
`explanation_pairs`, and a scenario pins it so a later cleanup cannot quietly
narrow it to the verdict graph.

## Risks / Trade-offs

- **No committed-fixture coverage exists for any of these three behaviors, and
  SDP findings have zero committed snapshot presence at all.** All pinning power
  comes from newly authored golden bytes; they are reviewed as product artifacts,
  not regenerated on sight.
- **`offset_dependency` drops the scope silently.** A dedicated languages test
  covers a reference that is offset out of a `cfg(test)` module.
- **The diff before-side is easy to miss.** A diff acceptance expectation on a
  repository with dev-dependency edges proves no spurious Worse row appears.
- **Witness and graph can diverge.** The same predicate is used at both sites and
  a test asserts a suppressed pair produces neither a cycle nor a witness.
- **Fewer architecture findings can read as a regression.** The five-repo matrix
  records that every previously validated true positive survives.

## Migration Plan

1. Accept and strict-validate this change before implementation.
2. `cfg(test)` scope: languages unit tests for all six matching forms first, then
   the `DependencyScope` field, detection, `offset_dependency` preservation, and
   the `evidence_role` `max()` flow. Add `cfg(test)` content to a generated
   fixture so JSON pins `"role":"test"`. No committed-snapshot churn expected.
3. Primary-only verdict graphs plus `explanation_pairs` plumbing at both call
   sites. New fixtures: a test-edge-driven would-be package cycle that yields no
   finding while its edges stay in JSON and its coupling stays explained; a
   primary stable-dependency fixture that authors the product's first SDP
   terminal and JSON evidence.
4. Ownership-pair cycle exclusion in the graph and the witness lookup, extending
   `rust_module_ownership_cycle_is_context_while_mutual_uses_are_a_verdict`
   (`crates/project/src/project.rs`) with a suppressed parent↔child case, a
   surviving sibling cycle, and a surviving mutual-uses pair.
5. README and `ARCHITECTURE.md`, then the five-repository verification matrix and
   a serial/parallel byte-identity check on tokio JSON.
6. Archive this change.

Rollback restores the single `affects_verdict()` predicate everywhere, drops the
scope field, and restores the previous graph inputs. No stored artifact requires
migration; reports are recomputed on every run.
