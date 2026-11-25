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
from functools import partial
from pathlib import Path
from unittest import mock

import quick_workflow

import migration

RECORDER = """#!{python}
import json, os, sys
with open(os.environ["RECORD"], "a") as record:
    record.write(json.dumps([os.path.basename(sys.argv[0]), sys.argv[1:], os.getcwd()]) + "\\n")
sys.exit(int(os.environ.get("FAIL_" + os.path.basename(sys.argv[0]).upper(), "0")))
"""


def quick_ruff(subcommand: str) -> list[str]:
    """Quick's own pinned Ruff line, so these tests retype no version."""
    return migration.ruff_command(quick_workflow.ruff_lines(), subcommand)


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

    def test_a_path_with_or_without_src_names_the_same_file(self) -> None:
        # grep prints src/... paths, which reviewers quote with or without src/.
        _, printed, _ = self.original("grep", "TileAdd", "map.cpp")
        self.assertEqual(printed, "src/map.cpp:2:two TileAdd\n")
        path = printed.split(":")[0]
        for spec in (f"{path}:2", "map.cpp:2", "./map.cpp:2"):
            self.assertEqual(
                self.original("show", spec), (0, "     2  two TileAdd\n", "")
            )
        self.assertEqual(self.original("grep", "TileAdd", "src/map.cpp")[1], printed)
        # A root file keeps its own path; a path in neither form is refused.
        self.assertEqual(self.original("show", "README")[1], "TileAdd outside src\n")
        self.assertEqual(self.original("show", "missing.cpp")[0], 1)

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


class SimulateListTests(unittest.TestCase):
    """`simulate --list` lists and builds nothing; `build --instructions` builds
    the instruction games (#1063)."""

    def setUp(self) -> None:
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        self.local = Path(scratch.name)
        self.built: list[bool] = []
        for target, value in (
            ("LOCAL", self.local),
            ("ensure_reference", mock.Mock(side_effect=AssertionError("reference"))),
            ("build_games", mock.Mock(side_effect=self.build)),
            ("sanitizer_exec", mock.Mock()),
            ("SANITIZE", False),
            ("TSAN", False),
        ):
            patch = mock.patch.object(migration, target, value)
            patch.start()
            self.addCleanup(patch.stop)

    def build(self, execution: "migration.Execution") -> None:
        self.built.append(execution.instruction_mode)
        raise RuntimeError("built")

    def listing(self, *argv: str) -> tuple[int | None, list[list[str]], str]:
        """The listing's exit code, the child commands and stderr."""
        errors = io.StringIO()
        run = mock.Mock(return_value=subprocess.CompletedProcess([], 3))
        with (
            mock.patch.object(migration.subprocess, "run", run),
            contextlib.redirect_stderr(errors),
        ):
            try:
                code = migration.run_listing(["simulate", *argv])
            except SystemExit as error:
                code = error.code if isinstance(error.code, int) else -1
        self.assertEqual(self.built, [])
        self.assertFalse((self.local / "verification").exists())
        commands = [call.args[0] for call in run.call_args_list]
        return code, commands, errors.getvalue()

    def test_a_listing_runs_only_the_child_and_returns_its_code(self) -> None:
        for argv in (
            ["--list"],
            ["--soak", "--list", "NAME"],
            ["--instructions", "--list"],
        ):
            with self.subTest(argv):
                code, commands, _ = self.listing("--jobs", "4", *argv)
                self.assertEqual(code, 3)
                self.assertEqual(
                    commands,
                    [
                        [
                            sys.executable,
                            str(migration.ROOT / "tools/simulate.py"),
                            "--jobs",
                            "2",
                            *argv,
                        ]
                    ],
                )

    def test_a_listing_halves_jobs_as_a_run_does(self) -> None:
        _, (command,), _ = self.listing("--jobs", "2", "--benchmark", "1", "--list")
        self.assertEqual(command[2:], ["--jobs", "1", "--benchmark", "1", "--list"])

    def test_a_listing_gives_a_runs_refusals_with_nothing_built(self) -> None:
        for argv, refusal in (
            (["--threaded"], "drop --threaded/--multiplayer"),
            (["--multiplayer"], "drop --threaded/--multiplayer"),
            (
                ["--sanitize", "--instructions"],
                "--sanitize conflicts with --instructions",
            ),
            (
                ["--multiplayer", "--benchmark", "1"],
                "--multiplayer requires ordinary simulate without profiling",
            ),
        ):
            with self.subTest(argv):
                code, commands, errors = self.listing(*argv, "--list")
                self.assertEqual(code, 2)
                self.assertEqual(commands, [])
                self.assertIn(refusal, errors)

    def test_a_sanitized_listing_gets_the_sanitized_childs_refusals(self) -> None:
        """The real child, which lists without building: sanitized games take no
        --benchmark or --frozen-runtimes (review of #1065)."""
        for argv in (
            ["--jobs", "2", "--benchmark", "1"],
            ["--frozen-runtimes", "foo"],
        ):
            with self.subTest(argv):
                quiet = mock.Mock(
                    side_effect=partial(subprocess.run, capture_output=True)
                )
                with (
                    mock.patch.dict(os.environ, {migration.SANITIZE_VARIABLE: ""}),
                    mock.patch.object(migration.subprocess, "run", quiet),
                ):
                    plain = migration.run_listing(["simulate", *argv, "--list"])
                    sanitized = migration.run_listing(
                        ["simulate", "--sanitize", *argv, "--list"]
                    )
                self.assertEqual((plain, sanitized), (0, 2))
                self.assertEqual(self.built, [])

    def test_other_commands_are_left_to_main(self) -> None:
        for argv in (["simulate", "NAME"], ["coverage", "--list"], ["build"]):
            with self.subTest(argv):
                self.assertIsNone(migration.run_listing(argv))

    def test_build_instructions_builds_the_instruction_games(self) -> None:
        with (
            mock.patch.object(migration, "ensure_reference"),
            mock.patch.object(sys, "argv", ["migration.py", "build", "--instructions"]),
            mock.patch.object(migration, "reference_lock"),
            contextlib.redirect_stdout(io.StringIO()),
            self.assertRaisesRegex(RuntimeError, "built"),
        ):
            sys.argv.append("--no-ccache")
            migration.main()
        self.assertEqual(self.built, [True])

    def test_only_build_takes_instructions(self) -> None:
        for argv in (["build", "--soak"], ["verify", "--instructions"]):
            errors = io.StringIO()
            with (
                self.subTest(argv),
                contextlib.redirect_stderr(errors),
                self.assertRaises(SystemExit),
            ):
                migration.arguments(argv)
            self.assertIn("unrecognized arguments", errors.getvalue())


if __name__ == "__main__":
    unittest.main()
