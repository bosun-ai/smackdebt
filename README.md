# Smackdebt

**Help your AI agent write code you can follow.**

Your agent finished the feature. Now you have to untangle it.

Smackdebt gives coding agents concrete feedback on complexity, tangled dependencies,
and code that keeps causing work. They can find the trouble, simplify it, and check
whether their changes helped before handing the code back to you.

Analysis runs locally. No service, account, or API key required.

## Get it

For Linux x86_64 and macOS (Intel or Apple Silicon):

```sh
curl -fsSL https://github.com/bosun-ai/smackdebt/releases/latest/download/install.sh | sh
```

Or choose Homebrew or Cargo:

```sh
brew install bosun-ai/tap/smackdebt
# or
cargo binstall smackdebt
```

Then give your agent the skill:

```sh
smackdebt init
```

Choose **Codex, Claude Code, Cursor, Copilot, or Gemini CLI**, then restart your agent.
Rerun `init` after upgrading to update its skill. [Agent setup →](docs/agents.md)

Installers arrive with the next published release. Until then, build from this
checkout with Rust 1.97+: `cargo install --locked --path crates/cli`.

## Give “make it cleaner” some teeth

Ask your agent:

> Use Smackdebt to review this change. Simplify the code where you've made it
> harder to follow, then run the tests and check the diff again.

The skill guides your agent to inspect the affected code before editing, compare
against its starting commit afterward, and fix regressions within the task.
For a required CI check, [set up a debt gate](docs/guide.md#gate).

You can run the same checks yourself:

```sh
smackdebt                 # Find the trouble spots
smackdebt src/auth        # Zoom in
smackdebt diff main       # Did this change help?
smackdebt --json          # Use the report in your own tools
```

## Less spaghetti, with receipts

Reports name the problem, show the evidence, and point to the code:

```text
PROBLEMS
  high hot and complex · src/auth.rs
    cognitive 28 · hot (11 commits)

next: smackdebt src/auth.rs
```

Each function gets a **healthy**, **watch**, or **high** rating from its complexity,
size, nesting, and parameter count. A difficult function cannot hide in a project
average. Dependency checks reveal cycles and wide change impact; Git history
highlights difficult code you keep touching.

Use `--all` for detail. [Measurements](docs/guide.md#read-the-ratings) ·
[Problem patterns](docs/guide.md#read-the-problems) · [Checked examples](docs/examples.md)

## Languages

**C, C++, C#, Go, Java, JavaScript/JSX, PHP, Python, Ruby, Rust, TypeScript/TSX,
and Vue** (scripts and templates). React uses JSX/TSX support. Reports flag
unsupported files and parse failures so you can see what the check missed.

---

[User guide](docs/guide.md) · [JSON schemas](schemas/README.md) ·
[Architecture](ARCHITECTURE.md) · [Development & releases](RELEASING.md)

MIT licensed. Built with [tree-sitter](https://tree-sitter.github.io/tree-sitter/).
