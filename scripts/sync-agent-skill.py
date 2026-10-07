"""Keep the Cargo-packaged skill identical to the plugin's authoring source."""

from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "plugins/smackdebt/skills/smackdebt/SKILL.md"
DESTINATION = ROOT / "crates/cli/assets/smackdebt/SKILL.md"

if __name__ == "__main__":
    if sys.argv[1:] == ["--check"]:
        if not DESTINATION.exists() or SOURCE.read_bytes() != DESTINATION.read_bytes():
            raise SystemExit("Run python3 scripts/sync-agent-skill.py before packaging")
    elif sys.argv[1:]:
        raise SystemExit("Usage: sync-agent-skill.py [--check]")
    else:
        DESTINATION.parent.mkdir(parents=True, exist_ok=True)
        DESTINATION.write_bytes(SOURCE.read_bytes())
