## MODIFIED Requirements

### Requirement: Architecture integrates with progressive reports
Default terminal reports SHALL show architecture health totals and rated cycle
witnesses without arbitrary dependency-edge rows. A rated cycle SHALL reach a
codebase reader as one problem card carrying its existing witness, and a
package's degree SHALL reach a reader only as aggregate problem evidence such as
a fan-in or fan-out count.

No human view SHALL list dependency relations as rows at any scope, in any
mode, or at any detail level. Unresolved and ambiguous relation rows SHALL
appear only when `--all` is supplied or the selected scope is a file; at every
other scope the grouped warning sentence is their whole terminal presence.

This REVERSES the previously accepted rule that `--all` and path drill show
relevant incoming, outgoing, unresolved, ambiguous, advisory, and
module-ownership relations. That rule is the root cause of the reported defect:
at a directory scope holding 296 files, "relevant" meant every edge touching the
scope, so the report printed about 1,270 unrated rows — one per import, test
imports and module wiring included — and the reader could not find a problem in
them. Relations are graph facts, not decisions; the machine report is where a
consumer reads them.

The machine report SHALL retain complete relation tables in whichever schema
version is current, so retiring one version never retires the relation tables.

#### Scenario: A repository has many ordinary uses and no cycles
- **WHEN** default terminal output is rendered
- **THEN** architecture states its health without listing arbitrary edges

#### Scenario: A user drills into one package
- **WHEN** incoming, outgoing, ownership, or advisory relations touch it
- **THEN** no relation row is printed and the relationship reaches the reader only through aggregate problem evidence

#### Scenario: A user asks for all detail at a directory
- **WHEN** `--all` is supplied at a directory scope
- **THEN** unresolved and ambiguous rows appear while no `uses` or `module_ownership` row appears

#### Scenario: A machine consumer reads relations
- **WHEN** the current machine report is parsed
- **THEN** every dependency edge, package edge, external dependency, resolution diagnostic, and package-graph row remains present

## ADDED Requirements

### Requirement: An extracted dependency specifier is one line
The target text an extracted dependency reference carries SHALL be a single
line. Extraction SHALL take the first line of the syntax it read, collapse runs
of whitespace within that line to one space, and trim the result. It SHALL NOT
shorten the text to a fixed length, because the terminal already wraps a long
row and a shortened specifier cannot be searched for in the source.

The rule SHALL apply wherever a target reaches a retained resolution diagnostic,
so no newline, carriage return, or tab can reach terminal or machine output
through a dependency target. This closes the defect where a language fell back
to the whole declaration node for a form it could not narrow, and a dynamic
import written across several lines printed raw source into the terminal.

#### Scenario: A dynamic import spans three lines
- **WHEN** a JavaScript file imports through a template literal written across three lines
- **THEN** the retained diagnostic's target is the first line only and contains no newline

#### Scenario: A specifier contains repeated whitespace
- **WHEN** the read syntax contains runs of spaces or tabs
- **THEN** the target collapses them to single spaces and carries no leading or trailing space

#### Scenario: A specifier is long
- **WHEN** a one-line specifier is wider than the resolved terminal width
- **THEN** it is retained in full and the renderer wraps it rather than the extractor shortening it
