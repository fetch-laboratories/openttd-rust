#!/usr/bin/env python3
"""Exercise simulation determinism, runtime isolation and semantic speed reports."""

import contextlib
import hashlib
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from simulation import core as simulate
from simulation.roads import SPEED_BUDGETS, speed_budget


class SimulationDeterminismTests(unittest.TestCase):
    def test_each_requested_selector_must_match_before_any_run(self):
        stderr = io.StringIO()
        with (
            patch.object(simulate, "scenario_list", return_value=[{"name": "valid"}]),
            patch.object(sys, "argv", ["simulate.py", "valid", "misspelled", "--list"]),
            contextlib.redirect_stderr(stderr),
        ):
            with self.assertRaises(SystemExit) as error:
                simulate.main()
        self.assertEqual(error.exception.code, 2)
        self.assertIn("misspelled", stderr.getvalue())

    def test_exact_and_substring_selectors_preserve_selection(self):
        output = io.StringIO()
        with (
            patch.object(
                simulate,
                "scenario_list",
                return_value=[
                    {"name": "valid"},
                    {"name": "valid-extended"},
                    {"name": "other-test"},
                ],
            ),
            patch.object(sys, "argv", ["simulate.py", "valid", "other", "--list"]),
            contextlib.redirect_stdout(output),
        ):
            self.assertEqual(simulate.main(), 0)
        self.assertEqual(output.getvalue().splitlines(), ["valid", "other-test"])

    def test_end_mismatch_compares_exits_and_full_logs_without_retry(self):
        calls, compared = [], []

        def run(scenario, binary, build, folder, *args, **kwargs):
            calls.append(folder)
            return {
                "exit": 0,
                "seconds": 0,
                "snapshots": (
                    [folder / "save/autosave/dmp_cmds_extra.sav"]
                    if folder.name == "candidate"
                    else []
                )
                + [folder / "save/autosave/exit.sav"],
                "log": ["same", "extra"] if folder.name == "candidate" else ["same"],
                "stdout": b"",
            }

        with (
            patch.object(simulate, "scenario_modules", return_value=[]),
            patch.object(simulate, "run_game", side_effect=run),
            patch.object(
                simulate,
                "save_moment",
                side_effect=lambda p: (1, 0, 2 if "candidate" in p.parts else 1),
            ),
            patch.object(
                simulate,
                "compare_saves",
                side_effect=lambda a, b, *args: compared.append(a) or [],
            ),
        ):
            result = simulate.run_scenario(
                {"name": "strict", "short_checkpoint": True, "ticks": 1},
                {"reference": None, "candidate": None},
                {"reference": None, "candidate": None},
                Path("unused"),
                20,
                10,
                {},
            )
        self.assertFalse(result["passed"])
        self.assertEqual(len(calls), 4)
        self.assertEqual(len(compared), 2)
        self.assertEqual(len(result["problems"]), 4)
        self.assertEqual(
            [d["snapshot"] for d in result["differences"]],
            ["snapshots/log", "plain/log"],
        )

    @unittest.skipUnless(sys.platform == "linux", "Linux child resource policy")
    def test_launcher_denies_threads_without_changing_parent_limit(self):
        import resource

        before = resource.getrlimit(resource.RLIMIT_NPROC)
        result = subprocess.run(
            [
                sys.executable,
                str(simulate.ROOT / "tools/simulation/game_launcher.py"),
                sys.executable,
                "-c",
                "import resource, _thread; print(resource.getrlimit(resource.RLIMIT_NPROC)); "
                "\ntry: _thread.start_new_thread(lambda: None, ())"
                "\nexcept RuntimeError: print('denied')",
            ],
            capture_output=True,
            text=True,
        )
        self.assertEqual(resource.getrlimit(resource.RLIMIT_NPROC), before)
        if os.geteuid() == 0:
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("did not prevent threads", result.stderr)
        else:
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout.splitlines(), ["(0, 0)", "denied"])


