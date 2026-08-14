## 1. SourceRole and recovery trust

- [x] 1.1 Add primary, test, example, benchmark, fixture, and generated roles and prove verdict participation for each.
- [x] 1.2 Implement precedence from explicit configuration through language-generated markers, generic filename and path rules, and primary fallback.
- [x] 1.3 Reject same-level role conflicts with exit 2, exact stderr, and empty stdout.
- [x] 1.4 Remove the blanket generated-directory ignore while preserving Git, user, and dependency ignores.
- [x] 1.5 Retain recovered units and dependency context as advisory JSON and `--all` facts while excluding them from health, default, architecture verdicts, coupling, and diff verdicts.

## 2. Stable package identity and root labels

- [x] 2.1 Build package rows before source filtering and assign stable typed IDs in repository-relative order.
- [x] 2.2 Preserve empty packages and append base-only packages without renumbering current rows across codebase, path, and diff views.
- [x] 2.3 Keep machine root path `.` and render it as `repository root` in every terminal context.

## 3. Static relation meaning

- [x] 3.1 Model relation kind as `uses` or `module_ownership` with SourceRole, trust, resolution, span, and count as separate evidence.
- [x] 3.2 Implement Rust external-module ownership, inline-module, import, qualified-path, and safely resolved macro-path behavior in the Rust language implementation.
- [x] 3.3 Drive verdict graphs only from eligible parsed uses and prove ownership, advisory, fixture, and generated relations cannot create health or coupling explanations.
- [x] 3.4 Preserve relation kind and evidence independently in ref and worktree diffs.

## 4. History fields, coupling, and de-duplication

- [x] 4.1 Preserve exact file, package, coverage, coupling, and contributor-concentration fields named by the specification, including SourceRole for source-derived observations.
- [x] 4.2 Require three shared commits, at least 20% Jaccard, sufficient history, and no eligible trusted use for an unexplained coupling finding.
- [x] 4.3 Retain weaker coupling in JSON and `--all` and prove threshold edges, static explanations, shallow history, and fixture/generated exclusion.
- [x] 4.4 Remove repeated package history, coupling, concentration, and operand rows from default terminal sections.

## 5. JSON version 3

- [x] 5.1 Replace version 2 with flat version-3 tables for packages, roles, trust, advisory source facts, relations, exact history, findings, comparisons, coverage, and diagnostics.
- [x] 5.2 Add the checked version-3 schema and semantic index validation for empty and base-only packages, machine root `.`, advisory links, and every fact type.
- [x] 5.3 Prove codebase, clean ref-diff, and mixed worktree-diff JSON against schema, semantics, indexes, privacy, and exact reviewed bytes from each measured invocation.

## 6. Rank, terminal output, and documentation

- [x] 6.1 Implement the exact rank: rating, signals at that rating, total triggered signals, cognitive, cyclomatic, lines, activity, path, and span.
- [x] 6.2 Show unit kind and non-primary role, keep recovered findings advisory in `--all`, and remove arbitrary edge rows from default architecture while retaining witnesses.
- [x] 6.3 Make `--all` and path drill show relevant ordinary, ownership, advisory, unresolved, and ambiguous relations without duplicate rows.
- [x] 6.4 Update README executable examples and architecture documentation for roles, precedence, conflicts, recovery, relation meaning, history fields, coupling, rank, root labels, JSON version 3, privacy, and limits.

## 7. Public end-to-end and resource proof

- [x] 7.1 Extend generated fixtures for all six roles, every precedence level, conflict exit 2, recovery, failure, packages, Rust relations, exact history, 20% boundaries, rank ties, root labels, witnesses, detailed edges, and de-duplication.
- [x] 7.2 Assert exact status, stdout, stderr, semantics, schema, indexes, privacy, and bytes for codebase, path, clean committed ref-diff, and mixed worktree-diff flows.
- [x] 7.3 Use feature-gated live seam counters and prove real post-snapshot work changes totals while normal builds expose no evidence-only cost or API.
- [x] 7.4 Run every named public flow serially and automatically, proving exact work totals and byte-for-byte equality.

## 8. Three reviews and post-commit release evidence

- [x] 8.1 Pass public semantic, schema, byte, privacy, allocation, performance, API, architecture, license, and workspace gates before real-workload review.
- [x] 8.2 Review self: fixture cycles gone, package references valid, root labels readable, and every default coupling above threshold.
- [x] 8.3 Review the private mixed application: generated schema and client findings excluded, weak coupling and ownership cycles gone, substantial hand-written functions prominent, and every default coupling above threshold.
- [x] 8.4 Review the private Rust workspace: ownership cycles gone, real high-complexity functions visible, and every default coupling above threshold.
- [x] 8.5 Commit only privacy-safe aggregate workload families and outcome categories, with no private path, source, Git identity, or history.
- [x] 8.6 Create the reviewed implementation commit before recording release evidence.
- [x] 8.7 From clean reviewed HEAD, record every public profile and all three aggregate reviews with one workflow and require one shared revision plus `workspace_dirty` false.
- [x] 8.8 Run formatting, Clippy, workspace tests, default and all-feature API snapshots, strict OpenSpec validation, and final diff check before completion.
