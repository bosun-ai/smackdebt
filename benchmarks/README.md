# Performance workloads

The public harness creates mixed-language repositories without copying private
source. It records workload identity before a CLI command is measured and
checks the generated files outside the measured interval.

```console
python3 scripts/performance/workload.py generate \
  --output benchmarks/workloads/one-file \
  --profile one-file
python3 scripts/performance/workload.py check \
  --input benchmarks/workloads/one-file
python3 scripts/performance/workload.py metadata \
  --input benchmarks/workloads/one-file \
  --output benchmarks/results/one-file.metadata.json
```

Profiles are `one-file`, `hundred-file`, `small-diff`, `large-mixed`,
`graph-sparse`, `graph-dense`, `many-package`, `evolution-dense`, and
`large-dependency-diff`.
The large profile defaults to 100,000 source files and can be overridden with
`--files` for local iteration. The small-diff profile creates a committed base
and four modified files so diff workflows exercise a real worktree.
The graph profiles cover sparse imports, dense package relationships, many
package identities, and a 200-file dependency diff. Each graph source has a
checked target shape before timing begins.
`evolution-dense` creates 100 packages changed together in two commits. Its
codebase command uses the full generated history so the measured report proves
pair deduplication, one streamed history process, allocations, and peak memory.

Once the CLI exists, run it with the wrapper. The wrapper generates and checks
the workload first, then appends one wall-time record per invocation:

```console
scripts/performance/run.sh \
  --profile hundred-file \
  --output benchmarks/workloads/hundred-file \
  --repeat 5 \
  -- cargo run --release --manifest-path Cargo.toml --
```

Run correctness checks before this command and compare serial and automatic
parallel output byte-for-byte. The wrapper does not treat a faster but
different report as a passing performance result.

## Profiling

Cachegrind and DHAT wrap the complete command and are optional on hosts that
provide Valgrind:

```console
scripts/performance/profile.sh cachegrind benchmarks/results/cachegrind -- \
  cargo run --release --manifest-path Cargo.toml -- benchmarks/workloads/one-file
scripts/performance/profile.sh dhat benchmarks/results/dhat -- \
  cargo run --release --manifest-path Cargo.toml -- benchmarks/workloads/one-file
```

The profile commands return 77 when Valgrind is unavailable. They do not make a
public CI run depend on a profiler that the host cannot provide.

## Private repository run

Use the Fluyt wrapper only on a local checkout that is allowed to contain its
source. It reads metadata in place and does not copy source into the benchmark
tree:

```console
scripts/performance/fluyt.sh \
  --repo /path/to/fluyt \
  --output benchmarks/results/fluyt.json \
  -- cargo run --release --manifest-path /path/to/smackdebt/Cargo.toml -- .
```

The record includes the repository revision, dirty state, host, toolchain,
source bytes, language counts, and complete-flow wall time.

## Baselines

The first correctness-checked one-file and hundred-file measurements are in
`benchmarks/baselines`. Each record includes metadata, revision, dirty state,
toolchain, source bytes, supported-file count, wall time, p95, peak memory,
filesystem reads, Git processes, and absolute budgets.

Keep the workload profile, seed, source digest, and command in the same record.
If any source fixture or generation rule changes materially, create a new
baseline name and explain the difference. Investigate a regression before
raising a limit; the first limits should include ten percent room over the
measured value.

Allocation values use the CLI's `allocation-stats` release feature. The checked
analysis test separately proves that rating and a prepared aggregation update
make no allocations.
