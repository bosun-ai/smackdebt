"""Check that a release contains the source used to record its evidence."""

from __future__ import annotations

import subprocess
from pathlib import Path, PurePosixPath


PUBLIC_PROFILES = {
    "evolution-dense",
    "evolution-wide",
    "graph-dense",
    "graph-sparse",
    "hundred-file",
    "large-dependency-diff",
    "many-package",
    "one-file",
    "small-diff",
}


def evidence_path_is_allowed(value: str) -> bool:
    path = PurePosixPath(value)
    parts = path.parts
    if len(parts) == 3 and parts[:2] == ("benchmarks", "baselines"):
        return (
            path.name.removesuffix(".json") in PUBLIC_PROFILES
            and path.suffix == ".json"
        )
    if len(parts) == 3 and parts[:2] == ("benchmarks", "evidence"):
        if path.name == "workload-reviews.json":
            return True
        for suffix in (".metadata.json", ".runs.jsonl"):
            if path.name.endswith(suffix):
                return path.name.removesuffix(suffix) in PUBLIC_PROFILES
        return False
    return False


def release_revision_problem(
    recorded_revision: object,
    repository: Path,
    head: str | None = None,
) -> str | None:
    if head is None:
        head = _git(repository, "rev-parse", "HEAD")
    if (
        not isinstance(recorded_revision, str)
        or len(recorded_revision) != 40
        or any(character not in "0123456789abcdef" for character in recorded_revision)
    ):
        return "recorded revision must be a full Git commit id"
    try:
        _git(repository, "cat-file", "-e", f"{recorded_revision}^{{commit}}")
    except subprocess.CalledProcessError:
        return "recorded release commit is missing; fetch it before checking evidence"
    worktree_paths = _worktree_paths(repository)
    if recorded_revision == head:
        if not worktree_paths:
            return "pre-commit release check requires uncommitted release evidence"
        return _path_problem(worktree_paths)
    if worktree_paths:
        return "post-commit release check requires a clean worktree"
    # GitHub can merge, squash, or rebase the release PR. Compare the complete
    # trees, not parent counts: only the reviewed evidence may differ.
    changed = _git_paths(
        repository,
        "diff",
        "--no-renames",
        "--name-only",
        "-z",
        recorded_revision,
        head,
        "--",
    )
    return _path_problem(changed)


def _path_problem(paths: list[str]) -> str | None:
    if not paths or any(not evidence_path_is_allowed(path) for path in paths):
        return "release changes must use only approved evidence paths"
    if not any(
        path.startswith("benchmarks/baselines/")
        or path.startswith("benchmarks/evidence/")
        for path in paths
    ):
        return "release changes must include public performance or workload evidence"
    return None


def _worktree_paths(repository: Path) -> list[str]:
    commands = [
        ("diff", "--name-only", "-z", "HEAD"),
        ("diff", "--cached", "--name-only", "-z", "HEAD"),
        ("ls-files", "--others", "--exclude-standard", "-z"),
    ]
    return sorted(
        {
            path
            for arguments in commands
            for path in _git_paths(repository, *arguments)
        }
    )


def _git(repository: Path, *arguments: str) -> str:
    return _run_git(repository, *arguments).rstrip("\n")


def _git_paths(repository: Path, *arguments: str) -> list[str]:
    return [path for path in _run_git(repository, *arguments).split("\0") if path]


def _run_git(repository: Path, *arguments: str) -> str:
    return subprocess.run(
        ["git", *arguments],
        cwd=repository,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
