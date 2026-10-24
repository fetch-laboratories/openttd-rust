#!/usr/bin/env python3
"""Exercise simulation determinism, runtime isolation and semantic speed reports."""

import contextlib
import hashlib
import io
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from simulation import core as simulate
from simulation import instructions as ir
from simulation import runtime
from simulation.instructions import SPEED_BUDGETS, speed_budget

import migration


class SimulationDeterminismTests(unittest.TestCase):
    def test_unnamed_fixture_runs_and_named_river_diagnostics(self):
        # Disaster and aircraft preparation call run_game directly with unnamed
        # generation/setup and save/stepping dictionaries. All hooks run there.
        generated = dict(kind="generate", seed=86, map_log2=7, land_generator=1)
        cases = (
            (generated, []),
            (dict(kind="save", save="fixture.sav"), []),
            (dict(generated, name="generate-river-search"), ["-d", "yapf=3"]),
        )
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for scenario, diagnostics in cases:
                with (
                    self.subTest(scenario=scenario),
                    patch.object(
                        simulate, "game_slot", return_value=contextlib.nullcontext()
                    ),
                    patch.object(simulate.subprocess, "run") as launch,
                ):
                    launch.return_value.returncode = 0
                    run = simulate.run_game(
                        dict(scenario, ticks=1),
                        root / "openttd",
                        root,
                        root / "run",
                        10,
                        {},
                        False,
                    )
                    self.assertEqual(run["exit"], 0)
                    options = [
                        run["command"][index - 1 : index + 1]
                        for index, arg in enumerate(run["command"])
                        if arg == "yapf=3"
                    ]
                    self.assertEqual(options, [diagnostics] if diagnostics else [])
                    launch.assert_called_once()

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
    def test_record_and_fixed_ceilings(self):
        from math import inf, nextafter

        for name, ceiling in (("play-test", 1.5), ("generate-test", 1.3)):
            with self.subTest(name=name), patch.dict(SPEED_BUDGETS, {name: 1.25}):
                report = speed_budget(name, 1.4375)
                self.assertEqual(report["record"], 1.25)
                self.assertAlmostEqual(report["change_percent"], 15.0)
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

    def exercise(self, *, instructions=False, **failure):
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory)
            scenario = {"name": "speed", "ticks": 74}
            runs = [
                {
                    "exit": failure.get("exit_code", 0),
                    "dir": out,
                    "instructions": dict(
                        identity={"compatible": role},
                        cached=None if failure else 42,
                        cache=out / "cache",
                    ),
                    "seconds": seconds,
                    "command": [role],
                    "snapshots": [out / role / "exit.sav"],
                    "log": [role] if failure.get("log") else [],
                    "stdout": b"",
                }
                for role, seconds in (("reference", 1.0), ("candidate", 2.0))
            ]
            if instructions:
                identity = {"compatible": "reference"}
                cache = out / "instruction-cache" / f"{ir.digest(identity)}.json"
                cache.parent.mkdir()
                cache.write_text("42")
                with (
                    patch.object(migration, "COMMON_LOCAL", out),
                    patch.object(ir, "identity", return_value=identity),
                ):
                    command, runs[0]["instructions"] = ir.prepare(
                        scenario,
                        ["reference"],
                        Path("reference"),
                        out,
                        out,
                        {"VALGRIND_LIB": "/unused"},
                        True,
                    )
                self.assertEqual(command, ["reference"])  # Actual reference cache hit.
            with (
                patch.object(simulate, "scenario_modules", return_value=()),
                patch.object(simulate, "run_game", side_effect=runs * 3) as game,
                patch.object(
                    simulate,
                    "save_moment",
                    side_effect=[(1, 0, 74), (1, 0, 75)] * 3
                    if failure.get("moment")
                    else None,
                    return_value=(1, 0, 74),
                ),
                patch.object(
                    simulate,
                    "compare_saves",
                    return_value=[{"field": "world"}]
                    if failure.get("difference")
                    else [],
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
                    instructions=instructions,
                )
            if instructions and failure:
                self.assertNotIn("instruction_speed", result)
                self.assertRegex(result["problems"][-1], "semantically unequal")
                self.assertEqual(cache.read_text(), "42")
            return result, game.call_count, game.call_args_list

    def test_speed_ratio_needs_matching_semantic_pairs(self):
        result, _, _ = self.exercise(instructions=True)
        self.assertEqual(result["plain_speed"]["samples"], [])
        self.assertEqual(result["instruction_speed"]["reference"]["Ir"], 42)
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
        for failure in (
            {"difference": True},
            {"exit_code": 1},
            {"log": True},
            {"moment": True},
        ):
            with self.subTest(instructions=True, **failure):
                result, calls, _ = self.exercise(instructions=True, **failure)
                self.assertFalse(result["passed"])
                self.assertEqual(calls, 2)


class InstructionSpeedTests(unittest.TestCase):
    def test_fixed_anchor_cumulative_projection_and_replacement(self):
        anchor = dict(
            head="quiet", ratio=1.2314, Ir=100, identity={"compatible": "same"}
        )
        record = dict(anchor=anchor, integrated=dict(head="base", Ir=100))
        measurement = dict(Ir=100, identity={"compatible": "same"})
        for step in range(1, 4):
            measurement["Ir"] = 100 * 1.04**step
            report = ir.budget("play-test", measurement, record)
            record["integrated"] = dict(head=str(step), Ir=measurement["Ir"])
        self.assertAlmostEqual(report["projected_ratio"], 1.5389150592)
        self.assertFalse(report["within_ceiling"])
        self.assertEqual(anchor["Ir"], 100)
        self.assertEqual(ir.projection(anchor, 90), 1.2314)
        record["anchor"] = dict(anchor, ratio=1.1, Ir=measurement["Ir"])
        report = ir.budget("play-test", measurement, record)
        self.assertEqual(report["projected_ratio"], 1.1)
        incompatible = dict(measurement, identity={"compatible": "other"})
        with self.assertRaises(ValueError):
            ir.budget("play-test", incompatible, record)

    def test_real_nested_scope_extraction_identity_and_semantic_rejection(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source, binary, run = root / "probe.cpp", root / "openttd", root / "run"
            run.mkdir()
            (root / "CMakeCache.txt").write_text("settings")
            source.write_text(
                "volatile int n; void StateGameLoop_LinkGraphPauseControl(){for(int i=0;i<10000;i++)n=n+1;}\n"
                "void StateGameLoop(){StateGameLoop_LinkGraphPauseControl();}\n"
                "void _GenerateWorld(){for(int i=0;i<20000;i++)n=n+1;}\n"
                "int main(){_GenerateWorld(); StateGameLoop();}"
            )
            subprocess.run(
                ["g++", "-g", "-O0", str(source), "-o", str(binary)], check=True
            )
            scenario = dict(kind="save", ticks=1, save=str(source))
            env = dict(
                migration.environment(), VALGRIND_OPTS="--toggle-collect=StateGameLoop*"
            )
            ir.record_build(root, root, "reference", env)
            context = binary, root, run, env
            candidate = root / "build-rust"
            candidate.mkdir()
            shutil.copy2(binary, candidate / "openttd-rust")
            shutil.copy2(root / "instruction-build.json", candidate)

            # Adapter copies exercise export stripping without another compiler fixture.
            def adapter(build, folder, env):
                folder.mkdir()
                return Path(shutil.copy2(binary, folder / "adapter"))

            with contextlib.ExitStack() as stack:
                stack.enter_context(patch.object(simulate, "ROOT", root))
                stack.enter_context(patch.object(migration, "REFERENCE_BUILD", root))
                for module in simulate.scenario_modules():
                    if hasattr(module, "adapter"):
                        stack.enter_context(patch.object(module, "adapter", adapter))
                runtime.export(root / "bundle", migration)
            counts = []
            scenario["name"] = next(iter(SPEED_BUDGETS))

            def collect(case, binaries, builds, out, *args):
                self.assertTrue(args[-1])
                frozen = binaries["candidate"]
                context = frozen, builds["candidate"], run, env
                for kind in ("save", "generate"):
                    case = dict(scenario, kind=kind)
                    command, info = ir.prepare(case, [str(frozen)], *context, False)
                    subprocess.run(command, env=env, check=True)
                    counts.append(ir.count(run / "callgrind.out"))
                    (run / "callgrind.out").unlink()
                    (run / "callgrind.log").unlink()
                frozen.write_bytes(frozen.read_bytes() + b"changed")
                with self.assertRaisesRegex(ValueError, "stale instruction build"):
                    ir.prepare(case, [str(frozen)], *context, False)
                (builds["candidate"] / "instruction-build.json").unlink()
                with self.assertRaises(FileNotFoundError):
                    ir.prepare(case, [str(frozen)], *context, False)
                shutil.rmtree(out)
                raise RuntimeError("collected frozen profiles")

            cli = ["sim", "--instructions", f"--frozen-runtimes={root / 'bundle'}"]
            with (
                patch.object(simulate, "scenario_list", return_value=[scenario]),
                patch.object(simulate, "run_scenario", side_effect=collect),
                patch.object(sys, "argv", cli),
                self.assertRaisesRegex(RuntimeError, "collected frozen profiles"),
            ):
                simulate.main()
            # Wildcard nested toggle loses this work.
            self.assertGreater(counts[0], 50000)
            self.assertGreater(counts[1], counts[0] * 2)
            command, _ = ir.prepare(scenario, [str(binary)], *context, False)
            baseline = ir.identity(scenario, *context, Path(command[0]))
            (root / ".cargo").mkdir()
            for field, value in (("ticks", 2), ("console", ["unpause"])):
                changed = ir.identity(
                    dict(scenario, **{field: value}), *context, Path(command[0])
                )
                self.assertNotEqual(baseline, changed)
            # Worktree paths in CMake bookkeeping must not change the identity.
            (root / "regression").mkdir()
            bookkeeping = root / "regression/CTestTestfile.cmake"
            bookkeeping.write_text("/worktree-a")
            previous = ir.identity(scenario, *context, Path(command[0]))
            bookkeeping.write_text("/worktree-b")
            self.assertEqual(
                previous, ir.identity(scenario, *context, Path(command[0]))
            )
            for name in (
                "probe.cpp",
                "CMakeCache.txt",
                "Cargo.toml",
                ".cargo/config.toml",
                "run/script",
                "regression/main.nut",
            ):
                previous = ir.identity(scenario, *context, Path(command[0]))
                with (root / name).open("ab") as asset:
                    asset.write(b"changed")
                ir.record_build(root, root, "reference", env)
                current = ir.identity(scenario, *context, Path(command[0]))
                self.assertNotEqual(previous, current)
            with binary.open("ab") as asset:
                asset.write(b"changed")
            with self.assertRaisesRegex(ValueError, "stale instruction build"):
                ir.identity(scenario, *context, Path(command[0]))
            ir.record_build(root, root, "reference", env)
            refreshed = ir.identity(scenario, *context, Path(command[0]))
            self.assertEqual(current["compatible"], refreshed["compatible"])
            self.assertNotEqual(current["executable"], refreshed["executable"])
            tools = root / "valgrind-tools"
            tools.mkdir()
            for path in Path(env["VALGRIND_LIB"]).glob("memcheck-*"):
                (tools / path.name).symlink_to(path)
            context = binary, root, run, dict(env, VALGRIND_LIB=str(tools))
            for name in ("callgrind-test", "vgpreload_core-test.so"):
                (tools / name).write_bytes(b"original")
            for name in ("callgrind-test", "vgpreload_core-test.so"):
                previous = ir.identity(scenario, *context, Path(command[0]))
                (tools / name).write_bytes(b"changed")
                self.assertNotEqual(
                    previous, ir.identity(scenario, *context, Path(command[0]))
                )
            (tools / "vgpreload_core-test.so").unlink()
            with self.assertRaisesRegex(ValueError, "incomplete selected Valgrind"):
                ir.identity(scenario, *context, Path(command[0]))
            profile = run / "callgrind.out"
            good = "desc: Trigger: Program termination\nevents: Dr Ir\nsummary: 999 999\ntotals: 2 42\n"
            profile.write_text(good)
            self.assertEqual(ir.count(profile), 42)
            for bad in (
                "",
                good.replace("2 42", "2 0"),
                good.replace("2 42", "oops"),
                good.replace("Ir", "Dw"),
                good.replace("Program termination", "Client Request"),
            ):
                profile.write_text(bad)
                with self.assertRaises(ValueError):
                    ir.count(profile)
            profile.unlink()
            with self.assertRaises(FileNotFoundError):
                ir.count(profile)
            for key in ("differences", "known_failures", "problems"):
                failure = dict(differences=[], known_failures=[], problems=[])
                failure[key] = ["failure"]
                with self.assertRaises(ValueError):
                    ir.finish({}, failure)  # Reject before extraction/cache writes.

    def test_main_checkout_and_worktree_record_one_settings_identity(self):
        with tempfile.TemporaryDirectory() as directory:
            main, worktree = Path(directory) / "clone", Path(directory) / "wt"
            shared = main / ".local"
            recorded = []
            for source in (main, worktree):
                build = source / "build-rust"
                build.mkdir(parents=True)
                (build / "openttd").write_bytes(b"game")
                (build / "CMakeCache.txt").write_text(
                    f"{shared}/deps/lib;{source}/src;{build}/x;{shared}/cargo\n"
                )
                with patch.object(migration, "COMMON_LOCAL", shared):
                    ir.record_build(build, source, "reference", {"CARGO_HOME": "-"})
                recorded.append((build / "instruction-build.json").read_text())
            self.assertEqual(recorded[0], recorded[1])
            settings = json.loads(recorded[0])["settings"]
            self.assertIn(f"{shared}/deps/lib;<source>/src;<build>/x;", settings)

    def test_effective_compiled_debug_rejects_overrides_and_stripping(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text(
                '[package]\nname="openttd-kernels"\nversion="0.0.0"\nedition="2024"\n'
                '[lib]\npath="lib.rs"\ncrate-type=["staticlib"]\n'
            )
            (root / "lib.rs").write_text(
                "use std::{cell::RefCell, collections::BTreeMap};\n"
                '#[unsafe(no_mangle)] #[inline(never)] pub extern "C" fn probe(x: u32)->usize {\n'
                "let map = RefCell::new(BTreeMap::new());\n"
                "for i in 0..x { map.borrow_mut().insert(i, i as usize); }\n"
                "*map.borrow().get(&x.wrapping_sub(1)).unwrap_or(&0) }"
            )
            for flags in (
                "",
                "-Cdebuginfo=0",
                "-Cdebuginfo=1",
                "-Cdebuginfo=2",
                "-Cstrip=debuginfo",
            ):
                env = dict(
                    migration.environment(),
                    CARGO_PROFILE_RELEASE_DEBUG="line-tables-only",
                    RUSTFLAGS=flags,
                )
                subprocess.run(
                    ["cargo", "build", "--release", "--offline"],
                    cwd=root,
                    env=env,
                    check=True,
                    capture_output=True,
                )
                archive = root / "target/release/libopenttd_kernels.a"
                if flags.startswith("-Cdebuginfo="):
                    with self.assertRaisesRegex(
                        ValueError, "effective line-tables-only", msg=flags
                    ):
                        ir.verify_debug(archive)
                else:
                    ir.verify_debug(archive)

    def test_reused_build_restores_captured_release_debug(self):
        with tempfile.TemporaryDirectory() as directory:
            build = Path(directory)
            (build / "build.ninja").touch()
            (build / "CMakeCache.txt").write_text(
                f"CMAKE_HOME_DIRECTORY:PATH={build}\nOPTION_RUST:BOOL=ON\n"
            )
            configuration = build / "rust-build-configuration.txt"
            profile_env = {"CARGO_PROFILE_RELEASE_DEBUG": "line-tables-only"}
            configured = migration.configured_as
            for effective, expected in (("line-tables-only", True), ("<unset>", False)):
                configuration.write_text(f"CARGO_PROFILE_RELEASE_DEBUG={effective}\n")
                self.assertEqual(configured(build, build, [], profile_env), expected)
                self.assertEqual(configured(build, build, [], {}), not expected)


class SpeedPassTests(unittest.TestCase):
    def test_anchor_is_median_of_group_medians(self):
        from simulation import speed

        names = ["play-a", "generate-b"]
        groups = ([1.0, 1.25], [2.0, 1.5], [3.0, 1.75], [4.0, 2.0])
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory)
            profile = out / "profile.json"
            results = [
                {"scenario": name, "instruction_speed": {"candidate": {"Ir": 10 + i}}}
                for i, name in enumerate(names)
            ]
            profile.write_text(json.dumps({"results": results}))
            manifest = {
                "head": "0123456789abcdef",
                "profile": str(profile),
                "identities": {name: name + "-id" for name in names},
            }
            (out / "observations.jsonl").write_text(
                json.dumps({"load": [0.5, 0, 0], "unheld": []}) + "\n"
            )

            def write(group, ratios, passed=True):
                results = [
                    {
                        "scenario": name,
                        "passed": passed,
                        "plain_speed": {
                            "samples": [{}] * speed.PAIRS,
                            "median_candidate_reference_ratio": ratio,
                        },
                    }
                    for name, ratio in zip(names, ratios, strict=True)
                ]
                path = out / f"group-{group}.json"
                path.write_text(json.dumps({"results": results}))

            for group, ratios in enumerate(groups):
                write(group, ratios)
            with contextlib.redirect_stdout(io.StringIO()) as printed:
                speed.summarize(out, manifest, names, len(groups))
            rows = json.loads((out / "anchors.json").read_text())["rows"]
            self.assertEqual(
                rows,
                [
                    ["play-a", 2.5, 10, "play-a-id"],
                    ["generate-b", 1.625, 11, "generate-b-id"],
                ],
            )
            self.assertIn('    ("play-a", 2.5, 10, "play-a-id"),', printed.getvalue())
            for incomplete in (3, len(groups)):
                with self.subTest(groups=incomplete), self.assertRaises(SystemExit):
                    write(3, groups[3], passed=incomplete == 3)
                    speed.summarize(out, manifest, names, incomplete)


if __name__ == "__main__":
    unittest.main()
