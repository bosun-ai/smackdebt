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

JSON results are committed as the exact single-line bytes the CLI wrote, so a
raw diff is unreadable. `.gitattributes` marks snapshot and schema JSON with
`diff=json`; opt in once per clone to see structured diffs:

```console
git config diff.json.textconv "python3 -m json.tool --indent 2"
```

To read one committed result pretty-printed without configuring git:

```console
just show-snapshot unified-codebase.json
```

Work-count evidence is compiled only with its named test feature:

```console
just acceptance-evidence
```

The install smoke is slower and has its own release command:

```console
just acceptance-install
```
