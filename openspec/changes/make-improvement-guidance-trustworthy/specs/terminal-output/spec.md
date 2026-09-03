## MODIFIED Requirements

### Requirement: Human diagnostics are grouped and simple
Warnings SHALL be grouped into one sentence per kind and SHALL be introduced by
the word `warning`. Incomplete history SHALL be written as
`History is incomplete.` Rename gaps SHALL be written as
`Some renamed files could not be matched.` Unmatched and ambiguous imports
SHALL be combined into one sentence that states the total and then each cause
only when its count is non-zero, such as
`29 imports could not be followed · 24 named nothing in the repository · 5 matched more than one file`,
with correct singular and plural wording throughout.

Anonymous-unit match ambiguity SHALL be grouped separately from import
ambiguity. It SHALL count affected files once and state either
`1 file has anonymous units that could not be matched safely.` or
`<n> files have anonymous units that could not be matched safely.` One rendered
scope SHALL contain at most one such aggregate warning,
however many collision groups a file holds. It SHALL NOT expose an anchor, digest, parser node, candidate count,
or guessed direction. File detail SHALL remain available in `--all` and at a
selected file scope through the existing ambiguity comparison and diagnostic.

An empty history window SHALL be disclosed as
`No commits in the last <n> days.` parameterized on the selected window length,
emitted when the history stream is complete, a window is selected, and zero
commits fall inside it; the sentence SHALL follow availability warnings and
precede rename gaps. Nested repositories SHALL be disclosed in default output
with their own subject, `1 nested repository was not analyzed.` with correct
plural form. No warning SHALL expose command, status, parser, process, or other
implementation text, and no warning SHALL repeat its summary sentence for each
fact.

#### Scenario: Several files share a warning kind
- **WHEN** default output is rendered
- **THEN** one grouped sentence states the count for that kind instead of one line per file

#### Scenario: Imports could not be followed for both reasons
- **WHEN** 24 unresolved and 5 ambiguous imports are retained
- **THEN** one warning row states `29 imports could not be followed · 24 named nothing in the repository · 5 matched more than one file` with no implementation text

#### Scenario: Anonymous units are ambiguous in two files
- **WHEN** two files retain unsafe anonymous-unit match groups
- **THEN** one warning row states `2 files have anonymous units that could not be matched safely.` and neither file is counted twice

#### Scenario: One file has several collision groups
- **WHEN** one file retains three unsafe anonymous-unit match groups
- **THEN** one warning row states `1 file has anonymous units that could not be matched safely.`

#### Scenario: The history window is empty
- **WHEN** a complete stream yields zero commits inside a selected 90-day window
- **THEN** one warning row states `No commits in the last 90 days.` and the problem section still renders

#### Scenario: Nested repositories were pruned
- **WHEN** two nested checkouts were excluded from discovery
- **THEN** default output states `2 nested repositories were not analyzed.` without naming files as the subject

### Requirement: Human command text uses the same simple language
The system SHALL use short user-facing language for CLI help, argument errors,
runtime errors, terminal labels, and README examples and SHALL avoid internal
report, composition, stream, mapping, command, parser, and process terms.

A missing selected path SHALL exit with status 1, write empty stdout, and write
exact stderr bytes `smackdebt: path not found: <user-path>\n`. An existing
selected directory containing no recognized source SHALL do the same with
`smackdebt: no source files found under: <user-path>\n`. An existing selected
non-source file SHALL do the same with
`smackdebt: not a source file: <user-path>\n`. Each path error SHALL retain the
original user input without an absolute path or operating-system error text.

A missing Git ref SHALL exit with status 1, write empty stdout, and write exact
stderr bytes `smackdebt: Git ref not found: <ref>\n` without Git command,
process status, or fatal output. `--all --json` SHALL exit with status 2, write
empty stdout, and write exact stderr bytes
`smackdebt: --all cannot be used with --json\n`. None of these failures SHALL
append usage or help.

`--top` SHALL reject zero and SHALL conflict with `--json` and with `--all`,
each rejection exiting with status 2 in the same short language. A missing gate
baseline SHALL exit with status 2, write empty stdout, and write exact stderr
bytes `smackdebt: baseline not found: <path>\n` with no usage or help tail. Exit
status 3 SHALL mean the gate baseline was exceeded, and the gate's proposal line
SHALL read `next: smackdebt gate --update` in the same command vocabulary as the
codebase discover line.

#### Scenario: Missing selected path is reported
- **WHEN** the user selects a path that does not exist
- **THEN** status is 1, stdout is empty, and stderr is exactly `smackdebt: path not found: <user-path>\n` with no usage, help, or operating-system text

#### Scenario: Source-free directory is reported
- **WHEN** the user selects an existing directory containing no recognized source
- **THEN** status is 1, stdout is empty, and stderr is exactly `smackdebt: no source files found under: <user-path>\n`

