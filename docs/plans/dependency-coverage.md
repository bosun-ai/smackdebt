# Static dependency coverage

Keep cycle algorithms, ratings, source roles, and CLI/JSON schema 4 unchanged.
Fix language extraction and repository resolution; never execute project code.
Starting revision: `f2e07596dd25a3505b8c828efa0802041459defc`.

## Tasks

- [x] Collect multiple references per syntax node in reusable worker storage.
- [x] Fix JavaScript computed targets and Vue external scripts.
- [x] Resolve Python grouped imports and package submodules.
- [x] Index Java declarations and resolve C# static receivers with shadowing.
- [x] Resolve Rust path overrides and static Ruby load roots.
- [x] Resolve C/C++ includes from compilation database search paths.
- [x] Retain Go/PHP coverage and add the language cycle acceptance matrix.
- [x] Retain exhaustive graph/witness regression checks.
- [x] Update docs and intentional API/acceptance snapshots.
- [x] Run focused/full checks, independent review, and local project smoke tests.

## Decisions

- Static source and declarative configuration only; framework auto-imports,
  reflection, and execution of build files remain outside scope.
- Keep headline behavior separate from this change.
- Work in the existing worktree; preserve other project work.
- Language extraction remains in languages; repository lookup and configuration
  remain in project; discovery only extends its existing metadata inventory.
- Retain diagnostics for uncertain targets rather than guessing edges.

## Verification log

Initial investigation: all 14 basic language variants reported cycles. Expanded
probes found silent omissions in Python, Java, C#, Vue, C/C++, and Ruby; Rust path
overrides produced an unresolved reference; JavaScript computed require targets
could invent edges. The graph and witnesses passed 65,536 four-node graphs
against an independent reachability oracle.

Initial smoke reports are in `/tmp/smackdebt-smoke-*.json`: Fluyt 2,494 files,
8 file-cycle groups; Keploy 262 files, no cycles, five unresolved imports;
scikit-learn 951 files, one cycle group; cmux 17,527 files, 22 groups overall,
458 Java files with zero internal Java edges. All reports had incomplete graph
evidence. No standalone PHP/C# project was available; Pascal selected no source.

Implementation verification: the CLI cycle matrix covers C, C++, C#, Go, Java,
JavaScript, JSX, PHP, Python, Ruby, Rust, TypeScript, TSX, and Vue, with acyclic
controls, closed witnesses, source spans, and identical one-worker/default
output. Mixed JavaScript/TypeScript/Vue and cross-package cycles also pass.
The exhaustive graph check is retained as a permanent test.

Independent review reproduced and verified corrections for Java nested types,
Python absolute imports in namespace directories, ordinary Rust children under
path overrides, conditional Ruby load paths, and joined C forced-include flags.
The existing workspace-manifest Python acceptance test also caught a lost
fallback; it was restored without changing that test's assertions.

Smoke runs used `--json --history 1d --jobs 2` on local repositories, with cmux
selected at `cmux-tui/bindings/java`. Reports are `/tmp/smackdebt-after-*.json`.
These are correctness smoke runs, not performance benchmarks.

| Repository | Files | Dependency edges | File cycle groups |
| --- | ---: | ---: | ---: |
| Fluyt | 2,495 | 5,325 | 8 |
| Keploy | 262 | 4,890 | 0 |
| scikit-learn | 951 | 5,744 | 10 |
| cmux (whole-repository graph) | 17,527 | 10,682 | 30 |

cmux now resolves 2,280 Java edges and reports five Java cycle groups. A sampled
witness is Client -> Machine -> Client: Client constructs Machine, while Machine
stores a Client field. Keploy's Go graph is unchanged. Fluyt changed concurrently
(one additional source file and two additional dirty paths), so its totals are
not an exact before/after comparison. Revisions remained d9ba1f225 (Fluyt),
aa0b0a99 (Keploy), 6a2472fa5e (scikit-learn), and 17a7a91fcd (cmux); the latter
two repositories stayed clean. All four reports still flag incomplete evidence.
No private source was copied into fixtures.

The local debt gate first rejected the larger extraction and resolution functions.
They were split by language responsibility, with tests preserved, and no baseline
was changed. Generated `hundred-file` and `graph-sparse` workloads passed report
validation and identical serial/automatic output. Each recorded one inventory
walk, one source read per source file (100 and 1,000), and two Git processes.
Debug runs captured allocation, memory, and timing records under
`/tmp/smackdebt-cycle-perf-{hundred,graph}-verified`; these single samples do not
establish a release performance baseline.

Final validation: `rtk just check` passed (format, workspace Clippy, full tests,
architecture and API checks, performance harness tests, acceptance work counters,
and the debt gate). `rtk just licenses` and `rtk git diff --check` passed. The debt
baseline and dependency lockfile are unchanged. Four intentional JSON snapshot
updates reflect more complete Java extraction and honest computed-target evidence;
cycle ratings, headline behavior, and report schema remain unchanged.

Final smoke reports (`/tmp/smackdebt-final-*.json`) completed successfully on all
four repositories after cleanup. Every dependency-edge table and cycle-finding
table matches its earlier post-fix report, including cmux's Java results.
