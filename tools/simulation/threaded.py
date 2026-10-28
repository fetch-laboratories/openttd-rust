#!/usr/bin/env python3
"""SDL offscreen comparisons with real main/game threads and native graph workers (#506)."""

import argparse
import json
import os
import re
import subprocess
import sys
import time
from dataclasses import asdict
from datetime import datetime, timezone
from functools import partial
from pathlib import Path
from typing import TypedDict, cast

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from validation_execution import run_logged  # noqa: E402

import migration  # noqa: E402
from simulation import core, sanitizers, thread_sanitizer  # noqa: E402
from simulation.console_adapter import console_adapter  # noqa: E402


class Scenario(TypedDict):
    name: str
    kind: str
    save: str
    ticks: int
    snapshot_minimum: int
    moment: tuple[int, int, int]
    tile: int


class Run(TypedDict):
    dir: Path
    exit: int | str
    seconds: float
    command: list[str]
    instructions: None
    snapshots: list[Path]
    log: list[str]
    stdout: bytes
    sanitizer: object


class Result(TypedDict):
    scenario: str
    passed: bool
    seconds: float
    problems: list[str]
    differences: list[object]
    sanitizer: dict[str, object]


INPUTS = (
    ("opus-55-167-002", (727960, 0, 84677), 10814),
    ("padhattan-ridge-2000", (730851, 0, 298611), 14790),
)


def check(scenario: Scenario, run: Run) -> None:
    """Require real occupancy on the same tile, split threads, and the calendar callback phase."""
    log = "\n".join(run["log"])
    game = re.findall(
        r"^THREAD506 game tile (\d+) occupied true error (\d+)$", log, re.M
    )
    main = re.findall(
        r"^THREAD506 main split true tile (\d+) occupied true error (\d+) clear true error (\d+)$",
        log,
        re.M,
    )
    stop = re.findall(
        r"^THREAD506 stop game true calendar (\d+) fract (\d+) economy (\d+) fract (\d+)$",
        log,
        re.M,
    )
    date = scenario["moment"][0]
    if (
        log.splitlines().count("THREAD506 credits true") != 1
        or len(game) != 1
        or len(main) != 1
        or game[0][0] != str(scenario["tile"])
        or main[0] != (game[0][0], game[0][1], game[0][1])
        or stop != [(str(date), "0", str(date - 1), "73")]
        or "[driver:1] using thread for game-loop" not in log
    ):
        raise RuntimeError(
            "missing/invalid occupancy, credits, thread or calendar-phase markers"
        )
    exit_save = run["dir"] / "save/autosave/exit.sav"
    if not exit_save.is_file() or core.save_moment(exit_save) != scenario["moment"]:
        raise RuntimeError("missing exit save or wrong calendar-callback save moment")
    if not (run["dir"] / "console.log").is_file():
        raise RuntimeError("missing console log")


def prepare(
    scenario: Scenario, build: Path, run: Path, env: dict[str, str], desync: bool
) -> tuple[list[str], dict[str, str]]:
    run.mkdir(parents=True)
    core.write_config(scenario, build, run)
    config = (run / "openttd.cfg").read_text()
    for option in ("threaded_saves = false", "fast_forward_speed_limit = 0"):
        config = core.set_option(config, "gui", option)
    (run / "openttd.cfg").write_text(config)
    (run / "scripts").mkdir()
    (run / "scripts/game_start.scr").write_text(
        "script console.log\nscrollto instant 64 64\nschedule on-next-calendar-month scripts/stop.scr\nthreaded_start\n"
    )
    (run / "scripts/stop.scr").write_text(
        "threaded_stop\ngetdate\nscrollto instant 32 32\nexit\n"
    )
    command = [
        "-x",
        "-c",
        str(run / "openttd.cfg"),
        "-g",
        scenario["save"],
        "-vsdl",
        "-b32bpp-anim",
        "-snull",
        "-mnull",
        "-r",
        "640x480",
        "-d",
        "driver=1,script=2" + (",desync=3" if desync else ""),
    ]
    env = env | {
        "HOME": str(run),
        "XDG_DATA_HOME": str(run / "data"),
        "XDG_CONFIG_HOME": str(run / "config"),
        "XDG_CACHE_HOME": str(run / "cache"),
        "SDL_VIDEODRIVER": "offscreen",
    }
    if migration.TSAN:
        env |= thread_sanitizer.environment(run)
    return command, env


