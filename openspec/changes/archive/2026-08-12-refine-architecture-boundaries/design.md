## Context

Smackdebt is still a single hello-world package. The product documents already
define concise codebase and diff reports, but implementation boundaries remain
conceptual. The tool must scan large mixed repositories, retain honest coverage,
and eventually replace every upstream language implementation with owned code.

The initial real-world performance workload is the private Fluyt repository. It
contains Ruby, Vue, Rust, TypeScript, JavaScript, and Python, with several
directories containing manifests from more than one package manager. Public CI
cannot depend on Fluyt and therefore needs deterministic generated workloads.

The pinned `rust-code-analysis` revision owns each source buffer, builds a
tree-sitter tree, and allocates nested metric structures. Its directory runner
also combines traversal, scheduling, channels, and output. Smackdebt cannot make
that code allocation-free without maintaining a fork, which is not desired.

## Goals / Non-Goals

**Goals:**

- Keep policy, source parsing, filesystem access, Git access, orchestration, and
  output in small crates with one clear responsibility each.
- Make CLI behavior and JSON schema the supported first-release interfaces.
- Keep source memory proportional to worker count and avoid extra allocation in
  Smackdebt's rating, aggregation, comparison, and rendering loops.
- Use one private Rayon pool for CPU-heavy file analysis while keeping Git and
  filesystem I/O synchronous and explicit.
- Support every verified upstream language plus owned Ruby and Vue analysis.
- Replace upstream language implementations one at a time without changing
  project, report, or output behavior.
- Prove performance with repeatable workload metadata and structural checks,
  then set absolute budgets from the first trustworthy implementation.

**Non-Goals:**

- Runtime analyzer plugins or a stable Rust library API.
- A persistent analysis cache in the first release.
- Async I/O, a long-running service, or a terminal UI.
- Copying or forking `rust-code-analysis` source.
- Modeling package-manager dependency graphs or language-specific module trees.
- Publishing crates before a separate release change authorizes it.

## Decisions

### Seven responsibility-led crates

The workspace uses these crates:

| Crate | Owns | Must not own |
| --- | --- | --- |
| `smackdebt-analysis` | measurements, health policy, comparison, aggregation, diagnostics, report values | filesystem, Git, parser, Rayon, Serde, terminal |
| `smackdebt-languages` | detection, compiled dispatch, parser adapters, owned analyzers | file walking, Git, scheduling, output |
| `smackdebt-discovery` | one-pass inventory, ignore rules, package roots | file contents, metrics, Git history |
| `smackdebt-git` | repository discovery, refs, merge bases, status, history, renames, batch objects | source metrics, health, rendering |
| `smackdebt-project` | codebase and diff use cases, composition, source reads, Rayon pool | metric rules, terminal formatting |
| `smackdebt-output` | terminal and JSON writers over borrowed reports | source or Git analysis |
| `smackdebt` | argument parsing, dependency construction, exit codes | product policy or repository inspection |

Dependencies point inward to `smackdebt-analysis`. Infrastructure crates do not
depend on each other; `smackdebt-project` composes them. This is preferred over
three broad crates because filesystem and Git change for different reasons, and
language replacement must not disturb repository behavior. It is preferred over
one crate per language because language implementations share parser traversal,
fixtures, and one private dispatch contract.

All crates start with `publish = false`. Path and version metadata will be ready
for synchronized publication later, but `cargo install --path crates/cli` is the
development install command.

### Minimal interfaces with enforced direction

Workspace libraries export only values required by direct consumers. Modules,
parser types, callbacks, syntax nodes, upstream metric types, Rayon types,
process handles, and renderer helper types remain private.

The stable first-release product interfaces are:

- command syntax, standard streams, and exit codes;
- JSON schema version 1 and its documented field meanings.

Rust APIs remain pre-1.0 implementation seams. CI still records public API
snapshots for every library and checks workspace dependency edges from Cargo
metadata. A changed snapshot requires intentional review even though compatibility
is not promised yet.

