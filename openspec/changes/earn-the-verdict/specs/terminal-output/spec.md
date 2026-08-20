## MODIFIED Requirements

### Requirement: Human diagnostics are grouped and simple
Warnings SHALL be grouped into one sentence per kind and SHALL be introduced by the word
`warning`. Incomplete history SHALL be written as `History is incomplete.`
Rename gaps SHALL be written as `Some renamed files could not be matched.`
Unmatched and ambiguous imports SHALL be combined into one sentence that
states the total and then each cause only when its count is non-zero, such as
`29 imports could not be followed · 24 named nothing in the repository · 5
matched more than one file`, with correct singular and plural wording
throughout. An empty history window SHALL be disclosed as `No commits in the
last <n> days.` parameterized on the selected window length, emitted when the
history stream is complete, a window is selected, and zero commits fall inside
it; the sentence SHALL follow the availability warnings and precede rename
gaps. Nested repositories SHALL be disclosed in default output with their own
subject, `1 nested repository was not analyzed.` with correct plural form, and
each diagnostic kind SHALL name its own subject rather than reusing another
kind's. No warning SHALL expose command, status, parser, process, or other
implementation text, and no warning SHALL repeat its summary sentence for each
fact. File detail SHALL remain available in `--all` or a path view.

#### Scenario: Several files share a warning kind
- **WHEN** default output is rendered
- **THEN** one grouped sentence states the count for that kind instead of one line per file

#### Scenario: Imports could not be followed for both reasons
- **WHEN** 24 unresolved and 5 ambiguous imports are retained
- **THEN** one warning row states `29 imports could not be followed · 24 named nothing in the repository · 5 matched more than one file` with no implementation text

#### Scenario: Imports could not be followed for one reason
- **WHEN** only unresolved imports are retained
- **THEN** the sentence states the total and the unresolved fact and omits the zero-count cause

#### Scenario: The history window is empty
- **WHEN** a complete stream yields zero commits inside a selected 90-day window
- **THEN** one warning row states `No commits in the last 90 days.` and source and architecture sections still render

#### Scenario: Nested repositories were pruned
- **WHEN** two nested checkouts were excluded from discovery
- **THEN** default output states `2 nested repositories were not analyzed.` without naming files as the subject

### Requirement: Human command text uses the same simple language
The system SHALL use short user-facing language for CLI help, argument errors, runtime
errors, terminal labels, and README examples and SHALL avoid internal report,
composition, stream, mapping, command, parser, and process terms.

A missing selected path SHALL exit with status 1, write empty stdout, and write
exact stderr bytes `smackdebt: path not found: <user-path>\n` from the original
user input without an absolute path or operating-system error text. A missing
Git ref SHALL exit with status 1, write empty stdout, and write exact stderr
bytes `smackdebt: Git ref not found: <ref>\n` without a Git command, process
status, or fatal output. `--all --json` SHALL exit with status 2, write empty
stdout, and write exact stderr bytes `smackdebt: --all cannot be used with
--json\n`. None of these failures SHALL append usage or help.

`--top` SHALL reject zero and SHALL conflict with `--json` and with `--all`,
each rejection exiting with status 2 in the same short language. A missing
gate baseline SHALL exit with status 2, write empty stdout, and write exact
stderr bytes `smackdebt: baseline not found: <path>\n` with no usage or help
tail. Exit status 3 SHALL mean the gate baseline was exceeded, and the gate's
proposal line SHALL read `next: smackdebt gate --update` in the same command
vocabulary as the discover line.

#### Scenario: Missing selected path is reported
- **WHEN** the user selects a path that does not exist
- **THEN** status is 1, stdout is empty, and stderr is exactly `smackdebt: path not found: <user-path>\n` with no usage, help, or operating-system text

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
