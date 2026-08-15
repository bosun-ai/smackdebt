## MODIFIED Requirements

### Requirement: Terminal evidence is exact and stable
The system SHALL compare committed exact terminal bytes for codebase, diff, package,
directory, and file views at widths 120, 100, 80, and 50, and SHALL compare
piped bytes with terminal bytes for the same invocation. Evidence SHALL cover a
verdict block per codebase tier and per diff tier, word-labeled counts including
zero counts, a clean diff that prints the verdict line only, hot findings with
their commit counts, stacked cycle witnesses, stable-dependency rows, grouped
warnings, and diff findings with `path:line`, before-and-after measurements, and
human identities.

It SHALL prove that piped output contains no codepoint in the range
U+E000–U+F8FF, that decorated output uses exact glyph code points of one display
cell placed adjacent to their words, that U+EC3F is absent, that `--color
always`, `--color never`, `NO_COLOR`, and redirect behavior are exact, and that
stripping ANSI reproduces the plain words. Every ANSI-stripped line SHALL fit
its requested Unicode display width, and no measurement, count, cycle witness,
history evidence value, dependency state, command, or identity SHALL be silently
clipped at any required width.

#### Scenario: Terminal width changes
- **WHEN** each named report view is rendered at every required width
- **THEN** every result exactly matches reviewed bytes, stacks rows that do not fit, and loses no fact

#### Scenario: Output is piped
- **WHEN** the same command is captured through a pipe and through a terminal
- **THEN** the piped bytes state every meaning in words, contain no codepoint in U+E000–U+F8FF, and differ from the terminal bytes only by decoration and ANSI

#### Scenario: A cycle witness does not fit
- **WHEN** a cycle finding is rendered at 50 columns
- **THEN** its witness stacks across lines with no ellipsis and the full closure remains readable

### Requirement: Public command evidence covers every revised decision
Exact acceptance SHALL cover codebase, diff, package, directory, and file flows for the
verdict block, the frozen tier sentences, word-labeled counts including zero
counts, the family named in a diff verdict, the clean-diff verdict-only view,
five-area and three-finding limits, hot annotations, ranked order, stacked cycle
witnesses, stable-dependency rows, one row per coupling pair,
knowledge-concentration rows, grouped warnings, actionable diff findings, the
`next: smackdebt <path>` discover line, and `--all` as all useful debt without
raw edges, standard-library externals, churn dumps, cyclomatic-1 rows, weak
coupling, or healthy rows.

It SHALL cover full exact stderr bytes including one newline, required status,
and empty stdout for `smackdebt: path not found: <user-path>`, `smackdebt: Git
ref not found: <ref>`, and `smackdebt: --all cannot be used with --json`, with
no usage or help tail and no leaked absolute path, operating-system code, Git
command, status, fatal output, parser text, or process text.

JSON version-3 bytes, report facts, status classes, stream placement, rank,
serial and automatic bytes, work totals, analysis, allocation, and performance
evidence SHALL prove unchanged behavior.

#### Scenario: Human output is audited
- **WHEN** snapshots, help, errors, warnings, and README examples are checked
- **THEN** the word vocabulary, verdict block, and labeled counts appear while positional counts, omitted zero counts, ellipsis-truncated witnesses, generated internal identities, raw edges, and leaked implementation diagnostics do not

#### Scenario: An input failure occurs
- **WHEN** each of the three failures is invoked
- **THEN** status, empty stdout, and one exact newline-terminated stderr line match the required bytes with no usage or help tail

#### Scenario: Machine and analysis output is audited
- **WHEN** redesigned terminal flows run through the complete public matrix
- **THEN** JSON bytes, report facts, exit classes, streams, rank, work counts, allocations, and resource evidence remain unchanged
