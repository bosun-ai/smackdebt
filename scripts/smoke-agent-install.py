"""Install the exact release assets into a temporary profile without network access."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import tomllib


ROOT = Path(__file__).resolve().parents[1]
# The generated CLI installer still selects and unpacks its real platform archive.
# Only HTTP transport is replaced: every request must name a staged release asset.
CURL = '''#!/usr/bin/env python3
from pathlib import Path
import os, shutil, sys
args = sys.argv[1:]
url = next(value for value in args if value.startswith("https://"))
prefix = os.environ["SMACKDEBT_TEST_RELEASE"]
assert url.startswith(prefix), url
name = url[len(prefix):]
assert name and "/" not in name, url
source = Path(os.environ["SMACKDEBT_TEST_ASSETS"]) / name
output = args[args.index("-o") + 1]
shutil.copyfile(source, output)
'''


def check_install(assets, directory):
    profile = directory / "user with spaces"
    profile.mkdir()
    tools = directory / "tools"
    tools.mkdir()
    curl = tools / "curl"
    curl.write_text(CURL)
    curl.chmod(0o755)
    version = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    environment = {key: value for key, value in os.environ.items() if not key.startswith(("SMACKDEBT_", "CARGO_DIST_"))}
    environment.pop("CLAUDE_CONFIG_DIR", None)
    environment.update(
        HOME=str(profile), CARGO_HOME=str(profile / ".cargo"),
        XDG_CONFIG_HOME=str(profile / ".config"), XDG_DATA_HOME=str(profile / ".local/share"),
        PATH=f"{tools}{os.pathsep}{os.environ['PATH']}", INSTALLER_NO_MODIFY_PATH="1",
        SMACKDEBT_TEST_ASSETS=str(assets),
        SMACKDEBT_TEST_RELEASE=f"https://github.com/bosun-ai/smackdebt/releases/download/v{version}/",
    )
    result = subprocess.run(
        ["sh", "-s"], input=(assets / "install.sh").read_text(), env=environment,
        cwd=directory, text=True, capture_output=True, timeout=90,
    )
    assert result.returncode == 0, result.stdout + result.stderr
    command = profile / ".cargo/bin/smackdebt"
    actual = subprocess.check_output([command, "--version"], text=True).strip()
    assert actual == f"smackdebt {version}", actual
    for location in (".agents", ".claude"):
        installed = profile / location / "skills/smackdebt/SKILL.md"
        assert installed.read_bytes() == (ROOT / "plugins/smackdebt/skills/smackdebt/SKILL.md").read_bytes()
    source = directory / "sample.rs"
    source.write_text("fn identity(value: u32) -> u32 { value }\n")
    report = json.loads(subprocess.check_output([command, source, "--json"]))
    assert report["schema_version"] == 4 and report["summary"]["checked"] == 1


def main():
    assets = Path(sys.argv[1]).resolve()
    with tempfile.TemporaryDirectory(prefix="smackdebt-agent-smoke-") as temporary:
        check_install(assets, Path(temporary))
    print("agent installer: CLI and skill passed")


if __name__ == "__main__":
    main()
