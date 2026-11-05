"""Script host lifecycle, command continuation and events (#575).

Two AIs and a GameScript print ``HOST`` observations that existing AIs and
successful saves do not establish: start order, Sleep and command-delay
suspension, the opcode budget, typed command results, costs and errors, test,
exec, async and company modes, event payloads and FIFO order, a constructor
failure, a fatal mode release, Break's pause, and Save/Load data on reload.
The ordinary comparator also compares the full logs, snapshots and exit saves.
"""

import shutil
from pathlib import Path
from typing import Any

from . import core

AI_FOLDER = "script-host-ai"
CONSOLE = ["start_ai MigrationHost", "start_ai MigrationHostFail", "unpause"]
SETTINGS = {
    "game_scripts": {"MigrationHostGS": ""},
    "gui": {"ai_developer_tools": "true"},
}
# AIs run every fourth tick at the default competitor speed, so the reload input
# is saved while the AI idles after its command phase and before its fatal release.
RELOAD_INPUT_TICKS = 400
RUN = (
    "ai constructor",
    "ai start loaded=none",
    "ai slept 5",
    "ai slept 0",
    "ai sign",
    "ai groups",
    "ai error",
    "ai nested exec",
    "ai test",
    "ai exec",
    "ai rand",
    "ai busy",
    "ai idle 3",
    "ai death",
    "fail constructor",
    "gs start loaded=none",
    "gs typed",
    "gs async",
    "gs sync",
    "gs test",
    "gs company",
    "gs deity",
    "gs invalid",
    "gs restored",
    "gs break",
)
RELOAD = ("ai load", "ai start loaded=", "ai idle", "gs load", "gs start loaded=")
ABSENT = ("ai unreachable", "fail unreachable", "fail start", "gs after break")


class Scenario(core.Scenario):
    script_host: str


def scenarios(soak: bool) -> list[Scenario]:
    del soak
    base: Scenario = {
        "name": "script-host-generated",
        "kind": "generate",
        "script_host": "run",
        "ticks": 3000,
        "seed": 575,
        "map_log2": 6,
        "land_generator": 1,
        "console": CONSOLE,
        "settings": SETTINGS,
    }
    reload: Scenario = {
        **base,
        "name": "script-host-reload",
        "script_host": "reload",
        # Two 32-day snapshots follow the load.
        "ticks": 5000,
        "console": ["unpause"],
    }
    return [base, reload]


def uses_ai(scenario: dict[str, Any]) -> bool:
    return "script_host" in scenario


def install(scenario: dict[str, Any], run_dir: Path) -> None:
    if not uses_ai(scenario):
        return
    for kind in ("ai", "game"):
        for folder in (Path(scenario["scenario_ai"]) / kind).iterdir():
            shutil.copytree(folder, run_dir / kind / folder.name)


def game_args(scenario: dict[str, Any]) -> list[str]:
    # Script prints log at level 2; saves already pass it.
    return ["-d", "script=2"] if scenario.get("script_host") == "run" else []


def prepare(
    scenario: Scenario,
    binaries: dict[str, Path],
    builds: dict[str, Path],
    out: Path,
    timeout: int,
    env: dict[str, str],
    result: core.Result,
) -> Scenario:
    if scenario.get("script_host") != "reload":
        return scenario
    folder = out / scenario["name"] / "input"
    setup: Scenario = {
        **scenario,
        "kind": "generate",
        "script_host": "run",
        "ticks": RELOAD_INPUT_TICKS,
        "console": CONSOLE,
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
        raise RuntimeError("script host reload input did not produce a save")
    normalized, record = core.normalize(run["snapshots"][-1], folder)
    core.findings(result)["script_host_input"] = record
    return {**scenario, "kind": "save", "save": str(normalized)}


def observations(lines: list[str]) -> list[str]:
    return [line.split("HOST ", 1)[1] for line in lines if "HOST " in line]


def marker(record: str) -> str:
    """A record without the script tick that follows its script name."""
    script, _, rest = record.partition(" ")
    tick, _, text = rest.partition(" ")
    return f"{script} {text}" if tick.isdigit() else record


def missing(scenario: dict[str, Any], records: list[str]) -> list[str]:
    required = RUN if scenario["script_host"] == "run" else RELOAD
    markers = [marker(record) for record in records]
    absent = [name for name in required if not any(m.startswith(name) for m in markers)]
    present = [name for name in ABSENT if any(m.startswith(name) for m in markers)]
    return absent + [f"unexpected {marker}" for marker in present]


def check(
    scenario: dict[str, Any],
    run: dict[str, Any],
    mode: str,
    role: str,
    result: dict[str, Any],
) -> None:
    if not uses_ai(scenario):
        return
    records = observations(run["log"])
    problems = missing(scenario, records)
    if problems:
        raise ValueError(f"{role} script host evidence incomplete: {problems}")
    result[f"{mode}_{role}_script_host"] = records
    if role == "candidate" and records != result[f"{mode}_reference_script_host"]:
        result["problems"].append(f"{mode}: script host observations differ")