class SimulationRuntimeTests(unittest.TestCase):
    def exercise(
        self,
        *,
        custom=False,
        self_compare=False,
        passed=True,
        crash=False,
    ):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "tools").mkdir()
            (root / "head").write_text("initial-checkout")
            (root / "status").write_text(" M tracked-file")
            # A tiny driver fixture supplies paths; actual simulation
            # main(), runtime copying and executable launches stay exercised.
            (root / "tools/migration.py").write_text(
                "from contextlib import nullcontext\n"
                "from pathlib import Path\n"
                "ROOT = Path(__file__).resolve().parents[1]\n"
                "LOCAL = COMMON_LOCAL = ROOT / '.local'\n"
                "REFERENCE_BUILD = ROOT / 'reference'\n"
                "def reference_lock(shared=False): return nullcontext()\n"
                "def environment(): return {}\n"
                "def git(*args):\n"
                "    return (ROOT / ('head' if args[0] == 'rev-parse' else 'status')).read_text()\n"
            )
            ai_initial = {}
            for folder in ("town-name-observer", "tree-scenario-ai"):
                ai = root / "tools" / folder
                ai.mkdir()
                (ai / "main.nut").write_text(f"{folder}-before")
                ai_initial[folder] = hashlib.sha256(
                    (ai / "main.nut").read_bytes()
                ).hexdigest()
            originals, assets = {}, {}
            for role in ("reference", "candidate"):
                if role == "candidate" and self_compare:
                    continue
                build = root / (
                    "reference"
                    if role == "reference"
                    else "custom-build"
                    if custom
                    else "build-rust"
                )
                (build / "lang").mkdir(parents=True)
                asset = root / f"{role}-asset"
                asset.write_text(f"{role}-data-before")
                (build / "lang/linked-data").symlink_to(Path("../..") / asset.name)
                (build / "lang/missing-data").symlink_to("absent-relative-target")
                binary = build / ("openttd" if role == "reference" else "openttd-rust")
                binary.write_text(f"print('{role}-before')\n")
                originals[role], assets[role] = binary, asset

            invocations = []

            def run(scenario, binaries, builds, *args):
                for role in ("reference", "candidate"):
                    original_role = "reference" if self_compare else role
                    output = subprocess.check_output(
                        [sys.executable, binaries[role]], text=True
                    ).strip()
                    self.assertEqual(output, f"{original_role}-before")
                    self.assertEqual(
                        (builds[role] / "lang/linked-data").read_text(),
                        f"{original_role}-data-before",
                    )
                    self.assertFalse((builds[role] / "lang/missing-data").exists())
                folder = (
                    "town-name-observer"
                    if "town_style" in scenario
                    else "tree-scenario-ai"
                )
                self.assertEqual(
                    (Path(scenario["scenario_ai"]) / "main.nut").read_text(),
                    f"{folder}-before",
                )
                if crash:
                    raise RuntimeError("scenario failed unexpectedly")
                invocations.append(scenario["name"])
                if len(invocations) == 1:
                    for role, binary in originals.items():
                        replacement = binary.with_suffix(".new")
                        replacement.write_text(f"print('{role}-after')\n")
                        replacement.replace(binary)
                        assets[role].write_text(f"{role}-data-after")
                    for folder in ai_initial:
                        (root / "tools" / folder / "main.nut").write_text(
                            f"{folder}-after"
                        )
                    (root / "head").write_text("later-checkout")
                    (root / "status").write_text("")
                return {
                    "scenario": scenario["name"],
                    "plain_speed": {
                        "median_candidate_reference_ratio": 1.0,
                        "budget": speed_budget("generate-tgp-256-1", 1.0),
                    },
                    "passed": passed,
                    "snapshots": 1,
                    "stats": {"chunks": 1, "elements": 1, "masked": {}},
                    "differences": [],
                    "known_failures": [],
                    "problems": [],
                }

            argv = ["simulate.py", "--jobs", "1"]
            if self_compare:
                argv += ["--self", "--candidate", str(root / "missing")]
            elif custom:
                argv += ["--candidate", str(originals["candidate"])]
            with (
                patch.object(simulate, "ROOT", root),
                patch.object(
                    simulate,
                    "scenario_list",
                    return_value=[
                        {"name": "a", "town_style": 0},
                        {"name": "b", "trees": "commands"},
                    ],
                ),
                patch.object(simulate, "run_scenario", side_effect=run),
                patch.object(sys, "argv", argv),
                contextlib.redirect_stdout(io.StringIO()),
            ):
                if crash:
                    with self.assertRaisesRegex(RuntimeError, "scenario failed"):
                        simulate.main()
                    run = next((root / ".local/simulation").iterdir())
                    self.assertFalse((run / "reference-runtime").exists())
                    self.assertFalse((run / "candidate-runtime").exists())
                    self.assertFalse((run / "report.json").exists())
                    return
                self.assertEqual(simulate.main(), 0 if passed else 1)
            report_path = next((root / ".local/simulation").glob("*/report.json"))
            report = json.loads(report_path.read_text())
            self.assertEqual(report["passed"], passed)
            self.assertEqual(invocations, ["a", "b"])
            self.assertEqual(report["candidate_commit"], "initial-checkout")
            self.assertEqual(report["candidate_status"], " M tracked-file")
            for folder in ai_initial:
                frozen = report_path.parent / "scenario-ai" / folder
                self.assertEqual((frozen / "main.nut").read_text(), f"{folder}-before")
            for role in ("reference", "candidate"):
                self.assertFalse((report_path.parent / f"{role}-runtime").exists())

    def test_default_candidate_survives_rebuild(self):
        self.exercise()

    def test_custom_candidate_uses_its_own_frozen_runtime(self):
        self.exercise(custom=True)

    def test_reference_self_needs_no_candidate(self):
        self.exercise(self_compare=True)

    def test_failed_runs_remove_runtimes_and_preserve_results(self):
        self.exercise(passed=False)

    def test_unexpected_worker_failure_removes_runtimes(self):
        self.exercise(crash=True)

    def test_fixture_preparation_removes_runtimes_on_pass_and_fail(self):
        from simulation import aircraft, ships

        for module, name, arguments in (
            (aircraft, "prepare_aircraft_save", ["--prepare-aircraft-save"]),
            (ships, "prepare_water_save", ["--prepare-water-save", "ferry"]),
        ):
            for crash in (False, True):
                with self.subTest(preparation=name, crash=crash):
                    with tempfile.TemporaryDirectory() as directory:
                        root = Path(directory)
                        (root / "tools").mkdir()
                        (root / "tools/migration.py").write_text(
                            "from pathlib import Path\n"
                            "LOCAL = COMMON_LOCAL = Path(__file__).resolve().parents[1] / '.local'\n"
                        )

                        def prepare(*args, crash=crash):
                            out = args[-2]
                            (out / "reference-runtime").mkdir()
                            (out / "fixture.log").write_text("retained output")
                            if crash:
                                raise RuntimeError("fixture preparation failed")

                        with (
                            patch.object(simulate, "ROOT", root),
                            patch.object(
                                simulate, "scenario_list", return_value=[{"name": "a"}]
                            ),
                            patch.object(module, name, side_effect=prepare),
                            patch.object(sys, "argv", ["simulate.py", *arguments]),
                        ):
                            if crash:
                                with self.assertRaisesRegex(
                                    RuntimeError, "preparation failed"
                                ):
                                    simulate.main()
                            else:
                                self.assertEqual(simulate.main(), 0)
                        run = next((root / ".local/simulation").iterdir())
                        self.assertFalse((run / "reference-runtime").exists())
                        self.assertEqual(
                            (run / "fixture.log").read_text(), "retained output"
                        )

    def test_pruning_keeps_newest_ten_completed_runs_and_active_outputs(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            completed = []
            for day in range(1, 14):
                run = root / f"202601{day:02}T000000Z-123"
                (run / "failing-scenario").mkdir(parents=True)
                (run / "report.json").write_text('{"passed": false}')
                (run / "failing-scenario/output.log").write_text("failure evidence")
                completed.append(run)
            active = root / "20250101T000000Z-456"
            (active / "candidate-runtime").mkdir(parents=True)
            (active / "report.json.tmp").write_text("unpublished")
            simulate.prune_simulation_runs(root)
            self.assertEqual(set(root.iterdir()), set(completed[3:] + [active]))
            self.assertTrue((active / "candidate-runtime").is_dir())
            for run in completed[3:]:
                self.assertEqual(
                    (run / "failing-scenario/output.log").read_text(),
                    "failure evidence",
                )


class SimulationSpeedTests(unittest.TestCase):
    def test_record_guard_and_fixed_ceilings(self):
        from math import inf, nextafter

        for name, ceiling in (("play-test", 1.5), ("generate-test", 1.3)):
            with self.subTest(name=name), patch.dict(SPEED_BUDGETS, {name: 1.25}):
                report = speed_budget(name, 1.4375)
                self.assertEqual(report["record"], 1.25)
                self.assertAlmostEqual(report["change_percent"], 15.0)
                self.assertTrue(report["within_guard"])
                self.assertFalse(
                    speed_budget(name, nextafter(1.4375, inf))["within_guard"]
                )
                self.assertEqual(report["ceiling"], ceiling)
                self.assertTrue(speed_budget(name, ceiling)["within_ceiling"])
                self.assertFalse(
                    speed_budget(name, nextafter(ceiling, inf))["within_ceiling"]
                )
                self.assertTrue(speed_budget(name, 0.95 * ceiling)["near_ceiling"])
                self.assertFalse(
                    speed_budget(name, nextafter(0.95 * ceiling, 0))["near_ceiling"]
                )
                self.assertLess(speed_budget(name, 0.9)["change_percent"], 0)
                self.assertIsNone(speed_budget(name, None))
        self.assertIsNone(speed_budget("unrecorded", 1.0))

    def exercise(self, *, difference=False, exit_code=0):
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory)
            scenario = {"name": "speed", "ticks": 74}
            runs = [
                {
                    "exit": exit_code,
                    "seconds": seconds,
                    "command": [role],
                    "snapshots": [out / role / "exit.sav"],
                    "log": [],
                    "stdout": b"",
                }
                for role, seconds in (("reference", 1.0), ("candidate", 2.0))
            ]
            with (
                patch.object(simulate, "scenario_modules", return_value=()),
                patch.object(simulate, "run_game", side_effect=runs * 3) as game,
                patch.object(simulate, "save_moment", return_value=(1, 0, 74)),
                patch.object(
                    simulate,
                    "compare_saves",
                    return_value=[{"field": "world"}] if difference else [],
                ),
            ):
                result = simulate.run_scenario(
                    scenario,
                    {role: role for role in ("reference", "candidate")},
                    {role: out for role in ("reference", "candidate")},
                    out,
                    1,
                    10,
                    {},
                    benchmark_repetitions=3,
                )
            return result, game.call_count, game.call_args_list

    def test_speed_ratio_needs_matching_semantic_pairs(self):
        result, calls, locks = self.exercise()
        self.assertTrue(result["passed"])
        self.assertEqual(calls, 6)
        self.assertTrue(all(call.kwargs == {"isolate": True} for call in locks))
        self.assertEqual(len(result["plain_speed"]["samples"]), 3)
        self.assertEqual(result["plain_speed"]["median_candidate_reference_ratio"], 2.0)
        for failure in ({"difference": True}, {"exit_code": 1}):
            with self.subTest(**failure):
                result, calls, _ = self.exercise(**failure)
                self.assertFalse(result["passed"])
                self.assertEqual(calls, 2)  # Preserve the first failing repetition.
                self.assertIsNone(
                    result["plain_speed"]["median_candidate_reference_ratio"]
                )


if __name__ == "__main__":
    unittest.main()
