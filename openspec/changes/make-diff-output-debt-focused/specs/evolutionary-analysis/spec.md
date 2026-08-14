## MODIFIED Requirements

### Requirement: Diff reports use history only as context
The system SHALL retain complete history facts for changed files and packages
without treating unchanged values as before-and-after measurements. Human diff
history SHALL link only introduced actionable evolution findings as Worse and
removed actionable evolution findings as Better. Unchanged findings, Changed
context, weak or explained observations, activity, concentration, coverage, and
processing facts SHALL remain report and JSON only.
Evolution finding IDs SHALL remain in their own `DebtDiffSelection` list and
SHALL NOT contribute to source `DebtDiffCounts`, `QUALITY`, `AREAS`, or discover
guidance. A history-only result SHALL omit those source sections and show
`HISTORY` independently.

#### Scenario: Worktree introduces unexplained coupling
- **WHEN** an actionable evolution finding is introduced
- **THEN** human `HISTORY` shows it once as Worse with its commit and dependency evidence

#### Scenario: Worktree adds an explaining static edge
- **WHEN** an existing actionable coupling finding is removed
- **THEN** human `HISTORY` shows it once as Better and JSON retains unchanged historical values

#### Scenario: High-churn package changes without a finding transition
- **WHEN** diff analysis retains activity context but introduces or removes no actionable finding
- **THEN** human `HISTORY` is absent and JSON keeps the context

#### Scenario: Only a history finding changes
- **WHEN** the source and architecture selection lists are empty
- **THEN** `HISTORY` appears without source `QUALITY`, `AREAS`, `FINDINGS`, discover guidance, or a combined count
