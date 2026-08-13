## Context

The product promise is the behavior of one local command, not the success of an
isolated parser or graph function. Source metrics, dependency resolution,
history aggregation, report policy, rendering, Git reads, and exit decisions
must therefore be exercised together.

The repository already has CLI tests, but the unified engine requires a
deliberate fixture matrix with exact expected output. Tests must remain stable
across machines without hiding unexpected differences through broad output
replacement.

## Goals / Non-Goals

**Goals:**

- Prove documented codebase and diff flows through a real CLI process.
- Cover every supported language and every analysis family with hand-checkable
  repositories.
- Treat status, stdout, stderr, terminal bytes, and JSON shape as product
  contracts.
- Prove serial and parallel execution produce identical bytes.
- Prove recoverable coverage failures remain visible and fatal failures use the
  correct stream and exit status.
- Prove the packaged command works outside the source workspace.

**Non-Goals:**

- Browser tests, network tests, runtime tracing, or private repository fixtures.
- Replacing focused pure, crate, property, allocation, or performance tests.
- Automatically accepting changed snapshots in CI.
- Hiding unstable product data with unrestricted timestamp or path replacement.
- Publishing the first release as part of this change.

## Decisions

### Build repositories from a small test domain

The black-box harness owns value objects for a repository fixture, commit,
file, worktree edit, invocation, and expected process result. Helpers create
unique temporary repositories, set explicit Git identity and timestamps, and
write only declared public fixture content. Tests do not inherit user Git
configuration.

Five fixture families divide responsibility:

1. `all-languages` contains nested units for every supported language and exact
   cognitive, cyclomatic, and exclusive logical-line results;
2. `static-architecture` contains internal and external references, ambiguous
   and unresolved references, file and package cycles, and degree values;
3. `evolution` contains dated commits, renames, binary changes, repeated package
   coupling, mailmap normalization, and contributor concentration;
4. `worktree-change` contains code metric changes, dependency additions and
   removals, cycle changes, a rename, deletion, untracked source, and a static
   edge that explains an existing coupling finding;
5. `coverage-failures` contains unsupported source, recoverable parse failure,
   incomplete history, invalid refs, and an invocation outside Git.

Each fixture has a short manifest beside its source that states the business
facts it is intended to prove. Expected values are hand-calculated in that
manifest and shared by focused assertions and black-box snapshots.

### Invoke the product at its public seam

Acceptance tests locate the built `smackdebt` binary and start it with
`std::process::Command`. The process working directory, arguments, environment,
terminal width, color setting, and worker policy are explicit. Every invocation
asserts exit status, stdout bytes, and stderr bytes.

Success output stays on stdout. Diagnostics allowed by the report contract stay
inside the report on stdout. Invocation and report-level failures use stderr
and their documented nonzero exit status. Tests assert the unused stream is
empty where required.

The harness is infrastructure only. It does not call internal analysis APIs,
construct reports, or repeat product policy.

### Commit exact terminal and JSON results

Golden files live next to the acceptance fixture and are named by fixture,
flow, width, color mode, and format. Normal test runs only compare bytes. A
separate explicit developer command updates selected golden files and prints
the changed paths; CI never runs it.

Fixtures invoke the CLI from the repository root and use repository-relative
arguments so temporary parent paths do not appear in output. Git identities and
dates are fixed, output ordering is explicit, and elapsed timing does not enter
normal reports. Unexpected paths, times, or host facts fail the comparison
instead of being replaced broadly.

Every JSON result first validates against the committed JSON version-2 schema,
then receives an exact semantic-value assertion for its key tables, and finally
receives a byte comparison. This separates invalid structure, wrong business
facts, and presentation drift.

### Define the required command matrix

The complete matrix covers:

- default concise codebase output;
- `--all` detailed output and one file, directory, and package selection;
- explicit codebase and Git-ref diff flows;
- worktree diff with modified, renamed, deleted, and untracked source;
- terminal widths 120, 80, and 50;
- color disabled and forced color, with ANSI removal proving equal visible text;
- JSON version 2 for codebase, ref diff, and worktree diff;
- one worker and automatic parallelism with exact byte equality;
- healthy, Watch, High, improved, worsened, changed, partial-coverage, bad-ref,
  and outside-Git exit cases.

Each analysis family appears in concise and detailed output and in JSON. The
matrix is data-driven so a new supported language or public report section must
add a declared case instead of a second harness.

### Inspect work counts at the composition seam

Test-only instrumentation records inventory visits, current source reads,
before-object reads, Git process starts by purpose, parser visits, and renderer
entry. The black-box performance fixture asserts the agreed counts for the
complete command.

The output layer receives an already built borrowed report. An instrumented
render proves that selecting terminal or JSON does not revisit discovery, read
source, start Git, parse source, or rerun an algorithm. The test hook reports
counts only when explicitly enabled for tests and never changes normal output.

### Prove installation and documentation

One release smoke test installs all workspace crates into a unique temporary
prefix using the same local source and locked dependencies intended for the
release. It changes to a generated repository outside the workspace, invokes
the installed binary, and validates help, version, one terminal report, and one
JSON report.

README console blocks marked as executable are parsed into declared test cases.
They run against named generated fixtures and compare the documented status and
output. Explanatory fragments that cannot run are marked explicitly and remain
subject to documentation review.

### Keep the evidence layered

Pure algorithm tests prove exact policy without I/O. Language fixtures prove
syntax meaning. Crate tests prove adapter behavior. Black-box tests prove the
public command. Allocation and performance workloads measure complete correct
flows. A passing black-box snapshot does not replace a missing lower-level
truth test, and a unit test does not replace public behavior proof.

## Risks / Trade-offs

- **Golden files become noisy** -> Keep cases responsibility-focused and use an
  explicit update command with reviewable diffs.
- **Fixtures accidentally depend on the host** -> Fix Git environment, dates,
  width, color, locale, and worker count, and fail on leaked host values.
- **A large matrix becomes slow** -> Reuse built binaries, generate each fixture
  once per test group, and keep performance workloads outside the fast subset.
- **Tests duplicate product policy** -> Store intended business facts in fixture
  manifests and assert public results; never recreate analysis in the harness.
- **Parallel tests collide** -> Give every fixture an OS-created unique
  directory and isolate Git configuration and install prefixes.
- **Snapshots pass while schema is wrong** -> Validate schema and semantic values
  before comparing bytes.

## Migration Plan

1. Add fixture-domain value objects and isolated repository construction.
2. Add the five fixture families and hand-calculated fact manifests.
3. Add real-process assertions for status, stdout, and stderr.
4. Add terminal and JSON schema plus golden checks for codebase flows.
5. Add ref and worktree diff flows, width, color, and worker parity.
6. Add failure, privacy, and composition work-count assertions.
7. Add installed-command and executable README checks.
8. Run the complete validation and release evidence from a clean revision.

Rollback removes the new harness and release requirement but does not change
production analysis. Golden files have no migration or runtime effect.

## Open Questions

None.
