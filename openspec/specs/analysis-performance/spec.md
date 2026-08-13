# analysis-performance Specification

## Purpose
TBD - created by archiving change refine-architecture-boundaries. Update Purpose after archive.
## Requirements
### Requirement: Project owns all parallel scheduling
`smackdebt-project` SHALL own one private Rayon thread pool and every decision to
run source analysis serially or in parallel. Language, discovery, Git, analysis,
output, and CLI crates SHALL NOT create worker pools.

#### Scenario: Repository analysis uses several cores
- **WHEN** the selected work exceeds the measured parallel cutover
- **THEN** project orchestration schedules independent files through its private Rayon pool

#### Scenario: One file is selected
- **WHEN** analysis contains one source work item
- **THEN** it runs on the calling thread without entering parallel scheduling

### Requirement: Users can limit worker count
Omitting `--jobs` SHALL use available host parallelism. Supplying a non-zero
`--jobs N` SHALL create a pool with that maximum worker count and SHALL NOT alter
Rayon's process-wide pool.

#### Scenario: CI requests one worker
- **WHEN** the user runs Smackdebt with `--jobs 1`
- **THEN** analysis runs serially and produces the same report as automatic scheduling

### Requirement: Inventory and current source reads are single pass
Codebase analysis SHALL perform one ignore-aware inventory walk and SHALL read
each selected current source file at most once. Discovery SHALL produce stable
path order before parallel source work begins.

#### Scenario: Large repository is scanned
- **WHEN** a report covers many supported files
- **THEN** instrumented tests observe one inventory visit and at most one source read per current file

### Requirement: Source memory follows active work
Codebase analysis SHALL retain at most one current source buffer per active
worker. Diff analysis SHALL retain at most the before and after source buffers
for each active changed-file worker. Finished source buffers SHALL be released
before final report rendering.

#### Scenario: Worker count is fixed
- **WHEN** the same large repository runs with one worker and then several workers
- **THEN** source-buffer memory growth follows active workers rather than total source bytes

### Requirement: Owned analyzers reuse worker state
Each Rayon worker SHALL reuse parser and scratch storage for owned language
implementations. Parser state SHALL NOT be shared through a mutex across workers.

#### Scenario: Worker analyzes consecutive Ruby files
- **WHEN** one worker receives several Ruby files
- **THEN** it reuses its Ruby parser and scratch capacity while keeping file results isolated

### Requirement: Rating and aggregation avoid new heap allocation
Health assessment SHALL use fixed storage for its three signals. After report
arrays reserve their required capacity, rating units and updating aggregation
counts SHALL perform no heap allocation.

#### Scenario: Allocation-instrumented policy test runs
- **WHEN** preallocated measurements are rated and accumulated
- **THEN** the instrumented region records zero heap allocations

### Requirement: Full scans retain debt details only
Codebase analysis SHALL retain every Watch and High unit plus all scope totals.
It SHALL discard healthy unit detail after aggregation. JSON SHALL expose all
retained findings rather than only the terminal shortlist.

#### Scenario: Repository contains many healthy units
- **WHEN** a codebase report completes
- **THEN** report memory contains their aggregate counts but not one owned record for every healthy unit

### Requirement: Git work uses a fixed process shape
Recent activity SHALL use one streamed Git history process. Ref comparison SHALL
use one status operation and one `git cat-file --batch` process for base objects.
The implementation SHALL NOT start a Git process per source file.

#### Scenario: Diff contains many files
- **WHEN** an instrumented diff analyzes many base-side blobs
- **THEN** the observed Git process count remains fixed rather than growing with changed-file count

### Requirement: Batch object flow applies backpressure
Base-object reads SHALL pass through a limited queue between the single batch
reader and source workers. The reader SHALL stop requesting more blobs when the
queue is full.

#### Scenario: One source worker is slower than Git output
- **WHEN** batch object production outruns analysis
- **THEN** queued blob memory stays within the configured queue capacity

