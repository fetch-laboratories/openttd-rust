"""Engine lifecycle commands, preview, reliability and canonical modern reload."""

from pathlib import Path

from . import core, economy
from .console_adapter import console_adapter, prepared_console
from .core import (
    ROOT,
    WITNESS_ENVIRONMENT,
    branch_witnesses,
    compare_saves,
    witness_counts,
)

ADAPTER = ROOT / "tools/simulation/engines-console.cpp"
REQUIRED = {
    "monitors": (),
    "commands": ("rename", "visibility", "control"),
    "preview": ("offer", "accept", "join", "penalty"),
    "reliability": ("retire", "never_expire", "sync", "cutoff"),
    "reload": (),
    "subsidies": ("create", "award", "expire"),
    "subsidies-reload": (),
}
EXPECTED = {
    "monitors": ("controls true",),
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
    "subsidies": (
        "nearby true",
        "commands true true",
        "classes true true",
        "multipliers true",
        "wrap true",
        "disabled true true",
        "retries true",
        "routes true true true",
        "transfer true true",
    ),
    "subsidies-reload": ("loaded true",),
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
        for mode in ("commands", "preview", "reliability", "reload", "monitors")
    ]


def game_environment(scenario):
    return (
        WITNESS_ENVIRONMENT
        if scenario.get("engines") and scenario.get("witnesses")
        else {}
    )


def adapter(build, folder, env):
    economy.subsidy_reference(folder)
    return console_adapter(
        build, folder, env, ADAPTER, "engines", replace_objects=("saveload.cpp.o",)
    )


def prepare(scenario, binaries, builds, out, timeout, env, result):
    if "engines" not in scenario:
        return scenario
    folder = out / scenario["name"] / "input"
    normalized, record = core.normalize(Path(scenario["save"]), folder / "source")
    result["engines_input"] = record
    scenario = dict(scenario, save=str(normalized))
    executables = prepared_console("engines", adapter, binaries, builds, folder, env)
    if scenario["engines"] == "monitors":
        economy.monitor_load_probes(scenario, executables, builds, folder, timeout, env)
    if scenario["engines"] not in ("reload", "subsidies-reload"):
        return dict(
            scenario,
            executables=executables,
            console=["engines_scenario " + scenario["engines"], "unpause"],
        )
    subsidy = scenario["engines"] == "subsidies-reload"
    variant, prefix = ("subsidies", "SUBSIDY ") if subsidy else ("commands", "ENGINE ")
    saves = {role: folder / role / "engines.sav" for role in executables}
    runs = core.capture_pair(
        lambda role: dict(
            scenario,
            console=[f'engines_scenario {variant} "{saves[role]}"', "unpause"],
            ticks=1,
        ),
        executables,
        builds,
        folder,
        timeout,
        env,
    )
    captures = {}
    for role, run in runs.items():
        if (
            run["exit"]
            or not saves[role].is_file()
            or ("SUBSIDY transfer true true" if subsidy else "ENGINE hidden true")
            not in run["log"]
        ):
            raise RuntimeError("engine reload capture failed")
        captures[role] = [line for line in run["log"] if line.startswith(prefix)]
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
        console=[
            "engines_scenario " + ("subsidies-inspect" if subsidy else "inspect"),
            "unpause",
        ],
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
    component = (
        "monitor"
        if variant == "monitors"
        else ("subsidy" if variant.startswith("subsidies") else "engine")
    )
    prefix = component.upper() + " "
    if variant == "subsidies" and not any(
        line.startswith(prefix + "retry-rng ") for line in run["log"]
    ):
        raise RuntimeError("subsidy retry RNG result missing")
    for expected in EXPECTED[variant]:
        if prefix + expected not in run["log"]:
            raise RuntimeError("engine branch result missing: " + expected)
    if counts := branch_witnesses(scenario, run, role, component, REQUIRED[variant]):
        result[mode + "_" + component + "_branches"] = counts