#### Scenario: Non-source file is reported
- **WHEN** the user selects an existing file that is not recognized source
- **THEN** status is 1, stdout is empty, and stderr is exactly `smackdebt: not a source file: <user-path>\n`

#### Scenario: Missing Git ref is reported
- **WHEN** the user selects a Git ref that does not exist
- **THEN** status is 1, stdout is empty, and stderr is exactly `smackdebt: Git ref not found: <ref>\n` with no usage or help tail

#### Scenario: All detail conflicts with JSON
- **WHEN** the user supplies `--all --json`
- **THEN** status is 2, stdout is empty, and stderr is exactly `smackdebt: --all cannot be used with --json\n` with no usage or help tail

#### Scenario: A gate baseline is missing
- **WHEN** the user runs the gate without a baseline and without requesting an update
- **THEN** status is 2, stdout is empty, and stderr is exactly `smackdebt: baseline not found: <path>\n` with no usage or help tail

#### Scenario: An invalid top limit is rejected
- **WHEN** the user supplies `--top 0`, `--top` with `--json`, or `--top` with `--all`
- **THEN** each invocation exits with status 2 and a short product-language error

## ADDED Requirements

### Requirement: A short mixed diff represents its conclusion
When the default diff verdict is `mixed`, presentation SHALL select comparison
rows linked by the selected scope's debt-diff selection across source,
architecture, and history before applying one view-wide limit. The exact key
SHALL be direction Worse, Better, Changed; family source, architecture,
history; repository-relative subject path or path pair; start line with absent
after present; family kind in enum order; then comparison identity.

The default limit SHALL remain three. Default mixed selection SHALL reserve the
first row by that key for each present direction, then fill remaining positions
from the same key without duplicates. A reserved row SHALL remain in its owning
section, whose chosen rows use the same direction and stable-subject order.
Current-state history context that is not a diff comparison SHALL keep its
existing section-local handling and SHALL NOT consume the comparison limit.

`--all` SHALL retain all useful rows. An explicit `--top <n>` SHALL never emit
more than `n` debt-comparison rows across all three families and SHALL select the
first `n` rows by the exact key without reservation. No view SHALL add an
omitted-row notice.

Every diff that renders a comparison section or warning SHALL end, after any
warnings, with the exact undecorated line
`  inspect directories and files for more details\n`. It SHALL NOT print a
`next:` line. A `no_debt_change` diff SHALL remain verdict-only unless one of
these comparison-trust facts exists: an incomplete-coverage qualifier or
warning, an anonymous-match warning, a suppressed graph-comparison warning, or
a rename-gap or history-availability warning that affects the selected
comparison. When one exists, the report SHALL render only the applicable
qualifier and comparison-trust warning rows after the verdict and SHALL end with
the same detail footer. Unrelated current-state areas, history findings,
concentration, coupling, and other context SHALL NOT pierce verdict-only output.

#### Scenario: Worse and better source rows compete for the default limit
- **WHEN** a mixed diff has enough worse rows to fill the report and at least one better row
- **THEN** the default view includes the highest-ranked worse row and the highest-ranked better row

#### Scenario: Directions span comparison families
- **WHEN** source has Worse, architecture has Better, and history has Changed rows
- **THEN** the default view includes one row from each present direction in its owning section

#### Scenario: One row is requested
- **WHEN** the user supplies `--top 1` to a mixed diff
- **THEN** exactly one ranked row is visible and the report does not expand to represent every direction

#### Scenario: Diff detail is rendered
- **WHEN** a terminal diff has at least one visible comparison section or warning
- **THEN** its final bytes are exactly `  inspect directories and files for more details\n` and it contains no `next:` line

#### Scenario: No debt changed
- **WHEN** a fully checked diff has the `no_debt_change` verdict and no comparison-trust warning
- **THEN** the output ends after `No debt changed.` without the detail footer

#### Scenario: No debt changed but source was not all checked
- **WHEN** a diff has the `no_debt_change` verdict and an incomplete-coverage qualifier or warning
- **THEN** the qualifier and warning remain visible and the report ends with `  inspect directories and files for more details\n`

#### Scenario: No debt changed but graph evidence was withheld
- **WHEN** a diff has the `no_debt_change` verdict and a suppressed graph comparison warning
- **THEN** the warning remains visible and the report ends with the detail footer

#### Scenario: A documentation-only diff retains current history context
- **WHEN** a documentation-only change moves no debt while the current repository retains coupling, concentration, or other history findings unrelated to the comparison
- **THEN** terminal output is exactly the verdict form ending in `No debt changed.` with no section or footer

#### Scenario: Comparison history is incomplete
- **WHEN** a no-debt comparison has a rename gap or history-availability warning that affects its trust
- **THEN** that warning remains visible, unrelated current-state context stays hidden, and the report ends with the detail footer
