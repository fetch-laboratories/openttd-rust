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

    def test_original_restoration_and_fork_glue_transitions(self):
        baseline = METRICS.BASELINE
        original, glue = self.root / "src/town_gui.cpp", self.root / "src/rust_glue.cpp"
        original.unlink()
        glue.write_text("int glue = 2;\n")
        deleted = self.commit()
        keys = ("retired", "glue", "glue_removed", "net_glue")
        self.assertEqual(
            counts(METRICS.metrics(baseline, deleted), *keys), (1, 1, 0, 1)
        )
        original.write_text("int gui = 1;\n")
        restored = self.commit()
        self.assertEqual(
            counts(METRICS.metrics(deleted, restored), *keys), (-1, 0, 0, 0)
        )
        renamed = self.root / "src/renamed_glue.cpp"
        glue.rename(renamed)
        moved = self.commit()
        result = METRICS.metrics(restored, moved)
        self.assertEqual(counts(result, "glue", "glue_removed"), (0, 0))
        renamed.write_text("#ifndef WITH_RUST\nint glue = 2;\n#endif\n")
        portable = self.commit()
        keys = ("glue", "glue_removed", "net_glue")
        self.assertEqual(counts(METRICS.metrics(moved, portable), *keys), (2, 1, 1))
        self.assertEqual(counts(METRICS.metrics(portable, moved), *keys), (1, 2, -1))
        renamed.unlink()
        removed = self.commit()
        self.assertEqual(counts(METRICS.metrics(portable, removed), *keys), (0, 2, -2))

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
            counts(METRICS.metrics(METRICS.BASELINE, first), *keys), (3, 5, 0, 5)
        )
        self.assertEqual(counts(METRICS.metrics(first, second), *keys), (2, 3, 3, 0))

        # Ownership metrics equal the whole-fork delta and reverse exactly.
        before = METRICS.source_metrics(METRICS.BASELINE, first)
        after = METRICS.source_metrics(METRICS.BASELINE, second)
        result = METRICS.metrics(first, second)
        self.assertEqual(result["retired"], after["retired"] - before["retired"])
        reversed_result = METRICS.metrics(second, first)
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
            (3, 2, 4),
        )
        self.assertEqual(whole["simulation_percent"], 50)

    def test_simulation_inventory(self):
        for path in (
            "src/town_cmd.cpp",
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
