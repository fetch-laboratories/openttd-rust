#!/usr/bin/env python3
"""Exercise simulation determinism, runtime isolation and semantic speed reports."""

import contextlib
import gzip
import hashlib
import io
import json
import os
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import unittest
import unittest.mock
from collections.abc import Sequence
from functools import partial
from pathlib import Path
from types import ModuleType
from typing import Any, cast
from unittest.mock import patch

from code_lines import without_comments
from simulation import (
    cli as simulation_cli,
)
from simulation import (
    community_ai,
    community_newgrf,
    configuration,
    engines,
    newgrf_loading,
    newgrf_sets,
    platform_subset,
    registry,
    runtime,
    savegame,
    scenario_runner,
    stations,
)
from simulation import core as simulate
from simulation import (
    game as game_module,
)
from simulation import instructions as ir
from simulation.instructions import SPEED_BUDGETS, speed_budget

import migration

simulate.scenario_modules()


class SimulationDeterminismTests(unittest.TestCase):
    def test_save_comparison_distinguishes_non_minimal_gamma(self) -> None:
        g = simulate.gamma
        header = b"\x0a" + g(1) + b"s" + b"\x00"  # One SLE_FILE_STRING field "s".

        def save(element_length: bytes, body: bytes) -> bytes:
            chunk = b"TEST\x03" + g(len(header) + 1) + header
            return b"OTTN\0\0\0\0" + chunk + element_length + body + b"\x00" * 5

        body = g(2) + b"ab"
        minimal = simulate.parse_save(save(g(len(body) + 1), body))["TEST"]
        for name, element_length, element, expected in (
            (
                "element length",
                b"\x80" + g(len(body) + 1),
                body,
                ("", "gamma encoding", 0, 1),
            ),
            (
                "string length",
                g(len(body) + 2),
                b"\x80\x02ab",
                ("0", "encoding", "026162", "80026162"),
            ),
        ):
            candidate = simulate.parse_save(save(element_length, element))["TEST"]
            stats = cast(simulate.Stats, {"elements": 0, "masked": {}})
            with self.subTest(name):
                self.assertEqual(
                    list(simulate.compare_chunk("TEST", minimal, candidate, stats)),
                    [expected],
                )
        stats = cast(simulate.Stats, {"elements": 0, "masked": {}})
        self.assertEqual(
            list(simulate.compare_chunk("TEST", minimal, minimal, stats)), []
        )

    def test_station_spatial_requires_complete_unique_rows(self) -> None:
        for reload in (False, True):
            phases = (
                ("reload",)
                if reload
                else (
                    "build",
                    "move-unchanged",
                    "move-away",
                    "move-back",
                    "remove",
                    "reuse",
                )
            )
            lines = [
                f"STCMD spatial602-visit {phase} 1000 8 1:true:32767"
                for phase in phases
            ]
            lines += [
                "STCMD spatial602-visit zero 1000 0",
                "STCMD spatial602-visit edge 0 1",
            ]
            lines += [
                f"STCMD owner562-rng {phase} 1 2 3 4"
                for phase in ("spatial602-before", "spatial602")
            ]
            lines += [
                f"STCMD spatial602-town {initial} 10 1000"
                for initial in (-32768, -1000, 999, 32767)
            ]
            ratings = " ".join(f"{i}:0" for i in range(19 if reload else 20))
            lines += [
                f"STCMD spatial602-rating {radius} {amount} {ratings}"
                for radius in (0, 1, 2, 8)
                for amount in (-160, -1, 0, 1, 255)
            ]
            tiles = " ".join(f"{i}:0" for i in range(6))
            for axis in (0, 1):
                for blocked in ("false", "true"):
                    for mode in (0, 1, 2):
                        lines.append(
                            f"STCMD spatial602-animation {axis} {blocked} {mode} {tiles} rng 1 2 3 4"
                        )
                        if mode != 0:
                            lines.append(
                                f"STCMD spatial602-random {axis} {blocked} {mode} {tiles} rng 1 2 3 4"
                            )
                        if not reload:
                            lines.append(
                                f"STCMD spatial602-rail {axis} {blocked} {mode} 1000 3 2"
                            )
            if reload:
                lines.append("STCMD spatial602-crash-reload 40")
            else:
                lines += ["STCMD spatial602-crash-input 20 3 1 2 3 4"]
                lines += [
                    f"STCMD spatial602-crash-event {event} type 0 id 21 site 10005 reason 1 victims 3 expected 3 rng 1 2"
                    for event in ("ai-event", "game-event")
                ]
                lines += [
                    f"STCMD spatial602-crash {i} {operation} {40 if i == 3 else 200}"
                    for i, operation in enumerate(
                        ("ai-event", "game-event", "news", "sound")
                    )
                ]
                lines += [
                    "STCMD owner562-rng spatial602-crash 1 2 3 4",
                    "STCMD spatial602-stop occupied-articulated 0 1",
                    "STCMD spatial602-collisions true true",
                    "STCMD spatial602-deleted-tie 0 0",
                    "STCMD spatial602-waypoint 1000 1 1",
                ]
                lines += [
                    f"STCMD spatial602-stop {bus} {incompatible} 0 1"
                    for bus in ("true", "false")
                    for incompatible in ("false", "true")
                ]
            lines.append("STCMD complete")
            stations.check_spatial602(lines, reload)
            for index, line in enumerate(lines):
                with (
                    self.subTest(reload=reload, missing=line),
                    self.assertRaises(RuntimeError),
                ):
                    stations.check_spatial602(
                        lines[:index] + lines[index + 1 :], reload
                    )
                with (
                    self.subTest(reload=reload, duplicate=line),
                    self.assertRaises(RuntimeError),
                ):
                    stations.check_spatial602(lines + [line], reload)
                if line.endswith(" rng 1 2 3 4"):
                    with (
                        self.subTest(reload=reload, incomplete=line),
                        self.assertRaises(RuntimeError),
                    ):
                        stations.check_spatial602(
                            lines[:index]
                            + [line.split(" rng ")[0]]
                            + lines[index + 1 :],
                            reload,
                        )

    def test_engine_runtime_requires_complete_price_matrix_and_rng(self) -> None:
        records = [
            f"ENGINE properties-runtime {kind} {price} {dynamic} {base} 0 -1"
            for kind, price in ((0, 15), (0, 16), (1, 18), (2, 19), (3, 17))
            for dynamic in ("false", "true")
            for base in (2560, (1 << 63) - 1, -(1 << 63))
        ] + ["ENGINE properties-runtime-rng 0 1 2147483648 4294967295"]
        markers = [
            f"ENGINE loading-case {label} true"
            for label, count in (
                ("runtime-null-grf", 4),
                ("runtime-action3-grf", 4),
                ("runtime-climate", 4),
                ("runtime-invalid-engine", 4),
                ("runtime-enabled", 4),
                ("runtime-build", 30),
                ("runtime-running", 30),
                ("runtime-invalid-price", 3),
                ("runtime-range-default", 1),
                ("runtime-range-dynamic", 1),
                ("runtime-life", 8),
                ("runtime-rng", 1),
                ("runtime-built-life", 4),
            )
            for _ in range(count)
        ]
        lines = records + markers
        self.assertEqual(engines.runtime_markers(lines), records)
        for index in range(len(lines)):
            with self.subTest(missing=index), self.assertRaises(RuntimeError):
                engines.runtime_markers(lines[:index] + lines[index + 1 :])
        for index, line in enumerate(records):
            for end in range(2, len(line.split())):
                with (
                    self.subTest(truncated=index, end=end),
                    self.assertRaises(RuntimeError),
                ):
                    engines.runtime_markers(
                        lines[:index]
                        + [" ".join(line.split()[:end])]
                        + lines[index + 1 :]
                    )
        for old, new in (
            ("0 16 false", "0 15 false"),
            ("0 15 true", "0 15 false"),
            ("9223372036854775807", "2560"),
            (" 0 -1", " 9223372036854775808 -1"),
            (" 0 -1", " 0 broken"),
            ("4294967295", "4294967296"),
            ("4294967295", "-1"),
        ):
            with self.subTest(replaced=old, new=new), self.assertRaises(RuntimeError):
                engines.runtime_markers([line.replace(old, new) for line in lines])

    def test_townname_observations_require_every_style(self) -> None:
        lines = [
            "LOADING townname first styles 1",
            "LOADING townname first style 0 35414E54 0 true 41",
            "LOADING townname first style 1 00000000 8384 false 28",
        ]
        self.assertEqual(newgrf_loading.townname_observations(lines, ("first",)), lines)
        for index in range(len(lines)):
            with self.subTest(missing=index), self.assertRaises(RuntimeError):
                newgrf_loading.townname_observations(
                    lines[:index] + lines[index + 1 :], ("first",)
                )

    def test_unnamed_fixture_runs_and_named_river_diagnostics(self) -> None:
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
                        game_module, "game_slot", return_value=contextlib.nullcontext()
                    ),
                    patch.object(game_module.subprocess, "run") as launch,
                ):
                    launch.return_value.returncode = 0
                    run = simulate.run_game(
                        cast(simulate.Scenario, dict(scenario, ticks=1)),
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

    def test_unnamed_controller_inputs_install_their_ai(self) -> None:
        cases = (
            (
                {"aircraft_controller_setup": True, "aircraft_count": 1},
                "aircraft-controller-ai",
                "aircraft-controller",
                "main.nut",
                "AIRCRAFT_COUNT <- 1;",
                ["-d", "script=2"],
            ),
            (
                {"road_setup": "flooding"},
                "road-scenario-ai",
                "road-crossing",
                "setup.nut",
                'ROAD_SETUP <- "flooding";',
                [],
            ),
        )
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for keys, source, installed, main, parameter, diagnostics in cases:
                with self.subTest(source=source):
                    source_dir = simulate.ROOT / "tools" / source
                    run_dir = root / installed
                    plan = simulate.prepare_game(
                        cast(
                            simulate.Scenario,
                            dict(
                                kind="generate",
                                seed=86,
                                map_log2=7,
                                land_generator=1,
                                ticks=1,
                                scenario_ai=str(source_dir),
                                **keys,
                            ),
                        ),
                        root / "openttd",
                        root,
                        run_dir,
                        10,
                        {},
                        False,
                    )
                    ai = run_dir / "ai" / installed
                    self.assertEqual(
                        (ai / "main.nut").read_bytes(), (source_dir / main).read_bytes()
                    )
                    self.assertIn(parameter, (ai / "parameters.nut").read_text())
                    self.assertEqual(
                        [
                            list(pair)
                            for pair in zip(
                                plan["command"], plan["command"][1:], strict=False
                            )
                            if pair[0] == "-d"
                        ],
                        [diagnostics] if diagnostics else [],
                    )

    def test_each_requested_selector_must_match_before_any_run(self) -> None:
        stderr = io.StringIO()
        with (
            patch.object(
                simulation_cli, "scenario_list", return_value=[{"name": "valid"}]
            ),
            patch.object(sys, "argv", ["simulate.py", "valid", "misspelled", "--list"]),
            contextlib.redirect_stderr(stderr),
        ):
            with self.assertRaises(SystemExit) as error:
                simulate.main()
        self.assertEqual(error.exception.code, 2)
        self.assertIn("misspelled", stderr.getvalue())

    def test_exact_and_substring_selectors_preserve_selection(self) -> None:
        output = io.StringIO()
        with (
            patch.object(
                simulation_cli,
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

    def test_end_mismatch_compares_exits_and_full_logs_without_retry(self) -> None:
        calls: list[Path]
        compared: list[Path]
        calls, compared = [], []

        def run(
            scenario: simulate.Scenario,
            binary: Path,
            build: Path,
            folder: Path,
            *args: object,
            **kwargs: object,
        ) -> dict[str, Any]:
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
            patch.object(registry, "scenario_modules", return_value=[]),
            patch.object(scenario_runner, "run_game", side_effect=run),
            patch.object(
                scenario_runner,
                "save_moment",
                side_effect=lambda p: (1, 0, 2 if "candidate" in p.parts else 1),
            ),
            patch.object(
                scenario_runner,
                "compare_saves",
                side_effect=lambda a, b, *args: compared.append(a) or [],
            ),
        ):
            result = simulate.run_scenario(
                cast(
                    simulate.Scenario,
                    {"name": "strict", "short_checkpoint": True, "ticks": 1},
                ),
                cast(dict[str, Path], {"reference": None, "candidate": None}),
                cast(dict[str, Path], {"reference": None, "candidate": None}),
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
    def test_launcher_denies_threads_without_changing_parent_limit(self) -> None:
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
    @unittest.skipUnless(sys.platform == "linux", "ELF archive interposition")
    def test_interposition_reaches_callers_in_the_same_object(self) -> None:
        from simulation.console_adapter import interposed_archive

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "owner.c").write_text(
                "int callee(void) { return 1; }\n"
                "int caller(void) { return callee(); }\n"
            )
            (root / "console.c").write_text(
                "int __real_callee(void);\n"
                "int caller(void);\n"
                "int callee(void) { return 10 + __real_callee(); }\n"
                "int main(void) { return caller() == 11 ? 0 : 1; }\n"
            )
            run = partial(subprocess.run, cwd=root, check=True)
            run(["cc", "-c", "-O2", "-fPIC", "-ffunction-sections", "owner.c"])
            run(["ar", "rc", "libowner.a", "owner.o"])
            archive = interposed_archive(
                root / "libowner.a", root, ("callee",), dict(os.environ)
            )
            run(["cc", "console.c", str(archive), "-o", "console"])
            self.assertEqual(subprocess.run([root / "console"]).returncode, 0)

    def exercise(
        self,
        *,
        custom: bool = False,
        self_compare: bool = False,
        passed: bool = True,
        crash: bool = False,
    ) -> None:
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
                "REFERENCE = ROOT / 'reference-source'\n"
                "REFERENCE_BUILD = ROOT / 'reference'\n"
                "CANDIDATE_BUILD = ROOT / 'build-rust'\n"
                "SANITIZE = False\n"
                "def reference_lock(shared=False): return nullcontext()\n"
                "def environment(): return {}\n"
                "def git(*args):\n"
                "    return (ROOT / ('head' if args[0] == 'rev-parse' else 'status')).read_text()\n"
            )
            driver = ModuleType("fixture_driver")
            driver.__file__ = str(root / "tools/migration.py")
            exec((root / "tools/migration.py").read_text(), vars(driver))
            ai_initial: dict[str, str] = {}
            for folder in ("town-name-observer", "tree-scenario-ai"):
                ai = root / "tools" / folder
                ai.mkdir()
                (ai / "main.nut").write_text(f"{folder}-before")
                ai_initial[folder] = hashlib.sha256(
                    (ai / "main.nut").read_bytes()
                ).hexdigest()
            originals: dict[str, Path]
            assets: dict[str, Path]
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

            invocations: list[str] = []

            def run(
                scenario: simulate.Scenario,
                binaries: dict[str, Path],
                builds: dict[str, Path],
                *args: object,
            ) -> dict[str, Any]:
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
                patch.object(simulation_cli, "ROOT", root),
                patch.object(
                    simulation_cli,
                    "scenario_list",
                    return_value=[
                        {"name": "a", "town_style": 0},
                        {"name": "b", "trees": "commands"},
                    ],
                ),
                patch.object(simulation_cli, "run_scenario", side_effect=run),
                patch.object(sys, "argv", argv),
                patch.object(configuration, "_current", configuration.configuration()),
                contextlib.redirect_stdout(io.StringIO()),
            ):
                if crash:
                    with self.assertRaisesRegex(RuntimeError, "scenario failed"):
                        simulate.main(driver)
                    run = cast(Any, next((root / ".local/simulation").iterdir()))
                    self.assertFalse((run / "reference-runtime").exists())
                    self.assertFalse((run / "candidate-runtime").exists())
                    self.assertFalse((run / "report.json").exists())
                    return
                self.assertEqual(simulate.main(driver), 0 if passed else 1)
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

    def test_default_candidate_survives_rebuild(self) -> None:
        self.exercise()

    def test_custom_candidate_uses_its_own_frozen_runtime(self) -> None:
        self.exercise(custom=True)

    def test_reference_self_needs_no_candidate(self) -> None:
        self.exercise(self_compare=True)

    def test_failed_runs_remove_runtimes_and_preserve_results(self) -> None:
        self.exercise(passed=False)

    def test_unexpected_worker_failure_removes_runtimes(self) -> None:
        self.exercise(crash=True)

    def test_fixture_preparation_removes_runtimes_on_pass_and_fail(self) -> None:
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
                            "SANITIZE = False\n"
                            "REFERENCE = REFERENCE_BUILD = CANDIDATE_BUILD = LOCAL\n"
                        )

                        driver = ModuleType("fixture_driver")
                        driver.__file__ = str(root / "tools/migration.py")
                        exec((root / "tools/migration.py").read_text(), vars(driver))

                        def prepare(*args: object, crash: bool = crash) -> None:
                            out = cast(Path, args[-2])
                            (out / "reference-runtime").mkdir()
                            (out / "fixture.log").write_text("retained output")
                            if crash:
                                raise RuntimeError("fixture preparation failed")

                        with (
                            patch.object(simulation_cli, "ROOT", root),
                            patch.object(
                                simulation_cli,
                                "scenario_list",
                                return_value=[{"name": "a"}],
                            ),
                            patch.object(module, name, side_effect=prepare),
                            patch.object(sys, "argv", ["simulate.py", *arguments]),
                        ):
                            if crash:
                                with self.assertRaisesRegex(
                                    RuntimeError, "preparation failed"
                                ):
                                    simulate.main(driver)
                            else:
                                self.assertEqual(simulate.main(driver), 0)
                        run = next((root / ".local/simulation").iterdir())
                        self.assertFalse((run / "reference-runtime").exists())
                        self.assertEqual(
                            (run / "fixture.log").read_text(), "retained output"
                        )

    def test_pruning_keeps_newest_ten_completed_runs_and_active_outputs(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            completed: list[Path] = []
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
    def test_candidate_profile_entries_name_rust_functions(self) -> None:
        # A renamed export toggles no collection, so every profile reports zero loop Ir.
        source = simulate.ROOT / "rust/openttd-kernels/src"
        for entry in ir.CANDIDATE_ENTRIES:
            *module, name = entry.split("::")
            module = module[1:] if module[:1] == ["openttd_kernels"] else module
            files = (
                [source.joinpath(*module).with_suffix(".rs")]
                if module
                else sorted(source.rglob("*.rs"))
            )
            functions: set[str] = set()
            for path in files:
                if not path.is_file():
                    continue
                text = path.read_text()
                if name not in text:
                    continue
                text = without_comments(text, str(path), strings=True)
                depth = 0
                for token in re.finditer(r"[{}]|\bfn\s+(\w+)\s*[<(]", text):
                    if token[0] == "{":
                        depth += 1
                    elif token[0] == "}":
                        depth -= 1
                    elif depth == 0:
                        functions.add(token[1])
            with self.subTest(entry=entry):
                self.assertIn(name, functions, f"no Rust module function for {entry}")

    def test_record_and_fixed_ceilings(self) -> None:
        from math import inf, nextafter

        for name, ceiling in (("play-test", 1.6), ("generate-test", 1.3)):
            with self.subTest(name=name), patch.dict(SPEED_BUDGETS, {name: 1.25}):
                report = cast(ir.SpeedBudget, speed_budget(name, 1.4375))
                self.assertEqual(report["quiet_pass_mean"], 1.25)
                self.assertEqual(report["record"], 1.25)
                self.assertAlmostEqual(report["change_percent"], 15.0)
                self.assertEqual(report["ceiling"], ceiling)
                self.assertTrue(
                    cast(
                        ir.SpeedBudget, speed_budget(name, ceiling, anchor_ratio=1.25)
                    )["within_ceiling"]
                )
                self.assertFalse(
                    cast(
                        ir.SpeedBudget,
                        speed_budget(name, nextafter(ceiling, inf), anchor_ratio=1.25),
                    )["within_ceiling"]
                )
                self.assertTrue(
                    cast(
                        ir.SpeedBudget,
                        speed_budget(name, 0.95 * ceiling, anchor_ratio=1.25),
                    )["near_ceiling"]
                )
                self.assertFalse(
                    cast(
                        ir.SpeedBudget,
                        speed_budget(
                            name, nextafter(0.95 * ceiling, 0), anchor_ratio=1.25
                        ),
                    )["near_ceiling"]
                )
                self.assertLess(
                    cast(ir.SpeedBudget, speed_budget(name, 0.9))["change_percent"], 0
                )
                self.assertIsNone(speed_budget(name, None))
                self.assertTrue(
                    cast(ir.SpeedBudget, speed_budget(name, 2.0))["within_ceiling"]
                )  # A single wall ratio cannot breach the record.
        self.assertIsNone(speed_budget("unrecorded", 1.0))

    def test_three_pass_record_drives_decisions(self) -> None:
        from simulation.instructions import SPEED_ANCHORS, SPEED_PASS, budget

        name = "play-padhattan-ridge-2016-manual"
        ratios = next(
            ratios for scenario, ratios, _, _ in SPEED_PASS if scenario == name
        )
        # The record restarts on a new host; decisions use the passes it has.
        self.assertIn(len(ratios), (1, 2, 3))
        self.assertAlmostEqual(sum(ratios) / len(ratios), SPEED_BUDGETS[name])
        anchor = SPEED_ANCHORS[name]["anchor"]
        record = cast(
            ir.SpeedAnchor, {"anchor": anchor, "integrated": {"Ir": anchor["Ir"]}}
        )
        # A mean distinct from the latest pass shows which one each figure uses.
        mean = anchor["ratio"] - 0.05
        with patch.dict(SPEED_BUDGETS, {name: mean}):
            report = budget(
                name,
                {"Ir": anchor["Ir"] * 1.001, "identity": anchor["identity"]},
                record,
            )
        self.assertAlmostEqual(report["projected_ratio"], anchor["ratio"] * 1.002)
        self.assertAlmostEqual(report["record"], mean * 1.002)
        self.assertEqual(report["near_ceiling"], report["record"] >= 1.52)
        self.assertTrue(report["within_ceiling"])
        self.assertEqual(report["band"], 1.52)

    def exercise(
        self, *, instructions: bool = False, **failure: bool | int
    ) -> tuple[simulate.Result, int, Any]:
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory)
            scenario = cast(simulate.Scenario, {"name": "speed", "ticks": 74})
            runs: list[dict[str, Any]] = [
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
                patch.object(registry, "scenario_modules", return_value=()),
                patch.object(scenario_runner, "run_game", side_effect=runs * 3) as game,
                patch.object(
                    scenario_runner,
                    "save_moment",
                    side_effect=[(1, 0, 74), (1, 0, 75)] * 3
                    if failure.get("moment")
                    else None,
                    return_value=(1, 0, 74),
                ),
                patch.object(
                    scenario_runner,
                    "compare_saves",
                    return_value=[{"field": "world"}]
                    if failure.get("difference")
                    else [],
                ),
            ):
                result = simulate.run_scenario(
                    scenario,
                    cast(
                        dict[str, Path],
                        {role: role for role in ("reference", "candidate")},
                    ),
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

    def test_speed_ratio_needs_matching_semantic_pairs(self) -> None:
        result, _, _ = self.exercise(instructions=True)
        self.assertEqual(result["plain_speed"]["samples"], [])
        self.assertEqual(result["instruction_speed"]["reference"]["Ir"], 42)
        result, calls, locks = self.exercise()
        self.assertTrue(result["passed"])
        self.assertEqual(calls, 6)
        self.assertTrue(all(call.kwargs == {"isolate": True} for call in locks))
        self.assertEqual(len(result["plain_speed"]["samples"]), 3)
        self.assertEqual(result["plain_speed"]["median_candidate_reference_ratio"], 2.0)
        for failure in cast(
            tuple[dict[str, Any], ...], ({"difference": True}, {"exit_code": 1})
        ):
            with self.subTest(**failure):
                result, calls, _ = self.exercise(**failure)
                self.assertFalse(result["passed"])
                self.assertEqual(calls, 2)  # Preserve the first failing repetition.
                self.assertIsNone(
                    result["plain_speed"]["median_candidate_reference_ratio"]
                )
        for failure in cast(
            tuple[dict[str, Any], ...],
            (
                {"difference": True},
                {"exit_code": 1},
                {"log": True},
                {"moment": True},
            ),
        ):
            with self.subTest(instructions=True, **failure):
                result, calls, _ = self.exercise(instructions=True, **failure)
                self.assertFalse(result["passed"])
                self.assertEqual(calls, 2)


class InstructionSpeedTests(unittest.TestCase):
    def test_fixed_anchor_cumulative_projection_and_replacement(self) -> None:
        anchor: dict[str, Any] = dict(
            head="quiet", ratio=1.2314, Ir=100, identity={"compatible": "same"}
        )
        record: dict[str, Any] = dict(
            anchor=anchor, integrated=dict(head="base", Ir=100)
        )
        measurement: dict[str, Any] = dict(Ir=100, identity={"compatible": "same"})
        for step in range(1, 4):
            measurement["Ir"] = 100 * 1.04**step
            report = ir.budget("play-test", measurement, cast(ir.SpeedAnchor, record))
            record["integrated"] = dict(head=str(step), Ir=measurement["Ir"])
        self.assertAlmostEqual(report["projected_ratio"], 1.5389150592)
        self.assertTrue(report["within_ceiling"])
        self.assertEqual(anchor["Ir"], 100)
        self.assertEqual(ir.projection(cast(ir.Anchor, anchor), 90), 1.2314)
        record["anchor"] = dict(anchor, ratio=1.1, Ir=measurement["Ir"])
        report = ir.budget("play-test", measurement, cast(ir.SpeedAnchor, record))
        self.assertEqual(report["projected_ratio"], 1.1)
        incompatible = dict(measurement, identity={"compatible": "other"})
        with self.assertRaises(ValueError):
            ir.budget("play-test", incompatible, cast(ir.SpeedAnchor, record))

    def test_real_nested_scope_extraction_identity_and_semantic_rejection(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source, binary, run = root / "probe.cpp", root / "openttd", root / "run"
            run.mkdir()
            (root / "CMakeCache.txt").write_text("settings")
            source.write_text(
                "volatile int n; void StateGameLoop_LinkGraphPauseControl(){for(int i=0;i<10000;i++)n=n+1;}\n"
                "void StateGameLoop(){StateGameLoop_LinkGraphPauseControl();}\n"
                "void _GenerateWorld(){for(int i=0;i<20000;i++)n=n+1;}\n"
                'extern "C" void fixture_state_loop(){StateGameLoop();}\n'
                'extern "C" void fixture_generate_world(){_GenerateWorld();}\n'
            )
            obj = root / "probe.o"
            subprocess.run(
                ["g++", "-g", "-O0", "-c", str(source), "-o", str(obj)], check=True
            )
            rust = root / "probe.rs"
            rust.write_text(
                'unsafe extern "C" {fn fixture_state_loop(); fn fixture_generate_world();}\n'
                '#[unsafe(no_mangle)] pub extern "C" fn openttd_rust_session_state_loop_abi(){unsafe{fixture_state_loop();}}\n'
                "pub mod genworld {#[inline(never)] pub fn generate(){unsafe{super::fixture_generate_world();}}}\n"
                "fn main(){genworld::generate(); openttd_rust_session_state_loop_abi();}\n"
            )
            subprocess.run(
                [
                    "rustc",
                    "--edition=2024",
                    "--crate-name=openttd_kernels",
                    "-g",
                    "-Copt-level=0",
                    "-Clink-arg=" + str(obj),
                    str(rust),
                    "-o",
                    str(binary),
                ],
                env=migration.environment(),
                check=True,
            )
            scenario = cast(
                simulate.Scenario, dict(kind="save", ticks=1, save=str(source))
            )
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
            def adapter(build: Path, folder: Path, env: dict[str, str]) -> Path:
                folder.mkdir()
                return Path(shutil.copy2(binary, folder / "adapter"))

            with contextlib.ExitStack() as stack:
                stack.enter_context(patch.object(simulate, "ROOT", root))
                stack.enter_context(patch.object(migration, "REFERENCE_BUILD", root))
                for module in simulate.scenario_modules():
                    if hasattr(module, "adapter"):
                        stack.enter_context(patch.object(module, "adapter", adapter))
                runtime.export(root / "bundle", migration)
            counts: list[int] = []
            scenario["name"] = next(iter(SPEED_BUDGETS))

            def collect(
                case: simulate.Scenario,
                binaries: dict[str, Path],
                builds: dict[str, Path],
                out: Path,
                *args: object,
            ) -> None:
                self.assertTrue(args[-1])
                frozen = binaries["candidate"]
                context = frozen, builds["candidate"], run, env
                for reference in (True, False):
                    for kind in ("save", "generate"):
                        case = cast(simulate.Scenario, dict(scenario, kind=kind))
                        command, info = ir.prepare(
                            case, [str(frozen)], *context, reference
                        )
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
                patch.object(simulation_cli, "scenario_list", return_value=[scenario]),
                patch.object(simulation_cli, "run_scenario", side_effect=collect),
                patch.object(sys, "argv", cli),
                self.assertRaisesRegex(RuntimeError, "collected frozen profiles"),
            ):
                simulate.main()
            # Wildcard nested toggle loses this work.
            for play, generation in (counts[:2], counts[2:]):
                self.assertGreater(play, 50000)
                self.assertGreater(generation, play * 2)
            command, _ = ir.prepare(scenario, [str(binary)], *context, False)
            baseline = ir.identity(scenario, *context, Path(command[0]))
            candidate_identity = ir.identity(
                scenario, *context, Path(command[0]), reference=False
            )
            self.assertEqual(baseline["compatible"], candidate_identity["compatible"])
            self.assertNotEqual(ir.flags(scenario), ir.flags(scenario, reference=False))
            self.assertEqual(
                candidate_identity["entries"], ir.flags(scenario, reference=False)
            )
            (root / ".cargo").mkdir()
            for field, value in (("ticks", 2), ("console", ["unpause"])):
                changed = ir.identity(
                    cast(simulate.Scenario, dict(scenario, **{field: value})),
                    *context,
                    Path(command[0]),
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
                    # Reject before extraction/cache writes.
                    ir.finish({}, cast(simulate.Result, failure))

    def test_reference_cache_write_stays_within_the_cache_disk(self) -> None:
        rename = Path.replace

        def same_disk(source: Path, target: Path) -> Path:
            if Path(source).parent != Path(target).parent:
                raise OSError(18, "Invalid cross-device link")
            return rename(source, target)

        with tempfile.TemporaryDirectory() as directory:
            run, cache = Path(directory) / "simulation", Path(directory) / "cache"
            run.mkdir()
            runs = {
                role: {
                    "dir": run,
                    "instructions": dict(
                        identity={"compatible": role}, cached=value, cache=cache / "k"
                    ),
                }
                for role, value in (("reference", 40), ("candidate", 50))
            }
            clean = dict(differences=[], known_failures=[], problems=[])
            with patch.object(Path, "replace", autospec=True, side_effect=same_disk):
                self.assertEqual(
                    ir.finish(
                        cast(dict[simulate.Role, simulate.Run], runs),
                        cast(simulate.Result, clean),
                    )["Ir_ratio"],
                    1.25,
                )
            self.assertEqual(json.loads((cache / "k").read_text()), 40)
            self.assertEqual([path.name for path in cache.iterdir()], ["k"])

    def test_main_checkout_and_worktree_record_one_settings_identity(self) -> None:
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

    def test_rust_only_cache_keeps_anchored_compatible_spelling(self) -> None:
        anchored = (
            "GITCOMMAND:FILEPATH=/usr/bin/git\n\n//Global directory\n"
            "OPTION_PACKAGE_DEPENDENCIES:BOOL=OFF\n\n" + ir.RUST_OPTION + "//Survey\n"
            "//Details about finding Freetype\nFIND_PACKAGE_MESSAGE_DETAILS_Freetype:INTERNAL=[f]\n"
            "//Details about finding ICU\nGITCOMMAND-ADVANCED:INTERNAL=1\n"
            "//Test HAVE\n"
        )
        rust_only = (
            "GITCOMMAND:FILEPATH=/usr/bin/git\n\n"
            "//Git command line client\nGIT_EXECUTABLE:FILEPATH=/usr/bin/git\n\n"
            "//Global directory\nOPTION_PACKAGE_DEPENDENCIES:BOOL=OFF\n\n//Survey\n"
            "//Details about finding Freetype\nFIND_PACKAGE_MESSAGE_DETAILS_Freetype:INTERNAL=[f]\n"
            "//Details about finding Git\nFIND_PACKAGE_MESSAGE_DETAILS_Git:INTERNAL=[/usr/bin/git][v2()]\n"
            "//Details about finding ICU\nGITCOMMAND-ADVANCED:INTERNAL=1\n"
            "//ADVANCED property for variable: GIT_EXECUTABLE\nGIT_EXECUTABLE-ADVANCED:INTERNAL=1\n"
            "//Test HAVE\n"
        )
        self.assertEqual(ir.anchored_cache(anchored), anchored)
        self.assertEqual(ir.anchored_cache(rust_only), anchored)
        with tempfile.TemporaryDirectory() as directory:
            build = Path(directory)
            for name, text in (
                ("CMakeCache.txt", rust_only),
                ("rust-build-configuration.txt", "c"),
                ("rust-toolchain.txt", "t"),
                ("openttd-rust", "game"),
            ):
                (build / name).write_text(text)
            with (
                patch.object(ir, "verify_debug"),
                patch.object(migration, "rust_archive"),
            ):
                ir.record_build(build, build, "candidate", {"CARGO_HOME": "-"})
                settings = json.loads((build / "instruction-build.json").read_text())
            self.assertTrue(settings["settings"].startswith(anchored + "ct"))

    def test_pkg_config_scratch_keeps_real_build_inputs_distinct(self) -> None:
        settings = (
            "CMAKE_CXX_COMPILER:FILEPATH=/usr/bin/c++\n"
            "CMAKE_CXX_FLAGS:STRING=-O2\n"
            "CURL_LIBRARY_RELEASE:FILEPATH=/usr/lib/libcurl.so\n"
            "CURL_PREFIX:INTERNAL=/usr\n"
            "FIND_PACKAGE_MESSAGE_DETAILS_CURL:INTERNAL=[libcurl][8.5]\n"
            "prefix_result:INTERNAL=AsynchDNS;GSS-API;HSTS;HTTP2;HTTPS-proxy;IDN;"
            "IPv6;Kerberos;Largefile;NTLM;PSL;SPNEGO;SSL;TLS-SRP;UnixSockets;alt-svc;"
            "brotli;libz;threadsafe;zstd\n"
        )
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            build, run, tools = (root / name for name in ("build", "run", "tools"))
            for folder in (build, run, tools):
                folder.mkdir()
            binary = build / "openttd-rust"
            binary.write_bytes(b"game")
            valgrind = tools / "valgrind"
            valgrind.write_bytes(b"dispatcher")
            for name in ("callgrind-test", "vgpreload_core-test.so"):
                (tools / name).write_bytes(b"tool")
            metadata = build / "instruction-build.json"
            library = root / "libcurl.so"
            library.write_bytes(b"runtime library")
            env = {"VALGRIND_LIB": str(tools)}
            scenario = cast(simulate.Scenario, dict(kind="generate", ticks=1))

            def measured(
                text: str,
                reference: bool = False,
                details: ir.IdentityDetails | None = None,
            ) -> dict[str, Any]:
                metadata.write_text(
                    json.dumps(dict(settings=text, executable=ir.digest(b"game")))
                )
                return ir.identity(
                    scenario, binary, build, run, env, valgrind, reference, details
                )

            def output(command: list[str], **kwargs: object) -> str:
                return f"{library} (0x123)" if command[0] == "ldd" else "tool-v1"

            with patch.object(ir.subprocess, "check_output", side_effect=output):
                anchored = measured(settings)
                details = cast(ir.IdentityDetails, {})
                self.assertEqual(measured(settings, details=details), anchored)
                self.assertEqual(details["tool"][:2], (str(valgrind), "tool-v1"))
                self.assertEqual(
                    details["tool_files"],
                    [
                        str(tools / name)
                        for name in ("callgrind-test", "vgpreload_core-test.so")
                    ],
                )
                self.assertEqual(details["configuration"], settings)
                changed = cast(ir.IdentityDetails, {})
                measured(settings.replace("-O2", "-O3"), details=changed)
                self.assertIn("CMAKE_CXX_FLAGS:STRING=-O3", changed["configuration"])
                self.assertNotEqual(changed["configuration"], details["configuration"])
                self.assertEqual(
                    measured(settings, True)["compatible"], anchored["compatible"]
                )
                drifted = settings.replace(ir.ANCHORED_PREFIX_RESULT, "/usr")
                self.assertEqual(measured(drifted), anchored)
                self.assertNotEqual(measured(drifted, True), measured(settings, True))
                for before, after in (
                    ("/usr/bin/c++", "/usr/bin/clang++"),
                    ("-O2", "-O3"),
                    ("/usr/lib/libcurl.so", "/other/libcurl.so"),
                    ("CURL_PREFIX:INTERNAL=/usr", "CURL_PREFIX:INTERNAL=/other"),
                    ("[8.5]", "[8.6]"),
                    ("prefix_result:INTERNAL", "prefix_result:STRING"),
                ):
                    with self.subTest(input=before):
                        self.assertNotEqual(
                            measured(drifted.replace(before, after)), anchored
                        )
                for name in ("callgrind-test", "vgpreload_core-test.so"):
                    path = tools / name
                    path.write_bytes(b"different tool")
                    self.assertNotEqual(measured(drifted), anchored)
                    path.write_bytes(b"tool")
                library.write_bytes(b"changed runtime library")
                self.assertNotEqual(measured(drifted), anchored)
                library.write_bytes(b"runtime library")
                (run / "script").write_text("changed input")
                self.assertNotEqual(measured(drifted), anchored)
                (run / "script").unlink()
                with patch.object(
                    ir.subprocess, "check_output", return_value="tool-v2"
                ):
                    self.assertNotEqual(measured(drifted), anchored)
                binary.write_bytes(b"changed executable")
                with self.assertRaisesRegex(ValueError, "stale instruction build"):
                    measured(drifted)

    def test_effective_compiled_debug_rejects_overrides_and_stripping(self) -> None:
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

    def test_reused_build_restores_captured_release_debug(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            build = Path(directory)
            (build / "build.ninja").touch()
            (build / "CMakeCache.txt").write_text(
                f"CMAKE_HOME_DIRECTORY:PATH={build}\nRUST_TARGET:STRING=x86_64-unknown-linux-gnu\n"
            )
            configuration = build / "rust-build-configuration.txt"
            profile_env = {"CARGO_PROFILE_RELEASE_DEBUG": "line-tables-only"}
            configured = migration.configured_as
            for effective, expected in (("line-tables-only", True), ("<unset>", False)):
                configuration.write_text(f"CARGO_PROFILE_RELEASE_DEBUG={effective}\n")
                self.assertEqual(configured(build, build, [], profile_env), expected)
                self.assertEqual(configured(build, build, [], {}), not expected)


class SpeedPassTests(unittest.TestCase):
    def test_preparation_diagnoses_anchor_before_games(self) -> None:
        from simulation import speed

        name = "play-test"
        identities = {name: {"candidate": {"compatible": "current"}}}
        fields = cast(
            ir.IdentityDetails,
            dict(
                configuration="CMAKE_CXX_COMPILER:FILEPATH=/usr/bin/g++\nCMAKE_CXX_FLAGS:STRING=-O3\n",
                tool=(
                    "/selected/valgrind",
                    "valgrind-3.22",
                    "dispatcher-hash",
                    ["core-hash"],
                ),
                tool_files=["/selected/vgpreload_core-test.so"],
            ),
        )
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory)

            def copy_runtime(
                build: Path, binary: Path, target: Path, **kwargs: object
            ) -> None:
                target.mkdir()
                (target / "instruction-build.json").write_text("{}")

            def prepare_inputs(
                out: Path,
                binaries: dict[str, Path],
                builds: dict[str, Path],
                env: dict[str, str],
                details: dict[str, dict[str, ir.IdentityDetails]],
            ) -> tuple[list[simulate.Scenario], dict[str, dict[str, Any]]]:
                details[name] = {"candidate": fields}
                return [], identities

            with (
                patch.object(migration, "git", side_effect=["head", ""]),
                patch.object(
                    migration, "reference_lock", return_value=contextlib.nullcontext()
                ),
                patch.object(speed.core, "copy_runtime", side_effect=copy_runtime),
                patch.object(speed.core, "adapter_modules", return_value=[]),
                patch.object(speed, "prepare_inputs", side_effect=prepare_inputs),
                patch.object(
                    ir,
                    "SPEED_ANCHORS",
                    {name: {"anchor": {"identity": {"compatible": "anchor"}}}},
                ),
                patch.object(speed.core, "run_game") as game,
                contextlib.redirect_stdout(io.StringIO()) as printed,
            ):
                self.assertEqual(speed.prepare(out, None, migration), 0)
            game.assert_not_called()
            self.assertIn("current current, anchor anchor", printed.getvalue())
            self.assertIn("before profiling", printed.getvalue())
            for diagnostic in (
                "mismatch cause is unknown",
                "/selected/valgrind",
                "reported version=valgrind-3.22",
                "/selected/vgpreload_core-test.so: sha256=core-hash",
                "CMAKE_CXX_COMPILER:FILEPATH=/usr/bin/g++",
                "CMAKE_CXX_FLAGS:STRING=-O3",
            ):
                self.assertIn(diagnostic, printed.getvalue())
            self.assertEqual(
                json.loads((out / "manifest.json").read_text())["identity_details"][
                    name
                ]["candidate"]["configuration"],
                fields["configuration"],
            )
            self.assertEqual(
                json.loads((out / "manifest.json").read_text())["runtime_identities"],
                identities,
            )

    def test_collection_prepares_before_hold_and_compares_after_release(self) -> None:
        from simulation import speed

        events: list[str | tuple[str, str]]
        held, events = False, []

        class Hold:
            def __init__(self, log: Path) -> None:
                pass

            def start(self) -> None:
                nonlocal held
                held = True
                events.append("start")

            def release(self) -> None:
                nonlocal held
                held = False
                events.append("release")

        def prepare(
            scenario: simulate.Scenario,
            binary: str,
            build: str,
            folder: Path,
            *args: object,
            **kwargs: object,
        ) -> dict[str, Any]:
            self.assertFalse(held)
            return {"dir": folder, "role": binary, "scenario": scenario["name"]}

        def execute(plan: dict[str, Any]) -> dict[str, Any]:
            self.assertTrue(held)
            events.append((plan["scenario"], plan["role"]))
            return plan

        def read(run: dict[str, Any]) -> dict[str, Any]:
            self.assertFalse(held)
            return dict(
                run,
                exit=0,
                seconds=2 if run["role"] == "candidate" else 1,
                snapshots=[run["dir"] / "save/autosave/exit.sav"],
                log=["SAME"],
                stdout=b"",
                command=[],
            )

        compared: list[Path] = []

        def compare(reference: Path, candidate: Path, *args: object) -> list[Any]:
            self.assertFalse(held)
            self.assertEqual(reference.parts[:-4], candidate.parts[:-4])
            compared.append(reference)
            return []

        scenarios = cast(
            list[simulate.Scenario],
            [{"name": f"play-{i}", "ticks": 1} for i in range(6)],
        )
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(speed, "Hold", Hold),
            patch.object(registry, "scenario_modules", return_value=[]),
            patch.object(simulate, "prepare_game", side_effect=prepare),
            patch.object(simulate, "execute_game", side_effect=execute),
            patch.object(simulate, "read_game", side_effect=read),
            patch.object(scenario_runner, "save_moment", return_value=(1, 0, 74)),
            patch.object(scenario_runner, "compare_saves", side_effect=compare),
        ):
            out = Path(directory)
            roles = cast(dict[str, Path], {role: role for role in speed.ROLES})
            speed.collect_games(out, scenarios, roles, roles, {})
            self.assertEqual(
                events,
                ["start"]
                + [
                    (scenario["name"], role)
                    for group in range(4)
                    for scenario in scenarios
                    for pair in range(3)
                    for role in (
                        ("candidate", "reference")
                        if (group + pair) % 2
                        else ("reference", "candidate")
                    )
                ]
                + ["release"],
            )
            self.assertEqual(len(compared), 72)
            for group in range(4):
                results = json.loads((out / f"group-{group}.json").read_text())[
                    "results"
                ]
                self.assertTrue(all(result["passed"] for result in results))
                self.assertTrue(
                    all(
                        result["plain_speed"]["median_candidate_reference_ratio"] == 2
                        for result in results
                    )
                )

    def test_preparation_accepts_only_stale_anchor_failures(self) -> None:
        from simulation import speed

        def result(
            passed: bool,
            problems: Sequence[str] = (),
            differences: Sequence[dict[str, Any]] = (),
            measured: bool = True,
            known_failures: Sequence[dict[str, Any]] = (),
        ) -> dict[str, Any]:
            return {
                "passed": passed,
                "problems": list(problems),
                "differences": list(differences),
                "known_failures": list(known_failures),
                "instruction_speed": {"candidate": {"Ir": 1}} if measured else None,
            }

        stale = (
            "harness error: ValueError: instruction anchor identity mismatch; "
            "quiet speed pass required"
        )
        accepted = [[result(True)], [result(True), result(False, [stale])]]
        rejected = [
            [],
            [result(False, ["harness error: OSError: copy failed"])],
            [result(False, [stale, "harness error: OSError: copy failed"])],
            [result(False, [stale + "; copy failed"])],
            [result(False, ["copy failed: " + stale])],
            [result(False, [stale.replace("ValueError", "OSError")])],
            [result(False, [stale], differences=[{"field": "line"}])],
            [result(False, [stale], known_failures=[{"issue": 1}])],
            [result(False, [stale], measured=False)],
            [result(False)],
        ]
        for results in accepted:
            self.assertTrue(speed.profile_passed({"results": results}))
        for results in rejected:
            self.assertFalse(speed.profile_passed({"results": results}))

    def test_collection_rejects_changed_profile_and_runtime_before_hold(self) -> None:
        from simulation import speed

        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory)
            profile = out / "profile.json"
            profile.write_text("original profile")
            manifest = dict(
                head="head",
                profile=str(profile),
                profile_sha256=ir.digest(profile.read_bytes()),
                ld_library_path="libs",
                runtime_identities={"play-a": "original"},
            )
            (out / "manifest.json").write_text(json.dumps(manifest))
            for changed in ("profile", "runtime"):
                profile.write_text(
                    "changed" if changed == "profile" else "original profile"
                )
                with (
                    self.subTest(changed=changed),
                    patch.object(migration, "git", side_effect=["head", ""]),
                    patch.object(
                        migration,
                        "environment",
                        return_value={"LD_LIBRARY_PATH": "libs"},
                    ),
                    patch.object(
                        speed,
                        "prepare_inputs",
                        return_value=([], {"play-a": "changed"}),
                    ),
                    patch.object(migration, "COMMON_LOCAL", out),
                    patch.object(speed, "collect_games") as games,
                    self.assertRaisesRegex(SystemExit, "changed after preparation"),
                ):
                    speed.collect(out, migration)
                games.assert_not_called()

    def test_collection_refuses_an_already_collected_bundle(self) -> None:
        from simulation import speed

        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory)
            profile = out / "profile.json"
            profile.write_text("profile")
            manifest = dict(
                head="head",
                profile=str(profile),
                profile_sha256=ir.digest(profile.read_bytes()),
                ld_library_path="",
                runtime_identities={},
            )
            (out / "manifest.json").write_text(json.dumps(manifest))
            (out / "observations.jsonl").write_text('{"unheld": {"1": "cc1plus"}}\n')
            with (
                patch.object(migration, "git", side_effect=["head", ""]),
                patch.object(speed, "collect_games") as games,
                self.assertRaisesRegex(SystemExit, "already collected"),
            ):
                speed.collect(out, migration)
            games.assert_not_called()

    def test_preparation_identities_match_each_profiled_role(self) -> None:
        from simulation import speed

        calls: list[tuple[str, bool]] = []

        def prepare(
            scenario: simulate.Scenario,
            command: list[str],
            binary: str,
            build: str,
            folder: Path,
            env: dict[str, str],
            reference: bool,
            details: ir.IdentityDetails | None = None,
        ) -> tuple[list[str], dict[str, Any]]:
            calls.append((binary, reference))
            if details is not None:
                details["configuration"] = binary
            identity: dict[str, Any] = {"compatible": binary, "executable": binary}
            if not reference:
                identity["entries"] = ["candidate"]
            return command, {"identity": identity}

        scenario = {"name": "play-a", "kind": "play"}
        roles = cast(dict[str, Path], {role: role for role in speed.ROLES})
        diagnostics: dict[str, dict[str, ir.IdentityDetails]] = {}
        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(speed, "speed_scenarios", return_value=[scenario]),
            patch.object(registry, "scenario_modules", return_value=[]),
            patch.object(simulate, "prepare_scenario", return_value=scenario),
            patch.object(simulate, "prepare_run_dir"),
            patch.object(speed.instructions, "prepare", side_effect=prepare),
        ):
            _, identities = speed.prepare_inputs(Path(directory), roles, roles, {})
            _, diagnosed = speed.prepare_inputs(
                Path(directory), roles, roles, {}, diagnostics
            )
        self.assertEqual(diagnosed, identities)
        self.assertEqual(
            diagnostics["play-a"]["candidate"]["configuration"], "candidate"
        )
        self.assertEqual(
            diagnostics["play-a"]["reference"]["configuration"], "reference"
        )
        self.assertEqual(
            sorted(calls), [("candidate", False)] * 2 + [("reference", True)] * 2
        )
        self.assertNotIn("entries", identities["play-a"]["reference"])
        self.assertEqual(identities["play-a"]["candidate"]["entries"], ["candidate"])

    def test_hold_scans_before_start_returns_and_resumes_on_observer_error(
        self,
    ) -> None:
        from simulation import speed

        with (
            tempfile.TemporaryDirectory() as directory,
            patch.object(speed.os, "kill") as kill,
        ):
            hold = speed.Hold(Path(directory) / "observations.jsonl")

            def scan(*args: object) -> dict[str, Any]:
                if not hold.stopped:
                    hold.stopped.add(123)
                    return {}
                raise OSError("observation failed")

            with patch.object(hold, "scan", side_effect=scan):
                hold.start()
                self.assertEqual(hold.stopped, {123})
                with self.assertRaisesRegex(RuntimeError, "observation failed"):
                    hold.release()
            kill.assert_called_once_with(123, speed.signal.SIGCONT)

    def test_anchor_is_median_of_group_medians(self) -> None:
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
                json.dumps({"load": [0.5, 0, 0], "unheld": [], "overlap": {}}) + "\n"
            )

            def write(group: int, ratios: list[float], passed: bool = True) -> None:
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
            with (
                contextlib.redirect_stdout(io.StringIO()) as printed,
                patch.object(
                    ir,
                    "SPEED_PASS",
                    (
                        ("play-a", (0.8, 0.9, 1.0), 1, "old"),
                        ("generate-b", (0.1, 0.2, 0.3), 1, "old"),
                    ),
                ),
            ):
                speed.summarize(out, manifest, names, len(groups))
            rows = json.loads((out / "anchors.json").read_text())["rows"]
            self.assertEqual(
                rows,
                [
                    ["play-a", 2.5, 10, "play-a-id"],
                    ["generate-b", 1.625, 11, "generate-b-id"],
                ],
            )
            self.assertIn(
                '    ("play-a", (0.9, 1.0, 2.5), 10, "play-a-id"),', printed.getvalue()
            )
            self.assertIn(
                '    ("generate-b", (0.2, 0.3, 1.625), 11, "generate-b-id"),',
                printed.getvalue(),
            )
            for incomplete in (3, len(groups)):
                with self.subTest(groups=incomplete), self.assertRaises(SystemExit):
                    write(3, groups[3], passed=incomplete == 3)
                    speed.summarize(out, manifest, names, incomplete)
            write(3, groups[3])
            (out / "observations.jsonl").write_text(
                json.dumps(
                    {"load": [0, 0, 0], "unheld": {"123": "ninja"}, "overlap": {}}
                )
                + "\n"
            )
            with self.assertRaisesRegex(SystemExit, "no anchors"):
                speed.summarize(out, manifest, names, len(groups))

            for comm, preexisting, parent, ticks, state, rejected in (
                ("cc1plus", False, 1, 200, "R", True),
                ("cc1plus", False, 1, 200, "T", True),
                ("cc1plus", False, 1, 200, "Z", True),
                ("cc1plus", False, 1, 0, "T", False),
                ("cc1plus", True, 1, 0, "T", False),
                ("cc1plus", True, 1, 1, "T", True),
                ("unrelated", True, 1, 200, "R", False),
                ("openttd", False, os.getpid(), 200, "R", False),
                ("openttd", False, os.getpid(), 200, "Z", False),
            ):
                with self.subTest(
                    comm=comm, preexisting=preexisting, parent=parent, state=state
                ):
                    log = out / "observations.jsonl"
                    log.unlink()
                    (out / "anchors.json").unlink(missing_ok=True)
                    row = dict(
                        comm=comm, state="R", parent=parent, uid=os.getuid(), ticks=0
                    )
                    hold = speed.Hold(log)
                    hold.hz = 100
                    with (
                        patch.object(speed, "processes") as processes,
                        patch.object(speed.os, "kill") as kill,
                    ):
                        processes.return_value = {123: row} if preexisting else {}
                        before = hold.scan({}, 1, True)
                        processes.return_value = {
                            123: dict(row, ticks=ticks, state=state)
                        }
                        before = hold.scan(before, 2)
                        processes.return_value = {}
                        hold.scan(before, 2)
                        hold.release()
                    entries = [
                        json.loads(line) for line in log.read_text().splitlines()
                    ]
                    self.assertFalse(any(entry["unheld"] for entry in entries))
                    if comm == "unrelated":
                        self.assertEqual(entries[1]["busy"]["123"], [comm, 1.0, False])
                    if comm == "cc1plus":
                        self.assertEqual(
                            kill.call_args_list,
                            [
                                unittest.mock.call(123, speed.signal.SIGSTOP),
                                unittest.mock.call(123, speed.signal.SIGCONT),
                            ]
                            if preexisting or state == "R"
                            else [],
                        )
                    with contextlib.redirect_stdout(io.StringIO()) as printed:
                        if rejected:
                            with self.assertRaisesRegex(SystemExit, "no anchors"):
                                speed.summarize(out, manifest, names, len(groups))
                            self.assertFalse((out / "anchors.json").exists())
                            self.assertEqual(printed.getvalue(), "")
                        else:
                            self.assertEqual(
                                speed.summarize(out, manifest, names, len(groups)), 0
                            )


