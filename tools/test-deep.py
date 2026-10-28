#!/usr/bin/env python3
"""Exercise the deep-check action: commit report, failures, priority, skip and refusal."""

import contextlib
import io
import json
import shutil
import subprocess
import sys
import tempfile
import threading
import unittest
from collections.abc import Sequence
from pathlib import Path
from types import SimpleNamespace

import deep
from simulation import speed

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
if "advance-head" in sys.argv[3:]:
    subprocess.run(["git", "-c", "user.name=t", "-c", "user.email=t@t", "commit",
                    "--allow-empty", "--quiet", "-m", "Add: next commit"], check=True)
if "dirty" in sys.argv[3:]:
    open("stray.txt", "w").write("edit")
if any(s in sys.argv[3:] for s in ("pass", "advance-head", "dirty", "sanitized")):
    rows[1].update(passed=True, problems=[], differences=[])
passed = all(row["passed"] for row in rows)
report = os.path.join(out, "report.json")
head = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
status = subprocess.check_output(["git", "status", "--short"], text=True).strip()
sanitizers = dict(original={"UBSan src/a.cpp in f: overflow": ["a"]}, candidate_only={})
json.dump(dict(passed=passed, results=rows, candidate_commit=head, candidate_status=status,
               sanitizers=sanitizers if "sanitized" in sys.argv[3:] else None),
          open(report, "w"))
json.dump(dict(nice=os.nice(0), ionice=ionice.stdout, args=sys.argv[2:]),
          open(os.path.join(out, "priority.json"), "w"))
print("Report: " + report)
sys.exit(0 if passed else 1)
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
        (self.root / ".git/info/exclude").write_text(".local/\n")
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
        with deep.single_run(self.common / deep.LOCK_NAME) as held:
            self.assertTrue(held)
            code, output = self.run_deep()
        self.assertEqual(code, 0)
        self.assertIn("skipped", output)
        self.assertFalse((self.local / "deep").exists())
        self.assertFalse((self.drop / "priority.json").exists())

    def test_deep_skips_while_a_speed_collection_holds_the_lock(self) -> None:
        driver = SimpleNamespace(COMMON_LOCAL=self.common)
        with speed.collection_lock(driver):
            code, output = self.run_deep()
        self.assertEqual(code, 0)
        self.assertIn("skipped", output)
        self.assertFalse((self.local / "deep").exists())
        self.assertFalse((self.drop / "priority.json").exists())

    def test_speed_collection_waits_while_deep_runs(self) -> None:
        driver = SimpleNamespace(COMMON_LOCAL=self.common)
        started, entered = threading.Event(), threading.Event()

        def collect() -> None:
            started.set()
            with speed.collection_lock(driver):
                entered.set()

        waiter = threading.Thread(target=collect)
        with deep.single_run(self.common / deep.LOCK_NAME) as held:
            self.assertTrue(held)
            waiter.start()
            self.assertTrue(started.wait(5))
            self.assertFalse(entered.wait(0.5))
        self.assertTrue(entered.wait(5))
        waiter.join()

    def test_clean_matching_member_passes(self) -> None:
        code, _ = self.run_deep(["pass"])
        self.assertEqual(code, 0)
        self.assertTrue(self.only_report()["passed"])

    def test_original_sanitizer_reports_are_recorded_without_failing(self) -> None:
        code, output = self.run_deep(["sanitized"])
        self.assertEqual(code, 0)
        (result,) = self.only_report()["results"]
        self.assertEqual(
            result["original_reports"], {"UBSan src/a.cpp in f: overflow": ["a"]}
        )
        self.assertIn("1 sanitizer reports also in the original", output)

    def test_member_from_a_later_commit_is_rejected(self) -> None:
        code, _ = self.run_deep(["advance-head"])
        report = self.only_report()
        self.assertEqual(code, 1)
        self.assertFalse(report["passed"])
        self.assertEqual(report["commit"], self.head)
        self.assertIn("commit", report["results"][0]["problems"][0])

    def test_member_from_a_dirty_checkout_is_rejected(self) -> None:
        code, _ = self.run_deep(["dirty"])
        report = self.only_report()
        self.assertEqual(code, 1)
        self.assertFalse(report["passed"])
        self.assertIn("dirty", report["results"][0]["problems"][0])

    def test_lock_is_released_after_a_run(self) -> None:
        self.run_deep()
        with deep.single_run(self.common / deep.LOCK_NAME) as free:
            self.assertTrue(free)

    def test_dirty_checkout_is_refused(self) -> None:
        (self.root / "stray.txt").write_text("edit")
        with self.assertRaises(SystemExit):
            self.run_deep()
        self.assertFalse((self.drop / "priority.json").exists())


if __name__ == "__main__":
    unittest.main()
