## ADDED Requirements

### Requirement: Reports expose evolution as a separate concern

The system SHALL present history coverage, churn, unexplained coupling, and
contributor concentration separately from code health and static architecture.

#### Scenario: Default codebase output has all analysis families

- **WHEN** a repository contains source findings, a dependency cycle, and
  retained history
- **THEN** the default report has separate code, architecture, and evolution
  summaries
- **AND** no combined score hides the individual results

#### Scenario: A user selects an evolution detail target

- **WHEN** a package is selected for detailed output
- **THEN** its file and package churn, coupling relationships, concentration,
  and history coverage are shown
- **AND** unrelated history regions are omitted from terminal presentation

