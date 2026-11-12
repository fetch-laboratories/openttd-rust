#!/usr/bin/env python3
"""Check `migration.py fmt` with fake cargo and uvx, and `original` against a
temporary repository."""

import contextlib
import io
import json
import os
import shlex
import subprocess
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


def git(cwd: Path, *args: str) -> str:
    return subprocess.check_output(
        ["git", "-c", "core.hooksPath=/dev/null", *args],
        cwd=cwd,
        text=True,
        env=os.environ
        | {
            "GIT_AUTHOR_NAME": "Test",
            "GIT_AUTHOR_EMAIL": "test@example.invalid",
            "GIT_COMMITTER_NAME": "Test",
            "GIT_COMMITTER_EMAIL": "test@example.invalid",
        },
    ).strip()


class MigrationOriginalTests(unittest.TestCase):
    """`original` against a temporary repository whose baseline is its first commit."""

    def setUp(self) -> None:
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        self.root = Path(scratch.name).resolve() / "fork"
        self.root.mkdir()
        git(self.root, "init", "-q")
        (self.root / "src").mkdir()
        (self.root / "src/map.cpp").write_text("one\ntwo TileAdd\nthree\n")
        (self.root / "README").write_text("TileAdd outside src\n")
        git(self.root, "add", ".")
        git(self.root, "commit", "-q", "-m", "original")
        self.commit = git(self.root, "rev-parse", "HEAD")
        # The candidate's own src/ differs, committed and in the working tree.
        (self.root / "src/map.cpp").write_text("fork TileAdd\n")
        git(self.root, "commit", "-q", "-am", "fork")
        (self.root / "src/map.cpp").write_text("dirty TileAdd\n")
        self.reference = self.root.parent / "common/.local/reference/openttd"
        patches = {
            "ROOT": self.root,
            "REFERENCE": self.reference,
            "BASELINE": {"tag": "15.3", "commit": self.commit},
        }
        for name, value in patches.items():
            patch = mock.patch.object(migration, name, value)
            patch.start()
            self.addCleanup(patch.stop)

    def original(self, *argv: str) -> tuple[int, str, str]:
        output, errors = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(output), contextlib.redirect_stderr(errors):
            try:
                code = migration.run_agent_command(["original", *argv])
            except RuntimeError as error:  # The entry point prints these and exits 1.
                code, errors = 1, io.StringIO(str(error))
        assert code is not None
        return code, output.getvalue(), errors.getvalue()

    def add_reference(self, commit: str) -> None:
        self.reference.parent.mkdir(parents=True)
        git(self.root, "worktree", "add", "-q", "--detach", str(self.reference), commit)

    def test_show_reads_the_pinned_commit(self) -> None:
        self.assertEqual(
            self.original("show", "src/map.cpp"), (0, "one\ntwo TileAdd\nthree\n", "")
        )
        code, output, _ = self.original("show", "./src/map.cpp:2-3")
        self.assertEqual((code, output), (0, "     2  two TileAdd\n     3  three\n"))
        self.assertEqual(self.original("show", "src/map.cpp:3")[1], "     3  three\n")

    def test_show_refusals_name_the_remedy(self) -> None:
        for spec in ("../outside", "/etc/passwd", "src/../../outside"):
            code, _, errors = self.original("show", spec)
            self.assertEqual(code, 1, spec)
            self.assertIn("relative to the original's root", errors)
        code, _, errors = self.original("show", "src/missing.cpp")
        self.assertEqual(code, 1)
        self.assertIn("original grep PATTERN", errors)
        code, _, errors = self.original("show", "src/map.cpp:4-5")
        self.assertEqual(code, 1)
        self.assertIn("has 3 lines", errors)

    def test_grep_prints_paths_without_commit_prefix(self) -> None:
        code, output, _ = self.original("grep", "Tile(Add|Sub)")
        self.assertEqual((code, output), (0, "src/map.cpp:2:two TileAdd\n"))
        code, output, _ = self.original("grep", "TileAdd", "README", "src")
        self.assertEqual(output.splitlines()[0], "README:1:TileAdd outside src")
        code, output, errors = self.original("grep", "absent")
        self.assertEqual((code, output), (1, ""))
        self.assertIn("no match", errors)
        self.assertEqual(self.original("grep", "x", "../src")[0], 1)

    def test_missing_pinned_commit_names_the_fetch(self) -> None:
        missing = "0" * 40
        with mock.patch.object(migration, "BASELINE", {"commit": missing}):
            code, _, errors = self.original("grep", "TileAdd")
        self.assertEqual(code, 1)
        self.assertIn(f"git fetch --no-tags --depth=1 origin {missing}", errors)

    def test_path_checks_the_reference_worktree(self) -> None:
        code, output, errors = self.original("path")
        self.assertEqual((code, output), (1, ""))
        self.assertIn("migration.py build", errors)
        self.add_reference(self.commit)
        self.assertEqual(self.original("path"), (0, f"{self.reference}\n", ""))
        (self.reference / "src/map.cpp").write_text("edited\n")
        code, output, errors = self.original("path")
        self.assertEqual((code, output), (1, ""))
        self.assertIn("has changes", errors)
        # A dirty reference worktree does not change what show reads.
        self.assertIn("two TileAdd", self.original("show", "src/map.cpp")[1])

    def test_path_refuses_a_reference_off_the_pin(self) -> None:
        self.add_reference("HEAD")
        code, output, errors = self.original("path")
        self.assertEqual((code, output), (1, ""))
        self.assertIn(f"not the pinned {self.commit}", errors)


if __name__ == "__main__":
    unittest.main()
