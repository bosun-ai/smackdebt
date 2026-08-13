## ADDED Requirements

### Requirement: Terminal presentation separates selection from layout
The output crate SHALL build private borrowed presentation rows from one report
before choosing a width layout. Ranking, omission, and navigation decisions
SHALL occur once, and renderers SHALL not run analysis or inspect filesystem,
Git, environment, or terminal state.

#### Scenario: Several terminal widths render one report
- **WHEN** full, compact, and stacked layouts render the same completed report
- **THEN** they consume the same selected presentation rows and differ only in layout

#### Scenario: A later renderer needs progressive rows
- **WHEN** another in-process terminal renderer is introduced
- **THEN** the private presentation boundary can be reused without moving terminal state into the report domain
