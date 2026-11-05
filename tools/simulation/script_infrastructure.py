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
from typing import Any

from . import core

AI_FOLDER = "script-infrastructure-ai"
PROBE = "probe.nut"
CONSOLE = ["start_ai MigrationInfra", "unpause"]
SETTINGS = {
    "game_scripts": {"MigrationInfraGS": ""},
    "gui": {"ai_developer_tools": "true"},
}
WHO = ("ai", "gs.deity", "gs.company", "gs.invalid")
# Each probe ends with this record; absence means a script stopped early.
DONE = tuple(f"{who} done" for who in WHO)


class Scenario(core.Scenario):
    script_infrastructure: bool


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
    for kind in ("ai", "game"):
        for folder in (source / kind).iterdir():
            target = run_dir / kind / folder.name
            shutil.copytree(folder, target)
            shutil.copy(source / PROBE, target / PROBE)


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
