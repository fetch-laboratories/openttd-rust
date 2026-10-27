#!/usr/bin/env python3
"""Exercise the deep-check action: commit report, failures, priority, skip and refusal."""

import contextlib
import io
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from collections.abc import Sequence
from pathlib import Path

import deep

# Stands in for `migration.py simulate --soak`: records its own priority, writes a
# simulate-style report with one failing scenario and prints the `Report:` line.
FAKE_SIMULATE = """
import json, os, subprocess, sys
out = sys.argv[1]
ionice = subprocess.run(["ionice", "-p", str(os.getpid())], capture_output=True, text=True)
rows = [
    dict(scenario="a", seconds=2.5, passed=True, problems=[], differences=[]),
    dict(scenario="b", seconds=4.0, passed=False, problems=["end moments differ"],
         differences=[dict(chunk="PLYR", element=i) for i in range(5)]),
]
report = os.path.join(out, "report.json")
json.dump(dict(passed=False, results=rows), open(report, "w"))
json.dump(dict(nice=os.nice(0), ionice=ionice.stdout, args=sys.argv[2:]),
          open(os.path.join(out, "priority.json"), "w"))
print("Report: " + report)
sys.exit(1)
"""


class DeepTests(unittest.TestCase):
    def setUp(self) -> None:
        self.scratch = tempfile.TemporaryDirectory()
        self.addCleanup(self.scratch.cleanup)
        self.root = Path(self.scratch.name) / "repo"
        self.root.mkdir()
        for command in (
            ["init", "--quiet"],
            ["-c", "user.name=t", "-c", "user.email=t@t", "commit", "--allow-empty"]
            + ["--quiet", "-m", "Add: nothing"],
        ):
            subprocess.run(["git", *command], cwd=self.root, check=True)
        self.head = subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=self.root, text=True
        ).strip()
        self.fake = Path(self.scratch.name) / "fake_simulate.py"
        self.fake.write_text(FAKE_SIMULATE)
        self.drop = Path(self.scratch.name) / "drop"
        self.drop.mkdir()
        self.members = (deep.Member("fake", self.command),)
        self.local = self.root / ".local"
        self.common = Path(self.scratch.name) / "common"

    def command(self, root: Path, jobs: int, selectors: Sequence[str]) -> list[str]:
        return [sys.executable, str(self.fake), str(self.drop), str(jobs), *selectors]

    def run_deep(self, selectors: Sequence[str] = ()) -> tuple[int, str]:
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            code = deep.run_deep(
                self.root, self.local, self.common, 3, selectors, self.members
            )
        return code, output.getvalue()

    def only_report(self) -> dict:
        (path,) = (self.local / "deep").glob("*/report.json")
        self.assertTrue(path.parent.name.endswith(self.head[:12]))
        return json.loads(path.read_text())

    def test_report_names_commit_and_failures_at_idle_priority(self) -> None:
        code, output = self.run_deep(["sel"])
        self.assertEqual(code, 1)
        self.assertIn("FAIL fake: 2 scenarios, 1 failed, exit 1", output)
        report = self.only_report()
        self.assertEqual(report["commit"], self.head)
        self.assertEqual((report["partial"], report["passed"]), (True, False))
        (result,) = report["results"]
        self.assertEqual(result["scenarios"], 2)
        self.assertEqual(result["scenario_seconds"], 6.5)
        (failure,) = result["failures"]
        self.assertEqual(failure["scenario"], "b")
        self.assertEqual(failure["problems"], ["end moments differ"])
        self.assertEqual(len(failure["differences"]), deep.SHOWN_DIFFERENCES)
        priority = json.loads((self.drop / "priority.json").read_text())
        self.assertEqual(priority["args"], ["3", "sel"])
        self.assertEqual(priority["nice"], 19)
        if shutil.which("ionice"):
            self.assertIn("idle", priority["ionice"])

    def test_active_run_is_skipped_without_a_report(self) -> None:
        with deep.single_run(self.common / "deep.lock") as held:
            self.assertTrue(held)
            code, output = self.run_deep()
        self.assertEqual(code, 0)
        self.assertIn("skipped", output)
        self.assertFalse((self.local / "deep").exists())
        self.assertFalse((self.drop / "priority.json").exists())

    def test_lock_is_released_after_a_run(self) -> None:
        self.run_deep()
        with deep.single_run(self.common / "deep.lock") as free:
            self.assertTrue(free)

    def test_dirty_checkout_is_refused(self) -> None:
        (self.root / "stray.txt").write_text("edit")
        with self.assertRaises(SystemExit):
            self.run_deep()
        self.assertFalse((self.drop / "priority.json").exists())


if __name__ == "__main__":
    unittest.main()
