# Releasing Smackdebt

Smackdebt ships through GitHub Releases, `bosun-ai/homebrew-tap`, and crates.io.
Binary targets are Linux x86_64, Intel macOS, and Apple Silicon macOS. The CLI and
its six workspace dependencies share a version. Internal Rust APIs remain
implementation details, even though their crates are published to support installation.

## One-time setup

The `bosun-ai` organization already provides `CARGO_REGISTRY_TOKEN` and
`HOMEBREW_TAP_TOKEN` to all repositories, including Smackdebt. Smackdebt also has
a repository-level `RELEASE_PLZ_TOKEN`, which takes precedence over the organization
secret with the same name. Check their access before the first publication:

- `RELEASE_PLZ_TOKEN`: access to `bosun-ai/smackdebt`, with Contents and Pull requests
  read/write. Its owner must have repository write access. Keep the custom token:
  tags and PRs created with the built-in token do not start subsequent workflows.
- `CARGO_REGISTRY_TOKEN`: crates.io `publish-new` and `publish-update` access for
  the seven `smackdebt` packages. Confirm ownership or availability of those names.
- `HOMEBREW_TAP_TOKEN`: Contents write access to `bosun-ai/homebrew-tap`.

GitHub exposes secret names and visibility, but not token values or their effective
permissions. The first publication verifies registry and tap access in the workflows.

Allow Actions to create PRs and require the three check jobs before merging release PRs.
Never put tokens in the repository. Trusted publishing can replace the Cargo token
once the crates have been published and their trusted publisher settings are configured.

## Release flow

1. Release-plz opens or updates a version PR after pushes to `master`. Review the
   version, changelog, and passing checks. Only merging a release PR authorizes publication.
2. Release-plz publishes workspace crates in dependency order and creates the CLI's
   `vVERSION` tag. Cargo-dist alone owns GitHub Release creation.
3. Cargo-dist builds binaries, checksums, shell installers, and the Homebrew formula.
   Full checks and smoke tests against finished artifacts must pass.
4. Cargo-dist creates the public GitHub Release with notes and assets.
5. The post-release workflow checks public downloads and cargo-binstall on each
   supported platform. For stable releases, it installs the generated Homebrew
   formula, then commits it to the existing tap. An unchanged formula creates no commit.
   Prereleases do not update the stable Homebrew formula.

Registry publication precedes binary builds; cargo-binstall can report unavailable
binaries until the GitHub Release completes. It will not silently compile from source.
Private-workload evidence is an optional maintainer check and does not block publication.

## Develop and validate

Use Rust 1.97.0, Python 3.11+, `just`, nightly plus cargo-public-api 0.52.0,
cargo-deny, cargo-dist 0.32.0, and cargo-binstall.
Install Python dependencies from `scripts/requirements.txt` in a virtual environment.

```sh
python3 scripts/sync-agent-skill.py --check
just check
just licenses
just acceptance-install
cargo package --workspace --locked
dist generate --check
dist plan
dist build --artifacts=local --target aarch64-apple-darwin
dist build --artifacts=global
python3 scripts/smoke-release.py target/distrib/smackdebt-aarch64-apple-darwin.tar.xz
python3 scripts/smoke-agent-install.py target/distrib
```

Publishing uses the official `release-plz/action` workflow. To run the optional
release configuration integration fixtures locally, install release-plz 0.3.162
and run `SMACKDEBT_RELEASE_PLZ=release-plz python3 -m unittest scripts/tests/test_release_config.py`.

Use your host target. The Linux binary requires glibc. Edit `dist-workspace.toml`
and regenerate with `dist generate`; do not hand-edit the generated workflow.
The missing built-in Homebrew publish-job warning is expected: publication runs
in our post-release workflow so download URLs already exist.

The plugin skill is the authoring source. After changing it, run
`python3 scripts/sync-agent-skill.py` to refresh the Cargo-packaged copy and increment
both plugin manifest versions. CI checks both consistency and plugin versions.
`smackdebt init` embeds this packaged skill and performs no network access.

Optional performance evidence remains available through `just release-evidence`
and `just release-evidence-check`; see `benchmarks/README.md`. Do not copy private source
into public fixtures or evidence.

## Recovery

Rerun failed jobs for the same tag after infrastructure or credential failures.
A tap update retry skips an identical formula. If the GitHub Release is already public,
rerun only failed downstream jobs. Never move a published tag or overwrite released
artifacts with different code. Code changes require a new version and release PR.
The existing `v0.1.0` tag is preserved; the first registry release starts at `0.2.0` to avoid reusing that tag.

A PR's green checks prove packaging and local artifact installation. The post-release
workflow proves the actual public download paths. Check both before announcing a release.
