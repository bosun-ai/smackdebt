## 1. Manifest names are discovered facts

- [ ] 1.1 Extract the declared package name during existing manifest recognition for `Cargo.toml` `[package] name` with `[lib] name` override, `package.json` `name` including `@scope/name`, `pyproject.toml` `[project] name`, and gemspec name.
- [ ] 1.2 Store the manifest name on the package record without adding a second walk, extra read, or new process.
- [ ] 1.3 Treat a missing, empty, or unreadable declared name as absent rather than as a failure.
- [ ] 1.4 Add discovery fixtures for every manifest kind, a scoped npm name, a `[lib] name` override, and a manifest without a usable name.

## 2. Manifest-name resolution

- [ ] 2.1 Build a read-only manifest-name index over discovered internal packages beside the existing file and package index.
- [ ] 2.2 Consult the index only for references that path candidates would classify external, using the reference's first path segment.
- [ ] 2.3 Normalize hyphens and underscores for Rust and compare declared names exactly for other languages.
- [ ] 2.4 Create an internal `uses` edge only on exactly one internal match; keep more than one match ambiguous with its retained diagnostic and source location.
- [ ] 2.5 Target the matched package's entry file when it resolves and emit a package-scoped edge otherwise.
- [ ] 2.6 Add pure resolution tests for unique match, name normalization, shadowed ambiguous names, entry-file fallback, and unchanged path-candidate precedence.

## 3. Coupling correctness

- [ ] 3.1 Emit one retained coupling row per unordered package pair by aggregating role and trust variants into that row.
- [ ] 3.2 Keep the eligible parsed aggregate as the finding operand and keep per-role evidence in package history rows.
- [ ] 3.3 Exclude ancestor-descendant scope pairs before similarity is computed, in descriptive rows and findings.
- [ ] 3.4 Prove the `no code dependency` claim is false only when no trusted eligible `uses` relation exists in either direction after manifest-name resolution.
- [ ] 3.5 Add pure tests for pair de-duplication, ancestor-descendant exclusion, and an explained pair that stops being a finding.

## 4. Rust extraction fix

- [ ] 4.1 Treat Rust visibility tokens as item modifiers so they never become a dependency target.
- [ ] 4.2 Add a Rust language fixture proving `pub use a::b;` emits target `a::b` and no `pub` target.

## 5. Evidence and documentation

- [ ] 5.1 Add generated workspace fixtures per manifest kind proving internal package edges, package degree, and at least one package cycle that only manifest-name resolution can reveal.
- [ ] 5.2 Add evidence that a shadowed manifest name stays external or ambiguous with its diagnostic.
- [ ] 5.3 Prove JSON version 3 structure is unchanged while values move, and regenerate reviewed terminal and JSON snapshots file by file.
- [ ] 5.4 Update README wording for workspace package resolution and the meaning of `no code dependency`.
- [ ] 5.5 Prove serial and parallel runs stay byte-identical and Git work stays at one streamed history process plus one batch object process.
- [ ] 5.6 Re-run the release binary on smackdebt, swiftide, and fluyt and record that the false `crates/analysis ↔ crates/project · no code dependency` claim is gone and unmatched imports on smackdebt fall from 29 to near zero.
- [ ] 5.7 Pass formatting, Clippy, workspace tests, architecture checks, performance tests, acceptance evidence, strict OpenSpec validation, and the final diff check.
- [ ] 5.8 Archive this change before implementing `deepen-debt-signals`.
