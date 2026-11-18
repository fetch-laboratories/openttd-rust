#!/usr/bin/env python3
"""Check crash_stacks.py, the Cargo runner that prints a crashed test's stacks."""

import argparse
import contextlib
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
from collections.abc import Sequence
from pathlib import Path
from unittest.mock import patch

import crash_stacks

import migration

SCRIPT = Path(__file__).with_name("crash_stacks.py")
KILL = (
    "import os, signal\nprint('started', flush=True)\nos.kill(os.getpid(), signal.{})"
)
# A stand-in for gdb: prints a long multi-thread trace naming what it read.
DEBUGGER = (
    "import json, sys\n"
    "open(sys.argv[0] + '.calls', 'a').write(json.dumps(sys.argv[1:]) + '\\n')\n"
    "print('Thread 9 (LWP 9): first header', sys.argv[-2], sys.argv[-1])\n"
    "print('#0 frame\\n' * 3000)\n"
    "print('Thread 1 (LWP 1): main')\n"
)


class CrashStackTests(unittest.TestCase):
    def setUp(self) -> None:
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        self.root = Path(scratch.name)

    def crash(
        self, name: str, core: Path | None
    ) -> tuple[int, str, list[Path], list[list[str]]]:
        """Run a child that kills itself with the signal, in process, with the
        core lookup and the debugger stubbed and no core limit raised."""
        gdb = self.root / "gdb"
        gdb.write_text(f"#!{sys.executable}\n{DEBUGGER}")
        gdb.chmod(0o755)
        stderr = io.StringIO()
        command = [sys.executable, "-c", KILL.format(name)]
        with (
            patch.dict(os.environ, {"CRASH_STACKS_DIR": str(self.root / "stacks")}),
            patch.object(crash_stacks, "raise_core_limit", lambda: None),
            patch.object(crash_stacks, "prepare_cores", return_value=None),
            patch.object(crash_stacks, "core_file", return_value=core),
            patch.object(crash_stacks, "debugger", return_value=str(gdb)),
            contextlib.redirect_stderr(stderr),
        ):
            code = crash_stacks.run(command)
        traces = sorted((self.root / "stacks").glob("*.txt"))
        calls = self.root / "gdb.calls"
        debugged = (
            [json.loads(line) for line in calls.read_text().splitlines()]
            if calls.is_file()
            else []
        )
        calls.unlink(missing_ok=True)
        return code, stderr.getvalue(), traces, debugged

    def test_a_passing_child_passes_through_unchanged(self) -> None:
        for status in (0, 3):
            with self.subTest(status=status):
                done = subprocess.run(
                    [
                        sys.executable,
                        str(SCRIPT),
                        sys.executable,
                        "-c",
                        f"print('test result: ok'); raise SystemExit({status})",
                    ],
                    capture_output=True,
                    text=True,
                    # Off CI: the script changes no system setting.
                    env={k: v for k, v in os.environ.items() if k != "GITHUB_ACTIONS"},
                )
                self.assertEqual(done.returncode, status)
                self.assertEqual(done.stdout, "test result: ok\n")
                self.assertEqual(done.stderr, "")

    def test_a_segfault_reads_its_core_with_the_debugger_and_fails(self) -> None:
        core = self.root / "core.1"
        core.write_bytes(b"core")
        code, shown, traces, debugged = self.crash("SIGSEGV", core)
        self.assertEqual(code, 128 + 11)
        self.assertIn("died by SIGSEGV", shown)
        (command,) = debugged
        self.assertEqual(command[-2:], [sys.executable, str(core)])
        self.assertIn("thread apply all bt", command)
        (trace,) = traces
        text = trace.read_text()
        self.assertIn("Thread 9 (LWP 9): first header", text)
        self.assertIn("Thread 1 (LWP 1): main", text)
        # No core: the signal and the command that reruns under gdb.
        code, shown, traces, debugged = self.crash("SIGSEGV", None)
        self.assertEqual((code, debugged), (139, []))
        self.assertIn("no core file was written; to rerun under gdb: gdb", shown)

    def test_a_terminated_child_reports_the_signal_and_no_core(self) -> None:
        code, shown, traces, debugged = self.crash("SIGTERM", self.root / "x")
        self.assertEqual((code, traces, debugged), (128 + 15, [], []))
        self.assertIn("died by SIGTERM", shown)
        self.assertIn("SIGTERM writes no core", shown)

    def test_a_long_trace_reaches_the_failure_output_whole(self) -> None:
        stacks = self.root / migration.CRASH_STACKS
        stacks.mkdir()
        trace = "Thread 9 (LWP 9): first header\n" + "#0 frame\n" * 3000
        (stacks / "kernels-1.txt").write_text(trace)
        log = "x" * 20000
        shown = migration.with_crash_stacks("rust-tests", log, self.root)
        self.assertGreater(len(trace), 16000)
        self.assertIn(trace, shown)
        self.assertEqual(migration.failure_output("rust-tests", log), "x" * 16000)

    def test_rust_checks_runs_the_tests_through_the_runner(self) -> None:
        ran: dict[str, list[str]] = {}

        def run(
            name: str, command: Sequence[str | Path], cwd: Path = migration.ROOT
        ) -> Path:
            ran[name] = [str(part) for part in command]
            return self.root / f"{name}.log"

        execution = migration.Execution(
            args=argparse.Namespace(),
            simulate_args=[],
            report={},
            evidence=self.root,
            builds={},
            cache_executable=None,
            instruction_mode=False,
            run=run,
            select_role=lambda role: {},
        )
        with patch.object(migration, "clippy_command", return_value=["clippy"]):
            migration.command_rust_checks(execution)
        tests = ran["rust-tests"]
        stacks = self.root / migration.CRASH_STACKS
        self.assertEqual(tests[:2], ["env", f"CRASH_STACKS_DIR={stacks}"])
        self.assertIn(str(SCRIPT.resolve()), tests[4])
        self.assertIn("cfg(target_os", tests[4])
        self.assertEqual(tests[5:], ["test", "--workspace", "--locked"])

    def test_on_ci_the_core_pattern_is_set_then_put_back(self) -> None:
        pattern = self.root / "core_pattern"
        pattern.write_text("|/usr/share/apport/apport -p%p\n")
        stubs = self.root / "stubs"
        stubs.mkdir()
        calls = self.root / "sudo.calls"
        (stubs / "sudo").write_text(f'#!/bin/sh\necho "$*" >> {calls}\n')
        (stubs / "sudo").chmod(0o755)
        environment = {
            "GITHUB_ACTIONS": "true",
            "PATH": f"{stubs}{os.pathsep}{os.environ['PATH']}",
        }
        with (
            patch.dict(os.environ, environment),
            patch.object(crash_stacks, "PATTERN", pattern),
        ):
            code = crash_stacks.run([sys.executable, "-c", "pass"])
        self.assertEqual(code, 0)
        first, second = calls.read_text().splitlines()
        folder = first.removeprefix("-n sysctl -w kernel.core_pattern=").removesuffix(
            "/core.%p"
        )
        self.assertTrue(folder.startswith(tempfile.gettempdir()))
        self.assertFalse(Path(folder).exists())
        self.assertEqual(
            second, "-n sysctl -w kernel.core_pattern=|/usr/share/apport/apport -p%p"
        )

    def test_a_failed_debugger_is_not_reported_as_every_stack(self) -> None:
        gdb = self.root / "gdb"
        gdb.write_text("#!/bin/sh\necho 'warning: core file is truncated'\nexit 1\n")
        gdb.chmod(0o755)
        core = self.root / "core.1"
        core.write_bytes(b"core")
        stderr = io.StringIO()
        command = [sys.executable, "-c", KILL.format("SIGSEGV")]
        with (
            patch.dict(os.environ, {"CRASH_STACKS_DIR": str(self.root / "stacks")}),
            patch.object(crash_stacks, "raise_core_limit", lambda: None),
            patch.object(crash_stacks, "prepare_cores", return_value=None),
            patch.object(crash_stacks, "core_file", return_value=core),
            patch.object(crash_stacks, "debugger", return_value=str(gdb)),
            contextlib.redirect_stderr(stderr),
        ):
            code = crash_stacks.run(command)
        self.assertEqual(code, 139)
        self.assertIn("gdb exited 1", stderr.getvalue())
        self.assertNotIn("every thread's stack is in", stderr.getvalue())
        (trace,) = (self.root / "stacks").glob("*.txt")
        self.assertIn("may be incomplete", trace.read_text())

    def test_off_ci_a_piped_core_pattern_has_no_core_file(self) -> None:
        pattern = self.root / "core_pattern"
        (self.root / "core.7").write_bytes(b"core")
        written = (self.root / "core.7").stat().st_mtime
        for text, started, expected in (
            ("|/usr/share/apport/apport -p%p", written, None),
            (f"{self.root}/core.%p", written, self.root / "core.7"),
            (f"{self.root}/core.%e.%p", written, None),
            # A core older than the child is another process's dump.
            (f"{self.root}/core.%p", written + 60, None),
            ("core", written + 60, None),
        ):
            pattern.write_text(text + "\n")
            with (
                self.subTest(text=text, started=started),
                patch.object(crash_stacks, "PATTERN", pattern),
            ):
                found = crash_stacks.core_file(None, 7, self.root, started)
                self.assertEqual(found, expected)


if __name__ == "__main__":
    unittest.main()
