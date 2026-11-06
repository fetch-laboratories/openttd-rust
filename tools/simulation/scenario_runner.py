"""Run scenario pairs and collect their semantic observations."""

import shutil
import statistics
import time
from collections.abc import Callable
from dataclasses import asdict
from pathlib import Path
from typing import Any, cast

from .game import prepare_scenario, run_game
from .models import (
    ROLES,
    Check,
    Mode,
    Result,
    Role,
    Run,
    Scenario,
    empty_stats,
    findings,
)
from .registry import hook_modules
from .savegame import (
    Row,
    compare_saves,
    decode_element,
    first_difference,
    read_save,
    save_moment,
)

GameRunner = Callable[..., Run]


def run_scenario(
    scenario: Scenario,
    binaries: dict[str, Path],
    builds: dict[str, Path],
    out: Path,
    limit: int,
    timeout: int,
    env: dict[str, str],
    benchmark_repetitions: int = 0,
    instructions: bool = False,
    *,
    game_runner: GameRunner | None = None,
    prepared: bool = False,
) -> Result:
    """Run both binaries twice (with and without desync snapshots) and compare.

    The child launcher denies worker threads, selecting the original synchronous
    fallback for link graph computation. No wall-time-dependent join pauses
    consume null-driver iterations, so every save and complete log is compared."""
    if instructions:
        benchmark_repetitions = 1
    name = scenario["name"]
    started = time.monotonic()
    result: Result = {
        "scenario": name,
        "plain_speed": {"samples": [], "median_candidate_reference_ratio": None},
        "differences": [],
        "known_failures": [],
        "problems": [],
        "notes": [],
        "snapshots": 0,
        "stats": empty_stats(),
    }
    sanitizer_reports: dict[str, Any] = {}
    runs: dict[Role, Run] = {}

    def compare(mode: Mode, snapshot: str) -> None:
        records = compare_saves(
            next(p for p in runs["reference"]["snapshots"] if p.name == snapshot),
            next(p for p in runs["candidate"]["snapshots"] if p.name == snapshot),
            name,
            limit,
            result["stats"],
        )
        for record in records:
            record["snapshot"] = f"{mode}/{snapshot}"
            result["known_failures" if "issue" in record else "differences"].append(
                record
            )

    try:
        if not prepared:
            scenario = prepare_scenario(
                scenario, binaries, builds, out, timeout, env, result
            )
        result["requested_ticks"] = scenario["ticks"]
        # Only a distinct candidate counts branches; timed runs keep counters off.
        scenario = {
            **scenario,
            "witnesses": not benchmark_repetitions
            and binaries["candidate"] != binaries["reference"],
        }
        modes: tuple[tuple[Mode, bool], ...] = (
            (("plain", False),) * benchmark_repetitions
            if benchmark_repetitions
            else (("snapshots", True), ("plain", False))
        )
        for mode, desync in modes:
            runs = {
                role: (game_runner or run_game)(
                    cast(
                        Scenario,
                        dict(
                            scenario,
                            **scenario.get("role_inputs", {}).get(role, {}),
                            witnesses=scenario["witnesses"] and role == "candidate",
                        ),
                    ),
                    scenario.get("executables", binaries)[role],
                    builds[role],
                    out / name / mode / role,
                    timeout,
                    env,
                    desync,
                    isolate=bool(benchmark_repetitions),
                    **({"instructions": True} if instructions else {}),
                )
                for role in ROLES
            }
            exits = [
                runs[role]["snapshots"][-1]
                if runs[role]["snapshots"]
                and runs[role]["snapshots"][-1].name == "exit.sav"
                else None
                for role in ("reference", "candidate")
            ]
            clean = all(exits) and all(run["exit"] == 0 for run in runs.values())
            moments = [save_moment(path) if path else None for path in exits]
            findings(result)[f"{mode}_end_moments"] = moments
            same_end = clean and moments[0] == moments[1]
            for role, run in runs.items():
                if run.get("sanitizer") is not None:
                    from .sanitizers import Findings

                    found = sanitizer_reports.setdefault(role, Findings())
                    for key, text in run["sanitizer"].reports.items():
                        found.add(key, text)
                for module in hook_modules(scenario, "check"):
                    cast(Check, module.check)(scenario, run, mode, role, result)
                findings(result)[f"{mode}_{role}_seconds"] = run["seconds"]
                if run["exit"] != 0:
                    result["problems"].append(
                        f"{mode}: {role} exited with {run['exit']}"
                    )
                # Cache-check mismatches are repaired before the next tick, so they
                # never reach a save; any desync warning fails the scenario.
                warning = next(
                    (line for line in run["log"] if "[desync:" in line), None
                )
                if warning:
                    result["problems"].append(f"{mode}: {role} logged {warning!r}")
            if not all(exits):
                result["problems"].append(f"{mode}: an exit save is missing")
            periodic = [
                [p.name for p in runs[role]["snapshots"] if p.name != "exit.sav"]
                for role in ("reference", "candidate")
            ]
            if periodic[0] != periodic[1]:
                result["problems"].append(
                    f"{mode}: snapshot dates differ: {first_difference(*periodic)}"
                )
            common = sorted(set(periodic[0]) & set(periodic[1]))
            if mode == "snapshots":
                result["snapshots"] = len(common) + bool(same_end)
                if len(common) < scenario.get(
                    "snapshot_minimum",
                    0
                    if "effects" in scenario or scenario.get("short_checkpoint")
                    else 2,
                ):
                    result["problems"].append(
                        "fewer than the required periodic snapshots were written"
                    )
                # Console output is invisible without a GUI, so confirm each
                # setting line took effect in the first snapshot of both runs.
                # Values must be written as PATS stores them (numbers, not
                # true/false or names), and only saved game settings qualify.
                for role, run in runs.items():
                    first = next(
                        (p for p in run["snapshots"] if p.name != "exit.sav"), None
                    )
                    if first is None:
                        continue
                    for line in scenario.get("console", []):
                        if line.startswith("setting "):
                            _, key, value = line.split()
                            chunk = read_save(first)["PATS"]
                            saved = cast(
                                Row, decode_element(chunk, chunk["elements"][0][1])
                            ).get(key)
                            if str(saved) != value:
                                result["problems"].append(
                                    f"{role}: {key} is {saved}, not {value}, in {first.name}"
                                )
            for snapshot in common:
                compare(mode, snapshot)
                if result["differences"]:
                    break  # Later snapshots only repeat the first divergence.
            findings(result)[f"{mode}_exit_compared"] = bool(all(exits))
            if all(exits):
                compare(mode, "exit.sav")
            if not same_end and all(exits):
                result["problems"].append(
                    f"{mode}: runs ended at different moments: "
                    f"{moments[0]} vs {moments[1]}"
                )
            for label in ("log", "stdout"):
                a, b = (
                    runs[role]["log"]
                    if label == "log"
                    else runs[role]["stdout"].splitlines()
                    for role in ROLES
                )
                if a != b:
                    index, ref_line, cand_line = first_difference(a, b)
                    result["differences"].append(
                        {
                            "snapshot": f"{mode}/{label}",
                            "chunk": "",
                            "element": str(index + 1),
                            "field": "line",
                            "reference": repr(ref_line),
                            "candidate": repr(cand_line),
                        }
                    )
            if instructions:
                from . import instructions as profiler
                from .instructions import SPEED_ANCHORS

                result["instruction_speed"] = profiler.finish(runs, result)
                if name in SPEED_ANCHORS:
                    candidate = result["instruction_speed"]["candidate"]
                    result["instruction_speed"]["budget"] = profiler.budget(
                        name, candidate, SPEED_ANCHORS[name]
                    )
            if mode == "plain" and not instructions:
                reference_seconds = runs["reference"]["seconds"]
                candidate_seconds = runs["candidate"]["seconds"]
                ratio = (
                    candidate_seconds / reference_seconds
                    if (
                        same_end
                        and not result["differences"]
                        and not result["known_failures"]
                        and not result["problems"]
                        and reference_seconds > 0
                    )
                    else None
                )
                result["plain_speed"]["samples"].append(
                    {
                        "reference_seconds": reference_seconds,
                        "candidate_seconds": candidate_seconds,
                        "candidate_reference_ratio": ratio,
                        "same_end": same_end,
                    }
                )
                if benchmark_repetitions:
                    result["plain_commands"] = {
                        role: run["command"] for role, run in runs.items()
                    }
            if benchmark_repetitions and (result["differences"] or result["problems"]):
                break  # Preserve the first failing repetition instead of overwriting it.
    except (
        Exception
    ) as error:  # Record and continue, so the report covers every scenario.
        result["problems"].append(f"harness error: {type(error).__name__}: {error}")
    # A game ASan stopped can end the scenario early; its reports still count.
    if len(sanitizer_reports) == 2:
        from .sanitizers import compare as compare_reports

        comparison = compare_reports(sanitizer_reports)
        texts = sanitizer_reports["candidate"].reports
        result["sanitizer"] = {
            **asdict(comparison),
            "reports": {
                key: texts[key]
                for key in comparison.original + comparison.candidate_only
            },
        }
        for key in comparison.candidate_only:
            result["problems"].append(f"candidate-only sanitizer report: {key}")
    ratios = [
        sample["candidate_reference_ratio"]
        for sample in result["plain_speed"]["samples"]
        if sample["candidate_reference_ratio"] is not None
    ]
    if ratios:
        result["plain_speed"]["median_candidate_reference_ratio"] = statistics.median(
            ratios
        )
    if benchmark_repetitions:
        from .instructions import speed_budget

        result["plain_speed"]["budget"] = speed_budget(
            scenario["name"],
            result["plain_speed"]["median_candidate_reference_ratio"],
        )
    result["passed"] = not result["differences"] and not result["problems"]
    if result["passed"] and not benchmark_repetitions:
        shutil.rmtree(out / name, ignore_errors=True)  # Keep only failing runs.
    result["seconds"] = round(time.monotonic() - started, 3)
    return result
