## Why

The first progressive report proves the hierarchy, but its default terminal
view still mixes useful signals with healthy-only rows and omits local debt
rate. Large mixed repositories can also panic when discovery assigns a source
file to its fallback package and hierarchy construction tries to infer that
assignment again from path prefixes.

Smackdebt should answer three questions at a glance: how much source was
analyzed, how much rated code needs attention, and where attention is most
concentrated. Each deeper invocation should narrow those answers without noise.

## What Changes

- Preserve discovery's package assignment when building codebase scopes so
  large mixed repositories cannot fail on source outside manifest roots.
- Make the default overview a debt-focused view: show debt-bearing child areas,
  report healthy-only areas as one quiet summary, and keep `--all` for the full
  inventory.
- Add a local debt rate beside project debt share so users can distinguish a
  large area that owns much of the debt from a smaller area where debt is dense.
- Make summary language state rated units, attention rate, and excluded coverage
  directly.
- Make finding details explain which metric crossed which limit and make drill
  guidance point to the first displayed debt-bearing child.
- Keep routine non-source worktree changes out of coverage notes while retaining
  real source-analysis failures.
- Update product and architecture examples to match verified output.

## Impact

- Affected specs: `progressive-exploration`, `workspace-architecture`,
  `product-documentation`, `architecture-documentation`
- Affected crates: `smackdebt-project`, `smackdebt-output`, `smackdebt`
- JSON schema version 1 remains additive and complete; this change refines
  terminal display policy and fixes report construction.
