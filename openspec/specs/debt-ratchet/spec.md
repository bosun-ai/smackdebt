# debt-ratchet Specification

## Purpose
TBD - created by archiving change earn-the-verdict. Update Purpose after archive.
## Requirements
### Requirement: A committed baseline records ratcheted debt
The system SHALL read and write a ratchet baseline file, by default
`.smackdebt-baseline.tsv` at the analyzed root, intended to be committed. The
file SHALL be UTF-8 with LF line endings and tab-separated columns `path`,
`signal`, `high`, and `watch` under a version-1 header comment, with no
trailing whitespace. Rows SHALL be sorted byte-wise by path then signal, and
rows whose High and Watch counts are both zero SHALL be omitted. `path` SHALL
be the repository-relative display form. The parser SHALL reject an unknown
signal, a duplicate key, or an out-of-order row with exit status 2 and SHALL
never silently pass a malformed baseline.

#### Scenario: A baseline is written
- **WHEN** the observed snapshot is written as a baseline
- **THEN** the file is sorted by path then signal, contains no zero-zero row, and round-trips byte-identically

#### Scenario: A baseline is malformed
- **WHEN** the baseline contains an unknown signal or out-of-order rows
- **THEN** the gate exits with status 2 and reports the defect rather than passing

### Requirement: Ratcheted signals are time-invariant
The gate SHALL ratchet exactly these signals: `cognitive`, `cyclomatic`,
`logical_lines`, `nesting`, and `parameters` counted per rated unit at High
and Watch and attributed to the unit's file; `file_size` and `container_size`
from size findings; `package_cycle` and `file_cycle` from architecture
findings attributed to their first witness; and `stable_dependency`. Only
findings that affect the verdict SHALL count. History-derived signals such as
change coupling and knowledge concentration SHALL be excluded by rule, not
omission, because they move with wall-clock time and would make a committed
gate flaky. A unit tripping two signals SHALL produce two rows, each
ratcheting independently.

#### Scenario: A unit trips two signals
- **WHEN** one unit is High for cognitive complexity and Watch for nesting
- **THEN** its file carries one cognitive row and one nesting row

#### Scenario: History moves without code changing
- **WHEN** only wall-clock time passes over an unchanged tree
- **THEN** the gate result is unchanged because no ratcheted signal derives from history

### Requirement: Gate comparison is exact
The gate SHALL compare observed rows against baseline rows per key, treating
an absent key as zero counts. An observed High or Watch count above its
baseline SHALL be a regression; below SHALL be an improvement, reported but
never auto-applied; equal SHALL be unchanged and unprinted. A deleted file
SHALL read as an improvement to zero. Any regression SHALL exit with status 3;
otherwise the gate SHALL exit 0.

#### Scenario: One counter regresses
- **WHEN** a file's cognitive High count rises from 3 to 4
- **THEN** the gate prints that row as worse and exits with status 3

#### Scenario: Debt only improves
- **WHEN** every observed row is at or below its baseline
- **THEN** improvements are reported, the baseline file is not modified, and the exit status is 0

#### Scenario: A new file carries debt
- **WHEN** a file absent from the baseline has a High row
- **THEN** the absent key reads as zero and the row is a regression

### Requirement: The gate command reports in house vocabulary
`smackdebt gate [PATH]` SHALL analyze the selected root, compare against the
baseline at `<path>/.smackdebt-baseline.tsv` or the `--baseline` override, and
print a gate report using the existing word vocabulary: a `GATE` header naming
the baseline, `worse` and `better` rows naming path, signal, and the moved
counter as `<before> → <after>`, a totals line, and a `next:` line proposing
the update command when a regression exists. A missing baseline without
`--update` SHALL exit with status 2, write empty stdout, and write exact
stderr bytes `smackdebt: baseline not found: <path>\n` with no usage or help
tail. Exit status 3 SHALL mean the gate baseline was exceeded and SHALL be
documented beside the existing exit codes.

#### Scenario: The baseline is missing
- **WHEN** `smackdebt gate` runs without a baseline file and without `--update`
- **THEN** status is 2, stdout is empty, and stderr is exactly `smackdebt: baseline not found: <path>\n`

#### Scenario: A regression is reported
- **WHEN** one row regresses and one improves
- **THEN** the report shows one worse row, one better row, the totals line, and the update proposal, and status is 3

### Requirement: The baseline updates only on request
`smackdebt gate --update` SHALL write the observed snapshot verbatim and exit
0, accepting improvements and deliberate new debt alike. The gate SHALL never
tighten or rewrite the baseline implicitly, so a clean check run never dirties
the working tree. An immediate second `--update` over an unchanged tree SHALL
be byte-identical.

#### Scenario: The update is idempotent
- **WHEN** `--update` runs twice over an unchanged tree
- **THEN** the second run rewrites identical bytes and exits 0

#### Scenario: The baseline is looser than reality
- **WHEN** the baseline records more debt than observed and `--update` is not passed
- **THEN** the gate reports improvements, leaves the file untouched, and exits 0

### Requirement: Gate JSON is a checked contract
`smackdebt gate --json` SHALL emit one object with `schema_version: 1`, a
`clean` or `regressed` status, the baseline path, regression and improvement
rows each carrying path, signal, and the baseline and observed High and Watch
counts, and totals. The repository SHALL contain a checked JSON Schema for
this shape and acceptance evidence SHALL validate against it.

#### Scenario: A regressed gate is serialized
- **WHEN** `--json` runs over a regressed tree
- **THEN** the object validates against the gate schema and each regression row carries both counters' baseline and observed values

### Requirement: Smackdebt gates itself
The repository SHALL commit its own baseline and the complete check SHALL run
the gate over the workspace, so any commit that worsens a ratcheted signal
fails the check until the debt is removed or the baseline is deliberately
updated.

#### Scenario: A commit adds a deliberately complex function
- **WHEN** the complete check runs over that tree
- **THEN** the gate names the regressed row and the check fails with the gate's status

