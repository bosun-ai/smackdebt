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

Rerun the installer to update. It records the installed skill's checksum and
refuses to replace edited or unmanaged skills. Move those aside before switching
to this installer. Use your plugin manager to update a plugin installation.

To remove a standalone skill, delete its `smackdebt` directory from both skill
locations above. Remove the CLI executable from its install location if you no
longer need it. For plugins, use `codex plugin remove` or
`claude plugin uninstall smackdebt@smackdebt` instead.

If installation fails, the command returns a nonzero status and names completed
parts. It does not roll back a CLI installation after a later skill-write failure.
After fixing the reported problem, rerun the installer.
