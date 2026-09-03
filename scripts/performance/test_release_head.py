import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


REVIEW_CHECKER = Path(__file__).with_name("check-workload-reviews.py")
WORKLOAD_REVIEWER = Path(__file__).with_name("review-workloads.py")
sys.path.insert(0, str(WORKLOAD_REVIEWER.parent))
import release_head


def load_review_checker():
    spec = importlib.util.spec_from_file_location("performance_check_reviews", REVIEW_CHECKER)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def valid_review_record(revision="a" * 40):
    checker = load_review_checker()
    return {
        "schema_version": 1,
        "workspace_revision": revision,
        "workspace_dirty": False,
        "reviews": [
            {
                "family": family,
                "outcomes": {outcome: True for outcome in outcomes},
            }
            for family, outcomes in checker.EXPECTED.items()
        ],
    }


def valid_baselines(revision="a" * 40):
    return [
        {"workspace_revision": revision, "workspace_dirty": False}
        for _ in range(9)
    ]


def git(repository: Path, *arguments: str) -> str:
    return subprocess.run(
        ["git", *arguments],
        cwd=repository,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


def initialize_release_repository(repository: Path) -> str:
    git(repository, "init", "-q", "-b", "main")
    git(repository, "config", "user.name", "Release Test")
    git(repository, "config", "user.email", "release@example.invalid")
    return commit_release_path(repository, "README.md", "initial\n", "initial")


def commit_release_path(repository: Path, path: str, content: str, message: str) -> str:
    target = repository / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(content, encoding="utf-8")
    git(repository, "add", path)
    git(repository, "commit", "-qm", message)
    return git(repository, "rev-parse", "HEAD")



class ReleaseHeadTests(unittest.TestCase):

    def test_release_revision_accepts_head_and_one_evidence_only_child(self):
        checker = load_review_checker()
        for path in [
            "benchmarks/baselines/one-file.json",
            "benchmarks/baselines/evolution-wide.json",
            "benchmarks/evidence/one-file.metadata.json",
            "benchmarks/evidence/evolution-wide.metadata.json",
            "benchmarks/evidence/one-file.runs.jsonl",
            "benchmarks/evidence/evolution-wide.runs.jsonl",
            "benchmarks/evidence/workload-reviews.json",
            "openspec/changes/example/tasks.md",
        ]:
            with self.subTest(path=path):
                self.assertTrue(release_head.evidence_path_is_allowed(path))
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            recorded = initialize_release_repository(repository)
            self.assertIsNotNone(
                release_head.release_revision_problem(recorded, repository)
            )
            head = commit_release_path(
                repository,
                "benchmarks/baselines/one-file.json",
                "{}\n",
                "record evidence",
            )
            self.assertIsNone(
                release_head.release_revision_problem(recorded, repository, head)
            )
            self.assertEqual(
                checker.validate(
                    valid_review_record(recorded),
                    valid_baselines(recorded),
                    True,
                    head=head,
                    repository=repository,
                ),
                [],
            )

    def test_release_revision_accepts_approved_tracked_and_untracked_evidence(self):
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            initialize_release_repository(repository)
            recorded = commit_release_path(
                repository,
                "benchmarks/baselines/one-file.json",
                "{\"run\":1}\n",
                "initial evidence",
            )
            (repository / "benchmarks/baselines/one-file.json").write_text(
                "{\"run\":2}\n", encoding="utf-8"
            )
            untracked = repository / "benchmarks/evidence/one-file.metadata.json"
            untracked.parent.mkdir(parents=True, exist_ok=True)
            untracked.write_text("{}\n", encoding="utf-8")
            self.assertIsNone(
                release_head.release_revision_problem(recorded, repository)
            )

    def test_release_revision_checks_untracked_paths_without_tracked_changes(self):
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            recorded = initialize_release_repository(repository)
            evidence = repository / "benchmarks/evidence/one-file.metadata.json"
            evidence.parent.mkdir(parents=True, exist_ok=True)
            evidence.write_text("{}\n", encoding="utf-8")
            self.assertIsNone(
                release_head.release_revision_problem(recorded, repository)
            )

        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            recorded = initialize_release_repository(repository)
            source = repository / "src/private.rs"
            source.parent.mkdir(parents=True, exist_ok=True)
            source.write_text("fn private() {}\n", encoding="utf-8")
            self.assertEqual(
                release_head.release_revision_problem(recorded, repository),
                "release changes must use only approved evidence and task paths",
            )

        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            recorded = initialize_release_repository(repository)
            evidence = repository / "benchmarks/evidence/one-file.runs.jsonl"
            evidence.parent.mkdir(parents=True, exist_ok=True)
            evidence.write_text("{}\n", encoding="utf-8")
            documentation = repository / "private-notes.md"
            documentation.write_text("private\n", encoding="utf-8")
            self.assertEqual(
                release_head.release_revision_problem(recorded, repository),
                "release changes must use only approved evidence and task paths",
            )

    def test_release_revision_preserves_space_boundaries_in_git_paths(self):
        lookalikes = [
            " benchmarks/evidence/one-file.metadata.json",
            "benchmarks/evidence/one-file.metadata.json ",
        ]
        expected = "release changes must use only approved evidence and task paths"

        for path in lookalikes:
            with self.subTest(kind="tracked", path=path):
                with tempfile.TemporaryDirectory() as directory:
                    repository = Path(directory)
                    initialize_release_repository(repository)
                    recorded = commit_release_path(
                        repository, path, "before\n", "track lookalike"
                    )
                    (repository / path).write_text("after\n", encoding="utf-8")
                    self.assertEqual(
                        release_head.release_revision_problem(recorded, repository),
                        expected,
                    )

            with self.subTest(kind="untracked", path=path):
                with tempfile.TemporaryDirectory() as directory:
                    repository = Path(directory)
                    recorded = initialize_release_repository(repository)
                    evidence = repository / "benchmarks/evidence/one-file.runs.jsonl"
                    evidence.parent.mkdir(parents=True, exist_ok=True)
                    evidence.write_text("{}\n", encoding="utf-8")
                    lookalike = repository / path
                    lookalike.parent.mkdir(parents=True, exist_ok=True)
                    lookalike.write_text("{}\n", encoding="utf-8")
                    self.assertEqual(
                        release_head.release_revision_problem(recorded, repository),
                        expected,
                    )

    def test_release_revision_rejects_unrelated_or_multiple_children(self):
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            recorded = initialize_release_repository(repository)
            unrelated = commit_release_path(
                repository, "src/main.rs", "fn main() {}\n", "change source"
            )
            self.assertIsNotNone(
                release_head.release_revision_problem(
                    recorded, repository, unrelated
                )
            )

        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            recorded = initialize_release_repository(repository)
            commit_release_path(
                repository,
                "benchmarks/evidence/one-file.metadata.json",
                "{}\n",
                "record metadata",
            )
            second = commit_release_path(
                repository,
                "benchmarks/evidence/one-file.runs.jsonl",
                "{}\n",
                "record runs",
            )
            self.assertIsNotNone(
                release_head.release_revision_problem(recorded, repository, second)
            )

    def test_release_revision_rejects_path_boundaries_and_merges(self):
        for path in [
            "benchmarks/baselines/nested/one-file.json",
            "benchmarks/baselines/private-profile.json",
            "benchmarks/evidence/private-profile.metadata.json",
            "benchmarks/evidence/private-profile.runs.jsonl",
            "benchmarks/evidence/private.txt",
            "openspec/changes/example/notes/tasks.md",
        ]:
            with self.subTest(path=path), tempfile.TemporaryDirectory() as directory:
                repository = Path(directory)
                recorded = initialize_release_repository(repository)
                head = commit_release_path(repository, path, "x\n", "wrong path")
                self.assertIsNotNone(
                    release_head.release_revision_problem(recorded, repository, head)
                )

        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            initialize_release_repository(repository)
            git(repository, "switch", "-qc", "evidence")
            commit_release_path(
                repository,
                "benchmarks/evidence/one-file.metadata.json",
                "{}\n",
                "branch evidence",
            )
            git(repository, "switch", "-q", "main")
            recorded = commit_release_path(
                repository,
                "benchmarks/baselines/one-file.json",
                "{}\n",
                "main evidence",
            )
            git(repository, "merge", "--no-ff", "-qm", "merge evidence", "evidence")
            self.assertIsNotNone(
                release_head.release_revision_problem(recorded, repository)
            )

    def test_release_revision_rejects_dirty_code_in_both_modes(self):
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            recorded = initialize_release_repository(repository)
            (repository / "README.md").write_text("changed\n", encoding="utf-8")
            self.assertIsNotNone(
                release_head.release_revision_problem(recorded, repository)
            )

        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            recorded = initialize_release_repository(repository)
            commit_release_path(
                repository,
                "benchmarks/evidence/workload-reviews.json",
                "{}\n",
                "record review",
            )
            (repository / "README.md").write_text("changed\n", encoding="utf-8")
            self.assertIsNotNone(
                release_head.release_revision_problem(recorded, repository)
            )

    def test_release_revision_requires_evidence_beside_optional_tasks(self):
        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            recorded = initialize_release_repository(repository)
            task = repository / "openspec/changes/example/tasks.md"
            task.parent.mkdir(parents=True, exist_ok=True)
            task.write_text("- [x] done\n", encoding="utf-8")
            self.assertIsNotNone(
                release_head.release_revision_problem(recorded, repository)
            )

        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            recorded = initialize_release_repository(repository)
            tasks_only = commit_release_path(
                repository,
                "openspec/changes/example/tasks.md",
                "- [x] done\n",
                "update tasks",
            )
            self.assertIsNotNone(
                release_head.release_revision_problem(
                    recorded, repository, tasks_only
                )
            )

        with tempfile.TemporaryDirectory() as directory:
            repository = Path(directory)
            recorded = initialize_release_repository(repository)
            commit_release_path(
                repository,
                "benchmarks/evidence/workload-reviews.json",
                "{}\n",
                "record review",
            )
            task = repository / "openspec/changes/example/tasks.md"
            task.parent.mkdir(parents=True, exist_ok=True)
            task.write_text("- [x] done\n", encoding="utf-8")
            git(repository, "add", task.relative_to(repository).as_posix())
            git(repository, "commit", "--amend", "--no-edit", "-q")
            self.assertIsNone(
                release_head.release_revision_problem(recorded, repository)
            )


if __name__ == "__main__":
    unittest.main()
