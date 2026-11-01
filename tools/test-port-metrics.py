#!/usr/bin/env python3
"""Check original retirement against conversion and ownership history."""

import contextlib
import importlib.util
import io
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

SPEC = importlib.util.spec_from_file_location(
    "port_metrics", Path(__file__).with_name("port-metrics.py")
)
METRICS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(METRICS)

ORIGINAL = "int a = 1;\nint b = 2;\n\nint c = 3;\n"


def counts(result, *keys):
    return tuple(result[key] for key in keys)


class PortMetricsTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        for patch in (
            mock.patch.object(METRICS, "ROOT", self.root),
            mock.patch.object(METRICS, "BASELINE", None),
        ):
            patch.start()
            self.addCleanup(patch.stop)
        METRICS.git("init", "-q")
        METRICS.git("config", "user.name", "Metric regression")
        METRICS.git("config", "user.email", "metrics@example.invalid")
        (self.root / "src").mkdir()
        self.write("src/town_cmd.cpp", ORIGINAL)
        self.write("src/town_gui.cpp", "int gui = 1;\n")
        METRICS.BASELINE = self.commit()

    def write(self, path, text):
        (self.root / path).write_text(text)

    def commit(self):
        METRICS.git("add", "-A")
        METRICS.git("commit", "-qm", "Change: Metric fixture")
        return METRICS.git("rev-parse", "HEAD").strip()

    def test_generated_declarations_count_as_glue_and_retire(self) -> None:
        rust = self.root / "rust/openttd-kernels/src"
        rust.mkdir(parents=True)
        (rust / "record.rs").write_text("// Fixture source.\n")
        (self.root / "tools").mkdir()
        self.write(
            "tools/generate-abi.py",
            "def shared_records(root): return []\n"
            "def generate(root):\n"
            "    return {'abi-record-probe.h': "
            "'#pragma once\\nstruct Probe {\\n    uint16_t id;\\n};\\n'}\n",
        )
        baseline = METRICS.BASELINE
        added = self.commit()
        keys = ("glue", "glue_removed", "net_glue")
        result = METRICS.metrics(baseline, added, whole_fork=True)
        self.assertEqual(counts(result, *keys), (3, 0, 3))
        self.assertEqual(result["areas"]["Boundary layer"]["glue"], 3)
        self.assertEqual(result["compiled_cpp"], 7)
        self.assertEqual(METRICS.generated_glue(added, all_lines=True), 4)
        self.assertEqual(counts(METRICS.metrics(added, added), *keys), (0, 0, 0))
        (self.root / "tools/generate-abi.py").unlink()
        removed = self.commit()
        self.assertEqual(counts(METRICS.metrics(added, removed), *keys), (0, 3, -3))

    def test_original_restoration_and_fork_glue_transitions(self):
        baseline = METRICS.BASELINE
        original, glue = self.root / "src/town_gui.cpp", self.root / "src/rust_glue.cpp"
        original.unlink()
        glue.write_text("int glue = 2;\n")
        deleted = self.commit()
        keys = ("retired", "glue", "glue_removed", "net_glue")
        self.assertEqual(
            counts(METRICS.metrics(baseline, deleted, all_lines=True), *keys),
            (1, 1, 0, 1),
        )
        original.write_text("int gui = 1;\n")
        restored = self.commit()
        self.assertEqual(
            counts(METRICS.metrics(deleted, restored, all_lines=True), *keys),
            (-1, 0, 0, 0),
        )
        renamed = self.root / "src/renamed_glue.cpp"
        glue.rename(renamed)
        moved = self.commit()
        result = METRICS.metrics(restored, moved, all_lines=True)
        self.assertEqual(counts(result, "glue", "glue_removed"), (0, 0))
        renamed.write_text("#ifndef WITH_RUST\nint glue = 2;\n#endif\n")
        portable = self.commit()
        keys = ("glue", "glue_removed", "net_glue")
        self.assertEqual(
            counts(METRICS.metrics(moved, portable, all_lines=True), *keys), (2, 1, 1)
        )
        self.assertEqual(
            counts(METRICS.metrics(portable, moved, all_lines=True), *keys), (1, 2, -1)
        )
        renamed.unlink()
        removed = self.commit()
        self.assertEqual(
            counts(METRICS.metrics(portable, removed, all_lines=True), *keys),
            (0, 2, -2),
        )

    def test_conversion_counts_original_and_removed_glue(self):
        # A first port wraps two original lines; a second replaces its glue,
        # moves `#endif` and retires the remaining original blank and line.
        self.write(
            "src/town_cmd.cpp",
            "#ifdef WITH_RUST\nrust_a();\n\n#else\n"
            "int a = 1;\nint b = 2;\n#endif\n\nint c = 3;\n",
        )
        (self.root / "src/town_gui.cpp").unlink()  # Retired, but not simulation.
        first = self.commit()
        self.write(
            "src/town_cmd.cpp",
            "#if defined(WITH_RUST)\nrust_all();\n\n#else\n"
            "int a = 1;\nint b = 2;\n\nint c = 3;\n#endif\n",
        )
        second = self.commit()
        keys = ("retired", "glue", "glue_removed", "net_glue")
        self.assertEqual(
            counts(METRICS.metrics(METRICS.BASELINE, first, all_lines=True), *keys),
            (3, 5, 0, 5),
        )
        self.assertEqual(
            counts(METRICS.metrics(first, second, all_lines=True), *keys), (2, 3, 3, 0)
        )

        # Ownership metrics equal the whole-fork delta and reverse exactly.
        before = METRICS.source_metrics(METRICS.BASELINE, first, all_lines=True)
        after = METRICS.source_metrics(METRICS.BASELINE, second, all_lines=True)
        result = METRICS.metrics(first, second, all_lines=True)
        self.assertEqual(result["retired"], after["retired"] - before["retired"])
        reversed_result = METRICS.metrics(second, first, all_lines=True)
        self.assertEqual(reversed_result["retired"], -result["retired"])
        self.assertEqual(reversed_result["net_glue"], -result["net_glue"])

        # The whole-fork share counts only simulation sources, as a percentage.
        output = io.StringIO()
        argv = ["port-metrics.py", "--whole-fork", "--head", first, "--json"]
        with mock.patch("sys.argv", argv), contextlib.redirect_stdout(output):
            METRICS.main()
        whole = json.loads(output.getvalue())
        self.assertEqual(whole["base"], METRICS.BASELINE)
        self.assertEqual(
            counts(whole, "retired", "simulation_retired", "simulation_total"),
            (3, 2, 3),
        )
        self.assertAlmostEqual(whole["simulation_percent"], 200 / 3)
        self.assertEqual(whole["all_lines"]["simulation_percent"], 50)
        self.assertEqual(whole["original_total"], 4)
        self.assertEqual(whole["whole_fork_percent"], 75)

    def test_code_comments_guards_and_area_totals(self) -> None:
        self.assertEqual(
            METRICS.code_lines(
                "// comment\n\n#if defined(WITH_RUST)\n/* start\n end */ int x; // tail\n"
                'const char *s = R"(// not comment)";\n#endif\n',
                "src/test.cpp",
            ),
            {5, 6},
        )
        self.write(
            "src/town_cmd.cpp",
            "#ifdef WITH_RUST\n// glue\nrust_call();\n#else\n" + ORIGINAL + "#endif\n",
        )
        (self.root / "rust").mkdir()
        self.write("rust/lib.rs", "// comment\n\npub fn call() {}\n")
        (self.root / "tools").mkdir()
        self.write("tools/check.py", '# comment\n"""docstring"""\nx = 1\n')
        head = self.commit()
        result = METRICS.metrics(METRICS.BASELINE, head, whole_fork=True)
        self.assertEqual(
            counts(result, "rust", "tooling", "glue", "retired", "net_glue"),
            (1, 1, 1, 3, 1),
        )
        self.assertEqual(result["original_total"], 4)
        self.assertEqual(result["compiled_cpp"], 2)
        self.assertEqual(result["compiled_percent"], 50)
        self.assertEqual(
            result["totals"],
            dict(original=4, retired=3, remaining=1, glue=1, compiled=2),
        )
        self.assertEqual(sum(row["original"] for row in result["areas"].values()), 4)
        self.assertEqual(sum(row["compiled"] for row in result["areas"].values()), 2)
        for path, expected in {
            "src/command.cpp": "Simulation logic",
            "src/map.cpp": "Simulation storage",
            "src/town.h": "Simulation storage",
            "src/rust/ffi.h": "Boundary layer",
            "src/newgrf_act0.cpp": "NewGRF",
            "src/saveload/foo.cpp": "Save/load",
            "src/script/api/foo.cpp": "Script API",
            "src/network/foo.cpp": "Network",
            "src/core/pool_type.hpp": "Support",
            "src/gfx.cpp": "Graphics and viewport",
            "src/town_gui.cpp": "GUI",
            "src/os/unix/unix.cpp": "Platform",
            "src/table/foo.h": "Data tables",
            "src/3rdparty/foo.cpp": "Vendored",
            "src/tests/foo.cpp": "Upstream tests",
        }.items():
            self.assertEqual(METRICS.area(path), expected, path)

    def test_whole_fork_total_is_independent_of_branch_delta(self) -> None:
        self.write(
            "src/town_cmd.cpp",
            "#ifdef WITH_RUST\nrust_call();\n#else\n" + ORIGINAL + "#endif\n",
        )
        head = self.commit()
        code = METRICS.metrics(head, head, whole_fork=True)
        physical = METRICS.metrics(head, head, whole_fork=True, all_lines=True)
        self.assertEqual(code["retired"], 0)
        self.assertEqual(physical["retired"], 0)
        # These totals follow directly from the four-line original fixture:
        # its three statements retire; its blank line only counts physically.
        self.assertEqual(code["whole_fork_retired"], 3)
        self.assertEqual(physical["whole_fork_retired"], 4)
        output = io.StringIO()
        with (
            mock.patch("sys.argv", ["port-metrics.py", "--whole-fork"]),
            mock.patch.object(METRICS, "metrics", side_effect=[code, physical]),
            contextlib.redirect_stdout(output),
        ):
            METRICS.main()
        self.assertIn("Code: whole-fork retired 3 / 4 (75.00%)", output.getvalue())
        self.assertIn("All lines: whole-fork retired 4 / 5 (80.00%)", output.getvalue())

    def test_functions_attribute_remaining_lines(self) -> None:
        body = (
            "Foo::Foo() : a{1} {}\n"
            "/** Ported. */\nint Ported(int x)\n{\n\tint y = x;\n\treturn y;\n}\n\n"
            "int Facade(int x)\n{\n\treturn x * 2;\n}\n\n"
            "namespace {\nint Untouched()\n{\n\treturn '}';\n}\n}\n"
            "static const int _table[] = { 1, 2 };\n"
            "Bar::Bar() :\n\ta{1}\n{\n\tRemoved();\n}\n\n"
            "int InlineClose()\n{\n\tRemoved();\n\treturn []() { return 1; }(); }\n\n"
            "int InlineOpen() { Kept();\n\tRemoved();\n}\n"
        )
        self.write("src/town_cmd.cpp", body)
        METRICS.BASELINE = self.commit()
        self.write(
            "src/town_cmd.cpp",
            body.replace("\tint y = x;\n", "#ifndef WITH_RUST\n\tint y = x;\n#endif\n")
            .replace("x * 2", "RustFacade(x)")
            .replace("Removed();", "RustBody();"),
        )
        head = self.commit()
        output = io.StringIO()
        argv = ["port-metrics.py", "--functions", "Simulation logic", "--json"]
        with (
            mock.patch("sys.argv", [*argv, "--head", head]),
            contextlib.redirect_stdout(output),
        ):
            METRICS.main()
        result = json.loads(output.getvalue())
        self.assertEqual(
            [
                (r["function"], r["line"], r["remaining"], r["facade"])
                for r in result["functions"]
            ],
            [
                ("Foo::Foo", 1, 1, False),
                ("Ported", 3, 4, False),
                ("Facade", 9, 3, True),
                ("Untouched", 15, 4, False),
                ("Bar::Bar", 21, 4, False),
                ("InlineClose", 27, 3, False),
                ("InlineOpen", 32, 2, False),
                ("(file scope)", 14, 3, False),
            ],
        )
        self.assertEqual(
            sum(row["remaining"] for row in result["functions"]),
            result["areas"]["Simulation logic"]["remaining"],
        )

    def test_function_spans_separate_arguments_and_data(self) -> None:
        for text, expected in (
            (
                "int Callback(std::span<int> regs = {})\n{\n\treturn 1;\n}\n",
                [("Callback", 1, 2, 4)],
            ),
            (
                "struct Owner {\npublic:\nOwner() : a{1} {}\n"
                "protected:\nvoid Protected() {}\nprivate:\nvoid Private() {}\n};\n",
                [("Owner", 3, 3, 3), ("Protected", 5, 5, 5), ("Private", 7, 7, 7)],
            ),
            (
                "const Descriptor desc{\n\t[]() { Kept(); },\n};\n",
                [],
            ),
            (
                'namespace n {\nextern "C" {\nint Function() {}\n}\n}\n',
                [("Function", 3, 3, 3)],
            ),
            (
                "DEF_CMD_TRAIT(CMD_FIRST, First, {})\n"
                "DEF_CMD_TRAIT(CMD_SECOND, Second, {})\n\nvoid Function() {}\n",
                [("Function", 4, 4, 4)],
            ),
        ):
            with self.subTest(text=text):
                self.assertEqual(METRICS.spans(text, "src/town_cmd.cpp"), expected)

    def test_boundary_counts_private_callbacks_and_exports(self) -> None:
        (self.root / "rust").mkdir()
        self.write(
            "rust/lib.rs",
            "\n".join(
                (
                    "#[unsafe(no_mangle)]",
                    "#[allow(unsafe_code)]",
                    'pub(crate) unsafe extern "C" fn openttd_rust_call() {}',
                    "#[no_mangle]",
                    'extern "C" fn openttd_rust_old() {}',
                    "struct Services {",
                    '    pub call: extern "C" fn(),',
                    '    private: Option<unsafe extern "C" fn()>,',
                    "}",
                    'fn helper(callback: extern "C" fn()) {}',
                    'const EXAMPLE: &str = r#"#[no_mangle] pub extern "C" fn openttd_rust_fake() {}"#;',
                )
            ),
        )
        head = self.commit()
        self.assertEqual(METRICS.boundary(head), {"exports": 2, "callback_slots": 2})

    def test_simulation_inventory(self):
        for path in (
            "src/town_cmd.cpp",
            *(
                f"src/{name}.cpp"
                for name in "command cheat goal story league signs misc_cmd vehiclelist elrail autoslope".split()
            ),
            "src/map_func.h",
            "src/openttd.cpp",
            "src/pathfinder/yapf/yapf.hpp",
            *(
                f"src/timer/timer_game_{clock}.{ext}"
                for clock in ("calendar", "economy", "common")
                for ext in ("cpp", "h")
            ),
        ):
            self.assertTrue(METRICS.simulation_source(path), path)
        for path in (
            "src/town_gui.cpp",
            "src/linkgraph/linkgraph_gui.cpp",
            "src/linkgraph/linkgraph_gui.h",
            "src/3rdparty/foo.cpp",
            "src/os/unix/unix.cpp",
            "src/core/pool_type.hpp",
            "src/timer/timer.h",
            "src/timer/timer_manager.h",
            "src/timer/timer_game_realtime.cpp",
            "src/timer/timer_game_tick.h",
        ):
            self.assertFalse(METRICS.simulation_source(path), path)


if __name__ == "__main__":
    unittest.main()
