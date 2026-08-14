import copy
import json
import importlib.util
import subprocess
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("workload.py")
RUNNER = Path(__file__).with_name("run.sh")
REPORT_CHECKER = Path(__file__).with_name("check-report.py")
REVIEW_CHECKER = Path(__file__).with_name("check-workload-reviews.py")
WORKLOAD_REVIEWER = Path(__file__).with_name("review-workloads.py")


def load_report_checker():
    spec = importlib.util.spec_from_file_location("performance_check_report", REPORT_CHECKER)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load_review_checker():
    spec = importlib.util.spec_from_file_location("performance_check_reviews", REVIEW_CHECKER)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load_workload_reviewer():
    spec = importlib.util.spec_from_file_location("performance_review_workloads", WORKLOAD_REVIEWER)
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
        for _ in range(8)
    ]


def package_reference_report():
    return {
        "packages": [{"id": 0}, {"id": 1}],
        "files": [{"package": 0}, {"package": 1}],
        "dependency_edges": [{"source": 0, "target": 1}],
        "package_graph": [{"package": 0}, {"package": 1}],
        "package_edges": [{"source": 0, "target": 1, "file_edges": [0]}],
        "architecture_findings": [
            {"packages": [0, 1], "files": [0, 1], "witness_edges": [0]}
        ],
        "architecture_comparisons": [
            {"packages": [0, 1], "witness": [0, 1], "files": [0, 1]}
        ],
        "package_history": [{"package": 0}],
        "contributor_concentration": [{"package": 1}],
        "change_coupling": [{"left": 0, "right": 1}],
        "evolutionary_findings": [{"left": 0, "right": 1}],
        "evolutionary_comparisons": [{"left": 0, "right": 1}],
    }


def history_review_report():
    return {
        "root": 0,
        "paths": ["src/alpha.rs", "src/bravo.rs"],
        "files": [{"path": 0}, {"path": 1}],
        "packages": [
            {"id": 0, "path": "alpha"},
            {"id": 1, "path": "bravo"},
            {"id": 2, "path": "charlie"},
            {"id": 3, "path": "delta"},
            {"id": 4, "path": "echo"},
        ],
        "scopes": [
            {
                "evolutionary_findings": [0, 1, 2, 3],
                "architecture_findings": [],
            }
        ],
        "evolutionary_findings": [
            {
                "id": 0,
                "left": 0,
                "right": 1,
                "shared_commits": 3,
                "union_commits": 4,
                "similarity": 0.75,
            },
            {
                "id": 1,
                "left": 3,
                "right": 4,
                "shared_commits": 5,
                "union_commits": 10,
                "similarity": 0.5,
            },
            {
                "id": 2,
                "left": 0,
                "right": 2,
                "shared_commits": 5,
                "union_commits": 8,
                "similarity": 0.625,
            },
            {
                "id": 3,
                "left": 1,
                "right": 3,
                "shared_commits": 5,
                "union_commits": 8,
                "similarity": 0.625,
            },
            {
                "id": 4,
                "left": 2,
                "right": 4,
                "shared_commits": 2,
                "union_commits": 2,
                "similarity": 1.0,
            },
        ],
        "package_edges": [],
        "architecture_findings": [],
        "dependency_edges": [],
    }


def strongest_history_terminal():
    return (
        "\nHISTORY\n"
        " alpha ↔ charlie changed together in 5 of 8 commits · 63% · no code dependency\n"
        " bravo ↔ delta changed together in 5 of 8 commits · 63% · no code dependency\n"
        " delta ↔ echo changed together in 5 of 10 commits · 50% · no code dependency\n"
    )


