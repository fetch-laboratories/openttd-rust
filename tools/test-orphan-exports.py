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

from source_fixture import initialize

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

    def test_instantiation_only_member_is_no_production_user(self) -> None:
        files = {
            "rust/lib.rs": export("reassign"),
            "src/rust/ffi.h": "uint32_t openttd_rust_reassign(List *, uint8_t, uint8_t);\n",
            "src/cargopacket.h": "class VehicleCargoList {\n"
            "\ttemplate <MoveToAction Tfrom, MoveToAction Tto> uint Reassign(uint amount);\n"
            "};\n",
            "src/rust/storage.hpp": "template <VehicleCargoList::MoveToAction Tfrom, "
            "VehicleCargoList::MoveToAction Tto>\n"
            "uint VehicleCargoList::Reassign(uint amount) "
            "{ return openttd_rust_reassign(this->state, Tfrom, Tto); }\n"
            "template uint VehicleCargoList::Reassign<VehicleCargoList::MTA_DELIVER, "
            "VehicleCargoList::MTA_KEEP>(uint);\n"
            "template uint VehicleCargoList::Reassign<VehicleCargoList::MTA_LOAD, "
            "VehicleCargoList::MTA_KEEP>(uint);\n",
            "src/tests/cargo.cpp": "void Test(VehicleCargoList &l) "
            "{ l.Reassign<VehicleCargoList::MTA_LOAD, VehicleCargoList::MTA_KEEP>(1); }\n",
        }
        self.assertEqual(
            ORPHANS.violations(files), {"no production user | openttd_rust_reassign"}
        )
        files["src/economy.cpp"] = (
            "void Load(VehicleCargoList &l) "
            "{ l.Reassign<VehicleCargoList::MTA_LOAD, VehicleCargoList::MTA_KEEP>(1); }\n"
        )
        self.assertEqual(ORPHANS.violations(files), set())

    def test_uncalled_plain_member_is_reported_not_enforced(self) -> None:
        files = {
            "rust/lib.rs": export("age"),
            "src/rust/ffi.h": "void openttd_rust_age(List *);\n",
            "src/cargopacket.h": "class VehicleCargoList {\n\tvoid AgeCargo();\n};\n",
            "src/rust/storage.hpp": "void VehicleCargoList::AgeCargo() "
            "{ openttd_rust_age(this->state); }\n",
            "src/script/api/script_list.cpp": "void ScriptList::Bound() "
            "{ openttd_rust_age(nullptr); }\n",
        }
        self.assertEqual(ORPHANS.violations(files), set())
        self.assertEqual(ORPHANS.violations(files, broad=True), set())
        del files["src/script/api/script_list.cpp"]  # Generated bindings call these.
        self.assertEqual(ORPHANS.violations(files), set())
        self.assertEqual(
            ORPHANS.scan(files, broad=True),
            {"no production user | openttd_rust_age": {"VehicleCargoList::AgeCargo"}},
        )
        files["src/vehicle.cpp"] = "void Tick(Vehicle *v) { v->cargo.AgeCargo(); }\n"
        self.assertEqual(ORPHANS.violations(files, broad=True), set())

    def test_cli_ratchet_and_stale_entries(self) -> None:
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(ORPHANS, "ROOT", Path(directory)),
        ):
            root = Path(directory)
            initialize(
                root,
                {
                    "rust/lib.rs": export("orphan"),
                    "src/rust/ffi.h": "void openttd_rust_orphan();\n",
                    ORPHANS.BASELINE: "no production user | openttd_rust_orphan\n",
                },
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
            (root / "src/use.inc").write_text("void Run() { openttd_rust_orphan(); }")
            self.assertEqual(check(), 1)  # Retired exemptions must disappear.
            (root / ORPHANS.BASELINE).write_text("")
            self.assertEqual(check(), 0)

    def test_uncalled_baseline_only_shrinks_and_names_owners(self) -> None:
        """#737 batch O: --report-uncalled is enforced against an owned baseline."""
        found = {"openttd_rust_age": "VehicleCargoList::AgeCargo"}
        row = "openttd_rust_age | #656 vehicle pool: fleet-console.cpp calls AgeCargo\n"
        self.assertEqual(ORPHANS.uncalled_errors(found, row, row), [])
        self.assertEqual(
            ORPHANS.uncalled_errors(found, "", ""),
            [
                "new export held only by uncalled members: openttd_rust_age (VehicleCargoList::AgeCargo)"
            ],
        )
        self.assertEqual(
            ORPHANS.uncalled_errors({}, row, row),
            ["remove retired row: openttd_rust_age"],
        )
        self.assertEqual(
            ORPHANS.uncalled_errors(found, row, ""),
            ["uncalled baseline cannot grow: openttd_rust_age"],
        )
        unowned = "openttd_rust_age | fleet-console.cpp calls AgeCargo\n"
        self.assertEqual(
            ORPHANS.uncalled_errors(found, unowned, unowned),
            ["row names no owner issue: openttd_rust_age"],
        )

    def test_cli_enforces_the_uncalled_baseline(self) -> None:
        files = {
            "rust/lib.rs": export("age"),
            "src/rust/ffi.h": "void openttd_rust_age(List *);\n",
            "src/cargopacket.h": "class VehicleCargoList {\n\tvoid AgeCargo();\n};\n",
            "src/rust/storage.hpp": "void VehicleCargoList::AgeCargo() "
            "{ openttd_rust_age(this->state); }\n",
            ORPHANS.BASELINE: "",
            ORPHANS.UNCALLED: "",
        }
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(ORPHANS, "ROOT", Path(directory)),
        ):
            root = Path(directory)
            initialize(root, files)

            def check() -> int:
                with (
                    patch.object(
                        sys, "argv", ["orphan-exports.py", "--base-ref", "HEAD"]
                    ),
                    contextlib.redirect_stdout(io.StringIO()),
                ):
                    return ORPHANS.main()

            self.assertEqual(check(), 1)
            (root / ORPHANS.UNCALLED).write_text(
                "openttd_rust_age | #656 vehicle pool: fleet-console.cpp\n"
            )
            self.assertEqual(check(), 1)  # A new row is growth against the base.

    def test_help_and_unknown_options_run_no_check(self) -> None:
        for argv, code in ((["--help"], 0), (["-h"], 0), (["--bsae-ref", "X"], 2)):
            output = io.StringIO()
            with (
                patch.object(sys, "argv", ["orphan-exports.py", *argv]),
                patch.object(ORPHANS, "check_uncalled") as check,
                patch.object(ORPHANS, "report_uncalled") as report,
                patch.object(ORPHANS.baseline_ratchet, "main") as ratchet,
                contextlib.redirect_stdout(output),
                contextlib.redirect_stderr(output),
                self.assertRaises(SystemExit) as raised,
            ):
                ORPHANS.main()
            self.assertEqual(raised.exception.code, code, argv)
            for checker in (check, report, ratchet):
                checker.assert_not_called()
        self.assertIn("--report-uncalled", output.getvalue())

    def test_options_reach_their_checks(self) -> None:
        with (
            patch.object(sys, "argv", ["orphan-exports.py", "--report-uncalled"]),
            patch.object(ORPHANS, "check_uncalled") as check,
            patch.object(ORPHANS, "report_uncalled", return_value=0) as report,
        ):
            self.assertEqual(ORPHANS.main(), 0)
        report.assert_called_once_with()
        check.assert_not_called()
        with (
            patch.object(sys, "argv", ["orphan-exports.py", "--base-ref", "base"]),
            patch.object(ORPHANS.vcs, "merge_base", return_value="fork") as fork,
            patch.object(ORPHANS, "check_uncalled", return_value=0) as check,
            patch.object(ORPHANS.baseline_ratchet, "main", return_value=1) as ratchet,
        ):
            self.assertEqual(ORPHANS.main(), 1)
        fork.assert_called_once()
        check.assert_called_once_with("fork")
        ratchet.assert_called_once()


if __name__ == "__main__":
    unittest.main()