### Requirement: Output streams from one report
Terminal and JSON renderers SHALL write directly to `io::Write` from a borrowed
report. JSON output SHALL NOT require a second owned report model, and analysis
SHALL NOT depend on Serde.

#### Scenario: Large JSON report is emitted
- **WHEN** JSON output contains many findings
- **THEN** serialization writes incrementally without cloning the report or its paths

### Requirement: Parallel results are deterministic
Serial and parallel execution SHALL produce byte-for-byte identical terminal and
JSON output for the same repository state, request, width, and color setting.

#### Scenario: Acceptance fixture runs at two worker counts
- **WHEN** a fixture is analyzed with one worker and automatic parallelism
- **THEN** both terminal snapshots and JSON bytes are identical

### Requirement: Performance gates are evidence-led
Before absolute budgets exist, CI SHALL enforce inventory counts, source-read
counts, Git process counts, deterministic output, allocation rules, and memory
shape. The first correct release benchmark SHALL record absolute wall-time and
memory budgets with ten percent regression room.

#### Scenario: First trustworthy implementation is benchmarked
- **WHEN** the complete codebase and diff flows pass correctness checks
- **THEN** their measured baseline, environment, workload identity, and resulting budgets are checked in

#### Scenario: Benchmark workload changes
- **WHEN** source fixtures or workload definitions change materially
- **THEN** the change declares a new baseline instead of comparing unlike workloads

### Requirement: Benchmarks cover real and generated repositories
Developer benchmarks SHALL cover the private Fluyt repository and record its
revision and dirty state. Public checks SHALL use deterministic mixed-language
fixtures, including small diffs and a generated workload approaching 100,000
files or 10 million lines.

#### Scenario: Private workload cannot run in public CI
- **WHEN** public CI executes performance checks
- **THEN** generated fixtures exercise the same structural performance contracts without accessing Fluyt

### Requirement: No async runtime or persistent cache is introduced
The first release SHALL use synchronous filesystem and Git I/O and SHALL NOT
persist analysis results between invocations.

#### Scenario: Same repository is analyzed twice
- **WHEN** two independent invocations analyze the same repository
- **THEN** each result comes from current filesystem and Git state rather than a persistent analysis cache

### Requirement: Explicit paths limit codebase work
Inside a Git repository, an explicit codebase path SHALL preserve
repository-relative identity while discovery and source reads stay limited to
the selected directory subtree or selected file. Package ancestor inspection
SHALL visit each distinct ancestor directory at most once.

#### Scenario: User inspects one nested directory
- **WHEN** the repository contains source outside the selected directory
- **THEN** instrumentation observes no source reads outside the selection and displayed paths retain their repository-relative prefix

### Requirement: Diff package discovery avoids repeated ancestor work
Diff hierarchy construction SHALL inspect each distinct changed-path ancestor at
most once and SHALL retain changed manifest paths before filtering non-source
changes. Git and filesystem work SHALL NOT grow by one process or repeated
ancestor scan per changed source file.

#### Scenario: Many changed files share one package
- **WHEN** a diff contains many files below the same directory chain
- **THEN** package discovery inspects each directory in that chain once and Git process count remains within the existing fixed shape

### Requirement: Scope rendering performs no project work
Rendering a retained scope SHALL perform no inventory walk, source read, Git
operation, parsing, health assessment, or worker scheduling.

#### Scenario: Root report renders several paths
- **WHEN** an instrumented test renders repository, directory, and file scopes from one report
- **THEN** all project-work counters remain unchanged after report construction

### Requirement: Progressive links preserve measured report limits
Finding links, comparison links, path indexes, and diff counts SHALL use reserved
flat storage. The generated large-repository workloads SHALL measure their effect
on allocation count and peak memory before release limits are accepted.

#### Scenario: Large hierarchy is aggregated
- **WHEN** scopes reserve the required child-derived link capacity
- **THEN** the post-order aggregation update performs no new heap allocation and every retained item is linked once per applicable scope

