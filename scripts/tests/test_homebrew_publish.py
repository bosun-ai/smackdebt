"""Exercise formula publication and retries against a local Git remote."""
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class HomebrewPublishTests(unittest.TestCase):
    def test_publishes_only_the_formula_and_retry_does_not_create_a_commit(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            remote, tap = root / "remote", root / "tap"
            def git(*args, cwd=root):
                return subprocess.check_output(["git", *args], cwd=cwd, stderr=subprocess.DEVNULL, text=True).strip()
            git("init", "--bare", str(remote))
            git("clone", str(remote), str(tap))
            git("config", "user.name", "Test", cwd=tap)
            git("config", "user.email", "test@example.invalid", cwd=tap)
            (tap / "README.md").write_text("tap")
            git("add", ".", cwd=tap)
            git("commit", "-m", "initial", cwd=tap)
            git("push", "origin", "HEAD", cwd=tap)
            formula = root / "smackdebt.rb"
            formula.write_text('class Smackdebt < Formula\n  version "0.2.0"\nend\n')
            command = ["python3", str(ROOT / "scripts/publish-homebrew.py"), str(formula), str(tap), "v0.2.0"]
            subprocess.run(command, check=True, capture_output=True)
            first = git("rev-parse", "HEAD", cwd=tap)
            self.assertEqual(git("show", "--format=", "--name-only", "HEAD", cwd=tap), "Formula/smackdebt.rb")
            subprocess.run(command, check=True, capture_output=True)
            self.assertEqual(first, git("rev-parse", "HEAD", cwd=tap))
            formula.write_text('class Smackdebt < Formula\n  version "0.1.0"\nend\n')
            result = subprocess.run(command[:-1] + ["v0.1.0"], capture_output=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(first, git("rev-parse", "HEAD", cwd=tap))
            self.assertEqual(git("rev-parse", "HEAD", cwd=remote), first)
            result = subprocess.run(command[:-1] + ["v0.3.0-beta.1"], capture_output=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(first, git("rev-parse", "HEAD", cwd=tap))


if __name__ == "__main__":
    unittest.main()
