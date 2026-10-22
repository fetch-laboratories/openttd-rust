"""Engine lifecycle commands, preview, reliability and canonical modern reload."""

from pathlib import Path

from . import economy, orders, rails
from .core import (
    ROOT,
    RUNTIME_DIRECTORIES,
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
    "subsidies": ("create", "award", "expire"),
    "subsidies-reload": (),
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
    "subsidies": (
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
        for mode in ("commands", "preview", "reliability", "reload")
    ]


def game_environment(scenario):
    return (
        WITNESS_ENVIRONMENT
        if scenario.get("engines") and scenario.get("witnesses")
        else {}
    )


def adapter(build, folder, env):
    economy.subsidy_reference(folder)
    return orders.adapter(
        build, folder, env, source=ADAPTER, name="engines", afterload=False
    )


def prepare(scenario, binaries, builds, out, timeout, env, result):
    if "engines" not in scenario:
        return scenario
    import migration

    from .runtime import prepared_adapter

    folder = out / scenario["name"] / "input"
    normalized, record = rails.normalize(Path(scenario["save"]), folder / "source")
    result["engines_input"] = record
    scenario = dict(scenario, save=str(normalized))
    executables = {}
    for role in ("reference", "candidate"):
        prepared = prepared_adapter("engines", role, builds, folder / role)
        if prepared is None:
            native = (
                migration.REFERENCE_BUILD
                if role == "reference" or binaries[role] == binaries["reference"]
                else ROOT / "build-rust"
            )
            lock = (
                migration.reference_lock(shared=True)
                if native == migration.REFERENCE_BUILD
                else migration.candidate_lock(native, shared=True)
            )
            with lock:
                prepared = adapter(native, folder / role, env)
        executables[role] = prepared
        for name in RUNTIME_DIRECTORIES:
            if (builds[role] / name).is_dir():
                (folder / role / name).symlink_to(
                    builds[role] / name, target_is_directory=True
                )
    if scenario["engines"] not in ("reload", "subsidies-reload"):
        return dict(
            scenario,
            executables=executables,
            console=["engines_scenario " + scenario["engines"], "unpause"],
        )
    subsidy = scenario["engines"] == "subsidies-reload"
    variant, prefix = ("subsidies", "SUBSIDY ") if subsidy else ("commands", "ENGINE ")
    saves, captures = {}, {}
    for role in ("reference", "candidate"):
        save = folder / role / "engines.sav"
        setup = dict(
            scenario,
            console=[f'engines_scenario {variant} "{save}"', "unpause"],
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
        if (
            run["exit"]
            or not save.is_file()
            or ("SUBSIDY transfer true true" if subsidy else "ENGINE hidden true")
            not in run["log"]
        ):
            raise RuntimeError("engine reload capture failed")
        saves[role] = save
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
    component = "subsidy" if variant.startswith("subsidies") else "engine"
    prefix = component.upper() + " "
    for expected in EXPECTED[variant]:
        if prefix + expected not in run["log"]:
            raise RuntimeError("engine branch result missing: " + expected)
    lines = [line for line in run["log"] if line.startswith(prefix)]
    key = mode + "_" + component + "_results"
    if role == "reference":
        result[key] = lines
    elif result[key] != lines:
        raise RuntimeError("engine command/branch results differ")
    if counts := branch_witnesses(scenario, run, role, component, REQUIRED[variant]):
        result[mode + "_" + component + "_branches"] = counts
