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
    cargo build -p smackdebt --bin smackdebt --features evidence-stats
    cargo test -p smackdebt --features evidence-stats --test unified_acceptance selected_binary_contains_the_requested_evidence_feature -- --exact
    cargo test -p smackdebt --features evidence-stats --test unified_acceptance composition_work_counts

show-snapshot case:
    python3 -m json.tool --indent 2 crates/cli/tests/snapshots/{{case}}

acceptance-install:
    cargo test -p smackdebt --test unified_acceptance installed_command_runs_outside_the_workspace -- --ignored --exact

architecture:
    python3 scripts/check-dependency-direction.py
    python3 scripts/check-entry-modules.py
    python3 scripts/check-api-snapshots.py
    python3 scripts/check-api-snapshots.py --all-features --package smackdebt-git --package smackdebt-project
    python3 -m unittest discover -s scripts/tests

gate:
    cargo run --quiet -p smackdebt --bin smackdebt -- gate

licenses:
    cargo deny check licenses bans sources

performance-tests:
    python3 -m unittest discover -s scripts/performance -p 'test_*.py'
    python3 scripts/performance/check-baselines.py

release-workflow-tests:
    python3 -m unittest scripts/performance/test_release_head.py

release-evidence-check:
    python3 scripts/performance/check-baselines.py --release-head
    python3 scripts/performance/check-workload-reviews.py --release-head

check: fmt lint test architecture performance-tests acceptance-evidence gate
    git diff --check

release-baselines self mixed rust accept="":
    scripts/performance/release-baselines.sh {{self}} {{mixed}} {{rust}} {{accept}}
    just release-evidence-check

release-evidence self mixed rust accept="": check licenses acceptance-install (release-baselines self mixed rust accept)
