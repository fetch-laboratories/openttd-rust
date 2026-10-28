#!/usr/bin/env python3
"""Exercise sanitized-game report capture and the original/candidate-only split (#505)."""

import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from simulation import core, sanitizers

import migration

# Overflows a signed int in Grow, then reads freed memory when given two arguments.
PROBE = r"""
#include <climits>
#include <cstdio>
#include <cstdlib>
__attribute__((noinline)) int Grow(int value) { return value + 1; }
int main(int argc, char **)
{
	std::fprintf(stderr, "[t] dbg: before\n");
	int grown = Grow(INT_MAX - 1 + (argc > 1));
	std::fprintf(stderr, "[t] dbg: after %d\n", grown & 1);
	int *freed = static_cast<int *>(std::malloc(sizeof(int)));
	std::free(freed);
	return argc > 2 ? freed[0] : 0;
}
"""

UBSAN = "UBSan {} in Grow(int): signed integer overflow: N + N cannot be represented in type 'int'"


@unittest.skipUnless(shutil.which("g++"), "needs g++ with its sanitizer runtimes")
class SanitizedGameTests(unittest.TestCase):
    def test_launched_game_reports_split_from_its_compared_log(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "probe.cpp").write_text(PROBE)
            flags = migration.SANITIZER_FLAGS.split()
            subprocess.run(
                ["g++", "-O1", "-g", *flags, "probe.cpp", "-o", "probe"],
                cwd=root,
                check=True,
            )
            for arguments, exit_code, kinds in (
                (["x"], 0, {"UBSan"}),
                (["x", "y"], 1, {"UBSan", "ASan"}),
            ):
                with self.subTest(arguments=arguments):
                    run_dir = root / f"run-{len(arguments)}"
                    run_dir.mkdir()
                    with (
                        (run_dir / "stdout.log").open("wb") as out,
                        (run_dir / "stderr.log").open("wb") as err,
                    ):
                        launched = subprocess.run(
                            [
                                sys.executable,
                                str(core.ROOT / "tools/simulation/game_launcher.py"),
                                str(root / "probe"),
                                *arguments,
                            ],
                            env=sanitizers.environment(run_dir),
                            stdout=out,
                            stderr=err,
                        )
                    self.assertEqual(launched.returncode, exit_code)
                    with (
                        patch.object(migration, "SANITIZE", True),
                        patch.object(migration, "CANDIDATE_BUILD", root),
                    ):
                        run = core.read_game({"dir": run_dir})
                    self.assertEqual(run["log"], ["dbg: before", "dbg: after 0"])
                    reports = run["sanitizer"].reports
                    self.assertEqual({key.split()[0] for key in reports}, kinds)
                    self.assertIn(UBSAN.format("probe.cpp"), reports)
                    if "ASan" in kinds:
                        self.assertTrue(
                            any(
                                key.startswith("ASan heap-use-after-free at ")
                                and key.endswith("/probe.cpp in main")
                                for key in reports
                            ),
                            reports,
                        )


