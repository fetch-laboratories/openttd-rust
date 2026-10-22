"""Engine lifecycle commands, preview, reliability and canonical modern reload."""

from pathlib import Path

from . import orders, rails
from .console_adapter import prepared_console
from .core import (
    ROOT,
    WITNESS_ENVIRONMENT,
    branch_witnesses,
    compare_saves,
    run_game,
    witness_counts,
)

ADAPTER = ROOT / "tools/simulation/engines-console.cpp"
REQUIRED = {
    "commands": ("rename", "visibility", "control"),
    "preview": ("offer", "accept", "join", "penalty"),
    "reliability": ("retire", "never_expire", "sync", "cutoff"),
    "reload": (),
}
EXPECTED = {
    "commands": (
        "company-bits true",
        "clear-special true",
        "hidden true",
        "disabled true",
    ),
    "preview": ("offered true true", "joined true true", "penalty true true"),
    "reliability": (
        "startup-life true",
        "retired true true true",
        "never-expire true true",
        "synced true",
        "cutoff true",
    ),
    "reload": ("reloaded true",),
}


def scenarios(soak):
    return [
        {
            "name": "engines-" + mode,
            "kind": "save",
            "save": str(ROOT / "migration/saves/padhattan-ridge-2000.sav"),
            "engines": mode,
            "short_checkpoint": True,
            "ticks": 74 if soak else 1,
        }
        for mode in REQUIRED
    ]


def game_environment(scenario):
    return (
        WITNESS_ENVIRONMENT
        if scenario.get("engines") and scenario.get("witnesses")
        else {}
    )


def adapter(build, folder, env):
    return orders.adapter(
        build, folder, env, source=ADAPTER, name="engines", afterload=False
    )


def prepare(scenario, binaries, builds, out, timeout, env, result):
    if "engines" not in scenario:
        return scenario
    folder = out / scenario["name"] / "input"
    normalized, record = rails.normalize(Path(scenario["save"]), folder / "source")
    result["engines_input"] = record
    scenario = dict(scenario, save=str(normalized))
    executables = prepared_console("engines", adapter, binaries, builds, folder, env)
    if scenario["engines"] != "reload":
        return dict(
            scenario,
            executables=executables,
            console=["engines_scenario " + scenario["engines"], "unpause"],
        )
    saves, captures = {}, {}
    for role in ("reference", "candidate"):
        save = folder / role / "engines.sav"
        setup = dict(
            scenario,
            console=[f'engines_scenario commands "{save}"', "unpause"],
            ticks=1,
        )
        run = run_game(
            setup,
            executables[role],
            builds[role],
            folder / role / "capture",
            timeout,
            env,
            False,
        )
        if run["exit"] or not save.is_file() or "ENGINE hidden true" not in run["log"]:
            raise RuntimeError("engine reload capture failed")
        saves[role] = save
        captures[role] = [line for line in run["log"] if line.startswith("ENGINE ")]
    differences = compare_saves(
        saves["reference"],
        saves["candidate"],
        scenario["name"],
        10,
        {"chunks": 0, "elements": 0, "masked": {}},
    )
    if differences or captures["reference"] != captures["candidate"]:
        result["differences"].extend(differences)
        raise RuntimeError("engine reload capture differs")
    result["engines_capture"] = captures
    return dict(
        scenario,
        executables=executables,
        role_inputs={role: {"save": str(save)} for role, save in saves.items()},
        console=["engines_scenario inspect", "unpause"],
    )


def check(scenario, run, mode, role, result):
    if role == "candidate" and scenario.get("witnesses"):
        counts = witness_counts(run, "engine")
        if counts is not None:
            result[mode + "_engine_crossings"] = {
                key: counts[key] for key in ("entries", "services", "bytes")
            }
    if "engines" not in scenario or run["exit"] or not run["snapshots"]:
        return
    variant = scenario["engines"]
    for expected in EXPECTED[variant]:
        if "ENGINE " + expected not in run["log"]:
            raise RuntimeError("engine branch result missing: " + expected)
    lines = [line for line in run["log"] if line.startswith("ENGINE ")]
    key = mode + "_engine_results"
    if role == "reference":
        result[key] = lines
    elif result[key] != lines:
        raise RuntimeError("engine command/branch results differ")
    if counts := branch_witnesses(scenario, run, role, "engine", REQUIRED[variant]):
        result[mode + "_engine_branches"] = counts
