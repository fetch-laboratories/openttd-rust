"""Object ownership scenarios and named unchanged-original-body gap inputs."""

from pathlib import Path

from . import core, generated, rails
from .console_adapter import console_adapter, prepared_console
from .core import ROOT, WITNESS_ENVIRONMENT, branch_witnesses


def scenarios(soak):
    worlds = [s for s in generated.scenarios(soak) if "settings-" in s["name"]][:4]
    return [
        dict(s, name="objects-generation-" + str(i), objects_generation=True)
        for i, s in enumerate(worlds)
    ] + [
        dict(
            name="objects-" + mode,
            kind="save",
            objects=mode,
            ticks=1,
            save=str(ROOT / "migration/saves/padhattan-ridge-2000.sav"),
            short_checkpoint=True,
        )
        for mode in ("commands", "gaps", "reload")
    ]


def game_environment(scenario):
    selected = "objects" in scenario or "objects_generation" in scenario
    return WITNESS_ENVIRONMENT if selected and scenario.get("witnesses") else {}


def adapter(build, folder, env):
    return console_adapter(
        build,
        folder,
        env,
        ROOT / "tools/simulation/objects-console.cpp",
        "objects",
        replace_objects=("/src/object_cmd.cpp.o",),
    )


def prepare(scenario, binaries, builds, out, timeout, env, result):
    if "objects" not in scenario:
        return scenario
    folder = out / scenario["name"] / "input"
    source, _ = rails.normalize(Path(scenario["save"]), folder / "normalized")
    scenario = dict(scenario, save=str(source))
    executables = prepared_console("objects", adapter, binaries, builds, folder, env)
    if scenario["objects"] != "reload":
        return dict(
            scenario,
            executables=executables,
            console=["objects_scenario " + scenario["objects"], "unpause"],
        )
    saves = {role: folder / role / "objects.sav" for role in ("reference", "candidate")}
    for role in ("reference", "candidate"):
        setup = dict(
            scenario,
            console=[f'objects_scenario commands "{saves[role]}"', "unpause"],
        )
        capture = core.run_game(
            setup,
            executables[role],
            builds[role],
            folder / role / "capture",
            timeout,
            env,
            False,
        )
        if (
            capture["exit"]
            or not saves[role].is_file()
            or "OBJECT id-reused true" not in capture["log"]
        ):
            raise RuntimeError("object reload capture failed")
    differences = core.compare_saves(
        saves["reference"],
        saves["candidate"],
        scenario["name"],
        10,
        {"chunks": 0, "elements": 0, "masked": {}},
    )
    if differences:
        raise RuntimeError(f"object reload capture differs: {differences}")
    return dict(
        scenario,
        executables=executables,
        role_inputs={r: {"save": str(s)} for r, s in saves.items()},
        console=["objects_scenario inspect", "unpause"],
    )


def check(scenario, run, mode, role, result):
    if "objects_generation" in scenario:
        branches = ("generate",)
        if scenario["name"].endswith("-1"):
            branches += ("lighthouse", "transmitter")
        branch_witnesses(scenario, run, role, "object", branches)
    if "objects" not in scenario:
        return
    required = {
        "commands": ("hq-cargo 5 2 1", "id-reused true", "water-clear 1 16 2 17 0 17"),
        "gaps": (
            "count-wrap true|scratch-growth true true|callback-before-clear true|"
            "cleared-before-bridge true|callback-colour true|"
            "cancellation 0 true|cancellation 1 true|query-animation-order true"
        ).split("|"),
        "reload": ("afterload-counts true",),
    }[scenario["objects"]]
    for text in required:
        if "OBJECT " + text not in run["log"]:
            raise RuntimeError(f"{mode}/{role}: missing object evidence: {text}")
    lines = [line for line in run["log"] if line.startswith("OBJECT ")]
    if (
        scenario["objects"] == "gaps"
        and sum(line.startswith("OBJECT query ") for line in lines) != 96
    ):
        raise RuntimeError(f"{mode}/{role}: missing object query results")
    key = mode + "_object_results"
    if role == "reference":
        result[key] = lines
    elif result[key] != lines:
        raise RuntimeError(f"{mode}: object command/query results differ")

    branches = {
        "commands": "build clear hq_grow hq_relocate area cargo hq_pass hq_mail owner colours autoslope build_above".split(),
        "gaps": ("slope_callback",),
        "reload": (),
    }[scenario["objects"]]
    branch_witnesses(scenario, run, role, "object", branches)
    if scenario["objects"] == "commands":  # Object clearing redispatches to water.
        branch_witnesses(scenario, run, role, "water_cmd", ("owner",))
