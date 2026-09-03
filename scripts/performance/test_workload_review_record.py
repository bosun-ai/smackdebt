import importlib.util
import unittest
from pathlib import Path


REVIEW_CHECKER = Path(__file__).with_name("check-workload-reviews.py")


def load_review_checker():
    spec = importlib.util.spec_from_file_location(
        "performance_check_reviews", REVIEW_CHECKER
    )
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


class WorkloadReviewRecordTests(unittest.TestCase):
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
        self.assertIn(
            "workload reviews and public profiles must share one revision", problems
        )

    def test_aggregate_review_checker_requires_all_nine_profiles(self):
        checker = load_review_checker()
        problems = checker.validate(
            valid_review_record(), valid_baselines()[:-1], False
        )
        self.assertIn("exactly nine public profiles are required", problems)


if __name__ == "__main__":
    unittest.main()