### Flat facts and one ownership point

Inventory owns repository-relative paths once. It assigns typed indexes for
packages, scopes, and files. Analysis results use those indexes rather than
cloning paths.

Each language emits a flat per-file unit buffer. A unit contains:

- name and container identity;
- kind and source span;
- cognitive complexity, cyclomatic complexity, and exclusive logical lines;
- a parent unit index when nesting matters.

The health policy uses fixed storage for its three signals and returns an
assessment without heap allocation. Full scans retain file totals and every
Watch or High finding, but discard healthy unit details after aggregation.
Diff workers retain both versions of one changed file, compare sorted identities,
and discard unchanged healthy details before returning.

The report stores scopes, findings, diagnostics, and comparisons in flat arrays.
Scopes refer to parents, children, and findings by typed indexes. Each finding is
stored once. Terminal and JSON output borrow the same report. JSON serialization
uses a borrowed serializer view in `smackdebt-output`, avoiding both Serde in the
analysis crate and a second owned output model.

This is preferred over a recursive report tree because progressive views need
stable identity and shared findings, while recursive ownership repeats paths and
makes aggregation more allocation-heavy.

### Package roots, not package-manager graphs

One directory containing one or more recognized manifests is one report package.
Co-located manifests become ecosystem evidence on the same package. A file
belongs once to its nearest package-root ancestor. A repository with no manifest
gets `.` as its package.

The progressive hierarchy is repository, package, directory, file, container,
then code unit. The first release does not build Rust module trees, Java package
trees, Python import trees, or package dependency graphs.

### Metric extraction is specific to each metric

Smackdebt rates only cognitive complexity, cyclomatic complexity, and logical
lines. It does not carry the upstream Halstead or maintainability structures
through its core.

The temporary upstream adapter extracts direct cognitive and cyclomatic values
from each function space. Exclusive logical lines subtract direct child additive
totals. No generic subtract-children rule applies to every metric, and upstream
`CodeMetrics::merge` is not used for repository aggregation.

Each supported language has nested-unit fixtures proving direct and exclusive
values before it is listed as supported.

### Compile-time language registry and owned migration

One private language enum and one `match` select a concrete analyzer per file.
There is no trait-object registry, callback framework, runtime plugin ABI, or
dynamic library loading.

The temporary adapter pins `rust-code-analysis` revision
`37e5d83c056c8cbf827223d5814a93c5218df1a9`. Smackdebt bypasses its directory
runner and calls its per-file library entry point. The current source buffer moves
into that call once; returned data is reduced immediately to Smackdebt facts.

Verified initial upstream support is C/C++, Java, JavaScript/JSX, Python, Rust,
and TypeScript/TSX. Kotlin is not supported because its required measurement
implementations are empty in the pinned revision.

Ruby is owned from the first release. Classes and modules are containers;
methods, singleton methods, and lambdas are units. Nested control flow contributes
to cognitive and cyclomatic complexity, while nested units do not contribute to
their parent's direct measurements.

Vue is an owned document analyzer. It parses SFC structure, delegates script and
script-setup regions to JavaScript or TypeScript analysis with original file
spans, and emits a synthetic template unit for conditions, loops, conditional
expressions, logical branches, and inline handlers. Style sections contribute to
coverage but not debt measurements.

After Ruby and Vue, real repository coverage and profiles select the next
upstream replacement. Each replacement adds compatibility fixtures, an owned
implementation, performance evidence, and one registry switch. Project, Git,
analysis, report, and output crates remain unchanged.

### One Rayon pool in project orchestration

`smackdebt-project` owns one private Rayon pool. Omitted `--jobs` uses available
parallelism; `--jobs N` creates a fixed-size pool. One-file work runs on the
calling thread. Benchmarks select the higher serial-to-parallel cutover used by
the first implementation.

Discovery walks synchronously into stable path order. Current-file analysis uses
an indexed parallel iterator so collection order remains stable. Each worker
reuses its owned parser and scratch storage for owned analyzers.