def run_game(
    scenario: Scenario,
    binary: Path,
    build: Path,
    run_dir: Path,
    timeout: int,
    env: dict[str, str],
    desync: bool,
    *,
    isolate: bool = False,
    reports: dict[str, sanitizers.Findings] | None = None,
) -> Run:
    # Keep successful native runs too: full TSan reports and same-build repetitions are evidence.
    run_dir = run_dir.parents[2] / "native" / run_dir.relative_to(run_dir.parents[2])
    args, env = prepare(scenario, build, run_dir, env, desync)
    command = [str(binary), *args]
    started = time.monotonic()
    (run_dir / "stdout.log").write_bytes(b"")
    with core.game_slot():
        try:
            code: int | str = run_logged(
                command,
                cwd=run_dir,
                env=env,
                log=run_dir / "stderr.log",
                phase=run_dir.name,
                timeout=timeout,
            )
        except subprocess.TimeoutExpired:
            code = "timeout"
    run = cast(
        Run,
        core.read_game(
            {
                "dir": run_dir,
                "exit": code,
                "seconds": round(time.monotonic() - started, 3),
                "command": command,
                "instructions": None,
            }
        ),
    )
    if migration.TSAN:
        found = thread_sanitizer.findings(
            run_dir,
            (migration.REFERENCE.resolve(), core.ROOT),
            (migration.REFERENCE_BUILD, migration.CANDIDATE_BUILD),
        )
        if reports is None:
            raise RuntimeError(
                "TSan run has no collection for its actual executable role"
            )
        for key, text in found.reports.items():
            reports[build.name.removesuffix("-runtime")].add(key, text)
    check(scenario, run)
    run["stdout"] = (run_dir / "console.log").read_bytes()
    return run


def finish_reports(result: Result, reports: dict[str, sanitizers.Findings]) -> None:
    """Classify the fixed repetitions by executable, including same-build pairs."""
    comparison = sanitizers.compare(reports)
    result["sanitizer"] = {
        **asdict(comparison),
        "reports": reports["reference"].reports | reports["candidate"].reports,
    }
    for key in comparison.candidate_only:
        result["problems"].append(f"candidate-only sanitizer report: {key}")
    result["passed"] = result["passed"] and not comparison.candidate_only


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("names", nargs="*")
    parser.add_argument("--jobs", type=int, default=1)
    parser.add_argument("--timeout", type=int, default=1200)
    args = parser.parse_args()
    selected = [
        item
        for item in INPUTS
        if not args.names
        or any(name in item[0] or name == "threaded" for name in args.names)
    ]
    if not selected:
        parser.error("no threaded input matches selectors")
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    out = migration.LOCAL / "simulation" / f"{stamp}-threaded-{os.getpid()}"
    out.mkdir(parents=True)
    builds: dict[str, Path] = {}
    binaries: dict[str, Path] = {}
    for role, native in (
        ("reference", migration.REFERENCE_BUILD),
        ("candidate", migration.CANDIDATE_BUILD),
    ):
        with (
            migration.reference_lock(shared=True)
            if role == "reference"
            else migration.candidate_lock(native, shared=True)
        ):
            build = out / f"{role}-runtime"
            executable = core.copy_runtime(
                native,
                native / ("openttd" if role == "reference" else "openttd-rust"),
                build,
            )
            binaries[role] = console_adapter(
                native,
                build,
                migration.environment(),
                Path(__file__).with_name("threaded-console.cpp"),
                "threaded",
            )
            executable.unlink()
            builds[role] = build
    results: list[Result] = []
    for name, moment, tile in selected:
        reports = {role: sanitizers.Findings() for role in ("reference", "candidate")}
        group: list[Result] = []
        source, _ = core.normalize(
            core.ROOT / f"migration/saves/{name}.sav", out / "inputs" / name
        )
        scenario: Scenario = {
            "name": f"threaded-{name}",
            "kind": "save",
            "save": str(source),
            "ticks": 0,
            "snapshot_minimum": 0,
            "moment": moment,
            "tile": tile,
        }
        for pair in ("paired", "reference-repeat", "candidate-repeat"):
            roles = (
                ("reference", "candidate")
                if pair == "paired"
                else (pair.removesuffix("-repeat"),) * 2
            )
            result = cast(
                Result,
                core.run_scenario(
                    dict(scenario, name=f"{scenario['name']}-{pair}"),
                    dict(
                        zip(
                            ("reference", "candidate"),
                            (binaries[r] for r in roles),
                            strict=True,
                        )
                    ),
                    dict(
                        zip(
                            ("reference", "candidate"),
                            (builds[r] for r in roles),
                            strict=True,
                        )
                    ),
                    out,
                    20,
                    args.timeout,
                    migration.environment(),
                    game_runner=partial(run_game, reports=reports),
                    prepared=True,
                ),
            )
            results.append(result)
            group.append(result)
        if migration.TSAN:
            finish_reports(group[0], reports)
        for result in group:
            print(
                f"{'ok' if result['passed'] else 'FAIL'} {result['scenario']}: "
                f"{len(result['differences'])} differences; {result['problems']}",
                flush=True,
            )
    report = {
        "candidate_commit": migration.git("rev-parse", "HEAD"),
        "candidate_status": migration.git("status", "--short"),
        "execution": "SDL offscreen; native game/main and graph workers; combined stdout/stderr",
        "sanitizers": core.sanitizer_summary(results),
        "results": results,
        "passed": all(r["passed"] for r in results),
    }
    path = out / "report.json"
    path.write_text(json.dumps(report, indent=2) + "\n")
    print(f"Report: {path}", flush=True)
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
