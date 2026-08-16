## ADDED Requirements

### Requirement: Every selected scope has its own verdict
Analysis SHALL complete the root verdict while the report is built and SHALL
expose a pure function that produces the verdict for any other selected scope
from the completed report. A scope verdict SHALL use only that scope's counts,
findings, comparisons, and worst offender, so a package, directory, or file view
answers about that scope rather than the repository. Producing a scope verdict
SHALL perform no filesystem, Git, parser, or analysis work, and renderers SHALL
consume the completed verdict rather than deriving one.

#### Scenario: A user drills into a package
- **WHEN** a package scope is selected and its debt differs from the repository's
- **THEN** its verdict tier and counts describe that package

#### Scenario: A scope has no checked units
- **WHEN** a selected scope contains no rated units
- **THEN** its verdict tier is `empty`

#### Scenario: A scope verdict is requested for rendering
- **WHEN** a renderer needs the verdict for the selected scope
- **THEN** it reads completed facts and performs no analysis work
