"""Exercise the binary installer through stdin in an isolated user profile."""
import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("installer", ROOT / "scripts/build-agent-installer.py")
builder = importlib.util.module_from_spec(spec)
spec.loader.exec_module(builder)

class InstallerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.home = Path(self.temp.name)
        tools = self.home / "tools"
        tools.mkdir()
        curl = tools / "curl"
        curl.write_text('#!/bin/sh\nwhile [ "$1" != -o ]; do shift; done\ncp "$HOME/download" "$2"\n')
        curl.chmod(0o755)
        self.environment = dict(os.environ, HOME=str(self.home), PATH=f"{tools}:{os.environ['PATH']}", TMPDIR=str(self.home))
        self.download = self.home / "download"
        self.download.write_text('#!/bin/sh\nprintf binary > "$HOME/smackdebt"\n')
        artifact = os.environ.get("SMACKDEBT_TEST_INSTALLER")
        self.script = Path(artifact).read_text() if artifact else builder.render_installer()

    def run_installer(self, *args):
        return subprocess.run(["sh", "-s", "--", *args], input=self.script, text=True, capture_output=True, env=self.environment)

    def test_installs_only_binary_and_explains_agent_setup(self):
        result = self.run_installer()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.home / "smackdebt").read_text(), "binary")
        self.assertFalse((self.home / ".agents").exists())
        self.assertFalse((self.home / ".claude").exists())
        self.assertFalse((self.home / ".config").exists())
        self.assertIn("smackdebt init", result.stdout)
        self.assertEqual(list(self.home.glob("smackdebt-install.*")), [])

    def test_failed_binary_install_propagates_failure(self):
        self.download.write_text("#!/bin/sh\necho broken >&2\nexit 7\n")
        result = self.run_installer()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("broken", result.stderr)
        self.assertNotIn("smackdebt init", result.stdout)

    def test_old_skill_options_fail_before_any_write(self):
        for option in ["--no-cli", "--all", "--uninstall", "--typo"]:
            result = self.run_installer(option)
            self.assertEqual(result.returncode, 2, result.stderr)
            self.assertFalse((self.home / "smackdebt").exists())

    def test_help_needs_no_download(self):
        self.download.unlink()
        self.assertEqual(self.run_installer("--help").returncode, 0)

    def test_existing_skills_are_never_modified(self):
        skill = self.home / ".agents/skills/smackdebt/SKILL.md"
        skill.parent.mkdir(parents=True)
        skill.write_text("user content")
        self.assertEqual(self.run_installer().returncode, 0)
        self.assertEqual(skill.read_text(), "user content")

if __name__ == "__main__":
    unittest.main()