class ReportTests(unittest.TestCase):
    def test_ccache_relative_paths_match_between_game_builds(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            reference = root / ".local/reference/openttd"
            roles = {}
            for role, source in (
                ("reference", "../reference/openttd/src/a.cpp"),
                ("candidate", "../src/a.cpp"),
            ):
                run_dir = root / role
                run_dir.mkdir()
                (run_dir / "stdout.log").write_bytes(b"")
                (run_dir / "stderr.log").write_text(
                    f"{source}:3:7: runtime error: shift exponent 64\n"
                    f"    #0 0x5c5 in Foo(int) {source}:3\n\n"
                    "[t] dbg: unchanged\n"
                )
                (run_dir / sanitizers.REPORTS).mkdir()
                (run_dir / sanitizers.REPORTS / "report.42").write_text(
                    "==42==ERROR: AddressSanitizer: heap-buffer-overflow on address 0x1\n"
                    f"    #0 0x55 in Foo(int) {source}:{5 if role == 'reference' else 9}\n"
                )
                with (
                    patch.object(migration, "SANITIZE", True),
                    patch.object(migration, "REFERENCE", reference),
                    patch.object(
                        migration,
                        "REFERENCE_BUILD",
                        root / ".local/build-reference-sanitized",
                    ),
                    patch.object(
                        migration, "CANDIDATE_BUILD", root / "build-rust-sanitized"
                    ),
                    patch.object(core, "ROOT", root),
                ):
                    run = core.read_game({"dir": run_dir})
                self.assertEqual(run["log"], ["dbg: unchanged"])
                roles[role] = run["sanitizer"]
            self.assertEqual(
                sanitizers.compare(roles),
                sanitizers.Comparison(
                    [
                        "ASan heap-buffer-overflow at src/a.cpp in Foo(int)",
                        "UBSan src/a.cpp in Foo(int): shift exponent N",
                    ],
                    [],
                    [],
                ),
            )

    def test_asan_faults_in_different_files_are_not_shared(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            roles = {}
            for role, source in (
                ("reference", "src/a.cpp"),
                ("candidate", "src/b.cpp"),
            ):
                run_dir = root / role
                (run_dir / sanitizers.REPORTS).mkdir(parents=True)
                (run_dir / sanitizers.REPORTS / "report.42").write_text(
                    "==42==ERROR: AddressSanitizer: heap-buffer-overflow on address 0x1\n"
                    f"    #0 0x55 in Read() {root}/{source}:5\n"
                )
                roles[role] = sanitizers.Findings()
                sanitizers.file_findings(run_dir, (root,), roles[role])
            self.assertEqual(
                sanitizers.compare(roles),
                sanitizers.Comparison(
                    [],
                    ["ASan heap-buffer-overflow at src/b.cpp in Read()"],
                    ["ASan heap-buffer-overflow at src/a.cpp in Read()"],
                ),
            )

    def test_log_splitting(self) -> None:
        frame = "    #0 0x5c5 in Foo(int) {}/src/a.cpp:3"
        cases = (
            # A block ends at its closing blank line; game lines on both sides stay.
            (
                ["dbg: x", "{}/src/a.cpp:3:7: runtime error: shift exponent 64", frame]
                + ["", "dbg: y"],
                ["dbg: x", "dbg: y"],
                ["UBSan src/a.cpp in Foo(int): shift exponent N"],
            ),
            # vptr notes and shadow bytes belong to the block; the next line does not.
            (
                ["{}/src/b.cpp:9:1: runtime error: member call on address 0x60b"]
                + ["{}/src/b.cpp:2:7: note: object is of type 'B'", " 00 00 be be"]
                + ["              ^~~~~~~", frame, "dbg: z"],
                ["dbg: z"],
                ["UBSan src/b.cpp in Foo(int): member call on address N"],
            ),
            # The summary line is dropped and no report means an unchanged log.
            (
                ["SUMMARY: UndefinedBehaviorSanitizer: undefined-behavior", "dbg: w"],
                ["dbg: w"],
                [],
            ),
        )
        for root in (Path("/work/candidate"), Path("/common/reference/openttd")):
            for lines, kept, keys in cases:
                with self.subTest(root=root, lines=lines):
                    log = [line.replace("{}", str(root)) for line in lines]
                    remaining, findings = sanitizers.split_log(
                        log, (Path("/work/candidate"), root)
                    )
                    self.assertEqual(remaining, kept)
                    self.assertEqual(sorted(findings.reports), keys)

    def test_rust_hash_and_runtime_frames_leave_the_asan_key(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            run_dir = Path(directory)
            (run_dir / sanitizers.REPORTS).mkdir()
            (run_dir / sanitizers.REPORTS / "report.42").write_text(
                "==42==ERROR: AddressSanitizer: heap-buffer-overflow on address 0x1\n"
                "READ of size 4 at 0x1 thread T0\n"
                "    #0 0x7f in memcpy ../../../../src/libsanitizer/asan/x.cpp:1\n"
                "    #1 0x55 in openttd_kernels::town::grow::h0123456789abcdef"
                " rust/town.rs:5\n"
            )
            findings = sanitizers.Findings()
            sanitizers.file_findings(run_dir, (), findings)
            self.assertEqual(
                list(findings.reports),
                [
                    "ASan heap-buffer-overflow at rust/town.rs in openttd_kernels::town::grow"
                ],
            )

    def test_comparison_categories(self) -> None:
        roles = {role: sanitizers.Findings() for role in ("reference", "candidate")}
        for role, key in (
            ("reference", "both"),
            ("candidate", "both"),
            ("candidate", "fault"),
            ("reference", "ported"),
        ):
            roles[role].add(key, key)
        self.assertEqual(
            sanitizers.compare(roles),
            sanitizers.Comparison(["both"], ["fault"], ["ported"]),
        )


if __name__ == "__main__":
    unittest.main()
