## MODIFIED Requirements

### Requirement: Roles have exact verdict participation
Primary, test, example, and benchmark source SHALL affect default code verdicts.
Fixture and generated source SHALL remain visible in coverage, JSON, explicit
path drill, file inspection, and `--all` but SHALL NOT affect default verdicts,
architecture health, coupling findings, diff debt movement, root
worst-offender selection, default problem cards, or codebase navigation.

#### Scenario: A fixture contains a High unit
- **WHEN** parsed fixture source exceeds a High threshold
- **THEN** its fact remains inspectable and default health and findings do not change

#### Scenario: A benchmark contains a High unit
- **WHEN** parsed benchmark source exceeds a High threshold
- **THEN** it affects the default verdict and displays its non-primary role

#### Scenario: Generated source is the largest finding
- **WHEN** a generated file outranks every authored file by measurements
- **THEN** it remains machine and file-detail context but cannot become the root worst offender, a default problem, or the `next:` target

## ADDED Requirements

### Requirement: Generated JavaScript uses narrow name and content evidence
The role classifier SHALL, after explicit configuration and language-owned
generated markers, assign `generated` to a JavaScript source file whose file name
case-sensitively matches one of these shapes:

- `*.min.js`, `*.min.mjs`, or `*.min.cjs`;
- `*.bundle.js`, `*.bundle.mjs`, or `*.bundle.cjs`;
- `*-bundle.js`, `*-bundle.mjs`, or `*-bundle.cjs`.

The filename rule SHALL apply only to `.js`, `.mjs`, and `.cjs`, which identify
JavaScript. The classifier SHALL also assign `generated` when a JavaScript,
JSX, TypeScript, or TSX source buffer — including `.js`, `.mjs`, `.cjs`,
`.jsx`, `.ts`, and `.tsx`, but not a Vue document — is at least 65,536 bytes and its total byte
length is at least 512 times its count of nonempty physical lines. A buffer with
zero nonempty lines SHALL not match the content rule. This decision SHALL reuse
the source buffer selected for analysis and SHALL NOT read the file again.

The full precedence SHALL be explicit configuration, language-owned generated
markers, these JavaScript filename and content rules, existing generic filename
and path rules, test-declared fallback, then primary. Explicit configuration
SHALL win over both new rules. The new level produces only `generated`, and the
role SHALL be final before health, history, dependency, and report policy read
it.

No directory name SHALL trigger either rule. In particular, `public`, `share`,
and `assets` SHALL NOT classify every retained source file beneath them as
generated.

#### Scenario: A minified module name is selected
- **WHEN** retained source is named `vendor.min.mjs`
- **THEN** it receives the generated role and remains inspectable

#### Scenario: A bundle suffix is selected
- **WHEN** retained source is named `client-bundle.cjs` or `client.bundle.js`
- **THEN** it receives the generated role

#### Scenario: Explicit configuration overrides a generated name
- **WHEN** configuration assigns `primary` to `vendor.min.js`
- **THEN** it remains primary because explicit configuration wins

#### Scenario: TypeScript meets only the content rule
- **WHEN** `client.bundle.ts` is smaller than 65,536 bytes
- **THEN** its name does not trigger the JavaScript-suffix rule and its remaining role rules decide the result

#### Scenario: The size edge is met
- **WHEN** a JavaScript file is exactly 65,536 bytes with 128 nonempty physical lines
- **THEN** it receives the generated role because its average is exactly 512 bytes per nonempty line

#### Scenario: A large ordinary multiline file is retained as authored
- **WHEN** a JavaScript file is at least 65,536 bytes and has enough nonempty lines to average fewer than 512 bytes
- **THEN** the content rule does not classify it as generated

#### Scenario: A small authored one-line file is retained as authored
- **WHEN** a one-line JavaScript file is shorter than 65,536 bytes and its name matches no generated shape
- **THEN** it keeps the role assigned by the remaining precedence rules

#### Scenario: A common asset directory holds authored source
- **WHEN** `public/app.js`, `share/tool.js`, or `assets/editor.js` matches no filename or content rule
- **THEN** its directory name alone does not assign the generated role
