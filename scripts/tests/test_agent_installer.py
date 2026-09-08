"""Exercise the shipped installer through stdin without changing a real user profile."""

import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("agent_installer", ROOT / "scripts/build-agent-installer.py")
builder = importlib.util.module_from_spec(spec)
spec.loader.exec_module(builder)


class AgentInstallerTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="smackdebt-install-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.profile = self.root / "user with spaces"
        self.profile.mkdir()
        self.bin = self.root / "bin"
        self.bin.mkdir()
        self.environment = dict(os.environ)
        self.environment.update(HOME=str(self.profile), PATH=f"{self.bin}:/usr/bin:/bin", TMPDIR=str(self.root))
        for name in ("CLAUDE_CONFIG_DIR", "CARGO_HOME", "SMACKDEBT_INSTALL_DIR", "CARGO_DIST_FORCE_INSTALL_DIR"):
            self.environment.pop(name, None)
        self.destinations = [self.profile / folder / "skills/smackdebt" for folder in (".agents", ".claude")]
        artifact = os.environ.get("SMACKDEBT_TEST_INSTALLER")
        self.installer = Path(artifact).read_text() if artifact else builder.render_installer()
        self.executable("curl", '#!/bin/sh\nprintf "%s\\n" "$*" >> "$HOME/downloads"\nwhile [ "$1" != -o ]; do shift; done\ncp "$HOME/cli-installer" "$2"\n')
        (self.profile / "cli-installer").write_text('#!/bin/sh\ntouch "$HOME/cli-installed"\n')

    def executable(self, name, text):
        path = self.bin / name
        path.write_text(text)
        path.chmod(0o755)

    def run_installer(self, *arguments):
        return subprocess.run(
            ["/bin/sh", "-s", "--", *arguments], input=self.installer,
            env=self.environment, cwd=self.root, text=True, capture_output=True, timeout=15,
        )

    def assert_installed_skill(self):
        expected = builder.SKILL.read_bytes()
        for destination in self.destinations:
            self.assertEqual((destination / "SKILL.md").read_bytes(), expected)
            receipts = (destination / ".smackdebt.sha256").read_text().splitlines()
            self.assertEqual(receipts[0], hashlib.sha256(expected).hexdigest())
            self.assertLessEqual(len(receipts), 2)

    def test_default_installs_cli_and_the_shared_and_claude_skills(self):
        result = self.run_installer()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.profile / "cli-installed").exists())
        self.assert_installed_skill()
        self.assertIn("/releases/download/v", (self.profile / "downloads").read_text())
        self.assertNotIn("/latest/", (self.profile / "downloads").read_text())
        self.assertEqual(list(self.root.glob("smackdebt-install.*")), [])

    def test_cli_only_does_not_create_skill_directories(self):
        result = self.run_installer("--no-skill")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.profile / "cli-installed").exists())
        self.assertFalse(any(path.exists() for path in self.destinations))

    def test_skill_only_does_not_download_or_install_a_binary(self):
        self.executable("curl", "#!/bin/sh\nexit 99\n")
        result = self.run_installer("--no-cli")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assert_installed_skill()
        self.assertFalse((self.profile / "cli-installed").exists())

    def test_reinstall_updates_managed_skills_and_keeps_other_files(self):
        for destination in self.destinations:
            destination.mkdir(parents=True)
            (destination / "SKILL.md").write_text("older release\n")
            (destination / ".smackdebt.sha256").write_text(hashlib.sha256(b"older release\n").hexdigest())
            (destination / "notes.md").write_text("user notes")
        for _ in range(2):
            result = self.run_installer("--no-cli")
            self.assertEqual(result.returncode, 0, result.stderr)
        self.assert_installed_skill()
        self.assertTrue(all((path / "notes.md").read_text() == "user notes" for path in self.destinations))

    def test_local_edits_stop_installation_before_the_cli_changes(self):
        self.assertEqual(self.run_installer("--no-cli").returncode, 0)
        edited = self.destinations[1] / "SKILL.md"
        edited.write_text("user instructions")
        result = self.run_installer()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("local edits", result.stderr)
        self.assertEqual(edited.read_text(), "user instructions")
        self.assertFalse((self.profile / "downloads").exists())

    def test_an_existing_unmanaged_skill_is_preserved(self):
        existing = self.destinations[0]
        existing.mkdir(parents=True)
        (existing / "SKILL.md").write_text("user instructions")
        result = self.run_installer()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("not managed", result.stderr)
        self.assertEqual((existing / "SKILL.md").read_text(), "user instructions")

    def test_linked_skills_are_not_overwritten(self):
        external = self.root / "external"
        external.mkdir()
        self.destinations[0].parent.mkdir(parents=True)
        self.destinations[0].symlink_to(external)
        result = self.run_installer()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(list(external.iterdir()), [])

    def test_invalid_options_do_not_download_or_write_skills(self):
        for arguments in [("--no-cli", "--no-skill"), ("--typo",)]:
            with self.subTest(arguments=arguments):
                result = self.run_installer(*arguments)
                self.assertEqual(result.returncode, 2)
        self.assertFalse((self.profile / "downloads").exists())
        self.assertFalse(any(path.exists() for path in self.destinations))

    def test_failed_download_does_not_install_the_skill_or_claim_success(self):
        self.executable("curl", "#!/bin/sh\nexit 22\n")
        result = self.run_installer()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("completed: nothing", result.stderr)
        self.assertFalse(any((path / "SKILL.md").exists() for path in self.destinations))
        self.assertNotIn("installed", result.stdout)

    def test_failed_binary_install_keeps_the_error_visible(self):
        (self.profile / "cli-installer").write_text("#!/bin/sh\necho 'unsupported platform' >&2\nexit 1\n")
        result = self.run_installer()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unsupported platform", result.stderr)
        self.assertFalse(any((path / "SKILL.md").exists() for path in self.destinations))

    def test_custom_claude_configuration_directory_is_respected(self):
        custom = self.profile / "custom claude"
        self.environment["CLAUDE_CONFIG_DIR"] = str(custom)
        result = self.run_installer("--no-cli")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((custom / "skills/smackdebt/SKILL.md").is_file())
        self.assertFalse(self.destinations[1].exists())

    def test_a_later_write_failure_reports_the_cli_as_completed(self):
        self.executable("mv", "#!/bin/sh\necho 'disk full' >&2\nexit 1\n")
        result = self.run_installer()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("disk full", result.stderr)
        self.assertIn("completed: CLI", result.stderr)
        self.assertNotIn("installed", result.stdout)

    def assert_failed_replacement_can_be_retried(self, existing, failed_file):
        if existing:
            for destination in self.destinations:
                destination.mkdir(parents=True)
                (destination / "SKILL.md").write_text("older release\n")
                (destination / ".smackdebt.sha256").write_text(hashlib.sha256(b"older release\n").hexdigest())
        self.executable("mv", f'''#!/bin/sh
for destination do :; done
case "$destination" in */{failed_file}) echo 'disk full' >&2; exit 1 ;; esac
exec /bin/mv "$@"
''')
        result = self.run_installer("--no-cli")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("disk full", result.stderr)
        for destination in self.destinations:
            if existing:
                self.assertEqual((destination / "SKILL.md").read_text(), "older release\n")
            else:
                self.assertFalse((destination / "SKILL.md").exists())
        (self.bin / "mv").unlink()
        result = self.run_installer("--no-cli")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assert_installed_skill()

    def test_first_install_can_retry_after_receipt_write_fails(self):
        self.assert_failed_replacement_can_be_retried(False, ".smackdebt.sha256")

    def test_upgrade_can_retry_after_receipt_write_fails(self):
        self.assert_failed_replacement_can_be_retried(True, ".smackdebt.sha256")

    def test_first_install_can_retry_after_skill_write_fails(self):
        self.assert_failed_replacement_can_be_retried(False, "SKILL.md")

    def test_upgrade_can_retry_after_skill_write_fails(self):
        self.assert_failed_replacement_can_be_retried(True, "SKILL.md")

    def test_missing_hash_tools_fail_before_changing_the_cli(self):
        self.environment["PATH"] = str(self.bin)
        result = self.run_installer()
        self.assertEqual(result.returncode, 1)
        self.assertIn("sha256sum or shasum", result.stderr)
        self.assertFalse((self.profile / "downloads").exists())

    def test_missing_curl_fails_before_writing_skills(self):
        (self.bin / "curl").unlink()
        self.environment["PATH"] = str(self.bin)
        result = self.run_installer()
        self.assertEqual(result.returncode, 1)
        self.assertIn("curl is required", result.stderr)
        self.assertFalse(any(path.exists() for path in self.destinations))

    def test_a_file_in_place_of_a_skill_directory_is_preserved(self):
        self.destinations[0].parent.mkdir(parents=True)
        self.destinations[0].write_text("user data")
        result = self.run_installer()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.destinations[0].read_text(), "user data")
        self.assertFalse((self.profile / "downloads").exists())


class AgentPackageTests(unittest.TestCase):
    def test_both_marketplaces_resolve_the_shared_plugin_and_skill(self):
        codex = json.loads((ROOT / ".agents/plugins/marketplace.json").read_text())
        claude = json.loads((ROOT / ".claude-plugin/marketplace.json").read_text())
        roots = [ROOT / codex["plugins"][0]["source"]["path"], ROOT / claude["plugins"][0]["source"]]
        self.assertEqual(roots[0].resolve(), roots[1].resolve())
        for root in roots:
            self.assertEqual((root / "skills/smackdebt/SKILL.md").read_bytes(), builder.SKILL.read_bytes())
        manifests = [json.loads((roots[0] / kind / "plugin.json").read_text()) for kind in (".codex-plugin", ".claude-plugin")]
        self.assertEqual(manifests[0]["name"], manifests[1]["name"])
        self.assertEqual(manifests[0]["version"], manifests[1]["version"])


if __name__ == "__main__":
    unittest.main()
