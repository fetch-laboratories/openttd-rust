"""Seeded script commands; the original remains the outcome and state oracle."""

import shutil
from collections import Counter
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any, NotRequired, TypedDict

from . import core

AI_FOLDER = "command-scenario-ai"
FAMILIES = ("rail", "road", "station", "order", "vehicle", "terraform")


class Scenario(TypedDict):
    name: str
    kind: str
    command_seed: int
    command_count: int
    ticks: int
    console: list[str]
    save: NotRequired[str]
    seed: NotRequired[int]
    map_log2: NotRequired[int]
    land_generator: NotRequired[int]
    scenario_ai: NotRequired[str]


@dataclass(frozen=True)
class Outcome:
    index: int
    command: str
    arguments: str
    value: str
    outcome: str


def scenarios(soak: bool) -> list[Scenario]:
    cases: list[Scenario] = []
    for kind in ("save", "generate") if soak else ("save",):
        for seed in (1, 48271, 1234567) if soak else (1,):
            count = 12000 if soak else 360
            case: Scenario = {
                "name": f"commands-{'play' if kind == 'save' else 'generated'}-{seed}",
                "kind": kind,
                "command_seed": seed,
                "command_count": count,
                "ticks": max(5000, count * 3 + 5000),
                "console": ["unpause"],
            }
            if kind == "save":
                case["save"] = str(
                    core.ROOT / "migration/saves/padhattan-ridge-1996.sav"
                )
            else:
                case.update(seed=seed, map_log2=7, land_generator=1)
            cases.append(case)
    return cases


def uses_ai(scenario: dict[str, Any]) -> bool:
    return "command_seed" in scenario


def install(scenario: dict[str, Any], run_dir: Path) -> None:
    if not uses_ai(scenario):
        return
    folder = run_dir / "ai/commands"
    shutil.copytree(scenario["scenario_ai"], folder)
    (folder / "parameters.nut").write_text(
        f"COMMAND_SEED <- {scenario['command_seed']};\n"
        f"COMMAND_COUNT <- {scenario['command_count']};\n"
    )


def prepare(
    scenario: dict[str, Any],
    binaries: dict[str, Path],
    builds: dict[str, Path],
    out: Path,
    timeout: int,
    env: dict[str, str],
    result: dict[str, Any],
) -> dict[str, Any]:
    if not uses_ai(scenario):
        return scenario
    folder = out / scenario["name"] / "input"
    if scenario["kind"] == "generate":
        setup = {k: v for k, v in scenario.items() if k != "command_seed"}
        run = core.run_game(
            dict(setup, ticks=1),
            binaries["reference"],
            builds["reference"],
            folder / "generate",
            timeout,
            env,
            False,
        )
        if run["exit"] or not run["snapshots"]:
            raise RuntimeError("command map generation did not produce a save")
        source = run["snapshots"][-1]
    else:
        source = Path(scenario["save"])
    normalized, record = core.normalize(source, folder)
    if dict(core.read_save(normalized)["AIPL"]["elements"])[0] != core.EMPTY_AI:
        raise RuntimeError("command source already has an AI in company zero")
    prepared = folder / "command-input.sav"
    record["actor"] = core.patch(
        normalized, prepared, {"PLYR": {0: {"is_ai": True, "money": 1000000000}}}
    )
    config = core.ai_config(b"MigrationCommands") * 2 + b"\0"
    prepared.write_bytes(core.set_ai_slots(prepared.read_bytes(), {0: config}))
    result["command_input"] = record
    return dict(scenario, kind="save", save=str(prepared))


def outcomes(lines: list[str], count: int, seed: int) -> list[Outcome]:
    records = [
        Outcome(int(fields[0]), *fields[1:])
        for line in lines
        if "COMMAND " in line
        for fields in [line.split("COMMAND ", 1)[1].split(" ", 4)]
    ]
    if [row.index for row in records] != list(range(count)):
        raise ValueError("command log is missing, duplicated or incomplete")
    if any(
        row.command.split(".")[0] != FAMILIES[i % 6] for i, row in enumerate(records)
    ):
        raise ValueError("command family schedule differs")
    state = seed
    # Every command takes eight draws; rail takes one additional track draw.
    for i in range(count):
        for _ in range(9 if i % 6 == 0 else 8):
            state = state * 48271 % 2147483647
    markers = [
        line.split("COMMAND-END ", 1)[1] for line in lines if "COMMAND-END " in line
    ]
    beginnings = [
        line.split("COMMAND-BEGIN ", 1)[1].split()[:2]
        for line in lines
        if "COMMAND-BEGIN " in line
    ]
    if markers != [f"{count} {state}"] or beginnings != [[str(seed), str(count)]]:
        raise ValueError("command seed/start/completion marker differs")
    for family in FAMILIES:
        observed = {
            row.outcome == "OK"
            for row in records
            if row.command.startswith(family + ".")
        }
        if observed != {False, True}:
            raise ValueError(f"command {family} lacks successful and failed activity")
    for command in ("order.move", "order.skip", "order.flags"):
        if not any(row.command == command and row.outcome == "OK" for row in records):
            raise ValueError(f"required command activity absent: {command}")
    return records


def check(
    scenario: dict[str, Any],
    run: dict[str, Any],
    mode: str,
    role: str,
    result: dict[str, Any],
) -> None:
    if not uses_ai(scenario):
        return
    records = outcomes(run["log"], scenario["command_count"], scenario["command_seed"])
    histogram: dict[str, dict[str, int]] = {}
    for command in sorted({row.command for row in records}):
        histogram[command] = dict(
            sorted(
                Counter(
                    row.outcome for row in records if row.command == command
                ).items()
            )
        )
    result[f"{mode}_{role}_commands"] = {
        "outcomes": [asdict(row) for row in records],
        "histogram": histogram,
    }
    if role == "candidate":
        reference = result[f"{mode}_reference_commands"]["outcomes"]
        if reference != [asdict(row) for row in records]:
            result["problems"].append(f"{mode}: full command outcomes differ")
