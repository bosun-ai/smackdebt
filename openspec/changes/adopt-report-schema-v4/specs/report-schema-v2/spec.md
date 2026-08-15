## REMOVED Requirements

### Requirement: JSON version 2 includes evolutionary tables
**Reason**: The CLI has not presented version 2 since version 3; the capability
is stale.
**Migration**: `report-schema-v4` owns the evolutionary tables.

### Requirement: JSON records the history window and coverage
**Reason**: Stale version-2 statement of a contract version 4 now owns.
**Migration**: `report-schema-v4` requires coverage fields including the
selected window length and window-excluded commit count.

### Requirement: Version-2 examples are executable evidence
**Reason**: No version-2 example remains; version-4 examples are the evidence.
**Migration**: `report-schema-v4` requires checked version-4 examples.

### Requirement: JSON and terminal consume the same analysis result
**Reason**: Stated for version 2; the rule itself survives in workspace and
performance capabilities that require renderers to perform presentation work
only.
**Migration**: No behavior is lost; `analysis-performance` and
`architecture-documentation` retain the one-analysis, two-renderer rule.