class DrawingWorkloadTests(unittest.TestCase):
    def test_drawing_ratio_has_no_speed_ceiling(self) -> None:
        from simulation import drawing

        self.assertIsNone(ir.speed_budget(drawing.NAME, 1.4))
        self.assertIsNone(ir.speed_budget(drawing.NAME, 1.4, 1.2, 1.1))
        anchor = dict(ratio=1.1, Ir=100, identity={"compatible": "drawing"})
        report = ir.budget(
            drawing.NAME,
            dict(Ir=110, identity=anchor["identity"]),
            cast(ir.SpeedAnchor, dict(anchor=anchor, integrated=dict(Ir=100))),
        )
        self.assertEqual(set(report), {"projected_ratio"})
        self.assertAlmostEqual(report["projected_ratio"], 1.32)

    def test_profiled_launch_counts_drawing_in_both_builds(self) -> None:
        from simulation import drawing

        (scenario,) = cast(list[simulate.Scenario], drawing.scenarios(False))
        self.assertIn(scenario, simulate.scenario_list(False))
        self.assertIn(scenario["name"], ir.PROFILED)
        for reference in (True, False):
            self.assertEqual(
                ir.flags(scenario, reference)[-1], f"--toggle-collect={ir.DRAW_ENTRY}"
            )
        with tempfile.TemporaryDirectory() as directory:
            plan = simulate.prepare_game(
                scenario, Path("openttd"), Path(directory), Path(directory) / "run", 1
            )
        command = plan["command"]
        self.assertNotIn("-vnull:ticks=1", command)
        self.assertIn("-vsdl:no_threads", command)
        self.assertEqual(command[command.index("-b") + 1], drawing.BLITTER)
        self.assertEqual(plan["env"]["SDL_VIDEODRIVER"], "dummy")

    def test_images_compare_byte_for_byte_and_require_equal_graphics(self) -> None:
        from simulation import drawing

        scenario = dict(draw=True)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for role in ("reference", "candidate"):
                (root / role / "baseset").mkdir(parents=True)
                (root / role / "baseset/opengfx.grf").write_bytes(b"gfx")
                (root / role / "screenshot").mkdir()
            builds = {role: root / role for role in ("reference", "candidate")}
            save = simulate.ROOT / "migration/saves/padhattan-ridge-1996.sav"
            prepared = drawing.prepare(
                dict(scenario, name="draw", save=str(save)), {}, builds, root, 1, {}, {}
            )
            self.assertNotIn("GLOG", simulate.read_save(Path(prepared["save"])))
            (root / "candidate/baseset/opengfx.grf").write_bytes(b"other")
            with self.assertRaisesRegex(RuntimeError, "base graphics differ"):
                drawing.prepare(scenario, {}, builds, root, 1, {}, {})
            header = bytes(18) + (3).to_bytes(4, "little") + (2).to_bytes(4, "little")
            for candidate, equal in ((b"same", True), (b"diff", False)):
                result: dict[str, Any] = dict(differences=[])
                for role, body in (("reference", b"same"), ("candidate", candidate)):
                    image = root / role / "screenshot/drawing.bmp"
                    image.write_bytes(header + body)
                    drawing.check(
                        scenario, dict(dir=root / role), "plain", role, result
                    )
                self.assertEqual(result["plain_image"]["width"], 3)
                self.assertEqual(result["plain_image"]["height"], 2)
                self.assertEqual(not result["differences"], equal)
                self.assertEqual(image.exists(), not equal)
            self.assertEqual(result["differences"][0]["element"], "26")
            image.unlink()
            with self.assertRaisesRegex(RuntimeError, "candidate wrote screenshots"):
                drawing.check(
                    scenario, dict(dir=root / "candidate"), "plain", "candidate", {}
                )


