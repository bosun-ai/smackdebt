"""Build the version-pinned binary installer entry point."""

from pathlib import Path
import json
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[1]


def render_installer():
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--format-version=1", "--no-deps", "--locked"], cwd=ROOT,
    ))
    version = next(package["version"] for package in metadata["packages"] if package["name"] == "smackdebt")
    template = (ROOT / "scripts/install.sh.in").read_text()
    return template.replace("@VERSION@", version)


def main():
    destination = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "target/agent-installer/install.sh"
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(render_installer())
    destination.chmod(0o755)


if __name__ == "__main__":
    main()
