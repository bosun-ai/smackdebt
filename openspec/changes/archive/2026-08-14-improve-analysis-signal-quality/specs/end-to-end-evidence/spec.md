## MODIFIED Requirements

### Requirement: Generated fixtures cover all analysis families
Public generated repositories SHALL provide hand-calculated facts for six
roles, every precedence level, same-level conflict, parsed, recovered and failed
source, stable packages, both relation kinds, history fields, coupling threshold
edges, exact ranking, root labels, architecture witnesses, and de-duplication.

#### Scenario: Signal-quality fixture runs
- **WHEN** the real CLI analyzes the fixture
- **THEN** its fact manifest proves every expected role, trust, package, relation, history, finding, diagnostic, and coverage fact

#### Scenario: Role rules conflict
- **WHEN** two same-level rules assign different roles
- **THEN** exact acceptance proves exit 2, empty stdout, and the reviewed stderr

### Requirement: JSON evidence validates structure and meaning
Every acceptance JSON result SHALL validate against the version-3 schema and
pass semantic, index, privacy, and exact-byte checks from the same invocation
before its performance sample is accepted.

#### Scenario: A recovered High fact is emitted
- **WHEN** JSON validation runs
- **THEN** the advisory fact remains present and no health, graph-verdict, coupling-finding, or diff-verdict index refers to it

#### Scenario: A private value reaches output
- **WHEN** output contains source, an absolute private path, Git identity, or private history detail
- **THEN** privacy validation fails before evidence approval

### Requirement: Composition work is observable in complete flows
Feature-gated live evidence SHALL count inventory visits, reads, Git processes,
parser visits, algorithm passes, and renderer entry at their actual seams with
no evidence-only API or atomic cost in normal builds.

#### Scenario: Rendering starts from one completed report
- **WHEN** live snapshots are captured immediately before and after output
- **THEN** the delta has one renderer entry and zero inventory, read, Git, parser, and algorithm events

#### Scenario: Real work occurs after a snapshot
- **WHEN** one counted seam runs after an earlier live snapshot
- **THEN** a later snapshot changes and cannot be replaced by a frozen report value

## ADDED Requirements

### Requirement: Public command evidence covers every revised decision
Exact terminal and JSON acceptance SHALL cover codebase, path drill, clean
committed ref-diff, and mixed worktree-diff flows for roles, recovery, packages,
relations, history, ranking, root labels, witness-only defaults, detailed edges,
weak coupling, coverage failures, and conflict exit 2.

#### Scenario: A committed branch differs with a clean worktree
- **WHEN** role, package presence, relation, or history facts change between refs
- **THEN** exact status, stdout, and stderr contain no worktree-only facts

#### Scenario: A real source file fails analysis
- **WHEN** the fixture causes failure rather than parser recovery
- **THEN** exact coverage and diagnostics remain and no invented fact appears

### Requirement: Serial and automatic flows preserve bytes and work
Every named public flow SHALL run with one worker and automatic parallelism.
Status and bytes SHALL match, and inventory, read, Git, parser, algorithm, and
renderer totals SHALL equal the exact manifest values for that flow.

#### Scenario: Worker policy changes
- **WHEN** the same fixture command runs under both policies
- **THEN** terminal and JSON bytes and exact work totals agree

### Requirement: Three workload families have privacy-safe acceptance
After public proof passes, aggregate review SHALL cover self, a private mixed
application, and a private Rust workspace. Committed evidence SHALL name only
the workload family and expected outcome categories and SHALL contain no private
path, source, Git identity, or history.

#### Scenario: Self is reviewed
- **WHEN** the current implementation analyzes self
- **THEN** fixture cycles are gone, package references are valid, root labels are readable, and every default coupling meets the threshold

#### Scenario: Private mixed application is reviewed
- **WHEN** aggregate outcomes are inspected
- **THEN** generated schema and client findings are excluded, weak coupling is gone, Rust ownership cycles are gone, substantial hand-written functions remain prominent, and every default coupling meets the threshold

#### Scenario: Private Rust workspace is reviewed
- **WHEN** aggregate outcomes are inspected
- **THEN** ownership cycles are gone, real high-complexity functions remain visible, and every default coupling meets the threshold
