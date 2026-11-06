"""Infrastructure script API observations (#666).

An AI and a GameScript in deity, company and invalid-company modes print
``INFRA`` records through the production API boundary: map and tile query
results over every tile and the boundary tiles, slope, corner, transport and
price values, buildable rectangles, and test- and exec-mode command results with
their last error and cost, including refused preconditions. World equality alone
misses these return, error and order differences; the ordinary comparator also
compares the logs, snapshots and exit saves.
"""

import shutil
from pathlib import Path
from typing import Any, NotRequired

from . import core

AI_FOLDER = "script-infrastructure-ai"
PROBE = "probe.nut"
CONSOLE = ["start_ai MigrationInfra", "unpause"]
# The per-tile sweeps need many opcodes; a larger budget and the fastest AI
# speed finish them in fewer ticks than the default limits.
SETTINGS = {
    "game_scripts": {"MigrationInfraGS": ""},
    "gui": {"ai_developer_tools": "true"},
    "script": {"script_max_opcode_till_suspend": "250000"},
    "difficulty": {"competitor_speed": "4"},
}
WHO = ("ai", "gs.deity", "gs.company", "gs.invalid")
# Each probe ends with this record; absence means a script stopped early.
DONE = tuple(f"{who} done" for who in WHO)


class Scenario(core.Scenario):
    script_infrastructure: bool
    script_road_fatal: NotRequired[bool]


def scenarios(soak: bool) -> list[Scenario]:
    """Temperate, plus arctic snow and tropic desert/rainforest terrain branches."""
    del soak
    return [
        {
            "name": f"script-infrastructure-tile{suffix}",
            "kind": "generate",
            "script_infrastructure": True,
            "ticks": 8000,
            "seed": 666,
            "map_log2": 6,
            "land_generator": 1,
            "console": CONSOLE,
            "settings": {
                **SETTINGS,
                "game_creation": {"landscape": climate},
                "construction": {"build_on_slopes": slopes},
            },
        }
        for suffix, climate, slopes in (
            ("", "temperate", "true"),
            # LookupWithoutBuildOnSlopes is reached only with the setting off.
            ("-arctic", "arctic", "false"),
            ("-tropic", "tropic", "true"),
        )
    ]


def uses_ai(scenario: dict[str, Any]) -> bool:
    return "script_infrastructure" in scenario


def install(scenario: dict[str, Any], run_dir: Path) -> None:
    if not uses_ai(scenario):
        return
    source = Path(scenario["scenario_ai"])
    for kind in ("ai",) if scenario.get("script_road_fatal") else ("ai", "game"):
        for folder in (source / kind).iterdir():
            target = run_dir / kind / folder.name
            shutil.copytree(folder, target)
            shutil.copy(source / PROBE, target / PROBE)
            if scenario.get("script_road_fatal"):
                shutil.copy(source / "fatal.nut", target / "main.nut")


def prepare(
    scenario: Scenario,
    executables: dict[str, Path],
    builds: dict[str, Path],
    folder: Path,
    timeout: int,
    env: dict[str, str],
    result: core.Result,
) -> Scenario:
    if scenario["name"] != "script-infrastructure-tile":
        return scenario
    probe: Scenario = {
        **scenario,
        "script_road_fatal": True,
        "ticks": 1000,
        "settings": {
            key: value
            for key, value in scenario["settings"].items()
            if key != "game_scripts"
        },
    }
    runs = core.capture_pair(
        probe, executables, builds, folder, timeout, env, name="road-slope-fatal"
    )
    for role, run in runs.items():
        output = run["stdout"] + "\n".join(run["log"]).encode()
        if (
            not isinstance(run["exit"], int)
            or run["exit"] == 0
            or "road-slope fatal" not in observations(run["log"])
            or "road-slope returned" in observations(run["log"])
            or b"Reason: NOT_REACHED triggered at line " not in run["stdout"]
            or b"panicked at" in output
        ):
            raise RuntimeError(f"{role} script road slope native fatal path missing")
    core.findings(result)["script_road_slope_fatal"] = {
        role: run["exit"] for role, run in runs.items()
    }
    return scenario


def game_args(scenario: dict[str, Any]) -> list[str]:
    # Script prints log at level 2.
    return ["-d", "script=2"] if uses_ai(scenario) else []


def observations(lines: list[str]) -> list[str]:
    return [line.split("INFRA ", 1)[1] for line in lines if "INFRA " in line]


def missing(records: list[str]) -> list[str]:
    return [done for done in DONE if not any(r.startswith(done) for r in records)]


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
    problems = missing(records)
    if problems:
        raise ValueError(f"{role} infrastructure API evidence incomplete: {problems}")
    result[f"{mode}_{role}_script_infrastructure"] = records
    if (
        role == "candidate"
        and records != result[f"{mode}_reference_script_infrastructure"]
    ):
        result["problems"].append(f"{mode}: infrastructure API observations differ")
