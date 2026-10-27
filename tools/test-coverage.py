#!/usr/bin/env python3
"""Check coverage merging and scenario attribution against small independent fixtures."""

import contextlib
import io
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from simulation import coverage


class CoverageTests(unittest.TestCase):
    def test_merge_original_functions_and_report(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "original.cpp"
            source.write_text(
                "int never(int n) { return n ? 10 : 20; }\n"
                "int value(int n) { if (n) return 1; return 0; }\n"
                "int main(int argc, char **) { (void)value(argc - 1); }\n"
            )
            binary = root / "game"
            subprocess.run(
                ["g++", "--coverage", "-O0", str(source), "-o", str(binary)],
                cwd=root,
                check=True,
            )
            env = dict(os.environ, GCOV_PREFIX_STRIP="0", GCOV_EXIT_AT_ERROR="1")
            traces: dict[str, coverage.Trace] = {}
            for name, arguments in (("false", []), ("true", ["x"])):
                prefix = root / name
                subprocess.run(
                    [str(binary), *arguments],
                    cwd=root,
                    env=dict(env, GCOV_PREFIX=str(prefix / "process")),
                    check=True,
                )
                with (
                    patch.object(coverage.migration, "REFERENCE", root),
                    contextlib.redirect_stdout(io.StringIO()),
                ):
                    coverage.collect(
                        prefix,
                        root / f"{name}.json",
                        root,
                        coverage.source_filters(["original.cpp"], root),
                        env,
                    )
                traces[name] = coverage.read_trace(root / f"{name}.json")
            with contextlib.redirect_stdout(io.StringIO()):
                coverage.checked(
                    [
                        *coverage.GCOVR,
                        "--root",
                        str(root),
                        "--json-add-tracefile",
                        str(root / "false.json"),
                        "--json-add-tracefile",
                        str(root / "true.json"),
                        "--json",
                        str(root / "merged.json"),
                    ],
                    root / "merge.log",
                    env,
                )
            merged = json.loads((root / "merged.json").read_text())
            result = coverage.summarize(merged, traces.items())
            self.assertEqual(
                result["totals"],
                dict(lines=3, lines_covered=2, branches=4, branches_covered=2),
            )
            functions = {
                f.get("demangled_name", f.get("name")): f
                for f in result["files"][0]["functions"]
            }
            self.assertEqual(functions["never(int)"]["execution_count"], 0)
            self.assertEqual(functions["value(int)"]["execution_count"], 2)
            self.assertEqual(functions["value(int)"]["branches_covered"], 2)
            self.assertEqual(result["unique_lines"], {"false": [], "true": []})
            self.assertIn("2 of 3 (66.67%)", coverage.markdown(result))
            self.assertIn("2 of 4 (50.00%)", coverage.markdown(result))
            self.assertIn("`original.cpp`", coverage.markdown(result))

    def test_unique_physical_lines_ignore_function_instances(self) -> None:
        def trace(points: list[tuple[str, int, str]]) -> coverage.Trace:
            return {
                "files": [
                    {
                        "file": file,
                        "lines": [
                            {
                                "line_number": line,
                                "count": 1,
                                "branches": [],
                                "function_name": function,
                            }
                            for source, line, function in points
                            if source == file
                        ],
                        "functions": [],
                    }
                    for file in sorted({p[0] for p in points})
                ]
            }

        first = trace(
            [("a.cpp", 1, "f<int>"), ("a.cpp", 1, "f<char>"), ("a.cpp", 2, "g")]
        )
        second = trace([("a.cpp", 2, "g"), ("b.cpp", 1, "h")])
        result = coverage.summarize(
            trace([("a.cpp", 1, "f<int>"), ("a.cpp", 2, "g"), ("b.cpp", 1, "h")]),
            iter((("one", first), ("two", second))),
        )
        self.assertEqual(result["totals"]["lines"], 3)
        self.assertEqual(
            result["unique_lines"], {"one": [("a.cpp", 1)], "two": [("b.cpp", 1)]}
        )

    def test_legacy_gcov_function_names(self) -> None:
        trace: coverage.Trace = {
            "files": [
                {
                    "file": "original.cpp",
                    "functions": [
                        {
                            "demangled_name": "value(int)",
                            "lineno": 1,
                            "execution_count": 1,
                        }
                    ],
                    "lines": [
                        {
                            "function_name": "value(int)",
                            "line_number": 1,
                            "count": 1,
                            "branches": [{"count": 1}, {"count": 0}],
                        }
                    ],
                }
            ]
        }
        function = coverage.summarize(trace, [("one", trace)])["files"][0]["functions"][
            0
        ]
        self.assertEqual(function["lines_covered"], 1)
        self.assertEqual(function["branches_covered"], 1)
        self.assertEqual(function["branches"], 2)

    def test_empty_original_report_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "empty.json"
            path.write_text('{"files": []}')
            with self.assertRaisesRegex(RuntimeError, "Empty original coverage"):
                coverage.read_trace(path)

    def test_missing_profiles_fail(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "empty-process").mkdir()
            with self.assertRaisesRegex(RuntimeError, "Missing process coverage"):
                coverage.collect(root, root / "report.json", root, [], {})

    def test_failed_scenario_cannot_report_coverage(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with (
                patch.object(
                    coverage.core,
                    "run_scenario",
                    return_value={
                        "passed": False,
                        "problems": ["an exit save is missing"],
                    },
                ),
                patch.object(coverage, "collect") as collect,
            ):
                with self.assertRaisesRegex(
                    RuntimeError, "Reference scenario failed: broken"
                ):
                    coverage.measure(
                        {"name": "broken"}, root / "game", root, root, root, [], 1
                    )
            collect.assert_not_called()
            self.assertTrue((root / "runs/broken/result.json").is_file())


if __name__ == "__main__":
    unittest.main()
