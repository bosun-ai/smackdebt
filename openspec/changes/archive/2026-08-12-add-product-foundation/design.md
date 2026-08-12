## Context

The repository starts with a single hello-world binary. `rust-code-analysis` already returns per-file and nested source metrics, but its CLI does not produce repository summaries, package and directory aggregation, hotspot priority, or ref comparisons. Some upstream metric values also require care during aggregation because parent spaces contain children and not every metric supports cross-file merging.

This change defines the product and architecture before implementation. Later changes can use these documents as a shared contract.

## Goals / Non-Goals

**Goals:**

- Make the two user questions and minimal command forms clear.
- Define concise progressive reports for people and JSON consumers.
- Separate source facts from health policy and infrastructure.
- Record metric, Git, aggregation, performance, failure, and privacy behavior.
- Give later OpenSpec changes clear delivery seams.

**Non-Goals:**

- Implement analysis behavior in this change.
- Select every Rust dependency beyond the reviewed source-engine revision.
- Add a policy gate, terminal UI, web report, or server.
- Promise languages whose metric behavior has not been verified.

## Decisions

### Document the intended product now

The README presents the complete intended commands and output. OpenSpec records implementation state. Restricting the README to the hello-world binary would leave no product contract for the first implementation changes.

### Use separate rated signals

The docs publish fixed cognitive complexity, cyclomatic complexity, and function-size limits. Scope summaries count ratings. They do not compress different measurements into one score. This keeps every finding explainable and avoids calibration claims that the project has not tested.

### Use activity only for hotspot priority

Recent Git touches order code that already needs attention. The default 90-day window is configurable and has no effect on static ratings or which refs diff mode compares.

### Use one report domain

Terminal and JSON views consume the same `Report`. Analysis and rendering remain separate, and machine output cannot drift into a different interpretation of health.

### Isolate the source engine

`rust-code-analysis` sits behind `LanguageAnalyzer`. The rest of Smackdebt does not depend on upstream syntax or metric types. A future Go analyzer can prove this interface without changing health, Git, aggregation, or presentation code.

### Split implementation by user-visible flow

Codebase reports come first, Git hotspots second, and ref comparison third. Each change ends with useful behavior and can revise the written contract through a reviewed OpenSpec delta if implementation evidence exposes a problem.

## Risks / Trade-offs

- **README commands precede implementation**: OpenSpec shows delivery state, while acceptance snapshots will keep implemented output aligned with the examples.
- **Fixed limits vary by team and language**: Configuration can replace them, and reports expose raw measurements.
- **The upstream metric tree can duplicate nested work**: The architecture requires exclusive additive values and fixture tests for nested units.
- **Symbol identity can be unclear**: Diff mode falls back to file-level results and reports the reason instead of guessing.
- **Git history can cost time on large repositories**: The adapter streams one history process and batches base objects.

## Migration Plan

1. Merge the product documents and this OpenSpec change.
2. Implement the listed changes in delivery order.
3. Turn README examples into acceptance snapshots as each command becomes available.
4. Update docs and specs together when verified behavior changes.

No rollback or data migration is needed because this change adds documentation only.

## Open Questions

None for this change.
