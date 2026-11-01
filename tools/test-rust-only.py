#!/usr/bin/env python3
"""Exercise the Rust-only source gate on live files and directive forms."""

import importlib.util
import unittest
from pathlib import Path
from unittest.mock import patch

spec = importlib.util.spec_from_file_location(
    "rust_only", Path(__file__).with_name("rust-only.py")
)
assert spec and spec.loader
rust_only = importlib.util.module_from_spec(spec)
spec.loader.exec_module(rust_only)


class RustOnlyTests(unittest.TestCase):
    def test_conditional_forms(self) -> None:
        for directive in (
            "#ifdef WITH_RUST",
            "#ifndef WITH_RUST",
            "#if defined(WITH_RUST) && defined(OTHER)",
            "#elif !defined WITH_RUST",
            "# if /*comment*/ defined(\\\nWITH_RUST)",
        ):
            with self.subTest(directive=directive):
                self.assertTrue(rust_only.conditionals(directive + "\n", "x.cpp"))

    def test_comments_strings_and_other_macros(self) -> None:
        text = """/* #ifdef WITH_RUST */
// #ifndef WITH_RUST
const char *s = R"(
#ifdef WITH_RUST
)";
#if WITH_RUST_ABI_CHECKS
#endif
#define WITH_RUST
"""
        self.assertEqual(rust_only.conditionals(text, "x.cpp"), [])

    def test_sweep_exit_status(self) -> None:
        path = rust_only.ROOT / "src/strings.cpp"
        for status in (0, 1, 2):
            with (
                patch("sys.argv", ["rust-only.py", "--sweep", str(path)]),
                patch.object(Path, "read_text", return_value="#define WITH_RUST\n"),
                patch.object(rust_only.subprocess, "run") as run,
            ):
                run.return_value.returncode = status
                if status == 2:
                    with self.assertRaises(RuntimeError):
                        rust_only.main()
                else:
                    self.assertEqual(rust_only.main(), 0)
                self.assertEqual(
                    run.call_args.args[0][1:4], ["-DWITH_RUST", "-m", "-t"]
                )

    def test_gate_exit_status(self) -> None:
        path = rust_only.ROOT / "src/strings.cpp"
        with (
            patch("sys.argv", ["rust-only.py", str(path)]),
            patch.object(Path, "read_text", return_value="#ifdef WITH_RUST\n"),
            patch("builtins.print"),
        ):
            self.assertEqual(rust_only.main(), 1)

    def test_tools_cannot_be_swept(self) -> None:
        with self.assertRaises(ValueError):
            rust_only.source_files([Path(__file__)])


if __name__ == "__main__":
    unittest.main()
