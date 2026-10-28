#!/usr/bin/env python3
"""Check threaded witness failures and TSan conflict identity/control handling (#506)."""

import os
import re
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from simulation import core, sanitizers, thread_sanitizer, threaded

import migration

CONTROL = r"""
#include <atomic>
#include <mutex>
#include <thread>
volatile int state = 0;
std::atomic<bool> ready{false};
std::mutex lock;
void worker()
{
    while (!ready.load(std::memory_order_relaxed)) {}
    for (int i = 0; i < 1000; ++i) {
#ifdef LOCKED
        std::lock_guard<std::mutex> guard(lock);
#endif
        state = state + 1;
    }
}
int main()
{
    std::thread a(worker), b(worker);
    ready.store(true, std::memory_order_relaxed);
    a.join(); b.join();
}
"""


def report(root: Path, other: str = "Pause", global_name: str = "_pause_mode") -> str:
    return (
        "==================\nWARNING: ThreadSanitizer: data race (pid=42)\n"
        "  Read of size 1 at 0x123 by main thread:\n"
        f"    #0 GetGameInterval() {root}/src/video.cpp:12 (openttd+0x1)\n\n"
        "  Previous write of size 1 at 0x123 by thread T2:\n"
        f"    #0 {other}() {root}/src/command.cpp:22 (openttd+0x2)\n\n"
        f"  Location is global '{global_name}' of size 1 at 0x123 (openttd+0x3)\n\n"
        "SUMMARY: ThreadSanitizer: data race\n==================\n"
    )


class ThreadedTests(unittest.TestCase):
    def test_conflict_stacks_and_global_identity(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            roles = {}
            for role in ("reference", "candidate"):
                run = root / role
                (run / sanitizers.REPORTS).mkdir(parents=True)
                source = root / f"{role}-source"
                text = report(source)
                if role == "candidate":
                    text = (
                        re.sub(r"0x[0-9a-f]+", "0xabc", text)
                        .replace("T2", "T9")
                        .replace("pid=42", "pid=99")
                        .replace(":12", ":90")
                    )
                (run / sanitizers.REPORTS / "report.42").write_text(text)
                roles[role] = thread_sanitizer.findings(run, (source,), ())
            self.assertEqual(len(sanitizers.compare(roles).original), 1)
            for other, name in (("Resume", "_pause_mode"), ("Pause", "_game_speed")):
                (run / sanitizers.REPORTS / "report.42").write_text(
                    report(source, other, name)
                )
                roles["candidate"] = thread_sanitizer.findings(run, (source,), ())
                comparison = sanitizers.compare(roles)
                self.assertEqual(len(comparison.candidate_only), 1)
                self.assertEqual(len(comparison.reference_only), 1)
                self.assertEqual(comparison.original, [])
            for invalid in (
                "FATAL: ThreadSanitizer: unexpected memory mapping",
                "WARNING: ThreadSanitizer: other (pid=42)",
                report(source).replace("  Previous write", "  Missing write"),
                report(source).replace("  Location is global", "  Missing global"),
            ):
                (run / sanitizers.REPORTS / "report.42").write_text(invalid)
                with self.assertRaises(RuntimeError):
                    thread_sanitizer.findings(run, (source,), ())

    def test_runtime_control(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "src").mkdir()
            (root / "src/control.cpp").write_text(CONTROL)
            for locked in (False, True):
                executable = root / f"control-{locked}"
                subprocess.run(
                    [
                        "g++",
                        "-O1",
                        "-g",
                        *migration.TSAN_FLAGS.split(),
                        *(["-DLOCKED"] if locked else []),
                        str(root / "src/control.cpp"),
                        "-o",
                        str(executable),
                    ],
                    check=True,
                )
                run = root / f"run-{locked}"
                run.mkdir()
                executed = subprocess.run(
                    [str(executable)],
                    env=os.environ | thread_sanitizer.environment(run),
                    capture_output=True,
                    timeout=20,
                )
                self.assertEqual(executed.returncode, 0, executed.stderr)
                found = thread_sanitizer.findings(run, (root,), ())
                self.assertEqual(bool(found.reports), not locked)
                if not locked:
                    self.assertTrue(
                        all(
                            "global 'state'" in key
                            and "worker() at src/control.cpp" in key
                            for key in found.reports
                        )
                    )

    def test_missing_save_and_wrong_runtime_markers_fail(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            save = root / "save/autosave/exit.sav"
            save.parent.mkdir(parents=True)
            save.touch()
            (root / "console.log").write_text("Current date: 1993-02-01\n")
            moment = (727960, 0, 84677)
            scenario: threaded.Scenario = {
                "name": "test",
                "kind": "save",
                "save": "input",
                "ticks": 0,
                "snapshot_minimum": 0,
                "moment": moment,
                "tile": 10814,
            }
            log = [
                "THREAD506 credits true",
                "THREAD506 game tile 10814 occupied true error 4162",
                "THREAD506 main split true tile 10814 occupied true error 4162 clear true error 4162",
                "THREAD506 stop game true calendar 727960 fract 0 economy 727959 fract 73",
                "dbg: [driver:1] using thread for game-loop",
            ]
            run: threaded.Run = {
                "dir": root,
                "log": log,
                "exit": 0,
                "seconds": 0,
                "command": [],
                "instructions": None,
                "snapshots": [save],
                "stdout": b"",
                "sanitizer": None,
            }
            with patch.object(core, "save_moment", return_value=moment):
                threaded.check(scenario, run)
                for old, new in (
                    ("credits true", "credits false"),
                    ("split true", "split false"),
                    ("main split true tile 10814", "main split true tile 10815"),
                    ("clear true", "clear false"),
                    ("economy 727959", "economy 727960"),
                    ("game true calendar", "game false calendar"),
                    ("using thread", "using no thread"),
                ):
                    run["log"] = [line.replace(old, new) for line in log]
                    with self.subTest(old=old), self.assertRaises(RuntimeError):
                        threaded.check(scenario, run)
                run["log"] = log
                save.unlink()
                with self.assertRaises(RuntimeError):
                    threaded.check(scenario, run)


if __name__ == "__main__":
    unittest.main()
