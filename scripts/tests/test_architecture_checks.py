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
            original_tool_problem = module.tool_problem
            original_reachable_api = module.reachable_api
            module.LIBRARIES = {"smackdebt-analysis": Path("crates/analysis/src/lib.rs")}
            module.tool_problem = lambda _root: None
            module.reachable_api = lambda _root, _package: ["pub struct Changed"]
            try:
                problems = module.check(root, update=False)
            finally:
                module.LIBRARIES = original
                module.tool_problem = original_tool_problem
                module.reachable_api = original_reachable_api
        self.assertEqual(len(problems), 1)
        self.assertIn("smackdebt-analysis", problems[0])
        self.assertIn("public API changed", problems[0])


class EntryModuleTests(unittest.TestCase):
    def test_imports_modules_and_explicit_reexports_are_allowed(self):
        module = load_script("check-entry-modules.py")
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "lib.rs"
            path.write_text(
                "#![forbid(unsafe_code)]\nmod report;\npub use report::{\n    Report,\n};\n",
                encoding="utf-8",
            )
            self.assertEqual(module.violations(path), [])

    def test_behavior_and_wildcard_reexports_are_rejected(self):
        module = load_script("check-entry-modules.py")
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "lib.rs"
            path.write_text(
                "mod report;\npub use report::*;\npub fn build() {}\n",
                encoding="utf-8",
            )
            problems = module.violations(path)
            self.assertEqual(len(problems), 2)
            self.assertIn("wildcard reexport", problems[0])
            self.assertIn("behavior is not allowed", problems[1])


if __name__ == "__main__":
    unittest.main()
