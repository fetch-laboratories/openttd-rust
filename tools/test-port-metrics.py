#!/usr/bin/env python3
"""Check original retirement against conversion and ownership history."""

import contextlib
import importlib.util
import io
import json
import tempfile
import unittest
from pathlib import Path
from typing import Any
from unittest import mock

import code_lines

SPEC = importlib.util.spec_from_file_location(
    "port_metrics", Path(__file__).with_name("port-metrics.py")
)
assert SPEC and SPEC.loader
# Tests rebind BASELINE, which ModuleType does not declare.
METRICS: Any = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(METRICS)

ORIGINAL = "int a = 1;\nint b = 2;\n\nint c = 3;\n"


def counts(result: dict[str, int], *keys: str) -> tuple[int, ...]:
    return tuple(result[key] for key in keys)


class PortMetricsTests(unittest.TestCase):
    def setUp(self) -> None:
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

    def write(self, path: str, text: str) -> None:
        (self.root / path).write_text(text)

    def commit(self) -> str:
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
        generator = self.root / "tools/generate-abi.py"
        generator.write_text(generator.read_text().replace("uint16_t id", "int16_t id"))
        replaced = self.commit()
        # Equal-sized declarations can still replace a field's type. Report
        # both sides of that change, rather than just the zero signed net.
        for all_lines in (False, True):
            result = METRICS.metrics(
                added, replaced, all_lines=all_lines, whole_fork=True
            )
            self.assertEqual(
                counts(result, *keys),
                (1, 1, 0),
            )
            self.assertEqual(
                result["areas"]["Boundary layer"]["glue"], 4 if all_lines else 3
            )
        (self.root / "tools/generate-abi.py").unlink()
        removed = self.commit()
        self.assertEqual(counts(METRICS.metrics(added, removed), *keys), (0, 3, -3))

    def test_evidence_inventory_uses_archived_inputs_and_keeps_area_series(
        self,
    ) -> None:
        for name in ("rust/openttd-kernels/src", "src/rust", "tools/migration"):
            (self.root / name).mkdir(parents=True, exist_ok=True)
        self.write("rust/openttd-kernels/src/record.rs", "// record input\n")
        self.write("src/rust/service.hpp", "int first;\n")
        self.write("tools/migration/shared.cpp", "void openttd_rust_probe();\n")
        self.write("tools/migration/crypto-vectors.cpp", "int oracle_only;\n")
        self.write("tools/migration/unknown.inc", "int unknown_dependency;\n")
        self.write(
            "tools/generate-abi.py",
            "def generate(root):\n"
            "    service = (root / 'src/rust/service.hpp').read_text()\n"
            "    return {'abi-record-probe.h': 'struct Probe {};\\n',\n"
            "            'abi-visit-Probe.hpp': service,\n"
            "            'abi-layout-probe.inc': 'Layout(Probe);\\n'}\n",
        )
        first = self.commit()
        self.write("src/rust/service.hpp", "int second;\nint third;\n")
        second = self.commit()
        # Dirty working inputs must never affect either archived endpoint.
        self.write("src/rust/service.hpp", "broken working input\n" * 10)
        before = METRICS.metrics(METRICS.BASELINE, first, whole_fork=True)
        after = METRICS.metrics(METRICS.BASELINE, second, whole_fork=True)
        evidence = before["evidence_cpp"]
        self.assertEqual(evidence["tracked"], 3)
        self.assertEqual(evidence["generated"], 2)
        self.assertEqual(evidence["candidate_shared"], 3)
        self.assertEqual(evidence["reference_only"], 1)
        self.assertEqual(evidence["unresolved"], 1)
        self.assertEqual(after["evidence_cpp"]["generated"], 3)
        self.assertEqual(before["original_total"], 4)
        self.assertEqual(before["compiled_cpp"], 6)  # Four original + header + record.
        self.assertEqual(sum(r["compiled"] for r in before["areas"].values()), 6)
        self.assertEqual(before["fork_cpp"], 11)
        self.assertEqual(before["candidate_cpp_upper_bound"], 10)
        self.assertEqual(len({r["path"] for r in evidence["files"]}), 5)
        self.assertFalse(any("abi-record" in r["path"] for r in evidence["files"]))

    def test_original_restoration_and_fork_glue_transitions(self) -> None:
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
        renamed = self.root / "src/renamed_glue.inc"
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

    def test_conversion_counts_original_and_removed_glue(self) -> None:
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

    def test_embedded_cpp_counts_strings_recipes_and_extraction_sites(self) -> None:
        (self.root / "tools/simulation").mkdir(parents=True)
        (self.root / "tools/oracle-recipes").mkdir()
        self.write(
            "tools/simulation/stations.py",
            'COMMAND_SOURCE = r"""\nint a = 1;\n// note\nint b = 2;\n"""\n'
            'OTHER = "int c = 3;\\n"\n'
            "x = probe.body(text, 'void F(') + probe.body(text, 'void G(')\n",
        )
        self.write("tools/test-stations.py", 'EXAMPLE_SOURCE = "int d = 4;\\n"\n')
        self.write(
            "tools/oracle-recipes/case.patch",
            "--- a/capture.in\n+++ b/capture.in\n@@ -1 +1,2 @@\n int e;\n+int f = 5;\n",
        )
        head = self.commit()
        self.assertEqual(
            METRICS.embedded_cpp(head),
            {
                "strings": {"tools/simulation/stations.py:COMMAND_SOURCE": 2},
                "recipes": {"tools/oracle-recipes/case.patch": 1},
                "extraction_sites": {"tools/simulation/stations.py": 2},
            },
        )
        self.assertEqual(
            METRICS.embedded_cpp(head, all_lines=True)["strings"],
            {"tools/simulation/stations.py:COMMAND_SOURCE": 4},
        )

    def test_simulation_inventory(self) -> None:
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


