## MODIFIED Requirements

### Requirement: Reports identify an initial selected scope
Every completed codebase and diff report SHALL identify one initial selected
scope separately from the report facts. A renderer SHALL be able to render any
scope retained by that report without changing the report or running project
work again.

An explicit supported or recognized-unsupported source file SHALL select that
file. An explicit directory containing recognized source SHALL select that
directory. Project composition SHALL NOT replace either explicit selection with
the repository root.

An existing selected directory containing no recognized source SHALL fail with
status 1, empty stdout, and exact stderr
`smackdebt: no source files found under: <user-path>\n`. An existing selected
file that is not recognized source SHALL fail with status 1, empty stdout, and
exact stderr `smackdebt: not a source file: <user-path>\n`. A missing path keeps
the accepted path-not-found result. All three failures SHALL retain the user's
path bytes and append no usage or help.

#### Scenario: Root report is explored several times
- **WHEN** a renderer selects the repository, a package, and a directory from one completed root report
- **THEN** every view is produced from retained report facts without filesystem, Git, parser, or worker activity

#### Scenario: Explicit path limits the report
- **WHEN** a user runs Smackdebt with an existing directory containing recognized source
- **THEN** that directory is the initial selected scope and its shares use that scope's totals

#### Scenario: An unsupported source file is selected
- **WHEN** a user selects an existing `.astro` file
- **THEN** the file is the initial selected scope and the successful report records one selected unsupported source file

#### Scenario: Selected directory has no source
- **WHEN** an existing selected directory contains no recognized source file
- **THEN** status is 1, stdout is empty, and stderr is exactly `smackdebt: no source files found under: <user-path>\n`

#### Scenario: Selected file is not source
- **WHEN** an existing selected file has no recognized source language
- **THEN** status is 1, stdout is empty, and stderr is exactly `smackdebt: not a source file: <user-path>\n`

#### Scenario: Selected path is missing
- **WHEN** the selected path does not exist
- **THEN** the accepted `smackdebt: path not found: <user-path>\n` result is unchanged

### Requirement: Codebase summary states rated quality and coverage
The terminal SHALL open with a verdict block stating the selected scope, the
analysis-owned tier sentence, and the counts behind it with every count labeled
by its word, followed by the worst offender with its resolved path and reason
when one exists. When selected source is incomplete, the qualifier SHALL render
inside the verdict block directly under the tier sentence as two analysis-owned
lines: `Not all source was checked.` followed by
`<analyzed> of <selected> source files were analyzed.` There SHALL be no
percentage threshold for showing these lines.

When the verdict carries a repository-share fact, the share row SHALL render
inside the verdict block directly under the qualifier detail when one exists and
directly under the tier sentence otherwise, using the analysis-owned share
bytes. The block SHALL omit healthy counts, summary ratios, and decorative
quality bars used as data. Coverage gaps SHALL also use one grouped warning
sentence per cause only when a gap exists.

#### Scenario: Selection is fully analyzed
- **WHEN** every selected source file is analyzed
- **THEN** the verdict block omits coverage-success text and no coverage warning appears

#### Scenario: One selected file is unsupported
- **WHEN** one of 200 selected source files is unsupported
- **THEN** the verdict block states `Not all source was checked.` and `199 of 200 source files were analyzed.` despite the small share

#### Scenario: A scope has nothing to check
- **WHEN** a recognized unsupported file is the selected scope and no unit is checked
- **THEN** the verdict block states the `empty` tier sentence, zero counts with their words, and `0 of 1 source files were analyzed.`

#### Scenario: A sub-scope view frames the repository
- **WHEN** a package or directory scope is selected, coverage is incomplete, and the repository holds High debt
- **THEN** both qualifier lines precede the analysis-owned repository-share line

### Requirement: Coverage notes describe source-analysis gaps
Coverage notes SHALL report selected source files that could not be analyzed.
The verdict qualifier SHALL appear whenever analyzed files are fewer than
selected source files, and grouped warnings SHALL retain the counts and causes
needed to inspect the gap. Routine changed files that are not source candidates
SHALL NOT appear as a coverage problem.

#### Scenario: Diff contains documentation and configuration changes
- **WHEN** a worktree contains changed source and non-source files
- **THEN** the diff analyzes source changes without a coverage note for routine non-source changes

#### Scenario: One changed Astro file is retained
- **WHEN** a diff contains one changed Astro file and one changed supported file
- **THEN** both are selected source, the Astro file is unsupported, and the diff qualifier states `1 of 2 source files were analyzed.`
