# Use Smackdebt with your agent

Install the CLI and skill for your user account:

```sh
curl -fsSL https://github.com/bosun-ai/smackdebt/releases/latest/download/install.sh | sh
```

Use `| sh -s -- --no-cli` for only the skill, or `| sh -s -- --no-skill`
for only the CLI. The installer requires standard shell tools and curl for a
CLI download; it does not need Node, Python, or Rust. The release installer
contains the skill and downloads the CLI installer from the same release tag.
The link becomes available when the first release containing this installer ships.

The CLI uses the existing cargo-dist install location, normally `~/.cargo/bin`.
The skill goes in `~/.agents/skills/smackdebt` for Codex, Cursor, Copilot, and
Gemini CLI, and `~/.claude/skills/smackdebt` for Claude Code. If you set
`CLAUDE_CONFIG_DIR`, its `skills` directory replaces the Claude location.
The installer remembers your component choice and absolute install locations in
`${XDG_CONFIG_HOME:-$HOME/.config}/smackdebt/install-state` for future updates and removal.
Open a new terminal or restart your agent if it cannot find the command or skill.
User skills stay on your machine; remote agents need an installation in their
own environment.

Ask your agent to use Smackdebt to review a change or find debt in an area.
The skill also guides checks around substantial source changes and keeps fixes
within the task. Your agent decides when to activate it; installation does not
enforce checks on every task. Smackdebt leaves repository instructions, thresholds,
and debt baselines alone. Reports stay local to the CLI; your agent receives the
output through its normal tools and data settings.

## Plugins

Both plugins bundle the same skill. Use either a plugin or a standalone skill
installation to avoid duplicate discovery. For the CLI alone, use `--no-skill`
with the installer above.

Switching from a standalone skill? Remove it while keeping the CLI:

```sh
curl -fsSL https://github.com/bosun-ai/smackdebt/releases/latest/download/install.sh | sh -s -- --uninstall --no-cli
```

If this installer also installed your CLI, future runs will update only the CLI.
Otherwise, run the installer with `--no-skill` once to install the CLI and save
that choice. Then install your plugin below.

Codex:

```sh
codex plugin marketplace add bosun-ai/smackdebt
codex plugin add smackdebt@smackdebt
```

Claude Code:

```sh
claude plugin marketplace add bosun-ai/smackdebt
claude plugin install smackdebt@smackdebt
```

Restart the agent after installing a plugin. Marketplace commands require a
version of the client that supports plugins.

For a project-level skill managed through the skills CLI:

```sh
npx skills add bosun-ai/smackdebt --skill smackdebt
```

That alternative installs the skill only and requires Node. You can also copy
`plugins/smackdebt/skills/smackdebt` from a release tag into your client's skill
directory. [Skills installer documentation](https://github.com/vercel-labs/skills).

## Update or remove

Rerun the same one-liner to update your previous selection at its saved locations,
even if `CARGO_HOME` or `CLAUDE_CONFIG_DIR` has changed. Add `--all` to install both,
or `--no-cli` / `--no-skill` to change what future runs update. These choices do
not remove components already installed.
Updates preserve edited or unmanaged standalone skills and report how to proceed.

To remove everything managed by the installer:

```sh
curl -fsSL https://github.com/bosun-ai/smackdebt/releases/latest/download/install.sh | sh -s -- --uninstall
```

Add `--no-cli` to remove only standalone skills, or `--no-skill` to remove only the
CLI. Removal keeps edited or unmanaged files and reports them with a nonzero
status. Unrelated files, shell PATH changes, and shared Cargo environment files
stay in place. Resolve any reported files and rerun to finish.

Plugins update separately from the CLI. For Codex:

```sh
codex plugin marketplace upgrade smackdebt
codex plugin add smackdebt@smackdebt
```

For Claude Code:

```sh
claude plugin marketplace update smackdebt
claude plugin update smackdebt@smackdebt
```

Claude users can enable automatic plugin updates in `/plugin` → Marketplaces →
smackdebt → Enable auto-update. This updates the plugin, not the CLI.

Restart the agent afterward. To remove a plugin, use `codex plugin remove smackdebt@smackdebt` or
`claude plugin uninstall smackdebt@smackdebt`. Plugin removal leaves the CLI installed.

If installation fails, the command returns a nonzero status and names completed
parts. It does not roll back a CLI installation after a later skill-write failure.
After fixing the reported problem, rerun the installer.
