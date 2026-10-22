"""Fleet hierarchy, renewal and actual replacement transactions."""

import hashlib
from pathlib import Path

from .core import (
    ROOT,
    RUNTIME_DIRECTORIES,
    WITNESS_ENVIRONMENT,
    branch_witnesses,
    compare_saves,
    run_game,
)

ADAPTER = ROOT / "tools/simulation/fleet-console.cpp"
# Rust construction and clone branches each transaction scenario must reach.
BRANCHES = {
    "transaction-build-road": ["road", "restore"],
    "transaction-build-aircraft": ["aircraft", "rotor", "restore"],
    "transaction-clone": ["clone_join_sell"],
}


def scenarios(soak):
    cases = [
        {
            "name": "fleet-" + mode,
            "kind": "save",
            "save": str(
                ROOT
                / "migration/saves"
                / (
                    "padhattan-ridge-1996.sav"
                    if mode.startswith("train")
                    or mode
                    in (
                        "transaction-refit-train",
                        "transaction-mass-train",
                        "transaction-clone",
                    )
                    else "padhattan-ridge-2000.sav"
                    if mode.startswith("drain") or mode == "transaction-meta"
                    else "water-ferry.sav"
                    if mode
                    in ("ship", "transaction-refit-ship", "transaction-mass-ship")
                    else "aircraft-route.sav"
                    if mode
                    in (
                        "aircraft",
                        "transaction-refit-aircraft",
                        "transaction-mass-aircraft",
                        "transaction-build-aircraft",
                    )
                    else "opus-55-167-002.sav"
                )
            ),
            "fleet": mode,
            "short_checkpoint": True,
            # The drain cases run past the scheduled witness on 2001-01-01.
            "ticks": 400 if mode.startswith("drain") else 74 if soak else 1,
            "console": ["fleet_scenario " + mode, "unpause"],
        }
        for mode in (
            "groups",
            "transaction-meta",
            "transaction-mass-train",
            "transaction-mass-road",
            "transaction-mass-ship",
            "transaction-mass-aircraft",
            "transaction-refit-train",
            "transaction-refit-road",
            "transaction-refit-ship",
            "transaction-refit-aircraft",
            "transaction-build-road",
            "transaction-build-aircraft",
            "transaction-clone",
            "list",
            "replace",
            "cash",
            "train",
            "train-rollback",
            "train-wagons",
            "ship",
            "aircraft",
            "drain",
            "drain-cash",
        )
    ]

    cases.append(dict(cases[0], name="fleet-reload", fleet="reload"))
    return cases


def game_environment(scenario):
    witnessed = scenario.get("fleet") in BRANCHES and scenario.get("witnesses")
    return WITNESS_ENVIRONMENT if witnessed else {}


def install(scenario, run_dir):
    if scenario.get("fleet", "").startswith("drain"):
        (run_dir / "fleet-drain.scr").write_text("fleet_scenario drain-inspect\n")


def adapter(build, folder, env):
    from .console_adapter import console_adapter

    return console_adapter(build, folder, env, ADAPTER, "fleet")


def prepare(scenario, binaries, builds, out, timeout, env, result):
    if "fleet" not in scenario:
        return scenario
    import migration

    folder = out / scenario["name"] / "input"
    if scenario["fleet"].startswith(("train", "drain")) or scenario["fleet"] in (
        "transaction-meta",
        "transaction-refit-train",
        "transaction-mass-train",
        "transaction-clone",
    ):
        from .rails import normalize

        source, record = normalize(Path(scenario["save"]), folder / "normalized")
        result["fleet_input"] = record
        scenario = dict(scenario, save=str(source))
    native_builds = {
        "reference": migration.REFERENCE_BUILD,
        "candidate": migration.REFERENCE_BUILD
        if binaries["reference"] == binaries["candidate"]
        else ROOT / "build-rust",
    }
    executables = {}
    for role in ("reference", "candidate"):
        from .runtime import prepared_adapter

        prepared = prepared_adapter("fleet", role, builds, folder / role)
        if prepared is not None:
            executables[role] = prepared
        else:
            lock = (
                migration.reference_lock(shared=True)
                if native_builds[role] == migration.REFERENCE_BUILD
                else migration.candidate_lock(native_builds[role], shared=True)
            )
            with lock:
                executables[role] = adapter(native_builds[role], folder / role, env)
        for name in RUNTIME_DIRECTORIES:
            if (builds[role] / name).is_dir():
                (folder / role / name).symlink_to(
                    builds[role] / name, target_is_directory=True
                )
    if scenario["fleet"] == "reload":
        saves, capture = {}, {}
        setup = dict(
            scenario, console=["fleet_scenario groups-store", "unpause"], ticks=1
        )
        for role in ("reference", "candidate"):
            run = run_game(
                setup,
                executables[role],
                builds[role],
                folder / role / "capture",
                timeout,
                env,
                desync=False,
            )
            if (
                run["exit"]
                or not run["snapshots"]
                or "FLEET groups true 1 1 true" not in run["log"]
            ):
                raise RuntimeError("fleet reload capture lacks live groups/rules")
            saves[role] = run["snapshots"][-1]
            capture[role] = {
                "input_sha256": hashlib.sha256(saves[role].read_bytes()).hexdigest(),
                "witnesses": [line for line in run["log"] if line.startswith("FLEET ")],
            }
        differences = compare_saves(
            saves["reference"],
            saves["candidate"],
            scenario["name"],
            20,
            result["stats"],
        )
        if (
            differences
            or capture["reference"]["witnesses"] != capture["candidate"]["witnesses"]
        ):
            result["differences"].extend(differences)
            raise RuntimeError("fleet reload capture differs")
        result["fleet_capture"] = capture
        return dict(
            scenario,
            executables=executables,
            role_inputs={role: {"save": str(save)} for role, save in saves.items()},
            console=["fleet_scenario inspect", "unpause"],
        )
    return dict(scenario, executables=executables)


