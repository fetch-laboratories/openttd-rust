#!/usr/bin/env python3
"""Exercise each harness inventory guard on the tree and on constructed losses."""

import contextlib
import copy
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import evidence_replacements as er
import harness_inventory as hi

ROOT = Path(__file__).resolve().parents[1]
RECORDED = hi.load((ROOT / hi.INVENTORY).read_text())
TREE, UNREAD = hi.tree_inventory()


def open_issues(number: int) -> str:
    return "open" if number == 1 else "closed"


def retired_files_absent() -> "contextlib.AbstractContextManager[object]":
    """Check retirements against an empty root, as after the files are deleted."""
    stack = contextlib.ExitStack()
    directory = stack.enter_context(tempfile.TemporaryDirectory())
    stack.enter_context(patch.object(er, "ROOT", Path(directory)))
    return stack


class InventoryTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tree, self.recorded = copy.deepcopy(TREE), copy.deepcopy(RECORDED)
        self.tree["comparisons"] = copy.deepcopy(RECORDED["comparisons"])

    def errors(self, base: hi.Inventory | None = None) -> list[str]:
        expected = hi.registered_inventory(self.recorded, self.tree)
        return hi.errors(self.tree, expected, base or RECORDED, open_issues)

    def test_unchanged_tree_matches_its_inventory(self) -> None:
        self.assertEqual(self.errors(), [])
        self.assertEqual(UNREAD, [])
        self.assertEqual(hi.removals(self.recorded, RECORDED), [])
        command = [sys.executable, str(ROOT / "tools/harness_inventory.py")]
        subprocess.run([*command, "--base-ref", "HEAD"], check=True, cwd=ROOT)

    def test_removed_scenario_or_driver_fails_until_recorded(self) -> None:
        for key in ("default", "soak"):
            self.setUp()
            self.tree[key].pop()
            self.assertRegex(" ".join(self.errors()), f"{key} .* only in the inventory")
            self.recorded[key].pop()
            self.assertEqual(self.errors(), [])
            self.assertEqual(len(hi.removals(self.recorded, RECORDED)), 1)
        del self.tree["comparisons"]["widget-parser-comparison.py"]
        self.assertIn("comparison drivers differ", " ".join(self.errors()))
        del self.recorded["comparisons"]["widget-parser-comparison.py"]
        self.assertEqual(self.errors(), [])
        removed = hi.removals(self.recorded, RECORDED)
        self.assertIn("comparison widget-parser-comparison.py", removed)

    def test_zero_missing_or_fewer_cases_fail(self) -> None:
        recorded = {"a.py": ["A passed: 27 O0 records, 3 cases"]}
        for printed, message in (
            ([], "no case inventory"),
            (["A passed: 0 O0 records, 3 cases"], "zero count"),
            (["A passed: 26 O0 records, 3 cases"], "fewer cases"),
            (["A other: 27 O0 records, 3 cases"], "fewer cases"),
        ):
            errors = hi.case_errors(recorded, {"a.py": printed})
            self.assertRegex(" ".join(errors), message)
        more = {"a.py": ["A passed: 28 O0 records, 3 cases"]}
        self.assertEqual(hi.case_errors(recorded, more), [])
        self.recorded["comparisons"]["widget-parser-comparison.py"] = [
            "M passed: 0 lines"
        ]
        self.tree["comparisons"] = self.recorded["comparisons"]
        self.assertRegex(" ".join(self.errors()), "zero count")

    def test_masks_cannot_grow_or_widen_but_can_shrink(self) -> None:
        widened = [mask.replace("revision.text", "*") for mask in RECORDED["masks"]]
        for masks in ([*RECORDED["masks"], "MAPS:*"], widened):
            self.tree["masks"] = self.recorded["masks"] = sorted(masks)
            self.assertRegex(" ".join(self.errors()), "mask added or widened")
        self.tree["masks"] = self.recorded["masks"] = RECORDED["masks"][1:]
        self.assertEqual(self.errors(), [])
        self.recorded["masks"] = RECORDED["masks"]
        self.assertIn("only in the inventory", " ".join(self.errors()))

    def test_record_rejects_empty_or_zero_runs_before_writing(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            tools = root / "tools"
            tools.mkdir()
            for name in (
                "run-comparisons.py",
                "harness_inventory.py",
                "evidence_replacements.py",
                "vcs.py",
                "validation_execution.py",
            ):
                shutil.copyfile(ROOT / "tools" / name, tools / name)
            inventory = root / hi.INVENTORY
            inventory.parent.mkdir()
            driver = "fixture-comparison.py"
            recorded = copy.deepcopy(RECORDED)
            recorded["comparisons"] = {driver: ["Fixture passed: 3 cases"]}
            original = json.dumps(recorded)
            for line, code in (
                ("", 1),
                ("Fixture passed: 0 cases", 1),
                ("Fixture passed: 2 cases", 1),
                ("Fixture passed: 4 cases", 0),
            ):
                with self.subTest(line=line):
                    inventory.write_text(original)
                    (tools / driver).write_text(
                        "print('test result: ok. 1 passed; 0 failed; 0 ignored; "
                        "0 measured; 108 filtered out; finished in 0.00s')\n"
                        f"print({line!r})\n"
                    )
                    run = subprocess.run(
                        [sys.executable, str(tools / "run-comparisons.py"), "--record"],
                        cwd=root,
                        capture_output=True,
                        text=True,
                    )
                    self.assertEqual(run.returncode, code, run.stdout + run.stderr)
                    if code:
                        self.assertEqual(inventory.read_text(), original)
                    else:
                        self.assertEqual(
                            hi.load(inventory.read_text())["comparisons"],
                            {driver: [line]},
                        )

    def test_new_known_failure_names_an_open_issue(self) -> None:
        for issue, message in (("#1", None), ("#2", "open issue"), ("x", "open")):
            failures = {"play:VEHS:x": issue}
            self.tree["known_failures"] = self.recorded["known_failures"] = failures
            found = " ".join(self.errors())
            self.assertRegex(found, message) if message else self.assertEqual(found, "")
        self.recorded["known_failures"] = {}
        self.assertIn("KNOWN_FAILURES differs", " ".join(self.errors()))

    def test_scenario_key_no_tool_reads_fails(self) -> None:
        scenario = {"name": "a", "check_signs": True, "chekc_signs": True}
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "signs.py"
            source.write_text(
                'SCENARIOS = [{"name": "a", "chekc_signs": True}]\n'
                'def check(s):\n    return s["name"], s.get("check_signs")\n'
            )
            self.assertEqual(hi.unread_keys([scenario], [source]), ["chekc_signs"])


class ReplacementTests(unittest.TestCase):
    # This existing production test checks the unchanged Monocypher vectors,
    # rather than a test invented to echo the replacement manifest.
    RUST = "rust:lib:crypto::tests::monocypher_known_answers"

    def record(self) -> er.Replacement:
        return {
            "retires": [
                "source:src/tests/test_network_crypto.cpp",
                "ctest:crypto vectors",
            ],
            "oracle": "Pinned original Monocypher known-answer vectors.",
            "observations": {"crypto known answers": [self.RUST]},
        }

    def test_reference_names_require_registered_nonempty_replacements(self) -> None:
        self.enterContext(retired_files_absent())
        record = self.record()
        reference = {"crypto vectors", "retained original"}
        candidate = {"retained original", er.ctest_name(self.RUST)}
        self.assertEqual(er.errors([record], TREE["default"]), [])
        self.assertEqual(er.ctest_errors(reference, candidate, [record]), [])
        for records, names in (([], candidate), ([record], {"retained original"})):
            self.assertIn(
                "without executable replacement",
                " ".join(er.ctest_errors(reference, names, records)),
            )
        record["observations"] = {
            "crypto known answers": ["scenario:" + TREE["default"][0]]
        }
        self.assertIn(
            "require Rust CTest", " ".join(er.errors([record], TREE["default"]))
        )
        self.assertTrue(er.ctest_errors(reference, candidate, [record]))

    def test_source_target_and_driver_loss_cannot_be_recorded_away(self) -> None:
        for key, identity in (
            ("native_sources", "source"),
            ("native_tests", "source"),
            ("native_targets", "target"),
            ("comparisons", "comparison"),
            ("default", "scenario"),
        ):
            tree = copy.deepcopy(TREE)
            entries = tree[key]
            name = next(iter(entries))
            if isinstance(entries, dict):
                del entries[name]
            else:
                entries.remove(name)
            self.assertIn(
                f"missing replacement contract: {identity}:{name}",
                hi.replacement_errors(tree, TREE),
            )

    def test_manifest_accounts_for_retirement_without_shared_inventory_edits(
        self,
    ) -> None:
        self.enterContext(retired_files_absent())
        record = self.record()
        tree = copy.deepcopy(TREE)
        archived = copy.deepcopy(RECORDED)
        # Keep a legitimate earlier retirement while testing the next removal.
        prior_source = "tools/simulation/already-retired.cpp"
        archived["native_sources"].append(prior_source)
        prior = dict(self.record(), retires=["source:" + prior_source])
        registered = [*er.load(), prior]
        source = "src/tests/test_network_crypto.cpp"
        tree["native_sources"].remove(source)
        tree["native_tests"].remove(source)
        with patch.object(er, "load", return_value=[*registered, record]):
            expected = hi.registered_inventory(archived, tree)
            self.assertEqual(hi.errors(tree, expected, archived), [])
            self.assertEqual(hi.replacement_errors(tree, archived), [])
        with patch.object(er, "load", return_value=registered):
            expected = hi.registered_inventory(archived, tree)
            self.assertIn(
                "native_sources differs", " ".join(hi.errors(tree, expected, archived))
            )
            self.assertIn(
                f"missing replacement contract: source:{source}",
                hi.replacement_errors(tree, archived),
            )
        self.assertIn(prior_source, archived["native_sources"])
        self.assertIn(source, RECORDED["native_sources"])

    def test_empty_manifests_and_observations_fail(self) -> None:
        record = self.record()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / "migration/evidence-replacements/crypto.json"
            path.parent.mkdir(parents=True)
            for value in (
                [],
                [{}],
                [{**record, "observations": {}}],
                [{**record, "observations": {"crypto": []}}],
                [{**record, "oracle": ""}],
            ):
                path.write_text(json.dumps(value))
                with self.assertRaisesRegex(ValueError, "empty|incomplete"):
                    er.load(root)
            path.write_text(json.dumps([record]))
            self.assertEqual(er.load(root), [record])

    def test_linux_only_records_register_rust_checks_only_on_linux(self) -> None:
        record = self.record()
        linux: er.Replacement = {**record, "platform": "linux"}
        text = er.cmake([linux])
        guarded = text.split('if(CMAKE_SYSTEM_NAME STREQUAL "Linux")\n', 1)
        self.assertEqual(len(guarded), 2)
        self.assertIn(er.ctest_name(self.RUST), guarded[1].split("endif()", 1)[0])
        # A check another record runs everywhere stays unconditional.
        self.assertNotIn("if(", er.cmake([linux, record]))
        wide: er.Replacement = {**record, "platform": "64-bit"}
        self.assertIn("if(CMAKE_SIZEOF_VOID_P EQUAL 8)\n", er.cmake([wide]))
        # Records naming one check under different platforms leave it unconditional.
        self.assertNotIn("if(", er.cmake([linux, wide]))
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / "migration/evidence-replacements/crypto.json"
            path.parent.mkdir(parents=True)
            path.write_text(json.dumps([{**record, "platform": "windows"}]))
            with self.assertRaisesRegex(ValueError, "incomplete"):
                er.load(root)
            path.write_text(json.dumps([linux]))
            self.assertEqual(er.load(root), [linux])

    def test_destinations_must_be_default_scenarios_or_real_rust_targets(self) -> None:
        for check in (
            "scenario:missing",
            "scenario:" + TREE["soak"][0],
            "rust:missing:case",
            "rust:lib:",
            "rust:lib:--list",
        ):
            record = self.record()
            record["observations"] = {"crypto known answers": [check]}
            self.assertTrue(er.errors([record], TREE["default"]), check)
        record = self.record()
        record["retires"] = ["source:tools/migration/crypto-vectors.cpp"]
        record["observations"] = {
            "crypto known answers": ["scenario:" + TREE["default"][0]]
        }
        self.assertEqual(er.errors([record], TREE["default"]), [])

    def test_retired_source_must_be_deleted(self) -> None:
        # A partial transfer that names a kept file as source: would make CMake
        # drop the file and silently stop building its remaining cases (#736).
        record = self.record()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            kept = root / "src/tests/test_network_crypto.cpp"
            kept.parent.mkdir(parents=True)
            kept.touch()
            self.assertIn(
                "retired source still exists: source:src/tests/test_network_crypto.cpp",
                er.errors([record], TREE["default"], root),
            )
            partial = self.record()
            partial["retires"] = ["ctest:crypto vectors"]
            self.assertEqual(er.errors([partial], TREE["default"], root), [])
            kept.unlink()
            self.assertEqual(er.errors([record], TREE["default"], root), [])

    def test_cmake_registers_rust_and_filters_only_retired_native_source(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "src/tests").mkdir(parents=True)
            # Use the actual common CMake helper and existing native fixture names.
            # The retired file is deleted but still listed; only the filter drops it.
            (root / "src/tests/history_func.cpp").touch()
            tools = root / "tools"
            tools.mkdir()
            shutil.copyfile(
                ROOT / "tools/evidence_replacements.py", tools / "register.py"
            )
            # Configure hosts such as Emscripten cannot import the simulation
            # runtime. Exercise the actual registration entry under that limit.
            (tools / "simulation.py").write_text(
                "raise ImportError('simulation runtime unavailable at configure')\n"
            )
            manifest = root / "migration/evidence-replacements/crypto.json"
            manifest.parent.mkdir(parents=True)
            record = self.record()
            record["observations"]["crypto known answers"].append("scenario:missing")
            manifest.write_text(json.dumps([record]))
            subprocess.run(
                [
                    sys.executable,
                    str(tools / "register.py"),
                    "--cmake",
                    str(root / "replacements.cmake"),
                ],
                check=True,
                capture_output=True,
            )
            self.assertIn(
                "not a default harness scenario",
                " ".join(er.errors([record], TREE["default"])),
            )
            (root / "src/tests/CMakeLists.txt").write_text(
                "add_test_files(history_func.cpp test_network_crypto.cpp)\n"
            )
            (root / "CMakeLists.txt").write_text(
                "cmake_minimum_required(VERSION 3.17)\nproject(registration NONE)\n"
                'enable_testing()\nset(CARGO_EXECUTABLE cargo)\nset(Python3_EXECUTABLE "'
                + sys.executable
                + '")\n'
                'include("' + str(ROOT / "cmake/SourceList.cmake") + '")\n'
                "include(replacements.cmake)\nadd_custom_target(openttd_test)\n"
                "add_subdirectory(src/tests)\n"
                "get_target_property(sources openttd_test SOURCES)\n"
                'file(WRITE "${CMAKE_BINARY_DIR}/sources.txt" "${sources}")\n'
            )
            build = root / "build"
            subprocess.run(
                ["cmake", "-S", str(root), "-B", str(build)],
                check=True,
                capture_output=True,
            )
            sources = (build / "sources.txt").read_text().split(";")
            self.assertEqual(sources, [str(root / "src/tests/history_func.cpp")])
            inventory = json.loads(
                subprocess.check_output(
                    ["ctest", "--show-only=json-v1"], cwd=build, text=True
                )
            )
            self.assertEqual(
                [test["name"] for test in inventory["tests"]],
                [er.ctest_name(self.RUST)],
            )
            self.assertEqual(
                inventory["tests"][0]["command"][-2:], ["--run-rust", self.RUST]
            )


if __name__ == "__main__":
    unittest.main()
