#!/usr/bin/env python3
"""Check the pinned inherited commit checker checkout."""

import sys
import tempfile
import unittest
from pathlib import Path
from subprocess import CompletedProcess
from unittest.mock import patch

import preflight


class PreflightTests(unittest.TestCase):
    def test_checker_exit_code_and_revision_range_are_preserved(self) -> None:
        for checker_code, rebound_code in ((0, 0), (0, 1), (1, 0)):
            with (
                patch.object(preflight, "hooks_checkout", return_value=Path("/hooks")),
                patch.object(preflight, "git", side_effect=["base", "head", "/git"]),
                patch.object(preflight.subprocess, "run") as run,
                patch.object(sys, "argv", ["preflight.py"]),
            ):
                run.side_effect = [
                    CompletedProcess([], checker_code),
                    CompletedProcess([], rebound_code),
                ]
                self.assertEqual(preflight.main(), checker_code or rebound_code)
                checker = run.call_args_list[0]
                self.assertEqual(
                    checker.args[0],
                    ["sh", "/hooks/hooks/check-commits.sh", "base..head"],
                )
                self.assertEqual(checker.kwargs["env"]["GIT_DIR"], "/git")
                self.assertEqual(run.call_count, 1 if checker_code else 2)
                if not checker_code:
                    self.assertEqual(
                        run.call_args.args[0],
                        [
                            sys.executable,
                            str(preflight.migration.ROOT / "tools/rebound-callees.py"),
                            "--base-ref",
                            "base",
                        ],
                    )

    def test_supplied_checker_must_match_pin_and_be_clean(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with patch.object(preflight.migration, "COMMON_LOCAL", root):
                with patch.object(preflight, "git", side_effect=["bad revision"]):
                    with self.assertRaisesRegex(ValueError, "pinned revision"):
                        preflight.hooks_checkout(root)
                with patch.object(
                    preflight,
                    "git",
                    side_effect=[preflight.HOOKS_REVISION, " M hooks/check-diff.py"],
                ):
                    with self.assertRaisesRegex(ValueError, "dirty"):
                        preflight.hooks_checkout(root)


if __name__ == "__main__":
    unittest.main()
