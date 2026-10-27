"""File-loaded community NewGRF sets on a generated arctic map (#507).

The sets in `migration/newgrf` load through the original `[newgrf]` new-game
configuration, so loading, Action2/3 and callbacks run on real programs; the
scenario AI builds a train and buses from them."""

import shutil
from collections.abc import Mapping
from pathlib import Path
from typing import NotRequired, TypedDict

from .core import (
    ROOT,
    SNAPSHOT_TICKS,
    TICKS_PER_DAY,
    WITNESS_ENVIRONMENT,
    branch_witnesses,
    read_save,
)

AI_FOLDER = "newgrf-sets-scenario-ai"
SETS = ROOT / "migration/newgrf"
# Each set's Action14 parameter defaults: the original applies them only in the
# GUI, and a `[newgrf]` line without values leaves every parameter at zero.
GRFS = {
    "ogfx-trains.grf": "0 14 0 0",
    "ogfx-rv.grf": "0 9 9 1 1 1",
    "ogfx-industries.grf": "16 0 0 0 0 0 0 0 0 0",
}
# Candidate counters that must be nonzero; the reference has no witnesses.
REQUIRED = {
    "common_action2": ("binary", "error", "linear", "reseed", "storage"),
    "industry_resolver": (
        "location",
        "probability",
        "root",
        "tile_root",
        "tile_variable",
        "variable",
    ),
}


class Scenario(TypedDict):
    name: str
    kind: str
    seed: int
    map_log2: int
    land_generator: int
    newgrf_sets: bool
    ticks: int
    console: list[str]
    settings: Mapping[str, Mapping[str, str | int]]
    scenario_ai: NotRequired[str]
    witnesses: NotRequired[bool]


class Run(TypedDict):
    exit: int
    dir: Path
    log: list[str]
    snapshots: list[Path]


class Result(TypedDict, total=False):
    newgrf_sets_witnesses: dict[str, dict[str, dict[str, int]]]


def scenarios(soak: bool) -> list[Scenario]:
    cases = [("newgrf-sets-arctic", 7, 2)]
    if soak:
        cases.append(("newgrf-sets-arctic-long", 8, 8))
    return [
        {
            "name": name,
            "kind": "generate",
            "seed": 507,
            "map_log2": size,
            "land_generator": 1,
            "newgrf_sets": True,
            "ticks": years * 365 * TICKS_PER_DAY + SNAPSHOT_TICKS,
            "console": ["start_ai MigrationNewGRFSets"],
            "settings": {
                "game_creation": {"landscape": "arctic"},
                "difficulty": {"max_loan": 1000000},
                "newgrf": GRFS,
            },
        }
        for name, size, years in cases
    ]


def uses_ai(scenario: Scenario) -> bool:
    return bool(scenario.get("newgrf_sets"))


def install(scenario: Scenario, run_dir: Path) -> None:
    if not uses_ai(scenario):
        return
    shutil.copytree(scenario["scenario_ai"], run_dir / "ai/newgrf-sets")
    # Personal data directories of both builds, as in newgrf_loading.install.
    for personal in (
        ".openttd",
        ".openttd-rust",
        "xdg-data/openttd",
        "xdg-data/openttd-rust",
    ):
        target = run_dir / personal / "newgrf"
        target.mkdir(parents=True, exist_ok=True)
        for grf in GRFS:
            shutil.copy2(SETS / grf, target / grf)


def game_args(scenario: Scenario) -> list[str]:
    return ["-d", "script=2", "-d", "grf=1"] if uses_ai(scenario) else []


def game_environment(scenario: Scenario) -> dict[str, str]:
    return (
        WITNESS_ENVIRONMENT if uses_ai(scenario) and scenario.get("witnesses") else {}
    )


def check(
    scenario: Scenario,
    run: Run,
    mode: str,
    role: str,
    result: Result,
) -> None:
    if not uses_ai(scenario) or run["exit"]:
        return
    if not any(line.endswith("NEWGRF-SETS ready") for line in run["log"]):
        raise RuntimeError("NewGRF set scenario AI did not finish building")
    if run["snapshots"]:
        loaded = len(read_save(run["snapshots"][-1])["NGRF"]["elements"])
        if loaded != len(GRFS):
            raise RuntimeError(f"expected {len(GRFS)} saved NewGRFs, found {loaded}")
    for prefix, required in REQUIRED.items():
        if counts := branch_witnesses(scenario, run, role, prefix, required):
            result.setdefault("newgrf_sets_witnesses", {}).setdefault(mode, {})[
                prefix
            ] = counts
