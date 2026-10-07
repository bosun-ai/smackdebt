# Use Smackdebt with your agent

Install the binary with curl, Homebrew, or Cargo as shown in the [README](../README.md).
Then run:

```sh
smackdebt init
```

Choose agents by entering their numbers, separated by spaces. Enter keeps the
saved selection; `q` cancels without changing files. Skills are installed for
your user account and work across projects. Restart your agent to discover them.

For scripts, choose agents explicitly:

```sh
smackdebt init --agent codex --agent claude-code
smackdebt init --all --dry-run
smackdebt init --agent codex --dest /absolute/path/to/skills
```

Supported names are `codex`, `claude-code`, `cursor`, `copilot`, and `gemini`.
The destinations are `~/.codex/skills`, `~/.claude/skills`, `~/.cursor/skills`,
`~/.copilot/skills`, and `~/.gemini/skills`, respectively. `CODEX_HOME` and
`CLAUDE_CONFIG_DIR` override their clients' configuration roots. `--dest` accepts
an absolute skills directory for exactly one agent; Smackdebt adds `smackdebt/`.

The selection and absolute destinations are remembered in
`${XDG_CONFIG_HOME:-$HOME/.config}/smackdebt/agents.toml`. Without a terminal,
`init` updates the saved selection or requires `--agent`/`--all` on first use.
Explicit selection changes future updates; it does not remove earlier installations.
Uninstall an agent before changing its saved destination.

## Update or remove

Update the binary using its original package manager or rerun the curl command.
Then run `smackdebt init` to update selected skills. The skill is bundled with
the installed binary: setup and skill updates need no network, Node, or Rust.

```sh
smackdebt init --uninstall                    # Remove all managed skills
smackdebt init --uninstall --agent codex      # Remove one agent's skill
smackdebt init --uninstall --dry-run          # Preview removal
```

Updates and removal preserve edited or unmanaged skills and unrelated files.
Move a conflicting skill aside before retrying. Interrupted operations can be
rerun; completed paths are printed. The CLI binary is never removed by `init`.

The old preview curl installer's skill and removal flags are no longer supported.
Existing preview skills stay untouched. Move old standalone skill directories
aside before using `init` for the same destination; remove old shared
`~/.agents/skills/smackdebt` copies yourself to avoid duplicate discovery.

## Plugins

Codex and Claude Code plugins remain alternatives to standalone skills. Use one
installation method per client to avoid duplicates. Remove an `init`-managed skill
with `smackdebt init --uninstall --agent codex` (or `claude-code`) before switching.

```sh
codex plugin marketplace add bosun-ai/smackdebt
codex plugin add smackdebt@smackdebt

claude plugin marketplace add bosun-ai/smackdebt
claude plugin install smackdebt@smackdebt
```

Plugins install the skill; the binary is installed separately. If it is missing,
the skill offers an installation command and waits for your approval before running
it. Once installed, the agent checks its version and continues the debt review.
Plugin managers update and remove their own plugins independently of the binary.
The portable skill can also be installed through `npx skills add bosun-ai/smackdebt --skill smackdebt`.

## What the skill does

Ask your agent to review a change or find debt in an area. The skill guides checks
around substantial changes and keeps fixes within the task. Activation is advisory;
installation does not enforce checks on every task. Smackdebt leaves repository
instructions, thresholds, and baselines alone. Analysis stays local; agents receive
command output through their normal tools and data settings.
