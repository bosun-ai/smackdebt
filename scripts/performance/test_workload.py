import json
import subprocess
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("workload.py")
RUNNER = Path(__file__).with_name("run.sh")


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

    def test_runner_checks_before_each_measured_command(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "workload"
            subprocess.run(["sh", str(RUNNER), "--profile", "one-file", "--output", str(root), "--repeat", "2", "--", "python3", "-c", "import pathlib,sys; assert pathlib.Path(sys.argv[1]).is_dir()"], check=True, capture_output=True, text=True)
            records = (root / "runs.jsonl").read_text().splitlines()
            self.assertEqual(len(records), 2)
            self.assertTrue(all(json.loads(record)["wall_time_ns"] > 0 for record in records))
            self.assertTrue(all("parser_time_ns" in json.loads(record) for record in records))


if __name__ == "__main__":
    unittest.main()
