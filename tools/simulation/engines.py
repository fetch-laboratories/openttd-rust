"""Engine lifecycle commands, preview, reliability and canonical modern reload."""

from pathlib import Path

from . import economy, rails
from .console_adapter import console_adapter, prepared_console
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
    "types": (),
    "types-reload": (),
    "monitors": (),
    "commands": ("rename", "visibility", "control"),
    "preview": ("offer", "accept", "join", "penalty"),
    "reliability": ("retire", "never_expire", "sync", "cutoff"),
    "reload": (),
    "subsidies": ("create", "award", "expire"),
    "subsidies-reload": (),
}
EXPECTED = {
    "types": (
        "types-case afterload-masks true",
        "types-case fixedpoint true",
        "types-case excluded-50 true",
        "types-case excluded-52 true",
        "types-case excluded-53 true",
        "types-case excluded-54 true",
        "types-case excluded-55 true",
        "types-case excluded-56 true",
        "types-case excluded-57 true",
        "types-case date-zero true",
        "types-case seeded-cycle true",
        "types-case inclusive-day true",
        "types-case max-date true",
        "types-case before-release true",
        "types-case company-bits true",
        "types-case all-multirail true",
        "types-case preview-owner true",
        "types-case one-year true",
        "types-case wagon-only true",
        "types-case climate true",
        "types-case lifecycle-enable true",
        "types-case lifecycle-disable true",
        "types-case monthly-publication true",
        "types-case daily-publication true",
        "types-case hidden-and-masks true",
        "types-case hidden-only-rail true",
        "types-case tram-only-mask true",
        "types-case road-only-mask true",
        "types-case company-hidden true",
        "types-case town true",
        "types-case deity true",
        "types-case editor true",
        "types-case generation true",
        "types-case restoration true",
    ),
    "types-reload": ("types-case afterload-masks true", "types-case restoration true"),
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
        for mode in (
            "commands",
            "preview",
            "reliability",
            "reload",
            "monitors",
            "types",
            "types-reload",
        )
    ]


def game_environment(scenario):
    env = (
        dict(WITNESS_ENVIRONMENT)
        if scenario.get("engines") and scenario.get("witnesses")
        else {}
    )
    if scenario.get("engines", "").startswith("types"):
        env["OPENTTD_ENGINE_TYPES"] = "1"
    return env


def adapter(build, folder, env):
    economy.subsidy_reference(folder)
    return console_adapter(
        build,
        folder,
        env,
        ADAPTER,
        "engines",
        wraps=("_Z18CopyTempEngineDatav",),
        replace_objects=("saveload.cpp.o",),
    )


def prepare(scenario, binaries, builds, out, timeout, env, result):
    if "engines" not in scenario:
        return scenario
    folder = out / scenario["name"] / "input"
    normalized, record = rails.normalize(Path(scenario["save"]), folder / "source")
    result["engines_input"] = record
    scenario = dict(scenario, save=str(normalized))
    executables = prepared_console("engines", adapter, binaries, builds, folder, env)
    if scenario["engines"] == "monitors":
        economy.monitor_load_probes(scenario, executables, builds, folder, timeout, env)
    if scenario["engines"] not in ("reload", "subsidies-reload", "types-reload"):
        return dict(
            scenario,
            executables=executables,
            console=["engines_scenario " + scenario["engines"], "unpause"],
        )
    types = scenario["engines"] == "types-reload"
    subsidy = scenario["engines"] == "subsidies-reload"
    variant, prefix = ("subsidies", "SUBSIDY ") if subsidy else ("commands", "ENGINE ")
    if types:
        variant, prefix = "types", "ENGINE "
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
        required = (
            EXPECTED["types"]
            if types
            else ("transfer true true",)
            if subsidy
            else ("hidden true",)
        )
        if (
            run["exit"]
            or not save.is_file()
            or any(prefix + marker not in run["log"] for marker in required)
            or (types and not type_markers(run["log"], capture=True))
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
            "engines_scenario "
            + (
                "types-inspect"
                if types
                else "subsidies-inspect"
                if subsidy
                else "inspect"
            ),
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
    if variant.startswith("types") and not type_markers(
        run["log"], capture=variant == "types"
    ):
        raise RuntimeError("type loadphase/mask/command/restoration markers missing")
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
    lines = [line for line in run["log"] if line.startswith(prefix)]
    key = mode + "_" + component + "_results"
    if role == "reference":
        result[key] = lines
    elif result[key] != lines:
        raise RuntimeError("engine command/branch results differ")
    if counts := branch_witnesses(scenario, run, role, component, REQUIRED[variant]):
        result[mode + "_" + component + "_branches"] = counts


def type_markers(lines, capture):
    required = ["ENGINE types-loadphase true", "ENGINE types-complete true"]
    if not all(marker in lines for marker in required):
        return False
    if not any(line.startswith("ENGINE types-masks loaded ") for line in lines):
        return False
    if capture:
        for label in (
            "rail-before",
            "road-before",
            "tram-before",
            "rail-after",
            "road-after",
            "tram-after",
            "hidden-company",
            "hidden-town",
        ):
            for execute in ("false", "true"):
                if not any(
                    line.startswith(f"ENGINE types-command {label} {execute} ")
                    for line in lines
                ):
                    return False
    return True
