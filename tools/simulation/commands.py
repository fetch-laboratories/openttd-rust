"""Seeded script commands; the original remains the outcome and state oracle."""

from collections import Counter
from collections.abc import Mapping
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any, NotRequired

from . import core
from .console_adapter import console_adapter, prepared_console

AI_FOLDER = "command-scenario-ai"
FAMILIES = ("rail", "road", "station", "order", "vehicle", "terraform")


class Scenario(core.Scenario):
    command_seed: NotRequired[int]
    command_count: NotRequired[int]
    command_money: NotRequired[bool]
    command_lifecycle: NotRequired[str]
    command_setup: NotRequired[bool]


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
                "ticks": count * 8 + 5000 if soak else 6080,
                "console": ["unpause"],
            }
            if kind == "save":
                case["save"] = str(
                    core.ROOT / "migration/saves/padhattan-ridge-1996.sav"
                )
            else:
                case.update(seed=seed, map_log2=7, land_generator=1)
            cases.append(case)
    cases.append(
        {
            "name": "commands-money",
            "short_checkpoint": True,
            "kind": "save",
            "command_seed": 1,
            "command_money": True,
            "ticks": 100,
            "console": ["unpause"],
            "save": str(core.ROOT / "migration/saves/padhattan-ridge-1996.sav"),
        }
    )
    cases.extend(
        {
            "name": f"commands-lifecycle-{mode}",
            "kind": "save",
            "command_lifecycle": mode,
            "save": str(core.ROOT / "migration/saves/padhattan-ridge-1996.sav"),
            "ticks": 1,
            "short_checkpoint": True,
        }
        for mode in ("control", "reload")
    )
    return cases


def adapter(build: Path, folder: Path, env: dict[str, str]) -> Path:
    return console_adapter(
        build,
        folder,
        env,
        core.ROOT / "tools/simulation/commands-console.cpp",
        "commands",
        wraps=(
            "_ZN2AI8StartNewE6PoolIDIh12CompanyIDTagLh15ELh255EE",
            "_Z10DebugPrintSt17basic_string_viewIcSt11char_traitsIcEEiONSt7__cxx1112basic_stringIcS1_SaIcEEE",
        ),
        replace_objects=("/src/network/network_command.cpp.o",),
        native_test_support=True,
    )


def uses_ai(scenario: Mapping[str, Any]) -> bool:
    return "command_seed" in scenario


def install(scenario: Scenario, run_dir: Path) -> None:
    if not uses_ai(scenario):
        return
    core.install_ai(
        scenario,
        run_dir,
        "commands",
        f"COMMAND_SEED <- {scenario['command_seed']};\n"
        f"COMMAND_COUNT <- {scenario.get('command_count', 0)};\n"
        f"COMMAND_MONEY <- {str(scenario.get('command_money', False)).lower()};\n"
        f"COMMAND_SETUP <- {str(scenario.get('command_setup', False)).lower()};\n",
    )


