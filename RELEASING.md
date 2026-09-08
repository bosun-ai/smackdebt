# Releasing Smackdebt

We ship one CLI through GitHub Releases: Linux x86_64, Intel macOS, and Apple Silicon macOS. Version numbers start at `0.1.0` and move together across the workspace. `install.sh` installs the CLI and the portable agent skill; Codex and Claude Code plugins bundle the same skill.

## One-time setup

- Add `RELEASE_PLZ_TOKEN` as a repository secret. Use a fine-grained token with `bosun-ai` as resource owner, access to `bosun-ai/smackdebt`, and **Contents** and **Pull requests** read/write permissions. Complete any required organization approval.
- Allow GitHub Actions to create pull requests. Require the three `check` matrix jobs before merging a release PR.
- Install Rust 1.97.0, `just`, `cargo-public-api@0.52.0`, `cargo-deny`, and Python 3.11+. API snapshots also require nightly Rust. Install Python dependencies with `python3 -m pip install -r scripts/requirements.txt` in a virtual environment.
- For release configuration changes, install `cargo-dist@0.32.0` and `release-plz@0.3.162`. Run `dist generate` after editing `dist-workspace.toml`; do not edit the generated release workflow.

The token is needed because PRs and tags created with the built-in `GITHUB_TOKEN` do not start further workflows. No Cargo registry token is used. Cargo manifests permit packaging because release-plz reconstructs previous workspace versions through a temporary registry; this does not make the internal Rust APIs supported interfaces. Release-plz has registry publishing disabled; cargo-dist creates the GitHub Release and attaches the artifacts.

## Prepare a release

Release-plz runs after pushes to `master` and opens or updates a release PR with the shared version and root changelog. Only merging that PR starts publication. The first release is `v0.1.0`.

1. Review the release PR's version, changelog, and passing CI. Changes in internal libraries must appear in the application changelog too.
   When changing plugin content, increment the version in both plugin manifests so plugin managers refresh their caches. CI enforces this for pull requests and pushes to master. Plugin versions track skill changes independently of the CLI. The combined installer embeds the CLI release version and that tag's skill during the build.
2. Check out its final candidate commit with a clean tree. Review public report examples before accepting any changed report digests.
3. Record evidence locally, supplying paths to Smackdebt, the private Fluyt workload, and the private Rust workload:

   ```sh
   just release-evidence /path/to/smackdebt /path/to/fluyt /path/to/rust-workspace
   ```

   Use the optional final argument `--accept-report-change` only after reviewing changed output. This runs the complete checks, source install smoke, nine generated performance profiles, and three workload reviews. Only privacy-safe aggregate evidence belongs in this repository.
4. Commit only the resulting files under `benchmarks/baselines/` and `benchmarks/evidence/` to the release PR. Run `just release-evidence-check` on that clean evidence commit.
5. Merge the release PR. Release-plz creates its `vVERSION` tag. Cargo-dist builds the archives, runs the complete checks, tests the actual archives, validates the release evidence, and publishes the GitHub Release.

Changes to source, manifests, lockfiles, documentation, or workflows after recording evidence require a new measurement. Merge, squash, and rebase are supported when the final tree matches the recorded candidate except for the approved evidence files. The release check fetches the recorded commit if rewriting history removed it from the checkout. Missing or invalid evidence blocks publication.

## Verify or recover

CI builds each supported target and tests its extracted archive outside the checkout: checksum, version, help, terminal output, JSON schema, serial/automatic equality, a debt-reducing diff, and gate behavior. Release builds repeat the archive smoke on the exact files uploaded by cargo-dist.

Useful local checks:

```sh
just check
just licenses
just acceptance-install
dist generate --check
dist plan
dist build --artifacts=global
sh target/distrib/install.sh --help
dist build --artifacts=local --target aarch64-apple-darwin
python3 scripts/smoke-release.py target/distrib/smackdebt-aarch64-apple-darwin.tar.xz
```

Use the target matching your machine. The Linux archive requires glibc; the generated installer checks platform compatibility.

Installer tests run through stdin with isolated user directories on all three
check runners. They cover remembered component choices and paths, updates,
removal, preserved user files, and failed installs. The real-artifact smoke also
checks a CLI-only update and removal of standalone skills while retaining the CLI.
`dist build --artifacts=global` must include `install.sh` and the existing
`smackdebt-installer.sh`. The release verification job tests the finished CLI
archives and combined installer on all three platforms before cargo-dist
publishes the GitHub Release in its announce step, including prereleases.
Release-plz puts the versioned combined install command first in each changelog
entry. Cargo-dist labels its additional install section "Smackdebt CLI only"
because that command does not install the skill.
After publication, verify the combined installer with
a fresh user profile before announcing the one-line install.

If an upload or runner fails, rerun the failed release jobs for the same tag. Never move a published tag. If code or evidence needs changing, prepare a new version through a release PR. After publication, verify the release's installer and download links before announcing it.
