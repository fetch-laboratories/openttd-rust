#!/usr/bin/env python3
"""Exercise the production-use scan and shrinking baseline on a fixture tree."""

import contextlib
import importlib.util
import io
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location(
    "orphans", Path(__file__).with_name("orphan-exports.py")
)
assert SPEC and SPEC.loader
ORPHANS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ORPHANS)


def export(name: str) -> str:
    return (
        f'#[unsafe(no_mangle)]\npub unsafe extern "C" fn openttd_rust_{name}() {{}}\n'
    )


class OrphanTests(unittest.TestCase):
    def test_declarations_are_not_users_and_bindings_are(self) -> None:
        files = {
            "rust/lib.rs": export("call")
            + export("table")
            + export("orphan")
            + 'const EXAMPLE: &str = r#"#[no_mangle] pub extern "C" fn openttd_rust_fake() {}"#;',
            "src/owner.h": 'extern "C" void openttd_rust_call();\n',
            "src/rust/ffi.h": "const Thing *openttd_rust_table(\n int, void (*callback)());\n"
            "void openttd_rust_orphan();\n",
            "src/owner.cpp": "void Run() { openttd_rust_call(); }\n"
            "const Services table{.callback = openttd_rust_table};\n"
            '// openttd_rust_orphan();\nconst char *s = "openttd_rust_orphan";\n',
            "src/tests/test.cpp": "void Test() { openttd_rust_orphan(); }",
            "tools/probe.cpp": "void Probe() { openttd_rust_orphan(); }",
            "migration/windows-abi.cpp": "void Probe() { openttd_rust_orphan(); }",
        }
        self.assertEqual(
            ORPHANS.violations(files), {"no production user | openttd_rust_orphan"}
        )
        files["rust/lib.rs"] += export("missing")
        files["src/owner.cpp"] += "\nvoid Run2() { openttd_rust_missing(); }"
        self.assertEqual(
            ORPHANS.violations(files),
            {
                "no production user | openttd_rust_orphan",
                "no declaration | openttd_rust_missing",
            },
        )

    def test_visibility_and_intervening_attributes(self) -> None:
        for visibility in ("", "pub ", "pub(crate) "):
            for attribute in ("", "#[allow(unsafe_code)]\n"):
                source = (
                    "#[unsafe(no_mangle)]\n"
                    + attribute
                    + visibility
                    + 'unsafe extern "C" fn openttd_rust_test() {}'
                )
                self.assertEqual(
                    ORPHANS.violations({"rust/lib.rs": source}),
                    {
                        "no declaration | openttd_rust_test",
                        "no production user | openttd_rust_test",
                    },
                )

    def test_cli_ratchet_and_stale_entries(self) -> None:
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(ORPHANS, "ROOT", Path(directory)),
        ):
            root = Path(directory)
            for path, text in {
                "rust/lib.rs": export("orphan"),
                "src/rust/ffi.h": "void openttd_rust_orphan();\n",
                ORPHANS.BASELINE: "no production user | openttd_rust_orphan\n",
            }.items():
                target = root / path
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_text(text)
            ORPHANS.git("init", "-q")
            ORPHANS.git("add", ".")
            ORPHANS.git(
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "-qm",
                "Add: fixture",
            )

            def check() -> int:
                with (
                    patch.object(
                        sys, "argv", ["orphan-exports.py", "--base-ref", "HEAD"]
                    ),
                    contextlib.redirect_stdout(io.StringIO()),
                ):
                    return ORPHANS.main()

            self.assertEqual(check(), 0)
            (root / "rust/lib.rs").write_text(export("orphan") + export("new"))
            self.assertEqual(check(), 1)
            with (root / ORPHANS.BASELINE).open("a") as output:
                output.write(
                    "no production user | openttd_rust_new\nno declaration | openttd_rust_new\n"
                )
            self.assertEqual(check(), 1)  # Adding exemptions never fixes it.
            (root / "rust/lib.rs").write_text(export("orphan"))
            (root / "src/use.cpp").write_text("void Run() { openttd_rust_orphan(); }")
            self.assertEqual(check(), 1)  # Retired exemptions must disappear.
            (root / ORPHANS.BASELINE).write_text("")
            self.assertEqual(check(), 0)


if __name__ == "__main__":
    unittest.main()
