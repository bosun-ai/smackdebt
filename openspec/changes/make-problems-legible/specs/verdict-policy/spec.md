## ADDED Requirements

### Requirement: A sub-scope verdict frames the repository
Analysis SHALL compute a repository-share fact for a selected scope that is not
the repository root, stating how much of the repository's High debt lives in
that scope. The fact SHALL retain the scope's High count and the repository's
High count as integers and SHALL carry the frozen sentence
`<n> of the repository's <total> high live here.`, so a package view reading
`42 of the repository's 136 high live here.` tells the reader what fraction of
the problem it is looking at.

The sentence SHALL be owned by analysis so every consumer prints identical
bytes, and no renderer SHALL compose, recompute, or reword it. The fact SHALL be
absent when the selected scope is the repository root, where it would restate
the verdict counts, and SHALL be absent when the repository's High count is
zero. It SHALL never change the selected tier, the counts behind it, or the
worst offender, exactly as the unsupported-coverage qualifier never does.

Producing the fact SHALL read only the completed report, performing no
filesystem, Git, parser, or analysis work, so every retained scope can be framed
from one completed root report.

#### Scenario: A package holds part of the repository's debt
- **WHEN** a package scope with 42 High units is selected in a repository with 136 High units
- **THEN** its verdict carries the share fact with both integer counts and the sentence `42 of the repository's 136 high live here.`

#### Scenario: The repository root is selected
- **WHEN** no path is selected, or the selected path resolves to the repository root
- **THEN** the verdict carries no share fact

#### Scenario: The repository has no High debt
- **WHEN** a directory scope is selected in a repository whose High count is zero
- **THEN** the verdict carries no share fact rather than a zero-of-zero sentence

#### Scenario: The share never moves the tier
- **WHEN** the same scope verdict is computed with and without the share fact available
- **THEN** the tier, its counts, and the worst offender are identical

#### Scenario: Two consumers print the share
- **WHEN** the same sub-scope report is rendered for a human and serialized for a machine
- **THEN** both carry the analysis-owned share bytes and counts
