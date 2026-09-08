"""Exercise a shipped archive from a fresh repository outside the checkout."""

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import tomllib

import jsonschema


ROOT = Path(__file__).resolve().parents[1]


def unpack_command(archive: Path, directory: Path) -> Path:
    checksum = Path(f"{archive}.sha256").read_text().split()[0]
    with archive.open("rb") as source:
        assert hashlib.file_digest(source, "sha256").hexdigest() == checksum, "archive checksum differs"
    command = directory / "smackdebt"
    with tarfile.open(archive) as bundle:
        binaries = [member for member in bundle if member.isfile() and Path(member.name).name == "smackdebt"]
        assert len(binaries) == 1, "archive must contain one smackdebt binary"
        with bundle.extractfile(binaries[0]) as source, command.open("wb") as destination:
            shutil.copyfileobj(source, destination)
    command.chmod(0o755)
    return command


def invoke(arguments, directory):
    environment = dict(os.environ)
    environment.update(
        GIT_CONFIG_GLOBAL=os.devnull,
        GIT_CONFIG_NOSYSTEM="1",
        NO_COLOR="1",
        COLUMNS="100",
    )
    return subprocess.run(
        arguments, cwd=directory, env=environment, check=True, capture_output=True
    )


def report(command, directory, arguments):
    output = invoke([command, *arguments], directory)
    assert output.stdout and not output.stderr, "report must have stdout and no stderr"
    return output.stdout


def checked_json(data):
    value = json.loads(data)
    schema = json.loads((ROOT / "schemas/report-v4.schema.json").read_text())
    jsonschema.Draft202012Validator(schema).validate(value)
    return value


def check_command(command, directory):
    version = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    assert report(command, directory, ["--version"]).decode().strip() == f"smackdebt {version}"
    assert b"diff" in report(command, directory, ["--help"])
    invoke(["git", "init", "-q", "-b", "main"], directory)
    source = directory / "work.js"
    source.write_text("export function work(x) {\n" + "  if (x) x--;\n" * 30 + "  return x;\n}\n")
    invoke(["git", "add", "work.js"], directory)
    invoke(["git", "-c", "user.name=Release Test", "-c", "user.email=release@example.invalid",
            "commit", "-qm", "initial code"], directory)
    for arguments in [["--color", "never"], ["--json"]]:
        serial = report(command, directory, [*arguments, "--jobs", "1"])
        automatic = report(command, directory, arguments)
        assert serial == automatic, "serial and automatic reports differ"
    before = checked_json(report(command, directory, ["--json"]))
    assert before["summary"]["high"] > 0, "complex source must carry debt"
    source.write_text("export function work(x) { return x; }\n")
    after = checked_json(report(command, directory, ["diff", "main", "--json"]))
    assert after["verdict"]["tier"] == "better", "simpler code must reduce debt"
    assert b"Debt decreased." in report(command, directory, ["diff", "main"])
    invoke([command, "gate", "--update"], directory)
    invoke([command, "gate"], directory)


def main():
    archive = Path(sys.argv[1]).resolve()
    with tempfile.TemporaryDirectory(prefix="smackdebt-release-") as temporary:
        directory = Path(temporary)
        command = unpack_command(archive, directory)
        repository = directory / "repository"
        repository.mkdir()
        check_command(command, repository)
    print(f"release archive: {archive.name} passed")


if __name__ == "__main__":
    main()
