"""File-loaded community NewGRF sets on a generated arctic map (#507).

The pinned BaNaNaS sets load through the original `[newgrf]` new-game
configuration, so loading, Action2/3 and callbacks run on real programs; the
scenario AI builds a train and buses from them."""

import gzip
import shutil
from pathlib import Path

from . import community_ai, core
from .core import (
    SNAPSHOT_TICKS,
    TICKS_PER_DAY,
    WITNESS_ENVIRONMENT,
    branch_witnesses,
    read_save,
)

AI_FOLDER = "newgrf-sets-scenario-ai"
# Each set's Action14 parameter defaults: the original applies them only in the
# GUI, and a `[newgrf]` line without values leaves every parameter at zero.
# TarScanner indexes members under "<archive>.tar/<member>".
GRFS = {
    "4f472b31-OpenGFX_Trains-0.3.0.tar/OpenGFX_Trains-0.3.0/ogfx-trains.grf": "0 14 0 0",
    "4f472b32-OpenGFX_Road_Vehicles-0.4.1.tar/OpenGFX_Road_Vehicles-0.4.1/ogfx-rv.grf": "0 9 9 1 1 1",
    "4f472b33-OpenGFX_Industries-VERSION.tar/OpenGFX_Industries-VERSION/ogfx-industries.grf": "16 0 0 0 0 0 0 0 0 0",
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


class Scenario(core.Scenario):
    newgrf_sets: bool


Run = core.Run


class Result(core.Result, total=False):
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
    install_sets(run_dir)


def install_sets(run_dir: Path) -> None:
    """Place the pinned set archives in both builds' personal NewGRF folders."""
    paths = community_ai.fetch()
    # OpenTTD scans complete content archives, retaining their packaged notices.
    for personal in (
        ".openttd",
        ".openttd-rust",
        "xdg-data/openttd",
        "xdg-data/openttd-rust",
    ):
        target = run_dir / personal / "newgrf"
        target.mkdir(parents=True, exist_ok=True)
        for package in community_ai.packages():
            if package.kind == "newgrf":
                (target / package.filename.removesuffix(".gz")).write_bytes(
                    gzip.decompress(paths[package.filename].read_bytes())
                )


def game_args(scenario: Scenario) -> list[str]:
    return ["-d", "script=2", "-d", "grf=1"] if uses_ai(scenario) else []


def game_environment(scenario: Scenario) -> dict[str, str]:
    return (
        WITNESS_ENVIRONMENT if uses_ai(scenario) and scenario.get("witnesses") else {}
    )


def check(
    scenario: Scenario,
    run: Run,
    mode: core.Mode,
    role: core.Role,
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
