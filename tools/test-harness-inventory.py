#!/usr/bin/env python3
"""Exercise each harness inventory guard on the tree and on constructed losses."""

import copy
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import harness_inventory as hi

ROOT = Path(__file__).resolve().parents[1]
RECORDED = hi.load((ROOT / hi.INVENTORY).read_text())
TREE, UNREAD = hi.tree_inventory()


def open_issues(number: int) -> str:
    return "open" if number == 1 else "closed"


class InventoryTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tree, self.recorded = copy.deepcopy(TREE), copy.deepcopy(RECORDED)
        self.tree["comparisons"] = copy.deepcopy(RECORDED["comparisons"])

    def errors(self, base: hi.Inventory | None = None) -> list[str]:
        return hi.errors(self.tree, self.recorded, base or RECORDED, open_issues)

    def test_unchanged_tree_matches_its_inventory(self) -> None:
        self.assertEqual(self.errors(), [])
        self.assertEqual(UNREAD, [])
        self.assertEqual(hi.removals(self.recorded, RECORDED), [])
        command = [sys.executable, str(ROOT / "tools/harness_inventory.py")]
        subprocess.run([*command, "--base-ref", "HEAD"], check=True, cwd=ROOT)

    def test_removed_scenario_or_driver_fails_until_recorded(self) -> None:
        for key in ("default", "soak"):
            self.setUp()
            self.tree[key].pop()
            self.assertRegex(" ".join(self.errors()), f"{key} .* only in the inventory")
            self.recorded[key].pop()
            self.assertEqual(self.errors(), [])
            self.assertEqual(len(hi.removals(self.recorded, RECORDED)), 1)
        del self.tree["comparisons"]["math-comparison.py"]
        self.assertIn("comparison drivers differ", " ".join(self.errors()))
        del self.recorded["comparisons"]["math-comparison.py"]
        self.assertEqual(self.errors(), [])
        removed = hi.removals(self.recorded, RECORDED)
        self.assertIn("comparison math-comparison.py", removed)

    def test_zero_missing_or_fewer_cases_fail(self) -> None:
        recorded = {"a.py": ["A passed: 27 O0 records, 3 cases"]}
        for printed, message in (
            ([], "no case inventory"),
            (["A passed: 0 O0 records, 3 cases"], "zero count"),
            (["A passed: 26 O0 records, 3 cases"], "fewer cases"),
            (["A other: 27 O0 records, 3 cases"], "fewer cases"),
        ):
            errors = hi.case_errors(recorded, {"a.py": printed})
            self.assertRegex(" ".join(errors), message)
        more = {"a.py": ["A passed: 28 O0 records, 3 cases"]}
        self.assertEqual(hi.case_errors(recorded, more), [])
        self.recorded["comparisons"]["math-comparison.py"] = ["M passed: 0 lines"]
        self.tree["comparisons"] = self.recorded["comparisons"]
        self.assertRegex(" ".join(self.errors()), "zero count")

    def test_masks_cannot_grow_or_widen_but_can_shrink(self) -> None:
        widened = [mask.replace("revision.text", "*") for mask in RECORDED["masks"]]
        for masks in ([*RECORDED["masks"], "MAPS:*"], widened):
            self.tree["masks"] = self.recorded["masks"] = sorted(masks)
            self.assertRegex(" ".join(self.errors()), "mask added or widened")
        self.tree["masks"] = self.recorded["masks"] = RECORDED["masks"][1:]
        self.assertEqual(self.errors(), [])
        self.recorded["masks"] = RECORDED["masks"]
        self.assertIn("only in the inventory", " ".join(self.errors()))

    def test_record_rejects_empty_or_zero_runs_before_writing(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            tools = root / "tools"
            tools.mkdir()
            for name in (
                "run-comparisons.py",
                "harness_inventory.py",
                "vcs.py",
                "validation_execution.py",
            ):
                shutil.copyfile(ROOT / "tools" / name, tools / name)
            inventory = root / hi.INVENTORY
            inventory.parent.mkdir()
            driver = "fixture-comparison.py"
            recorded = copy.deepcopy(RECORDED)
            recorded["comparisons"] = {driver: ["Fixture passed: 3 cases"]}
            original = json.dumps(recorded)
            for line, code in (
                ("", 1),
                ("Fixture passed: 0 cases", 1),
                ("Fixture passed: 2 cases", 0),
            ):
                with self.subTest(line=line):
                    inventory.write_text(original)
                    (tools / driver).write_text(f"print({line!r})\n")
                    run = subprocess.run(
                        [sys.executable, str(tools / "run-comparisons.py"), "--record"],
                        cwd=root,
                        capture_output=True,
                        text=True,
                    )
                    self.assertEqual(run.returncode, code, run.stdout + run.stderr)
                    if code:
                        self.assertEqual(inventory.read_text(), original)
                    else:
                        self.assertEqual(
                            hi.load(inventory.read_text())["comparisons"],
                            {driver: [line]},
                        )

    def test_new_known_failure_names_an_open_issue(self) -> None:
        for issue, message in (("#1", None), ("#2", "open issue"), ("x", "open")):
            failures = {"play:VEHS:x": issue}
            self.tree["known_failures"] = self.recorded["known_failures"] = failures
            found = " ".join(self.errors())
            self.assertRegex(found, message) if message else self.assertEqual(found, "")
        self.recorded["known_failures"] = {}
        self.assertIn("KNOWN_FAILURES differs", " ".join(self.errors()))

    def test_scenario_key_no_tool_reads_fails(self) -> None:
        scenario = {"name": "a", "check_signs": True, "chekc_signs": True}
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "signs.py"
            source.write_text(
                'SCENARIOS = [{"name": "a", "chekc_signs": True}]\n'
                'def check(s):\n    return s["name"], s.get("check_signs")\n'
            )
            self.assertEqual(hi.unread_keys([scenario], [source]), ["chekc_signs"])


if __name__ == "__main__":
    unittest.main()
