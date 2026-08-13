## 1. Acceptance fixture domain

- [x] 1.1 Add descriptive test value objects for generated repositories,
  commits, worktree edits, invocations, and complete process results.
- [x] 1.2 Create unique temporary repositories with isolated Git configuration,
  fixed identities, fixed dates, explicit branches, and cleanup owned by the
  fixture lifetime.
- [x] 1.3 Add fact manifests with hand-calculated values shared by focused
  assertions and public-output cases.

## 2. Unified repositories

- [x] 2.1 Add `all-languages` nested-unit fixtures that prove exact source
  metrics for every supported language.
- [x] 2.2 Add `static-architecture` fixtures for resolved, external, unresolved,
  ambiguous, file-cycle, package-cycle, degree, and instability facts.
- [x] 2.3 Add `evolution` fixtures for churn, renames, binary changes, repeated
  coupling, mailmap normalization, concentration, and privacy.
- [x] 2.4 Add `worktree-change` fixtures covering metric, dependency, cycle,
  rename, delete, untracked, and coupling-explanation outcomes.
- [x] 2.5 Add `coverage-failures` fixtures for unsupported and failed source,
  incomplete history, invalid refs, and outside-Git behavior.

## 3. Real CLI process harness

- [x] 3.1 Invoke the built CLI with explicit directory, arguments, environment,
  width, color, and workers and capture status, stdout, and stderr bytes.
- [x] 3.2 Assert success, policy, partial-coverage, invocation-failure, and
  report-failure exit and stream contracts.
- [x] 3.3 Keep the harness outside internal report construction and analysis
  APIs so every case crosses the public CLI seam.

## 4. Exact report evidence

- [x] 4.1 Add committed terminal results for default, `--all`, selected file,
  selected directory, selected package, codebase, ref-diff, and worktree-diff
  flows.
- [x] 4.2 Add terminal results at widths 120, 80, and 50 and prove forced-color
  output has the same visible text as color-disabled output.
- [x] 4.3 Validate every JSON result against the committed version-2 schema,
  assert key semantic values, and compare exact bytes.
- [x] 4.4 Prove one-worker and automatic-parallel terminal and JSON bytes are
  identical for the complete analysis families.
- [x] 4.5 Add an explicit local golden-update command that selects cases and
  reports changed files; ensure ordinary tests and CI cannot update evidence.

## 5. Work and privacy evidence

- [x] 5.1 Instrument inventory visits, source and object reads, Git processes,
  parser visits, algorithm passes, and renderer entry behind test-only hooks.
- [x] 5.2 Prove terminal and JSON rendering do not discover, read, parse, start
  Git, or rerun analysis after receiving the report.
- [x] 5.3 Prove generated fixture contributor names and addresses are absent from
  terminal, JSON, diagnostics, and golden files.
- [x] 5.4 Add complete-flow allocation, peak-memory, source-read, object-read,
  Git-process, wall-time, and p95 measurements for generated mixed repositories.

## 6. Installation and documentation

- [x] 6.1 Install locked local crates into a unique temporary prefix and run the
  installed CLI from a generated repository outside the workspace.
- [x] 6.2 Prove installed help, version, terminal report, JSON report, status, and
  stream behavior.
- [x] 6.3 Mark executable README console examples, run them against named public
  fixtures, and compare their documented behavior.
- [x] 6.4 Update `ARCHITECTURE.md`, schema documentation, test documentation, and
  release instructions with the evidence layers and update workflow.

## 7. Release gate

- [x] 7.1 Prove every requirement from the source engine, static architecture,
  evolutionary analysis, and unified evidence changes is represented in the
  acceptance matrix or a named lower-level truth test.
- [x] 7.2 Run formatting, Clippy, workspace tests, strict OpenSpec validation,
  dependency checks, API snapshots, license checks, acceptance snapshots,
  allocation checks, performance workloads, and diff checks.
- [ ] 7.3 Run the complete evidence from a clean revision and record workload,
  revision, dirty state, toolchain, host, source bytes, supported files, timing,
  memory, allocations, reads, and Git process counts.
- [ ] 7.4 Keep all crates private until the separate publication decision and
  recorded clean-revision evidence both pass.

The complete implementation evidence is recorded from this working revision;
tasks 7.3 and 7.4 remain open until the orchestrator reviews and commits the
change, then reruns the clean-revision release command. All workspace crates
still set `publish = false`.
