## Context

Smackdebt already composes source, static architecture, evolution, and diff
analysis into one report. The remaining work is signal quality: the report must
preserve evidence without letting low-trust or low-value evidence decide the
default verdict.

## Goals and non-goals

### Goals

- Give source role, parser trust, relation kind, and history operands distinct
  meanings.
- Keep package identity stable across codebase, path, and diff views.
- Make default terminal output concise, ranked, and free of duplicate evidence.
- Preserve advisory facts in JSON and detailed terminal output.
- Prove public behavior before privacy-safe review of real workloads.

### Non-goals

- Runtime analyzer plugins or executable configuration.
- Compiler-complete name or type resolution.
- A public Rust library compatibility promise.
- A persistent cache or support for JSON versions 2 and 3 together.

## Decisions

### Classify one SourceRole with explicit precedence

`SourceRole` has exactly six values: `primary`, `test`, `example`, `benchmark`,
`fixture`, and `generated`. Primary, test, example, and benchmark source affect
the default verdict. Fixture and generated source remain visible for coverage,
JSON, and `--all`, but do not affect the default verdict, architecture health,
or coupling findings.

Classification uses this order:

1. explicit declarative configuration;
2. language-owned generated markers in source or language metadata;
3. generic filename and repository-relative path rules;
4. primary as the fallback.

Several matches at one precedence level are a configuration or classification
conflict and the CLI exits 2 with no report on stdout. A higher-precedence match
replaces lower-precedence matches. Language implementations own generated
markers; generic filename and path rules are shared and do not contain
language-specific syntax.

Discovery no longer ignores a directory merely because its name is commonly
generated. It inventories supported candidates once, then role policy decides
their contribution. Normal Git and user ignore rules still apply.

### Keep recovered facts as advisory evidence

Parse outcomes are `parsed`, `recovered`, and `failed`. Parsed facts use normal
role policy. A recovered tree can still yield useful Watch or High measurements
and dependency context, so those facts remain in JSON and `--all`, carry
advisory trust, and show the recovery diagnostic. They do not enter health,
default terminal findings, architecture verdict edges, coupling, or diff
verdicts. Recovered facts are never counted as healthy.

Recovered dependency relations remain context with their original relation
kind, source role, and advisory trust. They never enter the trusted dependency
graph. Failed source has coverage and diagnostics but no invented syntax facts.

### Own the package table before source filtering

Discovery creates package rows from recognized roots in stable
repository-relative path order, including packages with no selected source.
Current rows receive IDs first. Diff-only base rows follow in stable base-path
order and state base-only presence. Files and all report facts refer to those
IDs; no later filtering or compaction renumbers them.

The package path stays `.` in the machine report. Terminal presentation renders
that package label as `repository root` everywhere.

### Keep relation kind orthogonal to evidence

Static relation kind has exactly two values: `uses` and `module_ownership`.
Every extracted relation separately carries source role, parse trust, source
span, resolution outcome, and reference count. These fields are evidence
dimensions, not more relation kinds.

For Rust, external `mod child;` is `module_ownership`. Inline modules remain in
their file. Imports, qualified paths, and safely resolved macro paths are
`uses`. Module ownership supplies containment and resolver context only; it
never changes fan-in, fan-out, instability, dependency cycles, or coupling
explanations. Trusted parsed `uses` from verdict-affecting roles form the
architecture graph. Advisory, fixture, and generated relations remain context.

Default architecture terminal output shows health totals and rated cycle
witnesses only. It does not print arbitrary edge rows. `--all` and path drill
show relevant incoming, outgoing, unresolved, ambiguous, advisory, and
ownership relations. JSON retains the complete relation tables.

### Preserve exact history evidence and filter only presentation

File and package history rows expose `touches`, `added_lines`, `deleted_lines`,
and `uncounted_changes`. History coverage exposes availability, revision,
commit count, newest and oldest timestamps, textual changes, uncounted changes,
excluded paths, rename gaps, and reason. Coupling rows expose left and right
package IDs, `shared_commits`, `union_commits`, and Jaccard similarity.
Contributor concentration exposes package ID, contributor count, numerator,
denominator, and ratio without identity.

Each source-derived history observation retains SourceRole. Fixture and
generated observations remain descriptive but cannot create default findings.
An unexplained coupling finding requires at least three shared commits, Jaccard
similarity of at least 0.20, sufficient history, and no trusted verdict `uses`
relation in either direction. Weaker observations remain in JSON and `--all`.

Default terminal output prints a history fact once at the nearest useful scope.
It does not repeat the same package history, coupling pair, or contributor
concentration in summary, architecture, and detail sections. Path drill and
`--all` may expand the retained evidence without duplicating an identical row
inside one view.

### Use one exact finding rank

All displayed source findings use this descending comparison sequence:

1. rating;
2. count of signals at that rating;
3. total triggered signals;
4. cognitive complexity;
5. cyclomatic complexity;
6. logical lines;
7. recent activity;
8. repository-relative path and source span.

Unit kind and every non-primary SourceRole are displayed with a finding. They
do not silently alter the comparison sequence. Advisory recovered findings are
ranked by the same keys only inside JSON and `--all`.

### Publish JSON version 3

Version 3 adds the package table, SourceRole, parse outcome and trust, relation
kind and orthogonal evidence, exact history fields, advisory facts, and stable
references. JSON keeps all retained facts and does not apply terminal limits.
Version 2 is retired before release and the CLI has no version selector.

### Prove public behavior, then review three workload families

Generated repositories prove every role and precedence level, same-level
conflicts and exit 2, recovered advisory facts, stable packages, Rust relation
semantics, history fields, 20% coupling boundaries, exact rank, root labels,
architecture witness-only defaults, detailed edges, de-duplication, privacy,
serial/automatic equality, and exact work counts.

After public proof passes, three workload families receive privacy-safe review:

- self: package references stay valid, false fixture cycles disappear, and root
  labels are readable;
- private mixed application: generated schema and client findings are excluded,
  weak coupling and Rust ownership cycles disappear, and substantial
  hand-written functions remain prominent;
- private Rust workspace: ownership cycles disappear and real
  high-complexity functions remain visible.

Every default coupling in all three reviews must meet three shared commits and
20% Jaccard. Committed evidence names only these workload families and aggregate
outcome categories. It contains no private repository path, source, Git
identity, or history.

### Record release evidence after the reviewed commit

Implementation and reviewed expectation changes are committed first. The
release workflow then requires a clean tree at that reviewed implementation
commit, captures one revision, toolchain, and host state, and records every
public profile plus the approved aggregate workload reviews. Evidence files
written during the workflow do not change the captured starting state. The
checker requires a clean start and one shared revision equal to reviewed HEAD.

## Implementation order

1. Source roles, precedence, conflicts, and recovery trust.
2. Stable package identity and root presentation.
3. Relation kind and orthogonal evidence.
4. Exact history fields, coupling policy, and de-duplication.
5. JSON version 3 and index checks.
6. Ranking and terminal detail policy.
7. Public end-to-end and resource proof.
8. Three workload reviews, reviewed commit, and clean release evidence.