def check(scenario, run, mode, role, result):
    if "fleet" not in scenario or run["exit"] or not run["snapshots"]:
        return
    required = {
        **{
            "transaction-mass-" + kind: [
                "FLEET mass-test true true",
                "FLEET mass true true true true",
                "FLEET mass-empty true",
            ]
            for kind in ("train", "road", "ship", "aircraft")
        },
        **{
            "transaction-refit-" + kind: ["FLEET refit-loaded true true true"]
            for kind in ("train", "road", "ship", "aircraft")
        },
        **{
            "transaction-build-" + kind: ["FLEET build-restored true true"]
            for kind in ("road", "aircraft")
        },
        "transaction-clone": ["FLEET clone-join false true true true"],
        "transaction-meta": [
            f"FLEET transaction-meta {kind} true 50 true true" for kind in range(4)
        ],
        "reload": ["FLEET reloaded true 1 true"],
        "list": [
            "FLEET cost list-test true 0 65535",
            "FLEET cost list-wide-id false 0 65535",
            "FLEET cost list-invalid false 0 65535",
            "FLEET list-rejected true true true true",
            "FLEET list-group true true true",
        ],
        "train-wagons": [
            "FLEET free-replaced true true true true",
            "FLEET wagons-replaced true true true 6 true 0",
        ],
        "groups": [
            "FLEET rule all true true",
            "FLEET rule protected true false",
            "FLEET rule inherited true true",
            "FLEET groups true 1 1 true",
            "FLEET deleted true true true",
        ],
        **{
            operation: [
                "FLEET test-rng true original true",
                "FLEET test-cargo true",
                "FLEET replaced true",
                "FLEET transferred true true",
            ]
            for operation in ("replace", "train", "ship", "aircraft")
        },
        # Frozen from the unchanged reference: the running vehicle entered its
        # service depot, was renewed (new ID 53, age 2 days) and left running;
        # with only the reserve it restarted unrenewed with money-limit news.
        "drain": [
            "FLEET drain input 12 139 191152",
            "FLEET drain old false false -1 money 176297",
            "FLEET drain new 53 139 2 false false",
        ],
        "drain-cash": [
            "FLEET drain input 12 139 100000",
            "FLEET drain old true false 5860 money 101673",
            "FLEET drain news 12 Autorenew failed on Road Vehicle #17 (money limit)",
        ],
        "cash": [
            "FLEET test-rng true original true",
            "FLEET rollback true true true true",
        ],
        "train-rollback": [
            "FLEET test-rng true original true",
            "FLEET rollback true true true true",
        ],
    }[scenario["fleet"]]
    for line in required:
        if line not in run["log"]:
            result["problems"].append(f"{mode}/{role}: missing {line}")
    if scenario["fleet"] in BRANCHES:
        result[mode + "_" + role + "_branches"] = branch_witnesses(
            scenario, run, role, "vehicle_transactions", BRANCHES[scenario["fleet"]]
        )
    witnesses = [line for line in run["log"] if line.startswith("FLEET ")]
    key = mode + "_fleet_witnesses"
    if role == "reference":
        result[key] = witnesses
    elif result[key] != witnesses:
        result["problems"].append(f"{mode}: native fleet witnesses differ")
