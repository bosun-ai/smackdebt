"""Prove that release-plz sees changes across the application workspace."""

import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib
import unittest


ROOT = Path(__file__).resolve().parents[2]
RELEASE_PLZ = os.environ.get("SMACKDEBT_RELEASE_PLZ")


def run(repository, *arguments):
    result = subprocess.run(arguments, cwd=repository, capture_output=True, text=True)
    if result.returncode:
        raise AssertionError(f"{arguments[0]} failed:\n{result.stdout}\n{result.stderr}")
    return result.stdout


def commit(repository, message):
    run(repository, "git", "add", ".")
    run(repository, "git", "commit", "-qm", message)


def create_release_workspace(repository):
    workspace = tomllib.loads((ROOT / "Cargo.toml").read_text())
    manifest = (ROOT / "Cargo.toml").read_text()
    version = workspace["workspace"]["package"]["version"]
    (repository / "Cargo.toml").write_text(manifest.replace(f'version = "{version}"', 'version = "0.1.0"', 1))
    shutil.copyfile(ROOT / "release-plz.toml", repository / "release-plz.toml")
    (repository / "README.md").write_text("# Release test\n")
    (repository / ".gitignore").write_text("**/target/\n")
    for member in workspace["workspace"]["members"]:
        write_test_package(repository, member)
    run(repository, "git", "init", "-q", "-b", "master")
    run(repository, "git", "config", "user.name", "Release Test")
    run(repository, "git", "config", "user.email", "release@example.invalid")
    run(repository, "git", "remote", "add", "origin", "https://github.com/bosun-ai/smackdebt")
    run(repository, "cargo", "generate-lockfile")
    commit(repository, "feat: introduce smackdebt")


def package_field(key, value):
    if isinstance(value, dict):
        return f"{key}.workspace = true"
    if isinstance(value, bool):
        return f"{key} = {str(value).lower()}"
    return f'{key} = "{value}"'


def write_test_package(repository, member):
    original = tomllib.loads((ROOT / member / "Cargo.toml").read_text())
    package = original["package"]
    directory = repository / member
    (directory / "src").mkdir(parents=True)
    lines = ["[package]", *(package_field(key, value) for key, value in package.items()), "\n[dependencies]"]
    for name, dependency in original.get("dependencies", {}).items():
        if "path" in dependency:
            path = dependency["path"]
            lines.append(f'{name} = {{ path = "{path}", version = "0.1.0" }}')
    (directory / "Cargo.toml").write_text("\n".join(lines) + "\n")
    binary = package["name"] == "smackdebt"
    source = "fn main() {}\n" if binary else "pub fn value() -> usize { 1 }\n"
    (directory / "src" / ("main.rs" if binary else "lib.rs")).write_text(source)


class ReleaseConfigurationTests(unittest.TestCase):
    def test_the_release_flow_never_publishes_crates_to_a_registry(self):
        config = tomllib.loads((ROOT / "release-plz.toml").read_text())
        workspace = config["workspace"]
        for package in [workspace, *config["package"]]:
            self.assertFalse(package.get("publish", workspace["publish"]))
            self.assertTrue(package.get("git_only", workspace["git_only"]))

    @unittest.skipUnless(RELEASE_PLZ, "set SMACKDEBT_RELEASE_PLZ to the pinned release-plz binary")
    def test_release_pr_includes_a_library_only_fix_after_the_first_release(self):
        with tempfile.TemporaryDirectory(prefix="smackdebt-version-test-") as temporary:
            repository = Path(temporary)
            create_release_workspace(repository)
            run(repository, RELEASE_PLZ, "update")
            changelog = repository / "CHANGELOG.md"
            self.assertIn("0.1.0", changelog.read_text())
            self.assertIn("/releases/download/v0.1.0/install.sh | sh", changelog.read_text())
            self.assertEqual(list(repository.glob("crates/*/CHANGELOG.md")), [])
            commit(repository, "chore: release v0.1.0")
            run(repository, "git", "tag", "v0.1.0")
            source = repository / "crates/analysis/src/lib.rs"
            source.write_text("pub fn value() -> usize { 2 }\n")
            commit(repository, "fix: recognize changed analysis")
            run(repository, RELEASE_PLZ, "update")
            workspace = tomllib.loads((repository / "Cargo.toml").read_text())
            self.assertEqual(workspace["workspace"]["package"]["version"], "0.1.1")
            self.assertIn("recognize changed analysis", changelog.read_text())
            self.assertIn("/releases/download/v0.1.1/install.sh | sh", changelog.read_text())
            self.assertEqual(list(repository.glob("crates/*/CHANGELOG.md")), [])


if __name__ == "__main__":
    unittest.main()
