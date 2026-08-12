import importlib.util
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).parents[2]


def load_script(name):
    path = ROOT / "scripts" / name
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class DependencyDirectionTests(unittest.TestCase):
    def test_allowed_edges_are_accepted(self):
        module = load_script("check-dependency-direction.py")
        metadata = {
            "packages": [
                {"name": "smackdebt-analysis", "dependencies": []},
                {
                    "name": "smackdebt-project",
                    "dependencies": [
                        {"name": "smackdebt-analysis", "kind": None},
                        {"name": "smackdebt-git", "kind": "normal"},
                    ],
                },
            ]
        }
        self.assertEqual(module.violations(metadata), [])

    def test_error_names_the_offending_edge(self):
        module = load_script("check-dependency-direction.py")
        metadata = {
            "packages": [
                {
                    "name": "smackdebt-analysis",
                    "dependencies": [{"name": "smackdebt-git", "kind": None}],
                },
                {"name": "smackdebt-git", "dependencies": []},
            ]
        }
        self.assertEqual(
            module.violations(metadata),
            ["smackdebt-analysis -> smackdebt-git is not an allowed production edge"],
        )


class ApiSnapshotTests(unittest.TestCase):
    def test_private_and_development_declarations_are_not_snapshot_exports(self):
        module = load_script("check-api-snapshots.py")
        source = """
        pub struct Visible;
        pub(crate) struct Internal;
        pub fn visible() {}
        """
        self.assertEqual(module.public_surface(source), ["pub fn visible()", "pub struct Visible"])

    def test_changed_snapshot_names_the_library(self):
        module = load_script("check-api-snapshots.py")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "crates" / "analysis" / "src" / "lib.rs"
            source.parent.mkdir(parents=True)
            source.write_text("pub struct Changed;\n", encoding="utf-8")
            snapshot = root / "api-snapshots" / "smackdebt-analysis.txt"
            snapshot.parent.mkdir(parents=True)
            snapshot.write_text("pub struct Original\n", encoding="utf-8")
            original = module.LIBRARIES
            module.LIBRARIES = {"smackdebt-analysis": Path("crates/analysis/src/lib.rs")}
            try:
                problems = module.check(root, update=False)
            finally:
                module.LIBRARIES = original
        self.assertEqual(len(problems), 1)
        self.assertIn("smackdebt-analysis", problems[0])
        self.assertIn("public API changed", problems[0])


if __name__ == "__main__":
    unittest.main()
