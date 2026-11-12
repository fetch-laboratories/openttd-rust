#!/usr/bin/env python3
"""Check `migration.py fmt` with fake cargo and uvx executables."""

import contextlib
import io
import json
import os
import shlex
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import git_hooks

import migration

RECORDER = """#!{python}
import json, os, sys
with open(os.environ["RECORD"], "a") as record:
    record.write(json.dumps([os.path.basename(sys.argv[0]), sys.argv[1:], os.getcwd()]) + "\\n")
sys.exit(int(os.environ.get("FAIL_" + os.path.basename(sys.argv[0]).upper(), "0")))
"""


def quick_ruff(subcommand: str) -> list[str]:
    """Quick's own pinned Ruff line, so these tests retype no version."""
    return migration.ruff_command(git_hooks.ruff_lines(), subcommand)


class MigrationFmtTests(unittest.TestCase):
    def setUp(self) -> None:
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        root = Path(scratch.name).resolve()
        self.record = root / "record.jsonl"
        bin_directory = root / "bin"
        bin_directory.mkdir()
        for name in ("cargo", "uvx"):
            fake = bin_directory / name
            fake.write_text(RECORDER.format(python=sys.executable))
            fake.chmod(0o755)
        # No bootstrapped toolchain, so the fake cargo is the first on PATH.
        local = root / "local"
        local.mkdir()
        for name in ("LOCAL", "COMMON_LOCAL"):
            patch = mock.patch.object(migration, name, local)
            patch.start()
            self.addCleanup(patch.stop)
        patch = mock.patch.dict(
            os.environ,
            {
                "PATH": f"{bin_directory}{os.pathsep}{os.environ.get('PATH', '')}",
                "RECORD": str(self.record),
            },
        )
        patch.start()
        self.addCleanup(patch.stop)

    def fmt(self, *argv: str) -> tuple[int, str]:
        errors = io.StringIO()
        with (
            contextlib.redirect_stdout(io.StringIO()),
            contextlib.redirect_stderr(errors),
        ):
            code = migration.run_agent_command(["fmt", *argv])
        assert code is not None
        return code, errors.getvalue()

    def calls(self) -> list[list[str]]:
        lines = self.record.read_text().splitlines() if self.record.exists() else []
        calls = [json.loads(line) for line in lines]
        for _, _, cwd in calls:
            self.assertEqual(Path(cwd), migration.ROOT)
        return [[name, *argv] for name, argv, _ in calls]

    def test_formats_rust_and_quick_python_scope(self) -> None:
        self.assertEqual(self.fmt(), (0, ""))
        formatting = [word for word in quick_ruff("format") if word != "--check"]
        self.assertEqual(self.calls(), [["cargo", "fmt", "--all"], formatting])

    def test_check_writes_nothing_and_fails_on_changes(self) -> None:
        self.assertEqual(self.fmt("--check")[0], 0)
        self.assertEqual(
            self.calls(),
            [["cargo", "fmt", "--all", "--", "--check"], quick_ruff("format")],
        )
        self.assertIn("--check", quick_ruff("format"))
        with mock.patch.dict(os.environ, {"FAIL_CARGO": "1"}):
            code, errors = self.fmt("--check")
        self.assertEqual(code, 1)
        self.assertIn("run python3 tools/migration.py fmt", errors)

    def test_fix_runs_quick_lint_line_with_fix_before_formatting(self) -> None:
        self.assertEqual(self.fmt("--fix")[0], 0)
        calls = self.calls()
        self.assertEqual(calls[1], [*quick_ruff("check"), "--fix"])
        self.assertEqual(calls[2][-2:], ["format", "tools/"])

    def test_failure_runs_remaining_steps(self) -> None:
        with mock.patch.dict(os.environ, {"FAIL_CARGO": "101"}):
            code, errors = self.fmt()
        self.assertEqual(code, 1)
        self.assertIn("failed: cargo fmt --all", errors)
        self.assertEqual(len(self.calls()), 2)

    def test_check_and_fix_conflict(self) -> None:
        with self.assertRaises(SystemExit) as raised:
            self.fmt("--check", "--fix")
        self.assertEqual(raised.exception.code, 2)
        self.assertEqual(self.calls(), [])

    def test_ruff_line_selection(self) -> None:
        lines = ["uvx --from ruff==1.2.3 ruff check tools/", "uvx ty@0.1 check"]
        self.assertEqual(
            migration.ruff_command(lines, "check"),
            shlex.split(lines[0]),
        )
        with self.assertRaisesRegex(RuntimeError, "no `ruff format` line"):
            migration.ruff_command(lines, "format")


if __name__ == "__main__":
    unittest.main()
