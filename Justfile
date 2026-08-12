set shell := ["sh", "-cu"]

fmt:
    cargo fmt --all -- --check

lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

test:
    cargo test --workspace --all-features

architecture:
    python3 scripts/check-dependency-direction.py
    python3 scripts/check-api-snapshots.py
    python3 -m unittest discover -s scripts/tests

licenses:
    cargo deny check licenses bans sources

performance-tests:
    python3 -m unittest scripts/performance/test_workload.py
    python3 scripts/performance/check-baselines.py

check: fmt lint test architecture performance-tests
    openspec validate --all --strict
    git diff --check
