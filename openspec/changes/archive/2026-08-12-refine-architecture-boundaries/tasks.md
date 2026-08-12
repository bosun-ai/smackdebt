## 1. Workspace boundaries

- [x] 1.1 Replace the root package with the seven private workspace crates and shared Rust 1.97, edition 2024, license, lint, and release settings
- [x] 1.2 Add a Cargo-metadata check that accepts only the dependency edges defined by `workspace-architecture`
- [x] 1.3 Add checked public API snapshots for all six library crates and fail CI when an export changes unintentionally
- [x] 1.4 Add dependency license checks and verify that no workspace crate permits unsafe Rust
- [x] 1.5 Add focused tests for dependency-direction and API-snapshot scripts so failure output identifies the offending crate or edge

## 2. Pure analysis core

- [x] 2.1 Implement typed package, scope, file, unit, and finding indexes plus source spans, unit kinds, and the three retained measurements
- [x] 2.2 Implement the configurable health policy with fixed signal storage and tests for Healthy, Watch, High, and highest-signal behavior
- [x] 2.3 Implement flat scope, finding, diagnostic, coverage, activity, comparison, and report values without filesystem, Git, parser, Rayon, or Serde dependencies
- [x] 2.4 Implement one-pass post-order scope aggregation with capacity reservation and tests proving each unit and finding is counted once
- [x] 2.5 Implement sorted unit identity comparison for added, removed, improved, regressed, metric-changed, ambiguous, and unchanged units
- [x] 2.6 Add allocation-instrumented tests proving rating and reserved aggregation updates allocate no heap memory

## 3. Discovery and package identity

- [x] 3.1 Implement one ignore-aware filesystem walk that records stable relative paths and metadata without reading source contents
- [x] 3.2 Recognize Cargo, npm, Python, Maven, Gradle, CMake, Bundler, and gemspec manifests during the same walk
- [x] 3.3 Merge co-located manifests into one package root and assign every file once to its nearest package ancestor
- [x] 3.4 Add fixtures for nested packages, several manifests in one directory, no manifest, ignored source, unreadable paths, links, generated directories, and paths outside Git
- [x] 3.5 Instrument inventory and source access so acceptance tests can assert one walk and at most one current-file read

## 4. Language analysis

- [x] 4.1 Add the private compiled language identifier and static dispatch without exposing analyzer traits, callbacks, or parser types
- [x] 4.2 Pin `rust-code-analysis` to the specified revision and implement a per-file adapter that bypasses its walker, threads, channels, and output
- [x] 4.3 Reduce upstream results directly to Smackdebt units using direct cognitive and cyclomatic values and exclusive additive logical lines
- [x] 4.4 Add nested-unit compatibility fixtures for C/C++, Java, JavaScript/JSX, Python, Rust, and TypeScript/TSX before enabling each registry entry
- [x] 4.5 Add unsupported coverage tests proving Kotlin and unknown languages are never reported as healthy
- [x] 4.6 Implement owned Ruby containers, methods, singleton methods, lambdas, control-flow measurements, parser recovery, and original spans
- [x] 4.7 Implement owned Vue SFC discovery, JavaScript and TypeScript script delegation, synthetic template measurements, style coverage, and original spans
- [x] 4.8 Add Ruby and Vue fixtures drawn from representative Fluyt patterns without copying private product source into public fixtures
- [x] 4.9 Document and test the one-entry replacement procedure for moving an upstream-backed language to owned analysis

## 5. Project orchestration and output

- [x] 5.1 Implement codebase and diff request value objects with automatic or explicit non-zero execution width
- [x] 5.2 Create one private Rayon pool in project orchestration and reuse per-worker parser and scratch state for owned analyzers
- [x] 5.3 Keep one-file work serial, add serial and parallel execution paths, and collect indexed results in stable order
- [x] 5.4 Apply health policy on analysis workers, retain all Watch and High findings, and reduce healthy units to scope counts
- [x] 5.5 Implement concise terminal rendering from a borrowed report with width-aware layout, stable ranking, `NO_COLOR`, and one useful drill command
- [x] 5.6 Implement streaming JSON schema version 1 from a borrowed report view without adding Serde to the analysis crate
- [x] 5.7 Implement the thin CLI for path discovery, `diff`, `--history`, `--json`, `--jobs`, output routing, and documented exit codes
- [x] 5.8 Add byte-for-byte acceptance snapshots proving serial and parallel terminal and JSON output are identical

## 6. Git activity and comparison

- [x] 6.1 Implement safe structured Git command execution without shell interpolation and with refs separated from paths
- [x] 6.2 Implement repository discovery, default-ref selection, merge-base resolution, worktree status, renames, and non-ignored untracked files
- [x] 6.3 Implement one streamed non-merge history process with rename-aware touch counting and no per-file Git calls
- [x] 6.4 Add file activity to report construction and implement visible hotspot ordering without a hidden numeric score
- [x] 6.5 Implement one `git cat-file --batch` reader with a limited queue feeding the existing project worker pool
- [x] 6.6 Analyze each changed base and worktree file at most once and compare named units on the same worker
- [x] 6.7 Implement file-level fallback diagnostics for ambiguous identities, parser failures, unsupported sides, and unsafe ref input
- [x] 6.8 Add process instrumentation proving Git process count stays fixed as history and diff file counts grow
- [x] 6.9 Add acceptance fixtures covering committed, staged, unstaged, renamed, deleted, and untracked changes plus absent default refs

## 7. Performance evidence

- [x] 7.1 Add deterministic one-file, one-hundred-file, small-diff, and large mixed-language generated workloads with correctness checks outside measured intervals
- [x] 7.2 Add a private Fluyt benchmark command that records revision, dirty state, host, toolchain, supported files, and source bytes
- [x] 7.3 Record wall time, p95, peak memory, allocation counts, Git process counts, inventory visits, and source reads for codebase and diff workflows
- [x] 7.4 Add Cachegrind and DHAT harnesses for complete CLI workflows where the host supports them
- [x] 7.5 Measure serial-to-parallel crossover and set the first internal cutover from evidence
- [x] 7.6 Establish the first checked baseline and absolute latency and memory budgets with ten percent regression room
- [x] 7.7 Document how workload changes declare a new baseline and how regressions are investigated before changing a budget

## 8. Documentation and release readiness

- [x] 8.1 Update `ARCHITECTURE.md` with the enforced crate diagram, ownership lifetimes, metric extraction, language migration, Git process shape, and performance gates
- [x] 8.2 Update the README installation text for private development and list only implemented verified language behavior
- [x] 8.3 Document co-located package manifests and JSON retention of every Watch and High finding plus aggregate healthy counts
- [x] 8.4 Convert all implemented README command examples into black-box acceptance snapshots
- [x] 8.5 Run formatting, warning-free workspace linting, all tests, dependency checks, API snapshots, license checks, performance gates, strict OpenSpec validation, and Markdown diff checks
- [x] 8.6 Leave every crate private and create a separate release OpenSpec change before enabling registry publication
