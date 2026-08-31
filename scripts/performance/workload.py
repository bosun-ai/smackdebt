#!/usr/bin/env python3
"""Create and verify deterministic public performance repositories."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
import time
from pathlib import Path


SCHEMA_VERSION = 1
PROFILES = {
    "one-file": 1,
    "hundred-file": 100,
    "small-diff": 100,
    "large-mixed": 100_000,
    "graph-sparse": 1_000,
    "graph-dense": 500,
    "many-package": 1_000,
    "evolution-dense": 100,
    "evolution-wide": 2_000,
    "large-dependency-diff": 1_000,
}
GRAPH_PROFILES = {
    "graph-sparse",
    "graph-dense",
    "many-package",
    "evolution-dense",
    "evolution-wide",
    "large-dependency-diff",
}
DIFF_PROFILES = {"small-diff", "large-dependency-diff"}
PACKAGE_SIZES = {"many-package": 1, "evolution-dense": 1, "evolution-wide": 40}
# The wide evolution profile keeps its last packages out of the dependency ring,
# so a pair drawn from two of them has no connection path at all and the package
# stage of the absence proof settles it without a walk.
WIDE_ISOLATED_PACKAGES = 8
WIDE_HIDDEN_PAIRS = 4
WIDE_HIDDEN_COMMITS = 5
WIDE_LEAKY_COMMITS = 6
WIDE_NOISE_GROUPS = 4
WIDE_NOISE_COMMITS = 3
WIDE_NOISE_FILES = 3
WIDE_BULK_PACKAGES = 30
WIDE_EPOCH = 946684800
LANGUAGES = ("rust", "python", "javascript", "typescript", "tsx", "java", "c", "cpp", "ruby", "vue")
EXTENSIONS = {
    "rust": ".rs", "python": ".py", "javascript": ".js", "typescript": ".ts",
    "tsx": ".tsx", "java": ".java", "c": ".c", "cpp": ".cpp", "ruby": ".rb", "vue": ".vue",
}


def _stable_bytes(profile: str, seed: int, language: str, index: int, lines: int) -> bytes:
    """Return source with stable bytes and nested control flow."""
    digest = hashlib.sha256(f"{profile}:{seed}:{language}:{index}".encode()).hexdigest()[:12]
    loops = max(2, index % 7 + 2)
    if language == "rust":
        rows = [f"pub fn generated_{index}(value: i32) -> i32 {{", f"    let mut total = {index % 11}i32;", f"    for item in 0..{loops} {{", "        if item % 2 == 0 { total += item; } else { total -= item; }", "    }", "    total + helper(value)", "}", "fn helper(value: i32) -> i32 { value + 1 }"]
    elif language == "python":
        rows = [f"def generated_{index}(value):", f"    total = {index % 11}", f"    for item in range({loops}):", "        if item % 2 == 0:", "            total += item", "        else:", "            total -= item", "    return total + helper(value)", "def helper(value):\n    return value + 1"]
    elif language in {"javascript", "typescript", "tsx"}:
        typed = ": number" if language == "typescript" else ""
        rows = [f"export function generated_{index}(value{typed}){typed} {{", f"  let total{typed} = {index % 11};", f"  for (let item = 0; item < {loops}; item++) {{", "    if (item % 2 === 0) { total += item; } else { total -= item; }", "  }", "  return total + helper(value);", "}", "function helper(value) { return value + 1; }"]
        if language == "tsx":
            rows.extend(["export function View() {", "  return <section>{generated_0(1)}</section>;", "}"])
    elif language == "java":
        rows = [f"class Generated{index} {{", f"  int generated(int value) {{ int total = {index % 11};", f"    for (int item = 0; item < {loops}; item++) {{", "      if (item % 2 == 0) { total += item; } else { total -= item; }", "    }", "    return total + helper(value);", "  }", "  int helper(int value) { return value + 1; }", "}"]
    elif language in {"c", "cpp"}:
        rows = [f"int generated_{index}(int value) {{", f"    int total = {index % 11};", f"    for (int item = 0; item < {loops}; item++) {{", "        if (item % 2 == 0) { total += item; } else { total -= item; }", "    }", "    return total + value + 1;", "}"]
    elif language == "ruby":
        rows = [f"def generated_{index}(value)", f"  total = {index % 11}", f"  ({loops}).times do |item|", "    if item.even? then total += item else total -= item end", "  end", "  total + value + 1", "end"]
    else:
        rows = ["<template>", "  <section v-if=\"visible\">{{ value + 1 }}</section>", "</template>", "<script setup lang=\"ts\">", "const value = 1;", "const visible = value > 0;", "</script>"]
    filler = "#" if language in {"python", "ruby"} else "<!--" if language == "vue" else "//"
    while len(rows) < lines:
        suffix = " -->" if language == "vue" else ""
        rows.append(f"{filler} generated filler {digest}-{len(rows)}{suffix}")
    return ("\n".join(rows[:lines]) + "\n").encode()


def _profile_count(profile: str, files: int | None) -> int:
    if profile not in PROFILES:
        raise ValueError(f"unknown profile: {profile}")
    count = PROFILES[profile] if files is None else files
    if count < 1 or count > 100_000:
        raise ValueError("files must be between 1 and 100000")
    return count


def _package_size(profile: str) -> int:
    """The files one package of a graph profile holds."""
    return PACKAGE_SIZES.get(profile, 10)


def _wide_bytes(index: int, count: int) -> bytes:
    """Return one wide-evolution unit: a ring between packages, a spine inside
    each of them, and nothing at all in the isolated tail."""
    package_size = _package_size("evolution-wide")
    packages = (count + package_size - 1) // package_size
    connected = max(1, packages - WIDE_ISOLATED_PACKAGES)
    package = index // package_size
    first = package * package_size
    rows = []
    if package < connected and index == first:
        target = (package + 1) % connected
        rows.append(
            f"import unit_{target} from '../package-{target:04}/unit-{target * package_size:06}';"
        )
    elif package < connected:
        rows.append(f"import unit_{package} from './unit-{first:06}';")
    rows.extend(
        [
            f"export default function unit_{index}(value) {{",
            f"  return value + {index % 17};",
            "}",
        ]
    )
    return ("\n".join(rows) + "\n").encode()


def _graph_bytes(profile: str, index: int, count: int, changed: bool = False) -> bytes:
    if profile == "evolution-wide":
        return _wide_bytes(index, count)
    package_size = _package_size(profile)
    package = index // package_size
    package_count = (count + package_size - 1) // package_size
    targets = [(package + 1) % package_count]
    if profile == "graph-dense":
        targets = [(package + offset) % package_count for offset in range(1, 11)]
    if changed and index < 200:
        targets.append((package - 1) % package_count)
    imports = []
    for target in sorted(set(targets)):
        target_file = min(target * package_size, count - 1)
        imports.append(
            f"import dependency_{target} from '../package-{target:04}/unit-{target_file:06}';"
        )
    rows = [
        *imports,
        f"export default function unit_{index}(value) {{",
        f"  return value + {index % 17};",
        "}",
    ]
    return ("\n".join(rows) + "\n").encode()


def _inventory(root: Path) -> tuple[list[Path], int, str, dict[str, int]]:
    files: list[Path] = []
    language_counts: dict[str, int] = {}
    source_bytes = 0
    digest = hashlib.sha256()
    generated = {"manifest.json", "metadata.json", "check.json", "runs.jsonl"}
    for path in sorted(root.rglob("*")):
        if not path.is_file() or (path.parent == root and path.name in generated) or ".git" in path.parts:
            continue
        relative = path.relative_to(root)
        data = path.read_bytes()
        files.append(relative)
        source_bytes += len(data)
        digest.update(relative.as_posix().encode())
        digest.update(b"\0")
        digest.update(data)
        for language, extension in EXTENSIONS.items():
            if path.suffix == extension:
                language_counts[language] = language_counts.get(language, 0) + 1
                break
    return files, source_bytes, digest.hexdigest(), language_counts


def _manifest(root: Path) -> dict:
    path = root / "manifest.json"
    if not path.is_file():
        raise ValueError(f"missing {path}")
    return json.loads(path.read_text())


def _git_commit(root: Path) -> None:
    if not shutil.which("git"):
        raise RuntimeError("git is required for the small-diff profile")
    subprocess.run(["git", "init", "-q"], cwd=root, check=True)
    subprocess.run(["git", "config", "user.email", "benchmark@example.invalid"], cwd=root, check=True)
    subprocess.run(["git", "config", "user.name", "Smackdebt Benchmark"], cwd=root, check=True)
    # A generated repository is written and read in one breath, so background
    # repacking has nothing to gain and a maintenance run racing a commit has
    # been observed to leave the object store unreadable mid-generation.
    subprocess.run(["git", "config", "gc.auto", "0"], cwd=root, check=True)
    subprocess.run(["git", "config", "maintenance.auto", "false"], cwd=root, check=True)
    env = os.environ.copy()
    env.update({"GIT_AUTHOR_DATE": "2000-01-01T00:00:00Z", "GIT_COMMITTER_DATE": "2000-01-01T00:00:00Z"})
    subprocess.run(["git", "add", "-A"], cwd=root, check=True, env=env)
    subprocess.run(["git", "commit", "-qm", "generated baseline"], cwd=root, check=True, env=env)


def _git_commit_evolution(root: Path) -> None:
    _git_commit(root)
    for path in sorted(root.glob("package-*/unit-*.js")):
        path.write_bytes(path.read_bytes() + b"// second history touch\n")
    env = os.environ.copy()
    env.update({"GIT_AUTHOR_DATE": "2000-01-02T00:00:00Z", "GIT_COMMITTER_DATE": "2000-01-02T00:00:00Z"})
    subprocess.run(["git", "add", "-A"], cwd=root, check=True, env=env)
    subprocess.run(["git", "commit", "-qm", "generated evolution"], cwd=root, check=True, env=env)


def _wide_touch(root: Path, paths: list[Path], day: int) -> None:
    """Record one commit that touches exactly the named files."""
    for path in paths:
        path.write_bytes(path.read_bytes() + f"// history touch {day}\n".encode())
    stamp = f"@{WIDE_EPOCH + day * 86400} +0000"
    env = os.environ.copy()
    env.update({"GIT_AUTHOR_DATE": stamp, "GIT_COMMITTER_DATE": stamp})
    subprocess.run(["git", "add", "-A"], cwd=root, check=True, env=env)
    subprocess.run(["git", "commit", "-qm", f"generated touch {day}"], cwd=root, check=True, env=env)


def _wide_events(root: Path) -> list[list[Path]]:
    """The file set of every deliberate commit, in commit order.

    The leaky pair is an importer and the interface it follows; each hidden
    pair is drawn from two packages the ring never joined; each noise group
    stays one commit below the support floor so a retained pair that is not a
    finding is measured too; the sweeping commit is what the bulk guard has to
    decline.
    """
    firsts, seconds = [], []
    for package in sorted(root.glob("package-*")):
        units = sorted(package.glob("unit-*.js"))
        firsts.append(units[0])
        seconds.append(units[min(1, len(units) - 1)])
    isolated = firsts[-WIDE_ISOLATED_PACKAGES:]
    connected = firsts[: max(1, len(firsts) - WIDE_ISOLATED_PACKAGES)]
    events = [[connected[0], connected[1 % len(connected)]]] * WIDE_LEAKY_COMMITS
    for pair in range(WIDE_HIDDEN_PAIRS):
        left = isolated[(pair * 2) % len(isolated)]
        right = isolated[(pair * 2 + 1) % len(isolated)]
        events.extend([[left, right]] * WIDE_HIDDEN_COMMITS)
    for group in range(WIDE_NOISE_GROUPS):
        members = [
            connected[(2 + group * WIDE_NOISE_FILES + offset) % len(connected)]
            for offset in range(WIDE_NOISE_FILES)
        ]
        events.extend([members] * WIDE_NOISE_COMMITS)
    events.append(seconds[-WIDE_BULK_PACKAGES:])
    return events


def _git_commit_wide(root: Path) -> None:
    _git_commit(root)
    for day, paths in enumerate(_wide_events(root), start=1):
        _wide_touch(root, sorted(set(paths)), day)


HISTORY_BUILDERS = {"evolution-dense": _git_commit_evolution, "evolution-wide": _git_commit_wide}


def generate(root: Path, profile: str, seed: int, files: int | None, lines: int) -> dict:
    count = _profile_count(profile, files)
    if lines < 8 or lines > 1000:
        raise ValueError("lines must be between 8 and 1000")
    root.mkdir(parents=True, exist_ok=True)
    if (root / ".git").exists():
        shutil.rmtree(root / ".git")
    for path in sorted(root.rglob("*"), reverse=True):
        if ".git" in path.parts:
            continue
        if path.is_file() and path.name not in {"manifest.json", "metadata.json"}:
            path.unlink()
        elif path.is_dir() and path != root and ".git" not in path.parts:
            shutil.rmtree(path)
    if profile in GRAPH_PROFILES:
        package_size = _package_size(profile)
        for index in range(count):
            package = index // package_size
            folder = root / f"package-{package:04}"
            folder.mkdir(parents=True, exist_ok=True)
            manifest = folder / "package.json"
            if not manifest.exists():
                manifest.write_text(f'{{"name":"package-{package:04}","private":true}}\n')
            (folder / f"unit-{index:06d}.js").write_bytes(_graph_bytes(profile, index, count))
    else:
        (root / "Cargo.toml").write_text("[package]\nname = \"generated-workload\"\nversion = \"0.1.0\"\n")
        (root / "package.json").write_text('{"name":"generated-workload","private":true}\n')
        for index in range(count):
            language = LANGUAGES[index % len(LANGUAGES)]
            folder = root / "src" / f"group-{index // 1000:03d}"
            folder.mkdir(parents=True, exist_ok=True)
            (folder / f"generated-{index:06d}{EXTENSIONS[language]}").write_bytes(_stable_bytes(profile, seed, language, index, lines))
    if profile in HISTORY_BUILDERS:
        HISTORY_BUILDERS[profile](root)
        (root / ".git" / "info" / "exclude").write_text("manifest.json\nmetadata.json\ncheck.json\nruns.jsonl\n")
    elif profile in DIFF_PROFILES:
        _git_commit(root)
        (root / ".git" / "info" / "exclude").write_text("manifest.json\nmetadata.json\ncheck.json\nruns.jsonl\n")
        if profile == "small-diff":
            for index in (1, 17, 63, 88):
                language = LANGUAGES[index % len(LANGUAGES)]
                path = root / "src" / f"group-{index // 1000:03d}" / f"generated-{index:06d}{EXTENSIONS[language]}"
                path.write_bytes(_stable_bytes(profile, seed + 1, language, index, lines))
        else:
            package_size = 10
            for index in range(200):
                package = index // package_size
                path = root / f"package-{package:04}" / f"unit-{index:06d}.js"
                path.write_bytes(_graph_bytes(profile, index, count, changed=True))
    files_found, source_bytes, digest, language_counts = _inventory(root)
    document = {"schema_version": SCHEMA_VERSION, "profile": profile, "seed": seed, "files": len(files_found), "source_bytes": source_bytes, "content_digest": digest, "language_files": language_counts, "lines_per_file": lines, "diff_files": 4 if profile == "small-diff" else 200 if profile == "large-dependency-diff" else 0}
    (root / "manifest.json").write_text(json.dumps(document, indent=2, sort_keys=True) + "\n")
    return document


def check(root: Path) -> dict:
    document = _manifest(root)
    files, source_bytes, digest, language_counts = _inventory(root)
    actual = {"files": len(files), "source_bytes": source_bytes, "content_digest": digest, "language_files": language_counts}
    expected = {key: document[key] for key in actual}
    if actual != expected:
        raise ValueError(json.dumps({"expected": expected, "actual": actual}, indent=2, sort_keys=True))
    return {**document, "verified": True}


def _command_version(command: str) -> str | None:
    if not shutil.which(command):
        return None
    return subprocess.run([command, "--version"], check=True, capture_output=True, text=True).stdout.strip()


def metadata(root: Path, private_metadata: bool = False) -> dict:
    if private_metadata:
        _, source_bytes, digest, language_counts = _inventory(root)
        verified = {
            "schema_version": SCHEMA_VERSION,
            "profile": "private-repository",
            "files": sum(language_counts.values()),
            "source_bytes": source_bytes,
            "content_digest": digest,
            "language_files": language_counts,
            "verified": True,
        }
    else:
        verified = check(root)
    revision = None
    dirty = None
    if (root / ".git").exists():
        revision = subprocess.run(["git", "rev-parse", "HEAD"], cwd=root, check=True, capture_output=True, text=True).stdout.strip()
        dirty = bool(subprocess.run(["git", "status", "--porcelain"], cwd=root, check=True, capture_output=True, text=True).stdout)
    return {**verified, "git_revision": revision, "git_dirty": dirty, "host": platform.platform(), "python": platform.python_version(), "rustc": _command_version("rustc"), "generated_at_unix": int(time.time())}


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    generator = commands.add_parser("generate")
    generator.add_argument("--output", type=Path, required=True)
    generator.add_argument("--profile", choices=sorted(PROFILES), required=True)
    generator.add_argument("--seed", type=int, default=1)
    generator.add_argument("--files", type=int)
    generator.add_argument("--lines", type=int, default=12)
    checker = commands.add_parser("check")
    checker.add_argument("--input", type=Path, required=True)
    metadata_parser = commands.add_parser("metadata")
    metadata_parser.add_argument("--input", type=Path, required=True)
    metadata_parser.add_argument("--output", type=Path)
    metadata_parser.add_argument("--private", action="store_true")
    args = parser.parse_args(argv)
    try:
        if args.command == "generate":
            result = generate(args.output, args.profile, args.seed, args.files, args.lines)
        elif args.command == "check":
            result = check(args.input)
        else:
            result = metadata(args.input, args.private)
        output = json.dumps(result, indent=2, sort_keys=True) + "\n"
        if args.command == "metadata" and args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(output)
        else:
            sys.stdout.write(output)
    except (OSError, RuntimeError, ValueError, subprocess.CalledProcessError) as error:
        print(f"performance workload error: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