class CodeLineStoreTests(unittest.TestCase):
    """A stored result is exactly the lexed one, and no other text reaches it."""

    SOURCE = "// comment\n\nint x; /* tail */\n#if A\nint y;\n#endif\n"

    def setUp(self) -> None:
        self.lines = code_lines
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.directory = Path(directory.name)
        self.forget()
        self.addCleanup(self.forget)

    def forget(self) -> None:
        """Drop this process's results and its store."""
        self.lines._code_lines.cache_clear()
        self.lines._without_comments.cache_clear()
        self.lines.STORE = None

    def fresh(self) -> code_lines.Store:
        """Start as a new run would: nothing in memory, the store read from disk."""
        self.forget()
        store = self.lines.Store(self.directory)
        self.lines.STORE = store
        return store

    def test_a_stored_result_equals_the_lexed_one_in_a_later_run(self) -> None:
        lexed = self.lines.code_lines(self.SOURCE, "a.cpp")
        self.assertEqual(lexed, {3, 5})
        store = self.fresh()
        self.assertEqual(self.lines.code_lines(self.SOURCE, "a.cpp"), lexed)
        store.save()
        (stored,) = self.directory.iterdir()
        self.assertEqual(list(json.loads(stored.read_text()).values()), [[3, 5]])
        # A later run reads the result and lexes nothing.
        store = self.fresh()
        with mock.patch.object(self.lines, "lexed", side_effect=AssertionError):
            self.assertEqual(self.lines.code_lines(self.SOURCE, "b.cpp"), lexed)
        self.assertEqual(store.added, {})

    def test_another_text_or_lexer_never_reaches_a_stored_result(self) -> None:
        store = self.fresh()
        self.lines.code_lines(self.SOURCE, "a.cpp")
        store.save()
        store = self.fresh()
        # The same text is Rust under another lexer, where `#if A` is code.
        self.assertEqual(self.lines.code_lines(self.SOURCE, "a.rs"), {3, 4, 5, 6})
        self.assertEqual(
            self.lines.code_lines(self.SOURCE + "int z;\n", "a.cpp"), {3, 5, 7}
        )
        self.assertEqual(len(store.added), 2)

    def test_runs_merge_and_a_damaged_store_is_recomputed(self) -> None:
        first, second = (self.lines.Store(self.directory) for _ in range(2))
        first.added, second.added = {"one": [1]}, {"two": [2]}
        first.save()
        second.save()
        self.assertEqual(
            self.lines.Store(self.directory).entries, {"one": [1], "two": [2]}
        )
        first.path.write_text("{ damaged")
        self.fresh()
        self.assertEqual(self.lines.code_lines(self.SOURCE, "a.cpp"), {3, 5})

    def test_a_store_of_another_shape_is_ignored(self) -> None:
        store = self.fresh()
        for other in (
            '["list"]',
            '{"key": "text"}',
            '{"key": [1, "2"]}',
            '{"key": [true]}',
        ):
            store.path.write_text(other)
            self.assertEqual(self.lines.Store(self.directory).entries, {}, other)

    def test_an_unwritable_store_costs_no_result(self) -> None:
        occupied = self.directory / "file"
        occupied.write_text("")
        self.forget()
        store = self.lines.Store(occupied / "below")
        self.lines.STORE = store
        self.assertEqual(self.lines.code_lines(self.SOURCE, "a.cpp"), {3, 5})
        store.save()
        self.assertEqual(store.added, {})
        self.assertEqual(sorted(self.directory.iterdir()), [occupied])

    def test_text_that_utf8_cannot_encode_is_counted_with_and_without_a_store(
        self,
    ) -> None:
        text = "int x; // \udcff\n"
        self.assertEqual(self.lines.code_lines(text, "a.cpp"), {1})
        self.fresh()
        self.assertEqual(self.lines.code_lines(text, "a.cpp"), {1})

    def test_a_module_or_pygments_change_starts_an_empty_store(self) -> None:
        name = self.lines.Store(self.directory).path.name
        with mock.patch.object(self.lines.pygments, "__version__", "0"):
            self.assertNotEqual(self.lines.Store(self.directory).path.name, name)


if __name__ == "__main__":
    unittest.main()