Git history uses one streaming process. Base objects use one `git cat-file
--batch` process with a limited producer-consumer queue to prevent blobs from
accumulating in memory. Git output is parsed synchronously; CPU-heavy source
analysis enters the same Rayon pool. No async runtime is introduced.

### Performance is an evidence contract

The first implementation enforces these structural properties:

- one ignore-aware filesystem inventory walk;
- one read of each current file selected for analysis;
- at most two source buffers per active diff worker;
- one history process per report and one batch-object process per diff;
- no Git process per file;
- no new allocation while rating units or updating reserved aggregation arrays;
- direct terminal and JSON writes to `io::Write`;
- byte-for-byte identical serial and parallel reports;
- stable memory growth relative to worker count and retained findings.

The private Fluyt workload records revision, dirty state, host, toolchain,
supported files, source bytes, wall time, p95, peak memory, allocation counts,
and Git process counts. Public CI uses deterministic mixed-language fixtures at
one file, one hundred files, and a large generated workload approaching 100,000
files or 10 million lines. CPU and allocation profiles use Cachegrind and DHAT
where supported.

Absolute wall-time and memory limits are recorded only after the first correct
release benchmark. The accepted baseline includes ten percent regression room.
A workload change creates a new declared baseline rather than silently replacing
the old comparison.

### Security and failure isolation

Smackdebt crates forbid unsafe Rust. Parser dependencies may contain reviewed
low-level code outside the workspace.

Git commands use structured arguments and never a shell. Refs are separated from
paths with `--` where Git supports it. No configuration executes commands or
loads analyzer code. Source, paths, metrics, and history remain local.

One unreadable, oversized, unsupported, or failed file produces a coverage
diagnostic and does not stop a codebase report. Diff mode requires Git and fails
the report only when it cannot establish the requested comparison. JSON standard
output remains valid when non-fatal diagnostics exist.

## Risks / Trade-offs

- **The upstream adapter still allocates heavily** → Treat zero-copy and low
  allocation as Smackdebt-side rules, measure the adapter separately, and remove
  it language by language.
- **Seven crates add workspace overhead** → Keep interfaces small, enforce the
  exact dependency graph, and avoid crates for implementation details that can
  remain private modules.
- **Public API snapshots can create maintenance noise** → Review only intentional
  cross-crate surface changes; do not claim Rust compatibility before release.
- **Vue template metrics may not map perfectly to function metrics** → Label the
  result as a template unit, expose its raw inputs, and maintain Vue-specific
  fixtures.
- **Discarding healthy units limits JSON exploration** → Preserve every debt
  finding and all scope totals; require a future OpenSpec change before retaining
  all healthy unit details.
- **Fluyt is private and dirty state can vary** → Record state metadata and use
  generated fixtures for public regression gates.
- **Parallel work can increase peak memory** → Limit active jobs, reuse worker
  storage, keep batch queues small, and measure memory against worker count.

## Migration Plan

1. Create the seven-crate private workspace with dependency checks, API
   snapshots, and the performance harness.
2. Implement pure analysis values, policy, flat aggregation, comparison, and
   report construction.
3. Implement one-pass discovery and the verified upstream adapter set.
4. Add project orchestration, Rayon scheduling, terminal output, and JSON.
5. Add owned Ruby and Vue analysis before calling the Fluyt report usable.
6. Add streamed Git activity and hotspot ordering.
7. Add ref and worktree comparison with batch base-object reads.
8. Establish the first performance baseline and resulting absolute budgets.
9. Replace upstream language implementations according to observed use and
   profiles.

Each step is a separate OpenSpec implementation change and must leave the
existing black-box flows passing. Before publication, a release change removes
`publish = false`, publishes libraries in dependency order, and publishes the
CLI last. Before that point, rollback is removal of the incomplete private
workspace change; no user data migration exists.

## Open Questions

None. Performance numbers that require implementation evidence are explicitly
deferred to the first baseline rather than left as design choices.
