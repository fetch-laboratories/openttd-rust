#!/usr/bin/env python3
"""Check the pinned inherited commit checker checkout."""

import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import preflight


class PreflightTests(unittest.TestCase):
    def test_checker_exit_code_and_revision_range_are_preserved(self):
        for code in (0, 1):
            with (
                patch.object(preflight, "hooks_checkout", return_value=Path("/hooks")),
                patch.object(preflight, "git", side_effect=["base", "head", "/git"]),
                patch.object(preflight.subprocess, "run") as run,
                patch.object(sys, "argv", ["preflight.py"]),
            ):
                run.return_value.returncode = code
                self.assertEqual(preflight.main(), code)
                self.assertEqual(
                    run.call_args.args[0],
                    ["sh", "/hooks/hooks/check-commits.sh", "base..head"],
                )
                self.assertEqual(run.call_args.kwargs["env"]["GIT_DIR"], "/git")

    def test_supplied_checker_must_match_pin_and_be_clean(self):
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
