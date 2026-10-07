"""Publish a checked stable-release formula, safely allowing workflow retries."""

from pathlib import Path
import re
import subprocess
import sys


def publish(formula, tap, tag):
    if not re.fullmatch(r"v[0-9]+\.[0-9]+\.[0-9]+", tag):
        raise ValueError("only stable release tags can update Homebrew")
    content = formula.read_bytes()
    if f'version "{tag[1:]}"'.encode() not in content:
        raise ValueError("formula version does not match the release tag")
    destination = tap / "Formula/smackdebt.rb"
    if destination.exists():
        previous = re.search(r'^\s*version "([0-9]+\.[0-9]+\.[0-9]+)"', destination.read_text(), re.MULTILINE)
        if not previous:
            raise ValueError("cannot read the existing Homebrew formula version")
        if tuple(map(int, previous[1].split("."))) > tuple(map(int, tag[1:].split("."))):
            raise ValueError("a newer Homebrew release is already published")
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(content)
    def git(*args):
        subprocess.run(["git", *args], cwd=tap, check=True)
    git("add", "--", "Formula/smackdebt.rb")
    changed = subprocess.run(["git", "diff", "--cached", "--quiet", "--", "Formula/smackdebt.rb"], cwd=tap)
    if changed.returncode == 0:
        print("Homebrew already has this formula")
        return
    if changed.returncode != 1:
        raise RuntimeError("could not compare Homebrew formula")
    git("-c", "user.name=github-actions[bot]", "-c", "user.email=41898282+github-actions[bot]@users.noreply.github.com",
        "commit", "-m", f"chore: update smackdebt to {tag}", "--", "Formula/smackdebt.rb")
    git("push", "origin", "HEAD")


if __name__ == "__main__":
    if len(sys.argv) != 4:
        raise SystemExit("Usage: publish-homebrew.py FORMULA TAP VERSION_TAG")
    try:
        publish(Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3])
    except (ValueError, OSError, RuntimeError, subprocess.CalledProcessError) as error:
        raise SystemExit(str(error)) from error
