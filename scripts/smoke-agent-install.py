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


def install_environment(assets, directory):
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
        HOME=str(profile), CARGO_HOME=str(profile / "custom cargo"),
        CLAUDE_CONFIG_DIR=str(profile / "custom claude"),
        XDG_CONFIG_HOME=str(profile / ".config"), XDG_DATA_HOME=str(profile / ".local/share"),
        PATH=f"{tools}{os.pathsep}{os.environ['PATH']}", INSTALLER_NO_MODIFY_PATH="1",
        SMACKDEBT_TEST_ASSETS=str(assets),
        SMACKDEBT_TEST_RELEASE=f"https://github.com/bosun-ai/smackdebt/releases/download/v{version}/",
    )
    return profile, environment, version


def run_installer(assets, directory, environment, *arguments):
    result = subprocess.run(
        ["sh", "-s", "--", *arguments], input=(assets / "install.sh").read_text(), env=environment,
        cwd=directory, text=True, capture_output=True, timeout=90,
    )
    assert result.returncode == 0, result.stdout + result.stderr


def check_cli(command, version, directory):
    actual = subprocess.check_output([command, "--version"], text=True).strip()
    assert actual == f"smackdebt {version}", actual
    source = directory / "sample.rs"
    source.write_text("fn identity(value: u32) -> u32 { value }\n")
    report = json.loads(subprocess.check_output([command, source, "--json"]))
    assert report["schema_version"] == 4 and report["summary"]["checked"] == 1


def check_skills(skills):
    expected = (ROOT / "plugins/smackdebt/skills/smackdebt/SKILL.md").read_bytes()
    for path in skills:
        assert path.read_bytes() == expected, f"installed skill differs: {path}"


def check_install(assets, directory):
    profile, environment, version = install_environment(assets, directory)
    environment["CODEX_HOME"] = str(profile / "custom codex")
    run_installer(assets, directory, environment)
    command = profile / "custom cargo/bin/smackdebt"
    check_cli(command, version, directory)
    assert not (profile / ".agents").exists()
    assert not (profile / "custom claude").exists()
    run_installer(assets, directory, environment)
    subprocess.run([command, "init", "--agent", "codex", "--agent", "claude-code"], env=environment, cwd=directory, check=True)
    skills = [profile / location / "skills/smackdebt/SKILL.md" for location in ("custom codex", "custom claude")]
    check_skills(skills)
    environment.update(CODEX_HOME=str(profile / ".codex"), CLAUDE_CONFIG_DIR=str(profile / ".claude"))
    subprocess.run([command, "init"], stdin=subprocess.DEVNULL, env=environment, cwd=directory, check=True)
    check_skills(skills)
    assert not (profile / ".codex").exists()
    subprocess.run([command, "init", "--uninstall"], env=environment, cwd=directory, check=True)
    assert not any(path.exists() for path in skills)
    check_cli(command, version, directory)


def main():
    assets = Path(sys.argv[1]).resolve()
    with tempfile.TemporaryDirectory(prefix="smackdebt-agent-smoke-") as temporary:
        check_install(assets, Path(temporary))
    print("agent installer: install, update, and removal passed")


if __name__ == "__main__":
    main()
