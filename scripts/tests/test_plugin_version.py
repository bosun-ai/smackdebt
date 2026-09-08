"""Plugin changes must invalidate both clients' cached versions."""

import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "check-plugin-version.py"
SPEC = importlib.util.spec_from_file_location("plugin_version", SCRIPT)
GUARD = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GUARD)


class PluginVersionTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="smackdebt-plugin-version-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.git("init", "-q")
        self.git("config", "user.name", "Plugin Test")
        self.git("config", "user.email", "plugin@example.invalid")
        (self.root / "README.md").write_text("test\n")
        self.commit()
        self.empty = self.git("rev-parse", "HEAD").strip()
        self.write_versions("0.1.0", "0.1.0")
        self.skill = self.root / GUARD.PLUGIN / "skills/smackdebt/SKILL.md"
        self.skill.parent.mkdir(parents=True)
        self.skill.write_text("original skill\n")
        self.commit()
        self.base = self.git("rev-parse", "HEAD").strip()

    def git(self, *arguments):
        return subprocess.check_output(["git", *arguments], cwd=self.root, text=True)

    def commit(self):
        self.git("add", ".")
        self.git("commit", "-qm", "test: plugin fixture")

    def write_versions(self, codex, claude):
        for manifest, version in zip(GUARD.MANIFESTS, (codex, claude)):
            path = self.root / GUARD.PLUGIN / manifest
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(json.dumps({"name": "smackdebt", "version": version}))

    def test_first_plugin_addition_needs_no_previous_version(self):
        GUARD.check(self.root, self.empty)

    def test_cli_only_changes_do_not_require_plugin_release(self):
        (self.root / "README.md").write_text("changed CLI\n")
        GUARD.check(self.root, self.base)

    def test_skill_changes_require_both_clients_to_update(self):
        self.skill.write_text("updated skill\n")
        for versions in (("0.1.0", "0.1.0"), ("0.1.1", "0.1.0"), ("0.1.0", "0.1.1")):
            with self.subTest(versions=versions):
                self.write_versions(*versions)
                with self.assertRaisesRegex(ValueError, "increase"):
                    GUARD.check(self.root, self.base)
        self.write_versions("0.1.1", "0.1.1")
        GUARD.check(self.root, self.base)

    def test_removed_plugin_content_also_requires_new_versions(self):
        self.skill.unlink()
        with self.assertRaisesRegex(ValueError, "increase"):
            GUARD.check(self.root, self.base)

    def test_missing_comparison_commit_fails(self):
        with self.assertRaisesRegex(ValueError, "comparison commit"):
            GUARD.check(self.root, "does-not-exist")

    def test_semver_precedence_includes_prereleases_and_ignores_build_metadata(self):
        versions = ["0.1.0-alpha", "0.1.0-alpha.2", "0.1.0-alpha.10", "0.1.0-beta", "0.1.0", "0.1.1", "0.2.0", "1.0.0"]
        ordered = [GUARD.version_order(version) for version in versions]
        self.assertEqual(ordered, sorted(ordered))
        self.assertEqual(GUARD.version_order("1.0.0+build.2"), GUARD.version_order("1.0.0"))
        for invalid in ("v1.0.0", "01.0.0", "1.0.0-01", "1.0.0-a..b", "1.0.0+build..2"):
            with self.subTest(version=invalid), self.assertRaises(ValueError):
                GUARD.version_order(invalid)


if __name__ == "__main__":
    unittest.main()
