import copy
import json
import importlib.util
import sys
import unittest
from pathlib import Path


REPORT_CHECKER = Path(__file__).with_name("check-report.py")
REVIEW_CHECKER = Path(__file__).with_name("check-workload-reviews.py")
WORKLOAD_REVIEWER = Path(__file__).with_name("review-workloads.py")
sys.path.insert(0, str(WORKLOAD_REVIEWER.parent))


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


def problem_review_report(role="primary", path="app/main.rb"):
    return {
        "root": 0,
        "verdict": {"tier": "worn", "sentence": "Worn in the usual places."},
        "summary": {
            "checked": 10,
            "high": 1,
            "watch": 0,
            "worst": [{"path": path, "reason": "hot_and_complex"}],
        },
        "paths": [path, "db/schema.rb"],
        "files": [{"path": 0}, {"path": 1}],
        "findings": [
            {
                "file": 0,
                "name": "important",
                "container": "Suite",
                "start_line": 7,
                "role": role,
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
        "size_findings": [],
        "architecture_findings": [],
        "stable_dependency_findings": [],
        "evolutionary_findings": [],
        "knowledge_concentration_findings": [],
        "change_leakage_findings": [],
        "dependency_edges": [],
        "problems": [
            {
                "pattern": "hot_mess",
                "rating": "high",
                "visibility": "default",
                "anchor": {"kind": "file", "file": 0},
                "evidence": [{"kind": "findings", "index": 0}],
                "claimed": [{"table": "findings", "index": 0}],
            },
            {
                "pattern": "measured",
                "rating": "watch",
                "visibility": "detail",
                "anchor": {"kind": "file", "file": 1},
                "evidence": [{"kind": "findings", "index": 1}],
                "claimed": [{"table": "findings", "index": 1}],
            },
        ],
        "health": [
            {"high": 1, "watch": 0},
            {"high": 1, "watch": 0},
            {"high": 0, "watch": 1},
        ],
        "scopes": [
            {"health": 0, "children": [1, 2]},
            {"health": 1},
            {"health": 2},
        ],
        "packages": [{"path": "app"}, {"path": "support"}],
        "history_coverage": {
            "availability": "complete",
            "eligible_commits": 1,
            "window_days": 36500,
            "rename_gaps": 0,
        },
        "graph_evidence": {
            "suppressed_reach": 0,
            "suppressed_core": 0,
            "suppressed_leakage": 0,
        },
        "resolution_diagnostics": [{"kind": "unresolved"}],
        "diagnostics": [],
    }


def problem_review_terminal(path="app/main.rb"):
    return (
        "smackdebt · repository root\n"
        "  Worn in the usual places.\n"
        "1 high · 0 watch · 10 checked\n"
        f"worst: {path} — hot AND complex\n\n"
        "AREAS\n"
        "  app · 1 high · 0 watch\n"
        "  support · 0 high · 1 watch\n\n"
        "PROBLEMS\n"
        f"  high hot and complex · {path}\n\n"
        "WARNINGS\n"
        "  warning 1 import could not be followed · 1 named nothing in the repository\n\n"
        f"  next: smackdebt {path}\n"
    )


class WorkloadReviewTests(unittest.TestCase):

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

    def test_problem_review_uses_ranked_plain_codebase_contract(self):
        reviewer = load_workload_reviewer()
        report = problem_review_report()
        terminal = problem_review_terminal()

        self.assertTrue(reviewer.codebase_head_matches_report(report, terminal))
        self.assertTrue(reviewer.codebase_sections_match_report(report, terminal))
        self.assertTrue(reviewer.important_debt_leads(report, terminal))
        self.assertTrue(reviewer.primary_application_finding_leads(report, terminal))
        self.assertTrue(reviewer.generated_rails_schema_is_outside_default_debt(report, terminal))
        self.assertTrue(reviewer.weak_history_and_graph_facts_are_absent(report, terminal))

        primary_after_generated = copy.deepcopy(report)
        primary_after_generated["problems"][0]["claimed"] = [
            {"table": "findings", "index": 1},
            {"table": "findings", "index": 0},
        ]
        primary_after_generated["problems"][0]["evidence"].insert(
            0, {"kind": "findings", "index": 1}
        )
        primary_after_generated["problems"] = primary_after_generated["problems"][:1]
        self.assertTrue(
            reviewer.primary_application_finding_leads(
                primary_after_generated, terminal
            )
        )

        rust = problem_review_report(path="src/lib.rs")
        rust_terminal = problem_review_terminal(path="src/lib.rs")
        self.assertTrue(reviewer.useful_rust_finding_leads(rust, rust_terminal))

        for role in ("test", "example", "benchmark", "generated"):
            with self.subTest(role=role):
                non_primary = problem_review_report(role=role)
                self.assertFalse(
                    reviewer.primary_application_finding_leads(
                        non_primary, terminal
                    )
                )
        stale = terminal.replace("\nPROBLEMS\n", "\nFINDINGS\n")
        self.assertFalse(reviewer.codebase_sections_match_report(report, stale))
        decorated = terminal.replace("  high hot", "   high hot")
        self.assertFalse(reviewer.codebase_sections_match_report(report, decorated))
        empty_warning = terminal.replace(
            "  warning 1 import could not be followed · 1 named nothing in the repository\n",
            "",
        )
        self.assertFalse(
            reviewer.codebase_sections_match_report(report, empty_warning)
        )
        wrong_order = terminal.replace("\nAREAS\n", "\nLATER\n").replace(
            "\nWARNINGS\n", "\nAREAS\n"
        ).replace("\nLATER\n", "\nWARNINGS\n")
        self.assertFalse(reviewer.codebase_sections_match_report(report, wrong_order))
        wrong_first = terminal.replace(
            "high hot and complex · app/main.rb",
            "high hot and complex · db/schema.rb",
        )
        self.assertIsNone(reviewer.first_problem_review(report, wrong_first))
        fabricated_warning = terminal.replace(
            "1 named nothing in the repository",
            "1 matched more than one file",
        )
        self.assertFalse(
            reviewer.codebase_sections_match_report(report, fabricated_warning)
        )
        wrong_reason = terminal.replace("hot AND complex", "most complex")
        self.assertFalse(reviewer.codebase_head_matches_report(report, wrong_reason))

        malformed = copy.deepcopy(report)
        malformed["problems"][0]["pattern"] = "measured"
        malformed["problems"][0]["evidence"][0]["index"] = 99
        self.assertIsNone(reviewer.first_problem_review(malformed, terminal))

        malformed = copy.deepcopy(report)
        malformed["problems"][0]["anchor"] = {"kind": "files", "files": [99]}
        self.assertIsNone(reviewer.first_problem_review(malformed, terminal))

    def test_problem_rows_are_exact_and_support_real_wrapping(self):
        reviewer = load_workload_reviewer()
        report = problem_review_report()
        terminal = problem_review_terminal()
        additions = (
            "  watch does too much · invented.rb",
            "  arbitrary later card",
            "        arbitrary evidence",
        )
        for addition in additions:
            with self.subTest(addition=addition):
                changed = terminal.replace(
                    "\nWARNINGS\n", f"{addition}\n\nWARNINGS\n"
                )
                self.assertFalse(
                    reviewer.codebase_sections_match_report(report, changed)
                )

        long_path = f"src/{'a' * 90}.rs"
        wrapped = problem_review_report(path=long_path)
        wrapped_terminal = (
            "\nPROBLEMS\n"
            "  high hot and complex · src/\n"
            f"        {'a' * 90}.rs\n"
        )
        self.assertTrue(
            reviewer.displayed_problem_heads_match(wrapped, wrapped_terminal)
        )
        malformed = wrapped_terminal.replace("        ", "       ")
        self.assertFalse(reviewer.displayed_problem_heads_match(wrapped, malformed))

    def test_rust_review_rejects_weak_or_untrusted_visible_claims(self):
        reviewer = load_workload_reviewer()
        terminal = problem_review_terminal(path="src/lib.rs")

        weak_history = problem_review_report(path="src/lib.rs")
        weak_history["evolutionary_findings"] = [
            {"shared_commits": 2, "union_commits": 2}
        ]
        weak_history["problems"][0]["evidence"].append(
            {"kind": "evolutionary_findings", "index": 0}
        )
        weak_history["problems"][0]["claimed"].append(
            {"table": "evolutionary_findings", "index": 0}
        )
        self.assertFalse(
            reviewer.weak_history_and_graph_facts_are_absent(
                weak_history, terminal
            )
        )

        untrusted_graph = problem_review_report(path="src/lib.rs")
        untrusted_graph["dependency_edges"] = [
            {"relation": "uses", "role": "primary", "trust": "advisory"}
        ]
        untrusted_graph["architecture_findings"] = [{"witness_edges": [0]}]
        untrusted_graph["problems"][0]["evidence"].append(
            {"kind": "architecture_findings", "index": 0}
        )
        untrusted_graph["problems"][0]["claimed"].append(
            {"table": "architecture_findings", "index": 0}
        )
        self.assertFalse(
            reviewer.weak_history_and_graph_facts_are_absent(
                untrusted_graph, terminal
            )
        )

    def test_problem_review_checks_coverage_qualifier_bytes(self):
        reviewer = load_workload_reviewer()
        report = problem_review_report()
        report["verdict"]["qualifier"] = {
            "sentence": "Not all source was checked.",
            "detail": "9 of 10 source files were analyzed.",
        }
        terminal = problem_review_terminal().replace(
            "  Worn in the usual places.\n",
            "  Worn in the usual places.\n"
            "  Not all source was checked.\n"
            "  9 of 10 source files were analyzed.\n",
        )
        self.assertTrue(reviewer.codebase_head_matches_report(report, terminal))
        self.assertFalse(
            reviewer.codebase_head_matches_report(
                report, terminal.replace("9 of 10", "8 of 10")
            )
        )
        self.assertFalse(
            reviewer.codebase_head_matches_report(
                report,
                terminal.replace(
                    "  9 of 10 source files were analyzed.\n",
                    "  9 of 10 source files were analyzed.\n  unexpected verdict fact\n",
                ),
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

if __name__ == "__main__":
    unittest.main()
