set shell := ["sh", "-cu"]

fmt:
    cargo fmt --all -- --check

lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

test:
    cargo test --workspace --all-features

update-source-snapshots:
    SMACKDEBT_UPDATE_COMMAND=1 SMACKDEBT_UPDATE_SNAPSHOTS=1 cargo test -p smackdebt --test acceptance source_engine

update-acceptance-snapshots:
    SMACKDEBT_UPDATE_COMMAND=1 SMACKDEBT_UPDATE_SNAPSHOTS=1 cargo test -p smackdebt --test acceptance

update-unified-snapshot case:
    SMACKDEBT_UPDATE_COMMAND=1 SMACKDEBT_UPDATE_CASE={{case}} cargo test -p smackdebt --test unified_acceptance

acceptance:
    cargo test -p smackdebt --test acceptance
    cargo test -p smackdebt --test unified_acceptance

acceptance-evidence:
    cargo test -p smackdebt --features evidence-stats --test unified_acceptance composition_work_counts

acceptance-install:
    cargo test -p smackdebt --test unified_acceptance installed_command_runs_outside_the_workspace -- --ignored --exact

architecture:
    python3 scripts/check-dependency-direction.py
    python3 scripts/check-entry-modules.py
    python3 scripts/check-api-snapshots.py
    python3 scripts/check-api-snapshots.py --all-features --package smackdebt-git --package smackdebt-project
    python3 scripts/check-evidence-map.py
    python3 -m unittest discover -s scripts/tests

licenses:
    cargo deny check licenses bans sources

performance-tests:
    python3 -m unittest scripts/performance/test_workload.py
    python3 scripts/performance/check-baselines.py

check: fmt lint test architecture performance-tests acceptance-evidence
    openspec validate --all --strict
    git diff --check

release-baselines:
    scripts/performance/release-baselines.sh

release-evidence: check licenses acceptance-install release-baselines
