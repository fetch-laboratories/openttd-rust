#!/usr/bin/env python3
"""Check that bootstrap-local.py installs nothing without --install."""

import contextlib
import importlib.util
import io
import unittest
from pathlib import Path
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location(
    "bootstrap_local", Path(__file__).with_name("bootstrap-local.py")
)
assert SPEC and SPEC.loader
BOOTSTRAP = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BOOTSTRAP)


class BootstrapLocalTests(unittest.TestCase):
    def setUp(self) -> None:
        # Downloads and package commands fail the test if anything reaches them.
        for patcher in (
            patch.object(BOOTSTRAP.subprocess, "run", side_effect=AssertionError),
            patch.object(
                BOOTSTRAP.urllib.request, "urlopen", side_effect=AssertionError
            ),
        ):
            patcher.start()
            self.addCleanup(patcher.stop)

    def main(self, *argv: str) -> tuple[int | str | None, str, bool]:
        output = io.StringIO()
        with (
            patch.object(BOOTSTRAP, "install") as install,
            contextlib.redirect_stdout(output),
            contextlib.redirect_stderr(output),
        ):
            try:
                code: int | str | None = BOOTSTRAP.main(list(argv))
            except SystemExit as exit:
                code = exit.code
        return code, output.getvalue(), install.called

    def test_help_and_no_arguments_install_nothing(self) -> None:
        for argv in ((), ("--help",), ("-h",)):
            code, output, installed = self.main(*argv)
            self.assertFalse(installed, argv)
            self.assertIn("--install", output)
            self.assertEqual(code, 0 if argv else 2)
        self.assertIn("Nothing installed", self.main()[1])

    def test_mistyped_option_installs_nothing(self) -> None:
        code, output, installed = self.main("--instal")
        self.assertFalse(installed)
        self.assertEqual(code, 2)
        self.assertIn("usage:", output)

    def test_install_needs_the_explicit_flag(self) -> None:
        code, _, installed = self.main("--install")
        self.assertEqual(code, 0)
        self.assertTrue(installed)


if __name__ == "__main__":
    unittest.main()
