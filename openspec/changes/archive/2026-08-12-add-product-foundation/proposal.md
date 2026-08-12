## Why

`rust-code-analysis` exposes useful source metrics, but its command-line tool leaves repository discovery, aggregation, prioritization, and Git comparison to the user. Smackdebt needs a clear product contract before implementation so each later change contributes to the same two questions and the same concise report.

## What Changes

- Add a product README with the intended commands, report examples, ratings, discovery behavior, language support, and configuration.
- Add an architecture document that separates source analysis, Git access, health policy, aggregation, and presentation.
- Define the stable terms and interfaces that later OpenSpec changes will implement.
- Split implementation into small changes for codebase reports, Git hotspots, ref comparison, and language expansion.

## Capabilities

### New Capabilities

- `product-documentation`: Describes what Smackdebt does, how people run it, and how to read its reports.
- `architecture-documentation`: Defines the domain model, component responsibilities, data flow, performance rules, and extension points.

### Modified Capabilities

None.

## Impact

This change adds `README.md`, `ARCHITECTURE.md`, and repo-local OpenSpec artifacts. It does not change the current binary. Later changes will add dependencies and implement the documented behavior.
