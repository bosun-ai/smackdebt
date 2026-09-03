## ADDED Requirements

### Requirement: README explains trustworthy scope and diff guidance
The README SHALL state that an explicit supported or recognized-unsupported
source file is the selected file and an explicit source-bearing directory is
the selected directory. It SHALL show the exact empty-directory, non-source
file, and missing-path errors and SHALL NOT suggest that an explicit path can
fall back to repository results.

The supported-language section SHALL list Astro as recognized but unsupported
and SHALL explain that it remains in coverage and diff inventory without an
Astro parser. Coverage documentation SHALL show both exact qualifier lines,
state that any selected-versus-analyzed file gap triggers them, and explain the
JSON qualifier's selected and analyzed file counts.

Role documentation SHALL list the generated JavaScript filename and content
rules, state their exact limits, state that directory names alone do not assign
the role, state that explicit configuration wins and where the rules sit in role
precedence, and explain that generated files remain available in JSON, `--all`,
and file inspection while staying outside default decisions and navigation.

Diff documentation SHALL list the unchanged tier identifiers beside the four
neutral sentences, explain safe anonymous pairing and grouped ambiguity without
describing private digests as an interface, show a mixed default whose visible
rows support both directions, state that explicit `--top` is never exceeded,
state that no-debt expands only for source, anonymous-match, graph, rename, or
history-availability facts that narrow comparison trust while unrelated
current-state history remains hidden, and keep the exact final line
`inspect directories and files for more details` for a diff that shows
comparison detail or warnings. No diff example SHALL use `next:`.

#### Scenario: A user selects an Astro file
- **WHEN** they read the scope and language sections
- **THEN** they expect a successful file report with unsupported coverage rather than repository fallback or parser results

#### Scenario: A user sees incomplete coverage
- **WHEN** the verdict says `Not all source was checked.`
- **THEN** the next line gives exact analyzed and selected file counts and the JSON documentation names the same integers

#### Scenario: A generated bundle dominates measurements
- **WHEN** the user reads role documentation
- **THEN** they understand why it remains inspectable but does not own the default recommendation

#### Scenario: A user reads a mixed diff
- **WHEN** they compare the verdict, visible rows, and footer
- **THEN** the neutral conclusion is supported by visible worse and better evidence and the footer tells them to inspect directories and files

### Requirement: Documented trustworthy examples are executable
The README SHALL provide runnable examples for exact scope errors, Astro
coverage, anonymous matching, generated JavaScript context, all four neutral
diff sentences, representative mixed output, `--top 1`, and the diff footer,
each associated with a named public generated fixture. Documentation checks
SHALL execute the built CLI and match status, streams, and every declared output
fragment in order.

#### Scenario: A trustworthy example changes
- **WHEN** CLI behavior no longer matches a documented scope, qualifier, diff sentence, row, or footer
- **THEN** documentation acceptance fails with a reviewable difference
