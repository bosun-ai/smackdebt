## Context

Discovery already decides the nearest package for every selected file. The
progressive hierarchy currently accepts only package-root paths and performs a
second nearest-root lookup. When no root is a prefix of a file, discovery uses
its first package while hierarchy construction returns an empty root and
panics. Fluyt contains this shape.

The report also shows High, Watch, Healthy, and project debt share for up to ten
children. Healthy-only children consume attention even though the report is
primarily a guide to code needing work. Project share alone is incomplete: it
rewards size and cannot show whether an area is unusually debt-dense.

## Decisions

### Use discovery's package identity

The shared hierarchy builder will accept the chosen package root with each
codebase file. Diff will continue to choose a package root from the manifests it
can observe. The builder will not repeat package policy or panic on a missing
prefix.

### Separate contribution from concentration

Each debt-bearing row will show:

- High and Watch counts;
- `share`: the area's portion of all debt in the selected scope;
- `rate`: the area's High plus Watch units divided by all rated units in that
  area.

The selected-scope summary will use the same attention rate. Exact counts remain
the primary facts and percentages round to whole numbers.

### Keep the default focused

The default view will display at most ten debt-bearing children. Healthy-only
children will not appear as rows; one line will state how many quiet areas were
hidden. `--all` will show every child, including healthy-only areas, so the full
inventory remains inspectable.

If no child carries debt, the report will say so and avoid an empty table.

### Explain findings in product terms

Finding detail will show the rating and only the measured signals that reached
Watch or High, including the signal name and value. It will retain location,
container, activity, and all measurements in JSON. This removes repeated low
values from the default terminal view while explaining why a finding exists.

### Derive the drill command from visible rows

The `Explore` command will use the first row after display policy and sorting.
It will not independently rank hidden children.

## Risks

- Hiding healthy-only rows could conceal project shape. The quiet-area count and
  `--all` retain that information without making it compete with debt.
- A percentage can be mistaken for a score. Labels use `share` and `rate`, and
  the exact High and Watch counts stay visible.
- Diff package discovery has less information for deleted files. It keeps using
  current and previous manifest paths and a repository fallback package.

