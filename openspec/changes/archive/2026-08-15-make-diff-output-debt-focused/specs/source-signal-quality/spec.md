## MODIFIED Requirements

### Requirement: Roles have exact verdict participation
Primary, test, example, and benchmark source SHALL affect default code verdicts.
Fixture and generated source SHALL remain visible in coverage, JSON, path drill,
and `--all` but SHALL NOT affect default verdicts, architecture health, or
coupling findings for codebase output. Diff human output SHALL narrow that
general codebase/path/`--all` visibility rule: fixture and generated comparisons
SHALL be JSON-only in diff default, diff `--all`, and diff path views because
their roles are excluded from trusted diff verdict math. Their coverage, role,
measurements, comparisons, and diagnostics SHALL remain complete in JSON.

The diff human selection SHALL include trusted primary, test, example, and
benchmark Regressed, Improved, debt-bearing Added or Removed, and MetricChanged
when either side is Watch or High. Role remains a qualifier on a selected
comparison and SHALL NOT create another inclusion rule. Recovered trust SHALL
route only to the machine report and grouped changed-source warning, never a
human diff selection. Healthy-only and
Ambiguous comparisons remain report and JSON context.

#### Scenario: Test source debt regresses
- **WHEN** a trusted test unit changes from Watch to High
- **THEN** it is linked once as human Worse with its test qualifier

#### Scenario: Fixture measurement changes
- **WHEN** a fixture unit retains a comparison
- **THEN** diff default, diff `--all`, and diff path omit it while JSON retains its role, measurements, comparison, coverage, and diagnostics unchanged

#### Scenario: Generated measurement changes
- **WHEN** a generated unit retains a comparison
- **THEN** diff default, diff `--all`, and diff path omit it while JSON remains complete and codebase detail behavior is unchanged

#### Scenario: Benchmark contains a High unit
- **WHEN** its retained comparison meets the human debt matrix
- **THEN** it affects human diff output and displays its benchmark qualifier

### Requirement: Recovered facts are advisory
Recovered source SHALL retain measured units, Watch and High advisory facts,
dependency context, exact spans, role, and diagnostics in JSON and codebase
`--all`, preserving the accepted codebase behavior.
Recovered facts SHALL NOT contribute health, default code findings,
architecture verdict edges, coupling findings, or complete diff verdict counts
and SHALL NOT count healthy. A retained Watch or High advisory comparison that
meets the exact numeric relevance matrix SHALL remain JSON-only in diff default,
diff `--all`, and diff path. This diff-only rule narrows the earlier general
codebase/path/`--all` advisory visibility rule because recovered matching is not
trusted. It SHALL NOT enter `DebtDiffSelection`, `DebtDiffCounts`, `QUALITY`,
`AREAS`, discover guidance, or the no-debt decision. A selected recovered or
ambiguous changed-source gap SHALL contribute its file once to the grouped
changed-file warning across every diagnostic reason and SHALL NOT become a
trusted or invented comparison. Human output SHALL use the exact singular or
plural `changed file(s) could not be checked` row and JSON SHALL retain every
diagnostic reason.

#### Scenario: Advisory debt comparison is relevant
- **WHEN** a recovered unit has a retained debt-bearing MetricChanged comparison
- **THEN** diff default, diff `--all`, and diff path omit it, the grouped warning remains, and JSON retains its advisory comparison unchanged

#### Scenario: Recovered changed-source fact is the only issue
- **WHEN** no default-eligible debt ID exists
- **THEN** diff default, diff `--all`, and diff path are exactly `No debt changed in checked files.` followed by the exact singular/plural Warning-glyph changed-file row with no optional detail, while JSON retains the comparison

#### Scenario: Recovered dependency resolves
- **WHEN** recovered syntax identifies a repository target
- **THEN** the relation remains advisory JSON context and never participates in a verdict graph
