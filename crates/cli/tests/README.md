# CLI acceptance evidence

`acceptance.rs` keeps the focused black-box results added with each analysis
family. `unified_acceptance.rs` owns the generated repository domain and the
release matrix across source, static architecture, evolution, codebase, diff,
path drill, output width, color, worker policy, failures, privacy, and install.

Normal tests only read committed results:

```console
just acceptance
```

Update one reviewed result explicitly:

```console
just update-unified-snapshot unified-codebase.json
```

Use `all` only when every changed result is intended. The command prints each
file written. Review semantic assertions and schema checks before accepting a
byte change.

Work-count evidence is compiled only with its named test feature:

```console
just acceptance-evidence
```

The install smoke is slower and has its own release command:

```console
just acceptance-install
```