class PlatformSubsetTests(unittest.TestCase):
    def test_generation_requires_no_jobs_before_and_after_initialization(self) -> None:
        for recalc_time, jobs in (
            (0, {}),
            (18000, {}),
            (18000, {0: {"join_date": 999999}}),
        ):
            with (
                self.subTest(recalc_time=recalc_time, jobs=jobs),
                patch.object(platform_subset.core, "read_save", return_value={}),
                patch.object(
                    platform_subset.core,
                    "rows",
                    side_effect=lambda _state, chunk, recalc_time=recalc_time, jobs=jobs: (
                        {
                            "PATS": {0: {"linkgraph.recalc_time": recalc_time}},
                            "DATE": {0: {"economy_date": 0}},
                            "LGRJ": jobs,
                        }[chunk]
                    ),
                ),
            ):
                case = cast(
                    platform_subset.Scenario, {"kind": "generate", "ticks": 56388}
                )
                run = cast(simulate.Run, {"snapshots": [Path("checkpoint.sav")]})
                if jobs:
                    with self.assertRaisesRegex(ValueError, "unexpectedly has"):
                        platform_subset.check_no_join(case, run)
                else:
                    platform_subset.check_no_join(case, run)

    def test_prepared_inputs_are_identical_and_reject_a_due_join(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            first = platform_subset.scenarios(root / "one")
            second = platform_subset.scenarios(root / "two")
            for a, b in zip(first, second, strict=True):
                self.assertEqual(a["ticks"], 56388)
                self.assertEqual(a["settings"]["gui"]["threaded_saves"], "false")
                if a["kind"] != "save":
                    continue
                source = Path(a["save"])
                self.assertEqual(source.read_bytes(), Path(b["save"]).read_bytes())
                platform_subset.check_no_join(
                    a, cast(simulate.Run, {"snapshots": [source]})
                )
                jobs = simulate.rows(source, "LGRJ")
                target = root / "due.sav"
                simulate.patch(
                    source,
                    target,
                    {"LGRJ": {next(iter(jobs)): {"join_date": a["platform_end_date"]}}},
                )
                with self.assertRaisesRegex(ValueError, "join due"):
                    platform_subset.check_no_join(
                        a, cast(simulate.Run, {"snapshots": [target]})
                    )

    def test_stored_capture_requires_every_record_and_snapshot(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            value = platform_subset.identity()
            for name in platform_subset.NAMES:
                for mode, _ in platform_subset.MODES:
                    key = f"{name}/{mode}"
                    folder = root / "captures" / key
                    (folder / "save/autosave").mkdir(parents=True)
                    (folder / "stdout.log").write_bytes(b"")
                    (folder / "stderr.log").write_bytes(b"")
                    (folder / "save/autosave/exit.sav").write_bytes(b"snapshot")
                    value["captures"][key] = {
                        "exit": 0,
                        "seconds": 1.0,
                        "command": [],
                        "snapshots": ["exit.sav"],
                    }
            manifest = root / "bundle.json"
            manifest.write_text(json.dumps(value))
            self.assertEqual(platform_subset.load_bundle(root), value)
            first = next(iter(value["captures"]))
            (root / "captures" / first / "save/autosave/exit.sav").unlink()
            with self.assertRaisesRegex(ValueError, "Missing or failed"):
                platform_subset.load_bundle(root)
            value["captures"].pop(first)
            manifest.write_text(json.dumps(value))
            with self.assertRaisesRegex(ValueError, "omitted"):
                platform_subset.load_bundle(root)


class CommunityInputTests(unittest.TestCase):
    def test_reload_requires_the_generation_package_manifest(self) -> None:
        scenario = community_ai.scenarios(False)[-1]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = root / "packages.json"
            original = json.loads((community_ai.INPUTS / manifest.name).read_text())
            for change in ("none", "newgrf", "ai"):
                rows = json.loads(json.dumps(original))
                row = next(
                    r
                    for r in rows
                    if r["kind"] == ("ai" if change == "ai" else "newgrf")
                )
                if change != "none":
                    row["sha256"] = "0" * 64
                manifest.write_text(json.dumps(rows))
                with (
                    self.subTest(change=change),
                    patch.object(community_ai, "INPUTS", root),
                ):
                    if change == "ai":
                        with self.assertRaisesRegex(RuntimeError, "packages differ"):
                            community_ai.prepare(scenario, {}, {}, root, 1, {}, {})
                    else:
                        community_ai.prepare(scenario, {}, {}, root, 1, {}, {})

    def test_download_and_cached_archive_hashes_are_required(self) -> None:
        payload = b"pinned package bytes"
        package = community_ai.Package(
            name="Test+ Content",
            version="0.3.5",
            kind="newgrf",
            uid="12345678",
            md5="1" * 32,
            sha256=hashlib.sha256(payload).hexdigest(),
            license="GPL-2.0-or-later",
            metadata="test",
            publisher="test",
            dependencies=[],
            archive_filename="12345678-Test_Content-VERSION.tar.gz",
        )
        self.assertEqual(
            package.url,
            "https://bananas-cdn.openttd.org/newgrf/12345678/"
            "11111111111111111111111111111111/12345678-Test_Content-VERSION.tar.gz",
        )
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cache = root / ".local/community-ai" / package.filename
            for invalid in (True, False):

                def download(
                    command: list[str], *, check: bool, invalid: bool = invalid
                ) -> None:
                    Path(command[-1]).write_bytes(b"corrupt" if invalid else payload)

                with (
                    self.subTest(invalid=invalid),
                    patch.object(simulate, "ROOT", root),
                    patch.object(community_ai, "packages", return_value=[package]),
                    patch.object(
                        community_ai.subprocess, "run", side_effect=download
                    ) as curl,
                ):
                    if invalid:
                        with self.assertRaisesRegex(
                            RuntimeError, "download hash mismatch"
                        ):
                            community_ai.fetch()
                        self.assertFalse(cache.exists())
                    else:
                        self.assertEqual(
                            community_ai.fetch(), {package.filename: cache}
                        )
                        self.assertEqual(cache.read_bytes(), payload)
                        community_ai.fetch()
                        self.assertEqual(curl.call_count, 1)
                        cache.write_bytes(b"corrupt")
                        with self.assertRaisesRegex(
                            RuntimeError, "cached input hash mismatch"
                        ):
                            community_ai.fetch()

    def test_newgrf_archives_preserve_notices_without_evidence_content(self) -> None:
        import ci_native

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            scripts = root / "scripts"
            scripts.mkdir()
            archive = root / "fixture.tar.gz"
            with tarfile.open(archive, "w:gz") as tar:
                for name in ("fixture.grf", "readme.txt", "license.txt"):
                    data = b"third-party input"
                    member = tarfile.TarInfo("package/" + name)
                    member.size = len(data)
                    tar.addfile(member, io.BytesIO(data))
            package = community_ai.Package(
                name="fixture",
                version="1",
                kind="newgrf",
                uid="12345678",
                md5="1" * 32,
                sha256=hashlib.sha256(archive.read_bytes()).hexdigest(),
                license="test",
                metadata="test",
                publisher="test",
                dependencies=[],
            )
            run = root / ".local/simulation/run/newgrf-sets/plain/reference"
            run.mkdir(parents=True)
            with (
                patch.object(
                    community_ai, "fetch", return_value={package.filename: archive}
                ),
                patch.object(community_ai, "packages", return_value=[package]),
            ):
                newgrf_sets.install(
                    cast(
                        newgrf_sets.Scenario,
                        dict(newgrf_sets.scenarios(False)[0], scenario_ai=str(scripts)),
                    ),
                    run,
                )
                community_ai.install({"community_ai": "temperate"}, run)
            installed = list(run.rglob("*.tar"))
            self.assertEqual(len(installed), 4)
            for path in installed:
                self.assertEqual(
                    path.read_bytes(), gzip.decompress(archive.read_bytes())
                )
            (run / "stderr.log").write_text("scenario failed")
            (run / "exit.sav").write_bytes(b"save metadata")
            evidence = root / "evidence"
            with patch.object(ci_native, "ROOT", root):
                ci_native.collect_evidence(evidence)
            self.assertEqual(
                sorted(p.name for p in evidence.rglob("*") if p.is_file()),
                ["exit.sav", "stderr.log"],
            )

    def test_running_ai_identity_is_distinct_from_slot_configuration(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            source, _ = simulate.normalize(community_ai.FIXTURE, Path(directory))
            chunks = simulate.read_save(source)
        chunk = chunks["AIPL"]
        elements = dict(chunk["elements"])
        body = elements[0]
        reader = simulate.Reader(body)
        simulate.read_object(
            reader, cast(list[simulate.Field], chunk["header"]), "", {}
        )
        start = reader.pos
        reader.take(reader.gamma())
        for replaced in (False, True):
            # The configured SimpleAI prefix stays intact after native fallback.
            elements[0] = (
                body[:start] + b"\x07%_dummy" + body[reader.pos :] if replaced else body
            )
            chunks["AIPL"] = cast(
                simulate.Chunk, dict(chunk, elements=list(elements.items()))
            )
            run = cast(Any, community_ai.Run)(
                dir=Path("unused"), exit=0, snapshots=[source], log=[]
            )
            with (
                self.subTest(replaced=replaced),
                patch.object(simulate, "read_save", return_value=chunks),
                patch.object(savegame, "read_save", return_value=chunks),
            ):
                if replaced:
                    with self.assertRaisesRegex(RuntimeError, "AI 0 was replaced"):
                        community_ai.activity(run)
                else:
                    community_ai.activity(run)


class CommunityNewGRFSaveTests(unittest.TestCase):
    def test_reload_requires_the_complete_package_manifest(self) -> None:
        scenario = community_newgrf.scenarios(False)[0]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = root / "packages.json"
            original = json.loads((community_ai.INPUTS / manifest.name).read_text())
            for change in ("none", "newgrf", "ai"):
                rows = json.loads(json.dumps(original))
                if change != "none":
                    next(r for r in rows if r["kind"] == change)["sha256"] = "0" * 64
                manifest.write_text(json.dumps(rows, indent=2) + "\n")
                with (
                    self.subTest(change=change),
                    patch.object(community_ai, "INPUTS", root),
                ):
                    if change == "none":
                        community_newgrf.prepare(
                            scenario,
                            {},
                            {},
                            root,
                            1,
                            {},
                            cast(community_newgrf.Result, {}),
                        )
                    else:
                        with self.assertRaisesRegex(RuntimeError, "packages differ"):
                            community_newgrf.prepare(
                                scenario,
                                {},
                                {},
                                root,
                                1,
                                {},
                                cast(community_newgrf.Result, {}),
                            )

    def test_fixture_records_its_saved_newgrf_identities(self) -> None:
        provenance = json.loads(
            community_newgrf.FIXTURE.with_suffix(".json").read_text()
        )
        with tempfile.TemporaryDirectory() as directory:
            source, _ = simulate.normalize(community_newgrf.FIXTURE, Path(directory))
            saved = community_newgrf.saved_newgrfs(source)
        self.assertEqual(
            saved, [community_newgrf.NewGRF(**row) for row in provenance["newgrfs"]]
        )
        self.assertEqual(
            sorted(grf.filename for grf in saved), sorted(newgrf_sets.GRFS)
        )

    def test_install_places_both_ais_and_sets(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            archive = root / "fixture.tar.gz"
            archive.write_bytes(gzip.compress(b"package"))
            packages = [
                community_ai.Package(
                    name=kind,
                    version="1",
                    kind=kind,
                    uid="12345678",
                    md5="1" * 32,
                    sha256="0" * 64,
                    license="test",
                    metadata="test",
                    publisher="test",
                    dependencies=[],
                )
                for kind in ("ai", "ai-library", "newgrf")
            ]
            run = root / "run"
            with (
                patch.object(
                    community_ai,
                    "fetch",
                    return_value={p.filename: archive for p in packages},
                ),
                patch.object(community_ai, "packages", return_value=packages),
            ):
                community_newgrf.install(community_newgrf.scenarios(False)[0], run)
            installed = sorted(
                str(p.relative_to(run).parent) for p in run.rglob("*.tar")
            )
        self.assertEqual(
            installed,
            [
                ".openttd-rust/newgrf",
                ".openttd/newgrf",
                "ai",
                "ai/library",
                "xdg-data/openttd-rust/newgrf",
                "xdg-data/openttd/newgrf",
            ],
        )


if __name__ == "__main__":
    unittest.main()
