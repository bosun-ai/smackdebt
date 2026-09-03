## MODIFIED Requirements

### Requirement: Unsupported and failed files remain visible
Smackdebt SHALL produce coverage diagnostics for unsupported languages,
unreadable files, oversized files, and parser failures. It MUST NOT treat those
files as healthy source. `.astro` SHALL be a recognized source extension with a
private `Astro` language identity and `astro` machine label, but SHALL remain
explicitly unsupported until a separate change provides document-analysis
fixtures and an analyzer.

Recognized unsupported source SHALL remain selected in codebase, ref-diff, and
worktree-diff inventories. It SHALL contribute to selected and unsupported
coverage and graph-evidence trust, retain its path and source role, and produce
no invented units or dependencies.

#### Scenario: One file cannot be analyzed
- **WHEN** one selected file is unsupported or fails analysis
- **THEN** a codebase report still succeeds with that path, reason, and excluded coverage recorded

#### Scenario: An Astro document is encountered
- **WHEN** discovery supplies a `.astro` source file
- **THEN** it is labeled `astro`, reported as unsupported coverage, and contributes no unit or dependency fact

#### Scenario: An Astro document changes
- **WHEN** an Astro file differs between the current tree and selected ref
- **THEN** both diff inventories retain the file where present and graph evidence cannot treat its source side as fully analyzed

## ADDED Requirements

### Requirement: Anonymous units have safe private match evidence
Each analyzed unit SHALL keep its human display identity separate from the
opaque evidence used to match it across a diff. Analysis SHALL own the value and
export only narrow constructors so the inward-dependent language crate can
write it onto `UnitFact`; its fields and read access SHALL remain private to
analysis. The type MAY therefore appear in the private workspace API snapshot
but SHALL NOT be serialized or expose a dependency from analysis to languages.

A declared function or method SHALL match by its declared identity. An anonymous unit MAY carry a
language-supplied semantic anchor made from its enclosing declared container,
unit kind, and one stable syntax identity: an assignment or binding name; an
enclosing `computed`, `watch`, or callback call plus argument position; a
neighboring literal that identifies the callback; or a Ruby example or context
description.

For remaining anonymous units, the language adapter MAY construct a
deterministic 256-bit BLAKE3 digest and byte length from the unit's exact syntax
bytes while that file's active source buffer exists. Exact syntax SHALL NOT be
retained after the active file worker. Syntax nodes, semantic-anchor values,
digests, and byte lengths SHALL NOT appear in report values, JSON, diagnostics,
terminal output, or logs.

Within one file, comparison SHALL pair units in this order:

1. a declared identity occurring once on each side;
2. a semantic anchor occurring once on each side among remaining units;
3. an exact-syntax digest and byte length occurring once on each side among remaining units.

It SHALL NOT pair by line number, source span, measurement values, rating,
source ordinal, or fuzzy syntax similarity. A shared candidate with a repeated
side is unsafe and SHALL produce one `ambiguous` machine comparison for that
collision group plus one `ambiguous_identity` file diagnostic. A candidate
present on only one side SHALL produce one Added or Removed comparison per unit,
even when repeated. A unit without shared match evidence is likewise one-sided,
not ambiguous. Fingerprint evidence is fixed-size; any repeated fingerprint
bucket is ambiguous rather than guessed, so collision-like evidence cannot pair
a group.

#### Scenario: An unchanged callback moves
- **WHEN** an anonymous callback's exact syntax is unchanged and only its line position moves within its file
- **THEN** its unique semantic anchor or unique exact-syntax digest pairs it and no changed comparison remains

#### Scenario: A callback moves and is edited
- **WHEN** a callback moves and its measurements change while its unique call-site or binding anchor remains
- **THEN** one paired comparison carries the change instead of one added and one removed comparison

#### Scenario: A callback is genuinely added
- **WHEN** an anonymous callback has no before-side declared identity, semantic anchor, or exact-syntax match
- **THEN** it remains an addition rather than being paired by a nearby line, similar measurements, or source order

#### Scenario: An anchor is repeated
- **WHEN** a shared anonymous match candidate occurs twice on either side of one file comparison
- **THEN** the unsafe group remains ambiguous, no debt direction is guessed, and the file carries one ambiguity diagnostic

#### Scenario: A callback is genuinely removed
- **WHEN** an anonymous callback has no after-side declared identity, semantic anchor, or fingerprint match
- **THEN** it remains a removal rather than becoming ambiguous

#### Scenario: A candidate repeats only after the change
- **WHEN** the same candidate occurs twice after the change and not before
- **THEN** two Added comparisons are retained and no ambiguity diagnostic is emitted

#### Scenario: A candidate repeats only before the change
- **WHEN** the same candidate occurs twice before the change and not after
- **THEN** two Removed comparisons are retained and no ambiguity diagnostic is emitted

#### Scenario: Two candidates compete for one
- **WHEN** a shared anchor or fingerprint bucket holds two units on one side and one on the other
- **THEN** one ambiguous collision-group comparison is retained and no added, removed, or paired direction is guessed for that group

#### Scenario: A Ruby example description is stable
- **WHEN** a Ruby example block moves and retains its unique example or context description
- **THEN** that description anchors the unit without exposing parser data across the language seam
