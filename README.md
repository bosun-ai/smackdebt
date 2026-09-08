# Smackdebt

**Find the code that hurts. See whether your changes make it better.**

Smackdebt checks source, dependencies, and Git history to show where debt lives and what to tackle next. No setup required. Your code stays on your machine.

## Get it

For Linux x86_64 and macOS (Intel or Apple Silicon):

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/bosun-ai/smackdebt/releases/latest/download/smackdebt-installer.sh | sh
```

Or grab an archive from [Releases](https://github.com/bosun-ai/smackdebt/releases). Building from source? With Rust 1.97+, run `cargo install --locked --path crates/cli` from this repository.

## Show me the damage

```sh
smackdebt                 # Check the repository
smackdebt src/auth        # Look closer at a directory or file
smackdebt diff main       # Compare your worktree with a Git ref
smackdebt --json          # Feed a script, editor, or agent
smackdebt gate            # Check against a saved debt baseline
```

Reports name the problem and give you a next step. For example:

```text
PROBLEMS
  high hot and complex · src/auth.rs
    cognitive 28 · hot (11 commits)

next: smackdebt src/auth.rs
```

Use `--all` for detail or `--help` for options. [Set up the gate](docs/guide.md#gate) when you want CI to catch regressions.

## What gets measured?

A function takes its worst rating: **healthy**, **watch**, or **high**. One troublesome function cannot hide inside a project average.

| Metric | What it tells you | Watch | High |
| --- | --- | ---: | ---: |
| Cognitive complexity | How hard the control flow is to follow | 15 | 25 |
| Cyclomatic complexity | How many independent paths need testing | 11 | 21 |
| Logical statements | How much work a function contains | 50 | 100 |
| Nesting depth | How far you have to keep context in your head | 4 | 7 |
| Parameters | How much a caller needs to supply | 6 | 9 |

Dependencies reveal cycles and change reach. Git history adds hotspots, files that change together, and reliance on one contributor. [Measurement rules](docs/guide.md#read-the-ratings).

## How bad is it?

The repository verdict ranges from **“Clean. Ship it.”** through **“Solid, with rough edges.”**, **“Worn in the usual places.”**, and **“This code fights back.”** to **“The code is winning.”**

No analyzed code means **“Nothing was checked.”**, not a clean bill of health. A diff tells you whether debt increased, decreased, moved both ways, or stayed the same. [Read the verdicts](docs/guide.md#check-a-codebase).

## Familiar problems, now with receipts

| You see | What to look at |
| --- | --- |
| Does too much | A large or widely connected file with concentrated debt |
| Everything depends on this / depends on many files / change spreads far | A dependency hub or a file with wide reach |
| Circular dependency | Files or packages tied in a knot |
| Hot and complex | Difficult code you keep changing |
| Packages change together | Package changes linked by history rather than imports |
| Importers follow its changes | An interface whose callers keep needing edits |
| Change together without a dependency | Files with a hidden relationship |
| One author | Knowledge concentrated in one contributor |
| Depends on less stable code | A dependency pointing toward more dependent code |

Other rated findings appear under their own names. Each problem includes measured evidence. [Pattern rules](docs/guide.md#read-the-problems).

## Languages

**C, C++, Java, JavaScript/JSX, Python, Ruby, Rust, TypeScript/TSX, and Vue** (scripts and templates). Unsupported files and parse failures remain visible in coverage warnings.

## Keep digging

[User guide](docs/guide.md) · [Checked examples](docs/examples.md) · [JSON schemas](schemas/README.md) · [Architecture](ARCHITECTURE.md) · [Releasing](RELEASING.md)

MIT licensed. Built with [tree-sitter](https://tree-sitter.github.io/tree-sitter/) and its language grammars; Smackdebt owns the measurements and reports.
