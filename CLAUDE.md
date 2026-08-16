# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Read first

`AGENTS.md` is the engineering guide and holds the binding rules for this repo
(architecture boundaries, data/domain rules, language analysis, performance,
Rust style, tests, security). Follow it. `ARCHITECTURE.md` explains the
mechanisms behind those rules, and `README.md` is the product contract
(terminal shapes, thresholds, exit codes, JSON schema). This file only adds
orientation and commands.

## Commands

`just` is the entry point; every recipe is in `Justfile`.

```console
just fmt          # cargo fmt --all -- --check
just lint         # clippy --workspace --all-targets --all-features -D warnings
just test         # cargo test --workspace --all-features
just check        # fmt + lint + test + architecture + performance-tests
                  # + acceptance-evidence, then openspec validate --strict, git diff --check
```

Narrower loops:

```console
just acceptance             # both black-box CLI suites
just acceptance-evidence    # work-count checks (needs the evidence-stats feature)
just acceptance-install     # slow install smoke, ignored by default
just architecture           # dependency direction, entry modules, API + evidence snapshots
just licenses               # cargo deny check licenses bans sources
just performance-tests      # workload unit tests + baseline check
```

Single test / package:

```console
cargo test -p smackdebt-analysis                       # one crate
cargo test -p smackdebt --test unified_acceptance <name>
cargo test -p smackdebt --test unified_acceptance <name> -- --exact
```

Snapshots are read-only in normal runs. Update deliberately, then review the
printed paths:

```console
just update-unified-snapshot unified-codebase.json   # one result
just update-unified-snapshot all                     # only when every change is intended
just update-acceptance-snapshots
```

API snapshots under `api-snapshots/` are checked by `just architecture`;
regenerate via `python3 scripts/check-api-snapshots.py` variants only when a
cross-crate surface change is intended.

## OpenSpec is the source of truth

Accepted behavior and implementation order live in `openspec/`:
`openspec/specs/<capability>/` for accepted specs, `openspec/changes/<name>/`
(proposal, design, specs, tasks) for in-flight work, `openspec/changes/archive/`
for completed ones. Before implementing, read the active change; do not start
until `openspec validate --all --strict` passes for it, and tick task
checkboxes only after the behavior and its tests pass.

## Architecture in one pass

Workspace crates live under `crates/` (directory name ≠ crate name):

| Path | Crate | Owns |
| --- | --- | --- |
| `crates/analysis` | `smackdebt-analysis` | measurements, health policy, graph algorithms, aggregation, comparisons, report values |
| `crates/languages` | `smackdebt-languages` | detection, compiled tree-sitter dispatch, per-grammar dependency syntax |
| `crates/discovery` | `smackdebt-discovery` | one ignore-aware inventory walk, package assignment |
| `crates/git` | `smackdebt-git` | repo facts, refs, history, status, batch object reads |
| `crates/project` | `smackdebt-project` | codebase/diff use cases, composition, the only Rayon pool |
| `crates/output` | `smackdebt-output` | terminal and JSON writers over borrowed report data |
| `crates/cli` | `smackdebt` | arguments, wiring, streams, exit codes |

Dependencies point toward `analysis`. Adapter crates (`languages`,
`discovery`, `git`) never depend on each other — `project` composes them.
`lib.rs`/`mod.rs` are wiring only: private modules, imports, and a small
explicit reexport surface; no behavior, no inline modules, no tests. These are
enforced by `scripts/check-dependency-direction.py` and
`scripts/check-entry-modules.py` in `just architecture`, plus `unreachable_pub`
and `unsafe_code = "forbid"` workspace lints.

Data flow: discovery walks once and owns repository-relative paths → languages
parse each file once per worker (measurements and dependency syntax in the same
traversal) → analysis rates units and aggregates flat, index-linked tables into
a read-only report built through `ReportBuilder` → output streams terminal or
JSON from borrowed report data without rerunning analysis. Scope hierarchy is
repository → package → directory → file → container → unit.

Two invariants that break silently if ignored: serial and parallel runs must
produce byte-identical terminal and JSON output, and Git usage stays at one
streamed history process plus one `git cat-file --batch` — never a process per
file.

## Where tests live

Pure policy tests sit beside the rules in `crates/analysis`; language truth
fixtures in `crates/languages`; adapter tests at the discovery/git seams.
Black-box CLI evidence is `crates/cli/tests/acceptance.rs` (focused, per
analysis family) and `crates/cli/tests/unified_acceptance.rs` (generated
repository domain and the release matrix), with committed bytes in
`crates/cli/tests/snapshots/`. See `crates/cli/tests/README.md`. A policy
change belongs first in a pure test; a public behavior change also needs schema
review (`schemas/report-v4.schema.json`) and updated black-box evidence.

## Conventions

Rust 2024, MSRV pinned in `Cargo.toml` (`rust-version`); all crates are
`publish = false`. Conventional commit messages. Shell commands are prefixed
with `rtk`.