class WorkloadHarnessTests(unittest.TestCase):
    def run_tool(self, *arguments):
        return subprocess.run(["python3", str(SCRIPT), *arguments], check=True, capture_output=True, text=True)

    def test_generation_is_repeatable_and_check_reports_identity(self):
        with tempfile.TemporaryDirectory() as first, tempfile.TemporaryDirectory() as second:
            first_path = Path(first) / "workload"
            second_path = Path(second) / "workload"
            self.run_tool("generate", "--output", str(first_path), "--profile", "hundred-file", "--seed", "7")
            self.run_tool("generate", "--output", str(second_path), "--profile", "hundred-file", "--seed", "7")
            first_manifest = json.loads((first_path / "manifest.json").read_text())
            second_manifest = json.loads((second_path / "manifest.json").read_text())
            self.assertEqual(first_manifest, second_manifest)
            self.assertEqual(first_manifest["files"], 102)
            checked = json.loads(self.run_tool("check", "--input", str(first_path)).stdout)
            self.assertTrue(checked["verified"])

    def test_small_diff_has_fixed_changed_file_count_and_git_base(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "workload"
            self.run_tool("generate", "--output", str(root), "--profile", "small-diff", "--seed", "9")
            manifest = json.loads((root / "manifest.json").read_text())
            self.assertEqual(manifest["diff_files"], 4)
            status = subprocess.run(["git", "status", "--porcelain"], cwd=root, check=True, capture_output=True, text=True)
            self.assertEqual(len(status.stdout.splitlines()), 4)
            metadata = json.loads(self.run_tool("metadata", "--input", str(root)).stdout)
            self.assertEqual(metadata["schema_version"], 1)
            self.assertTrue(metadata["git_revision"])

    def test_check_rejects_changed_source(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "workload"
            self.run_tool("generate", "--output", str(root), "--profile", "one-file")
            source = next((root / "src").rglob("*.rs"))
            source.write_text(source.read_text() + "// changed\n")
            result = subprocess.run(["python3", str(SCRIPT), "check", "--input", str(root)], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)

    def test_graph_profiles_have_declared_package_and_edge_shapes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "workload"
            self.run_tool("generate", "--output", str(root), "--profile", "graph-dense", "--files", "40")
            manifest = json.loads((root / "manifest.json").read_text())
            self.assertEqual(manifest["language_files"], {"javascript": 40})
            self.assertEqual(len(list(root.glob("package-*/package.json"))), 4)
            source = (root / "package-0000/unit-000000.js").read_text()
            self.assertGreaterEqual(source.count("import dependency_"), 3)

    def test_large_dependency_diff_changes_declared_files_against_a_real_base(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "workload"
            self.run_tool("generate", "--output", str(root), "--profile", "large-dependency-diff", "--files", "300")
            manifest = json.loads((root / "manifest.json").read_text())
            self.assertEqual(manifest["diff_files"], 200)
            status = subprocess.run(["git", "status", "--porcelain"], cwd=root, check=True, capture_output=True, text=True)
            self.assertEqual(len(status.stdout.splitlines()), 200)

    def test_evolution_profile_has_dense_two_commit_package_history(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "workload"
            self.run_tool("generate", "--output", str(root), "--profile", "evolution-dense", "--files", "20")
            commits = subprocess.run(["git", "rev-list", "--count", "HEAD"], cwd=root, check=True, capture_output=True, text=True)
            self.assertEqual(commits.stdout.strip(), "2")
            self.assertEqual(len(list(root.glob("package-*/package.json"))), 20)
            self.assertFalse(subprocess.run(["git", "status", "--porcelain"], cwd=root, check=True, capture_output=True, text=True).stdout)

    def test_runner_checks_before_each_measured_command(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "workload"
            subprocess.run(["sh", str(RUNNER), "--profile", "one-file", "--output", str(root), "--repeat", "2", "--", "python3", "-c", "import pathlib,sys; assert pathlib.Path(sys.argv[1]).is_dir()"], check=True, capture_output=True, text=True)
            records = (root / "runs.jsonl").read_text().splitlines()
            self.assertEqual(len(records), 2)
            self.assertTrue(all(json.loads(record)["wall_time_ns"] > 0 for record in records))
            self.assertTrue(all("parser_time_ns" in json.loads(record) for record in records))

    def test_report_privacy_rejects_a_nested_identity_field(self):
        checker = load_report_checker()
        with self.assertRaises(SystemExit):
            checker.audit_private_values(
                {"outer": [{"contributor_email": "private@example.invalid"}]}
            )

    def test_workload_review_privacy_rejects_nested_identity_and_path_values(self):
        checker = load_review_checker()
        problems = checker.audit(
            {"reviews": [{"outcomes": {"author_identity": "private", "detail": "/private/repository"}}]}
        )
        self.assertEqual(
            problems,
            [
                "private field evidence.reviews[0].outcomes.author_identity",
                "absolute path at evidence.reviews[0].outcomes.detail",
            ],
        )

    def test_terminal_review_categories_match_relevant_default_output(self):
        reviewer = load_workload_reviewer()
        report = {
            "root": 0,
            "paths": ["app/main.rb", "db/schema.rb"],
            "files": [{"path": 0}, {"path": 1}],
            "findings": [
                {
                    "file": 0,
                    "name": "important",
                    "container": "Suite",
                    "start_line": 7,
                    "role": "primary",
                    "trust": "trusted",
                    "rating": "high",
                    "measurements": {"cognitive_complexity": 30},
                },
                {
                    "file": 1,
                    "name": "schema",
                    "container": None,
                    "start_line": 1,
                    "role": "generated",
                    "trust": "trusted",
                    "rating": "watch",
                    "measurements": {"cognitive_complexity": 15},
                },
            ],
            "health": [
                {"high": 1, "watch": 0},
                {"high": 1, "watch": 0},
                {"high": 0, "watch": 0},
            ],
            "scopes": [
                {
                    "health": 0,
                    "children": [1, 2],
                    "architecture_findings": [],
                    "evolutionary_findings": [0],
                },
                {"health": 1},
                {"health": 2},
            ],
            "packages": [{"path": "app"}, {"path": "support"}],
            "evolutionary_findings": [
                {
                    "id": 0,
                    "left": 0,
                    "right": 1,
                    "shared_commits": 3,
                    "union_commits": 4,
                    "similarity": 0.75,
                }
            ],
            "package_edges": [],
            "architecture_findings": [],
            "dependency_edges": [],
        }
        terminal = (
            "\nFINDINGS\n"
            "  Suite::important · function\n"
            "        app/main.rb:7\n\n"
            "HISTORY\n"
            " app ↔ support changed together in 3 of 4 commits · 75% · no code dependency\n"
        )
        self.assertTrue(reviewer.important_debt_leads(report, terminal))
        self.assertTrue(reviewer.empty_optional_sections_are_absent(report, terminal))
        self.assertFalse(
            reviewer.empty_optional_sections_are_absent(report, terminal + "\nAREAS\n")
        )
        self.assertTrue(
            reviewer.generated_rails_schema_is_outside_default_debt(report, terminal)
        )
        self.assertTrue(reviewer.weak_history_and_graph_facts_are_absent(report, terminal))

        generated_first = terminal.replace(
            "  Suite::important · function\n        app/main.rb:7",
            "  schema · function\n        db/schema.rb:1\n"
            "  Suite::important · function\n        app/main.rb:7",
        )
        self.assertFalse(reviewer.important_debt_leads(report, generated_first))

        test_first = copy.deepcopy(report)
        test_first["findings"][0]["role"] = "test"
        self.assertTrue(reviewer.important_debt_leads(test_first, terminal))
        self.assertFalse(
            reviewer.primary_application_finding_leads(test_first, terminal)
        )
        self.assertTrue(reviewer.primary_application_finding_leads(report, terminal))

        rust_first = copy.deepcopy(test_first)
        rust_first["paths"][0] = "src/lib.rs"
        rust_terminal = terminal.replace("app/main.rb:7", "src/lib.rs:7")
        self.assertTrue(reviewer.useful_rust_finding_leads(rust_first, rust_terminal))
        self.assertFalse(reviewer.useful_rust_finding_leads(test_first, terminal))
        wrong_container = terminal.replace("Suite::important", "Other::important")
        self.assertIsNone(reviewer.first_displayed_finding(report, wrong_container))

        no_finding = copy.deepcopy(report)
        no_finding["scopes"][0]["evolutionary_findings"] = []
        no_finding["evolutionary_findings"] = []
        self.assertFalse(
            reviewer.weak_history_and_graph_facts_are_absent(no_finding, terminal)
        )
        weak_history = terminal.replace(
            "\n\nHISTORY\n", "\n\nHISTORY\n  app · 3 commits\n"
        )
        self.assertFalse(reviewer.weak_history_and_graph_facts_are_absent(report, weak_history))

        graph_report = copy.deepcopy(report)
        graph_report["scopes"][0]["architecture_findings"] = [0]
        graph_report["dependency_edges"] = [
            {"source": 0, "target": 1},
            {"source": 1, "target": 0},
        ]
        graph_report["architecture_findings"] = [
            {
                "kind": "package_cycle",
                "witness_edges": [0, 1],
            }
        ]
        graph_terminal = terminal.replace(
            "\n\nHISTORY\n",
            "\n\nARCHITECTURE\n"
            " package dependency cycle\n"
            "        app/main.rb → db/schema.rb → app/main.rb\n\n"
            "HISTORY\n",
        )
        self.assertTrue(
            reviewer.weak_history_and_graph_facts_are_absent(
                graph_report, graph_terminal
            )
        )
        weak_graph = graph_terminal.replace(
            "        app/main.rb → db/schema.rb → app/main.rb",
            "  app/main.rb → support/main.rb · uses",
        )
        self.assertFalse(
            reviewer.weak_history_and_graph_facts_are_absent(graph_report, weak_graph)
        )

    def test_default_history_requires_exact_strongest_rows_and_no_graph_detail(self):
        reviewer = load_workload_reviewer()
        report = history_review_report()
        terminal = strongest_history_terminal()
        self.assertTrue(
            reviewer.weak_history_and_graph_facts_are_absent(report, terminal)
        )

        rows = terminal.splitlines()
        wrong_order = "\n".join([rows[0], rows[1], rows[3], rows[2], rows[4]]) + "\n"
        self.assertFalse(
            reviewer.weak_history_and_graph_facts_are_absent(report, wrong_order)
        )

        weak_report = copy.deepcopy(report)
        weak_report["scopes"][0]["evolutionary_findings"].append(4)
        weak_included = terminal + (
            " charlie ↔ echo changed together in 2 of 2 commits · 100% · "
            "no code dependency\n"
        )
        self.assertFalse(
            reviewer.weak_history_and_graph_facts_are_absent(
                weak_report, weak_included
            )
        )

        self.assertFalse(
            reviewer.weak_history_and_graph_facts_are_absent(
                report, terminal + "  extra history text\n"
            )
        )

        graph_report = copy.deepcopy(report)
        graph_report["scopes"][0]["architecture_findings"] = [0]
        graph_report["dependency_edges"] = [
            {"source": 0, "target": 1},
            {"source": 1, "target": 0},
        ]
        graph_report["architecture_findings"] = [
            {"kind": "file_cycle", "witness_edges": [0, 1]}
        ]
        graph = (
            "\nARCHITECTURE\n"
            " file dependency cycle\n"
            "        src/alpha.rs → src/bravo.rs → src/alpha.rs\n"
        )
        self.assertTrue(
            reviewer.weak_history_and_graph_facts_are_absent(
                graph_report, graph + terminal
            )
        )
        self.assertFalse(
            reviewer.weak_history_and_graph_facts_are_absent(
                graph_report,
                graph + "  src/alpha.rs → src/bravo.rs · 1 import\n" + terminal,
            )
        )

    def test_history_uses_displayed_root_name_for_tie_order(self):
        reviewer = load_workload_reviewer()
        report = history_review_report()
        report["packages"][0]["path"] = "."
        report["scopes"][0]["evolutionary_findings"] = [2, 3]
        terminal = (
            "\nHISTORY\n"
            " bravo ↔ delta changed together in 5 of 8 commits · 63% · "
            "no code dependency\n"
            " repository root ↔ charlie changed together in 5 of 8 commits · "
            "63% · no code dependency\n"
        )
        self.assertTrue(
            reviewer.weak_history_and_graph_facts_are_absent(report, terminal)
        )
        self.assertFalse(
            reviewer.weak_history_and_graph_facts_are_absent(
                report,
                terminal.replace(
                    " bravo ↔ delta changed together in 5 of 8 commits · 63% · "
                    "no code dependency\n"
                    " repository root ↔ charlie",
                    " repository root ↔ charlie changed together in 5 of 8 commits "
                    "· 63% · no code dependency\n bravo ↔ delta",
                ),
            )
        )

    def test_empty_optional_headings_are_not_treated_as_absent(self):
        reviewer = load_workload_reviewer()
        report = history_review_report()
        report["scopes"][0]["evolutionary_findings"] = []
        terminal = "\nQUALITY\n1 rated unit\n"
        self.assertTrue(
            reviewer.weak_history_and_graph_facts_are_absent(report, terminal)
        )
        self.assertFalse(
            reviewer.weak_history_and_graph_facts_are_absent(
                report, terminal + "\nHISTORY\n"
            )
        )
        self.assertFalse(
            reviewer.weak_history_and_graph_facts_are_absent(
                report, terminal + "\nARCHITECTURE\n"
            )
        )
        self.assertFalse(
            reviewer.weak_history_and_graph_facts_are_absent(
                report, terminal + "\nHISTORY\n\nARCHITECTURE\n"
            )
        )

    def test_package_reference_check_covers_every_package_bearing_table(self):
        reviewer = load_workload_reviewer()
        report = package_reference_report()
        self.assertTrue(reviewer.package_references_are_valid(report))
        mutations = {
            "package ids": lambda value: value["packages"][1].update(id=2),
            "file packages": lambda value: value["files"][0].update(package=2),
            "file edge source": lambda value: value["dependency_edges"][0].update(source=2),
            "file edge target": lambda value: value["dependency_edges"][0].update(target=2),
            "package graph": lambda value: value["package_graph"][0].update(package=2),
            "package edge source": lambda value: value["package_edges"][0].update(source=2),
            "package edge target": lambda value: value["package_edges"][0].update(target=2),
            "package file edge": lambda value: value["package_edges"][0].update(file_edges=[1]),
            "architecture packages": lambda value: value["architecture_findings"][0].update(packages=[2]),
            "architecture files": lambda value: value["architecture_findings"][0].update(files=[2]),
            "architecture edges": lambda value: value["architecture_findings"][0].update(witness_edges=[1]),
            "comparison packages": lambda value: value["architecture_comparisons"][0].update(packages=[2]),
            "comparison witness": lambda value: value["architecture_comparisons"][0].update(witness=[2]),
            "comparison files": lambda value: value["architecture_comparisons"][0].update(files=[2]),
            "package history": lambda value: value["package_history"][0].update(package=2),
            "concentration": lambda value: value["contributor_concentration"][0].update(package=2),
            "coupling order": lambda value: value["change_coupling"][0].update(left=1, right=0),
            "finding package": lambda value: value["evolutionary_findings"][0].update(right=2),
            "comparison order": lambda value: value["evolutionary_comparisons"][0].update(left=1, right=0),
        }
        for name, mutate in mutations.items():
            with self.subTest(name=name):
                changed = copy.deepcopy(report)
                mutate(changed)
                self.assertFalse(reviewer.package_references_are_valid(changed))

    def test_aggregate_review_checker_accepts_the_exact_shape(self):
        checker = load_review_checker()
        self.assertEqual(
            checker.validate(valid_review_record(), valid_baselines(), False), []
        )

    def test_aggregate_review_checker_rejects_state_and_shape_changes(self):
        checker = load_review_checker()
        mutations = {
            "revision": lambda value: value.update(workspace_revision="short"),
            "dirty": lambda value: value.update(workspace_dirty=True),
            "family": lambda value: value["reviews"][0].update(family="changed"),
            "outcome": lambda value: value["reviews"][0]["outcomes"].pop(
                "empty_optional_sections_absent"
            ),
        }
        for name, mutate in mutations.items():
            with self.subTest(name=name):
                record = valid_review_record()
                mutate(record)
                self.assertTrue(checker.validate(record, valid_baselines(), False))

    def test_aggregate_review_checker_rejects_mixed_baseline_revisions(self):
        checker = load_review_checker()
        baselines = valid_baselines()
        baselines[-1]["workspace_revision"] = "b" * 40
        problems = checker.validate(valid_review_record(), baselines, False)
        self.assertIn("workload reviews and public profiles must share one revision", problems)

    def test_aggregate_review_checker_requires_head_in_release_mode(self):
        checker = load_review_checker()
        problems = checker.validate(
            valid_review_record(), valid_baselines(), True, head="b" * 40
        )
        self.assertIn("workload review revision must equal HEAD", problems)


if __name__ == "__main__":
    unittest.main()
