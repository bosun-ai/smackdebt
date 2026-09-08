# Smackdebt engineering guide

Smackdebt is a fast local CLI for answering two questions:

- Where does a codebase carry the most costly debt?
- Did the current worktree improve or worsen that debt compared with a Git ref?

Read `README.md` and `ARCHITECTURE.md` before implementation.

Output must be concise, useful, beautiful, and actionable. Avoid noise.

@/Users/timonv/.codex/RTK.md

## Workflow

- Use `rtk` as the prefix for shell commands.
- Work through changes in dependency order and update task checkboxes
  only after the related behavior and tests pass.
- Keep each implementation change small enough to validate at its crate seam
  before running the complete workspace checks.
- Update architecture, product documentation, and acceptance examples
  together when verified behavior changes.
- Preserve unrelated work. Do not rewrite user changes.

## Architecture

Keep these responsibilities separate:

- `smackdebt-analysis`: measurements, health policy, comparison, aggregation,
  diagnostics, and report values.
- `smackdebt-languages`: language detection, compiled dispatch, temporary
  upstream adapters, and owned analyzers.
- `smackdebt-discovery`: one-pass inventory, ignore rules, and package roots.
- `smackdebt-git`: repository discovery, refs, merge bases, history, renames,
  status, and batch object reads.
- `smackdebt-project`: codebase and diff use cases, source reads, composition,
  and the private Rayon pool.
- `smackdebt-output`: terminal and JSON writing from borrowed reports.
- `smackdebt`: argument parsing, dependency construction, output selection, and
  exit codes.

Dependencies point toward `smackdebt-analysis`. Adapter crates do not depend on
each other; `smackdebt-project` composes them.

- Keep business rules free of filesystem, Git, parser, terminal, Rayon, and
  serialization concerns.
- Give each crate, module, type, and function one clear responsibility.
- Prefer composition and concrete value objects over broad traits or primitive
  parameters.
- Add an abstraction only when it removes real duplication or protects a clear
  seam.
- Keep public exports small. Parser nodes, upstream metric types, callback
  systems, worker types, process handles, and renderer helpers stay private.
- Do not create mapping layers between several nearly identical structures.
  Improve the owning data model or convert once at the adapter edge.
- Keep all crates private until a separate release change authorizes publishing.

The supported first-release interfaces are CLI behavior, exit codes, standard
stream behavior, and JSON schema version 1. Internal Rust APIs are not promised
to external consumers, but their checked snapshots must change intentionally.

## Data and domain rules

- Inventory owns repository-relative paths and package identities once. Shared
  analysis and report data refers to typed indexes instead of cloning paths.
- Use flat tables for scopes, units, findings, diagnostics, and comparisons.
- Store each debt finding once and let scope summaries refer to it.
- Model one report package per directory containing one or more recognized
  manifests. Assign each source file once to its nearest package root.
- Keep the progressive hierarchy repository, package, directory, file,
  container, then code unit.
- Retain cognitive complexity, cyclomatic complexity, exclusive logical lines,
  maximum nesting depth, and parameter count as the rated measurements. A unit's
  rating must be explainable from those five serialized values alone.
- Apply metric-specific extraction rules. Direct complexity and additive lines
  do not share one generic child-subtraction rule.
- Full codebase reports retain every Watch and High finding plus aggregate
  healthy counts. They do not retain one owned record for every healthy unit.
- Terminal and JSON output consume the same report and must not run analysis.

## Language analysis

- Use one private compiled language identifier and one static dispatch per file.
- Do not add runtime plugins, dynamic analyzer loading, or a public callback
  framework.
- Never leak upstream parser or metric values across the language crate seam.
- List a language as supported only after fixtures prove meaningful cognitive,
  cyclomatic, and line behavior.
- Ruby and Vue are owned Smackdebt analyzers from the first usable release.
- Treat Vue as a document: analyze JavaScript or TypeScript script regions and a
  separate synthetic template unit while retaining original file spans.
- Replace upstream implementations one language at a time. Compatibility
  fixtures and higher-level acceptance output must remain stable.
- Unsupported and failed files remain visible through coverage diagnostics and
  are never counted as healthy.

## Performance

Performance claims require evidence from complete, correctness-checked flows.

- `smackdebt-project` owns the only Rayon pool.
- Run one-file work on the calling thread. Choose the parallel cutover from
  measured evidence.
- Walk the selected source tree once and read each selected current file at most
  once.
- Keep source memory proportional to active workers. A diff worker may hold the
  before and after source for its current file, not the whole diff.
- Reuse parser and scratch storage per worker. Do not serialize parsers behind a
  shared mutex.
- Use one streamed Git history process and one `git cat-file --batch` process.
  Never start a Git process per source file.
- Apply backpressure between batch object reads and analysis workers.
- Reserve report storage before aggregation. Rating and aggregation hot loops
  should not allocate.
- Stream terminal and JSON output directly to `io::Write` without cloning the
  report.
- Serial and parallel runs must produce byte-for-byte identical terminal and
  JSON output.
- Do not add async I/O or a persistent analysis cache in the first release.
- Record workload identity, revision, dirty state, toolchain, host, source bytes,
  supported files, wall time, p95, peak memory, allocation counts, filesystem
  reads, and Git process counts.
- Use Fluyt as the private mixed-repository workload. Use deterministic generated
  repositories for public performance checks.
- Set absolute budgets only from a correct measured baseline. Declare a new
  baseline when the workload changes.

## Rust

- Use Rust 2024 and the workspace's pinned minimum toolchain.
- Forbid unsafe Rust in every Smackdebt crate.
- Borrow where ownership is unnecessary and avoid clones on source, path, and
  report hot paths.
- Use `Cow` only when a value can genuinely be either borrowed or owned.
- Prefer typed indexes, non-zero values, enums, and validated value objects over
  loosely related primitives.
- Iterate large collections once where practical. Avoid N+1 filesystem, Git, and
  parser work.
- Keep stable output ordering explicit after parallel work.
- Write errors in product language and keep recoverable file failures separate
  from report-level failures.
- Use conventional commit messages.

## Tests

- Tests must describe the business rule they prove.
- Keep pure policy tests free of filesystem and process setup.
- Add nested-unit fixtures for every supported language.
- Add black-box snapshots for documented terminal and JSON flows.
- Run acceptance fixtures with one worker and automatic parallelism and compare
  output bytes.
- Instrument source reads, inventory visits, Git processes, allocations, and
  peak memory where the related contract matters.
- Use generated mixed-language repositories for public scaling tests. Do not copy
  private Fluyt source into fixtures.
- Preserve existing high-level tests when refactoring internal seams unless a
  product requirement changed.

## Security and privacy

- Never commit secrets or private repository contents.
- Keep source, paths, metrics, and Git history local.
- Build Git commands from structured arguments and never invoke a shell.
- Separate refs from paths with `--` where Git supports it.
- Configuration must not execute commands or load analyzer code.
- Fail safely when a ref, path, object, or parser result cannot be trusted.

## Validation

Run the focused crate checks while working, then the complete applicable gate:

```console
rtk cargo fmt --all -- --check
rtk cargo clippy --workspace --all-targets --all-features -- -D warnings
rtk cargo test --workspace --all-features
rtk git diff --check
```

Also run dependency-direction checks, API snapshots, license checks, acceptance
snapshots, allocation checks, and performance workloads once their tasks add
those commands.