def prepare(
    scenario: Scenario,
    binaries: dict[str, Path],
    builds: dict[str, Path],
    out: Path,
    timeout: int,
    env: dict[str, str],
    result: core.Result,
) -> Scenario:
    if "command_lifecycle" in scenario:
        folder = out / scenario["name"] / "input"
        source, _ = core.normalize(Path(scenario["save"]), folder / "normalized")
        scenario = {**scenario, "save": str(source)}
        executables = prepared_console(
            "commands", adapter, binaries, builds, folder, env
        )
        if scenario["command_lifecycle"] == "control":
            return {
                **scenario,
                "executables": executables,
                "console": ["command_scenario probe", "unpause"],
            }
        saves = {role: folder / role / "commands.sav" for role in executables}

        def probe(role: str) -> Scenario:
            console = [f'command_scenario probe "{saves[role]}"', "unpause"]
            return {**scenario, "console": console}

        runs = core.capture_pair(
            probe,
            executables,
            builds,
            folder,
            timeout,
            env,
        )
        observations = {}
        for role, run in runs.items():
            observations[role] = lifecycle_observations(run["log"])
            if run["exit"] or not saves[role].is_file():
                raise RuntimeError(f"{role}: command lifecycle checkpoint missing")
        if observations["reference"] != observations["candidate"]:
            raise RuntimeError("command lifecycle observations differ before reload")
        differences = core.compare_saves(
            saves["reference"],
            saves["candidate"],
            scenario["name"],
            10,
            core.empty_stats(),
        )
        if differences:
            raise RuntimeError(f"command lifecycle checkpoint differs: {differences}")
        return {
            **scenario,
            "executables": executables,
            "role_inputs": {r: {"save": str(p)} for r, p in saves.items()},
            "console": ["unpause"],
        }
    if not uses_ai(scenario):
        return scenario
    folder = out / scenario["name"] / "input"
    if scenario["kind"] == "generate":
        setup: Scenario = {
            **scenario,
            "command_setup": True,
            "ticks": 1,
            "console": ["start_ai MigrationCommands", "unpause"],
        }
        run = core.run_game(
            setup,
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
        if set(core.rows(source, "PLYR")) != {0}:
            raise RuntimeError("command generation did not create its actor company")
    else:
        source = Path(scenario["save"])
    normalized, record = core.normalize(source, folder)
    if (
        scenario["kind"] == "save"
        and dict(core.read_save(normalized)["AIPL"]["elements"])[0] != core.EMPTY_AI
    ):
        raise RuntimeError("command source already has an AI in company zero")
    prepared = folder / "command-input.sav"
    record["actor"] = core.patch(
        normalized,
        prepared,
        {
            "PLYR": {
                0: {
                    "is_ai": True,
                    "money": 1 if scenario.get("command_money") else 1000000000,
                }
            }
        },
    )
    config = core.ai_config(b"MigrationCommands") * 2 + b"\0"
    prepared.write_bytes(core.set_ai_slots(prepared.read_bytes(), {0: config}))
    core.findings(result)["command_input"] = record
    return {**scenario, "kind": "save", "save": str(prepared)}


def outcomes(lines: list[str], count: int, seed: int) -> list[Outcome]:
    records = [
        Outcome(int(fields[0]), *fields[1:])
        for line in lines
        if "COMMAND " in line
        for fields in [line.split("COMMAND ", 1)[1].split(" ", 4)]
    ]
    if [row.index for row in records] != list(range(count)):
        raise ValueError(
            f"command log is missing, duplicated or incomplete: {len(records)} of {count}"
        )
    if any(
        row.command.split(".")[0] != FAMILIES[i % 6] for i, row in enumerate(records)
    ):
        raise ValueError("command family schedule differs")
    state = seed
    # Every command takes nine draws; rail takes one additional track draw.
    for i in range(count):
        for _ in range(10 if i % 6 == 0 else 9):
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
    for family in FAMILIES:
        contexts = {
            row.arguments.split(":", 1)[0]
            for row in records
            if row.command.startswith(family + ".")
        }
        if not {"infrastructure", "vehicle", "edge"} <= contexts:
            raise ValueError(f"command {family} lacks required location contexts")
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
    if "command_lifecycle" in scenario:
        if scenario["command_lifecycle"] == "control":
            observations = lifecycle_observations(run["log"])
            result[f"{mode}_{role}_lifecycle"] = observations
            if (
                role == "candidate"
                and observations != result[f"{mode}_reference_lifecycle"]
            ):
                result["problems"].append(
                    f"{mode}: command lifecycle observations differ"
                )
        return
    if not uses_ai(scenario):
        return
    if scenario.get("command_money"):
        observations = [
            line.split("COMMAND-MONEY ", 1)[1]
            for line in run["log"]
            if "COMMAND-MONEY " in line
        ]
        if len(observations) != 1 or observations[0].split()[:3] != [
            "true",
            "false",
            "true",
        ]:
            raise ValueError(
                f"missing script estimate/money rejection evidence: {observations}"
            )
        result[f"{mode}_{role}_money"] = observations
        if role == "candidate" and observations != result[f"{mode}_reference_money"]:
            result["problems"].append(f"{mode}: script money outcomes differ")
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


def lifecycle_observations(lines: list[str]) -> list[str]:
    records = [line for line in lines if line.startswith("INVOCATION ")]
    names = [line.split()[1] for line in records]
    if names != [
        "paused-rejected",
        "shift-estimate",
        "callback",
        "pause-marked",
        "extra-cash",
        "network-send",
        "callback",
        "unwind",
        "rng",
    ]:
        raise ValueError(f"incomplete command lifecycle evidence: {names}")
    return records
