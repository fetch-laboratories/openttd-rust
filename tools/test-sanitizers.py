#!/usr/bin/env python3
"""Run real sanitized programs through the harness report reader (#505)."""

import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from simulation import sanitizers

# Mode "ub" overflows an int on line 8; mode "free" reads freed memory on line 11.
PROGRAM = """#include <climits>
#include <cstdlib>
#include <cstring>

int main(int argc, char **argv) {
	int x = 0;
	if (argc > 1 && strcmp(argv[1], "ub") == 0) {
		x = INT_MAX + argc;
	}
	if (argc > 1 && strcmp(argv[1], "free") == 0) {
		int *p = new int[4]();
		delete[] p;
		x += p[1];
	}
	return x == 1;
}
"""
OVERFLOW = "UndefinedBehaviorSanitizer: signed integer overflow at src/bug.cpp:8"
FREED = "AddressSanitizer: heap-use-after-free at src/bug.cpp:13"


class SanitizerTests(unittest.TestCase):
    def setUp(self) -> None:
        if not shutil.which("c++"):
            self.skipTest("no C++ compiler")
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        self.scratch = Path(scratch.name)
        (self.scratch / "src").mkdir()
        (self.scratch / "src/bug.cpp").write_text(PROGRAM)
        flags = ["-fsanitize=address,undefined", "-g", "-O0"]
        built = subprocess.run(
            ["c++", *flags, "src/bug.cpp", "-o", "bug"], cwd=self.scratch
        )
        if built.returncode:
            self.skipTest("the compiler lacks AddressSanitizer or UBSan")
        self.run_dir = self.scratch / "run"
        self.suppressions = self.scratch / "suppressions.txt"
        self.suppressions.write_text("# header\n")
        self.addCleanup(sanitizers.suppressions.cache_clear)
        sanitizers.suppressions.cache_clear()
        original = sanitizers.SUPPRESSIONS
        sanitizers.SUPPRESSIONS = self.suppressions
        self.addCleanup(setattr, sanitizers, "SUPPRESSIONS", original)

    def game(self, *arguments: str) -> dict:
        shutil.rmtree(self.run_dir, ignore_errors=True)
        self.run_dir.mkdir()
        environment = sanitizers.environment(self.run_dir)
        with (self.run_dir / "stderr.log").open("wb") as log:
            subprocess.run(
                ["./bug", *arguments], cwd=self.scratch, env=environment, stderr=log
            )
        sanitizers.separate(self.run_dir)
        return {"dir": self.run_dir}

    def checked(self, mode: str, role: str = "candidate") -> dict:
        result: dict = {"problems": [], "notes": []}
        sanitizers.check(self.game(mode), role, "plain", result)
        return result

    def test_each_kind_of_report_fails_the_scenario_with_its_location(self) -> None:
        (problem,) = self.checked("ub")["problems"]
        self.assertTrue(problem.startswith(f"plain: candidate sanitizer: {OVERFLOW} ("))
        (problem,) = self.checked("free", "reference")["problems"]
        self.assertTrue(problem.startswith(f"plain: reference sanitizer: {FREED} ("))
        self.assertTrue(Path(problem.rsplit("(", 1)[1].rstrip(")")).is_file())

    def test_undefined_behavior_report_leaves_the_compared_log(self) -> None:
        run = self.game("ub")
        self.assertEqual((run["dir"] / "stderr.log").read_bytes(), b"")

    def test_suppressed_report_is_noted_and_other_kinds_still_fail(self) -> None:
        self.suppressions.write_text(f"{OVERFLOW} # original overflow (#1)\n")
        result = self.checked("ub")
        self.assertEqual(result["problems"], [])
        (note,) = result["notes"]
        self.assertTrue(note.endswith(f"{OVERFLOW}: original overflow (#1)"))
        self.assertEqual(len(self.checked("free")["problems"]), 1)

    def test_clean_game_has_no_reports(self) -> None:
        self.assertEqual(self.checked("")["problems"], [])


if __name__ == "__main__":
    unittest.main()
