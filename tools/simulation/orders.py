"""Orders command, backup and initialized unbunching evidence on the player save."""

import hashlib

from .console_adapter import prepared_console
from .core import (
    ROOT,
    SNAPSHOT_TICKS,
    WITNESS_ENVIRONMENT,
    branch_witnesses,
    capture_pair,
    compare_saves,
    decode_element,
    read_save,
)
from .play_saves import DISTRIBUTIONS

ADAPTER = ROOT / "tools/simulation/orders-console.cpp"
COMMON = "roadveh[0]/common[0]/"
# Station/depot visits and route prediction (vehicle/visits.rs) reached by the visits
# fixture's refit, autorefit, conditional, full-load, cross-graph, depot and
# unload-only orders. Unreached (#156): graph_failure (exhausted link-graph
# pool), depot_refit_failed (refit cost beyond cash), depot_elsewhere,
# and dummy. The visits adapter reserves real station cargo before departure.
VISITS = ["aircraft_mail", "autorefit", "begin_loading", "conditional_branch"]
VISITS += ["depot", "depot_halt", "depot_refit", "full_load_increase"]
VISITS += ["graph_join", "graph_merge_first", "graph_merge_second", "graph_new"]
VISITS += ["leave_empty", "leave_loaded", "loading_depart", "merge_blocked"]
VISITS += ["refit", "reset_refit"]


def scenarios(soak):
    return [
        {
            "name": "orders-" + mode,
            "kind": "save",
            "save": str(ROOT / "migration/saves/opus-55-167-002.sav"),
            "orders": mode,
            "short_checkpoint": True,
            "ticks": 74
            if soak and mode not in ("client-restore", "client-shared-restore")
            else 1,
            "console": ["unpause"],
        }
        for mode in (
            "commands",
            "offline-reload",
            "client-restore",
            "client-shared-restore",
            "active-reload",
            "implicit-wrap",
            "oversized",
        )
    ] + [
        {
            "name": "orders-visits",
            "kind": "save",
            "save": str(ROOT / "migration/saves/padhattan-ridge-2000.sav"),
            "orders": "visits",
            "console": [*DISTRIBUTIONS["cargodist"][:-1], "orders_scenario visits"]
            + ["unpause"],
            "ticks": 4 * SNAPSHOT_TICKS,
        },
        {
            "name": "orders-visits-reserved",
            "kind": "save",
            "save": str(ROOT / "migration/saves/padhattan-ridge-2000.sav"),
            "orders": "visits-reserved",
            "console": [
                *DISTRIBUTIONS["cargodist"][:-1],
                "orders_scenario visits-reserved",
                "unpause",
            ],
            "ticks": 1,
            "short_checkpoint": True,
        },
    ]


def game_environment(scenario):
    if scenario.get("orders", "").startswith("visits") and scenario.get("witnesses"):
        return WITNESS_ENVIRONMENT
    return {"OPENTTD_ORDERS_CLIENT": "1"} if scenario.get("orders_client") else {}


def adapter(build, folder, env, *, source=ADAPTER, name="orders", afterload=True):
    from .console_adapter import console_adapter

    return console_adapter(
        build, folder, env, source, name, ("_Z13AfterLoadGamev",) if afterload else ()
    )


def prepare(scenario, binaries, builds, out, timeout, env, result):
    if "orders" not in scenario:
        return scenario

    folder = out / scenario["name"] / "input"
    executables = prepared_console("orders", adapter, binaries, builds, folder, env)
    mode = scenario["orders"]
    if mode.startswith("visits"):
        from pathlib import Path

        from .core import normalize

        save, result["visits_input"] = normalize(Path(scenario["save"]), folder)
        return dict(scenario, executables=executables, save=str(save))
    if mode in ("commands", "implicit-wrap", "oversized"):
        return dict(
            scenario,
            executables=executables,
            console=[
                "orders_scenario " + ("setup" if mode == "commands" else mode),
                "unpause",
            ],
        )
    shared = mode == "client-shared-restore"
    saves, captures, active_inputs = {}, {}, {}

    def setup(role):
        backup = folder / role / "backups.sav"
        return dict(
            scenario,
            console=[
                f'orders_scenario setup "{backup}" {"shared" if shared else "unique"}',
                "unpause",
            ],
            ticks=1,
        )

    runs = capture_pair(setup, executables, builds, folder, timeout, env)
    for role, run in runs.items():
        backup = folder / role / "backups.sav"
        if run["exit"] or "ORDERS backups captured 2" not in run["log"]:
            raise RuntimeError(f"{role} did not capture two actual order backups")
        if len(read_save(backup)["BKOR"]["elements"]) != 1:
            raise RuntimeError("server-policy save has no actual BKOR contents")
        captures[role] = {
            "input_sha256": hashlib.sha256(backup.read_bytes()).hexdigest(),
            "commands": setup(role)["console"],
            "witnesses": [line for line in run["log"] if line.startswith("ORDERS ")],
        }
        saves[role] = backup
    if captures["reference"]["witnesses"] != captures["candidate"]["witnesses"]:
        raise RuntimeError("native command/departure witnesses differ")
    result["orders_capture"] = captures
    records = compare_saves(
        saves["reference"], saves["candidate"], scenario["name"], 20, result["stats"]
    )
    if records:
        result["differences"].extend(records)
        raise RuntimeError("serialized backup preparation states differ")
    if mode == "active-reload":
        runs = capture_pair(
            lambda role: dict(
                scenario,
                save=str(saves[role]),
                console=["orders_scenario active", "unpause"],
                ticks=1,
            ),
            executables,
            builds,
            folder,
            timeout,
            env,
            "active",
        )
        for role, run in runs.items():
            if (
                run["exit"]
                or "ORDERS active 7 8 1" not in run["log"]
                or not any(
                    line.startswith("ORDERS state conditional-active 126 4 2 3 3 ")
                    for line in run["log"]
                )
            ):
                raise RuntimeError(
                    f"{role} has no resolved conditional/implicit active state"
                )
            saves[role] = run["snapshots"][-1]
            active_inputs[role] = {
                "input_sha256": hashlib.sha256(saves[role].read_bytes()).hexdigest(),
                "witnesses": [
                    line for line in run["log"] if line.startswith("ORDERS ")
                ],
            }
        result["orders_active"] = active_inputs
        records = compare_saves(
            saves["reference"],
            saves["candidate"],
            scenario["name"],
            20,
            result["stats"],
        )
        if records:
            result["differences"].extend(records)
            raise RuntimeError("active preparation states differ")
    # Compare each preparation before loading its same-build save; crossing build
    # revisions would add a GLOG revision event unrelated to orders behavior.
    return dict(
        scenario,
        role_inputs={role: {"save": str(save)} for role, save in saves.items()},
        executables=executables,
        orders_client=mode in ("client-restore", "client-shared-restore"),
        console=[
            "orders_scenario restore " + ("shared" if shared else "unique")
            if mode in ("client-restore", "client-shared-restore")
            else "orders_scenario inspect",
            "unpause",
        ],
    )


def check(scenario, run, mode, role, result):
    if "orders" not in scenario or run["exit"] or not run["snapshots"]:
        return
    operation = scenario["orders"]
    if operation == "visits":
        if "VISITS setup 3 3 120 0" not in run["log"]:
            result["problems"].append(f"{mode}/{role}: missing visits setup")
        if counts := branch_witnesses(scenario, run, role, "vehicle_visits", VISITS):
            result[f"{mode}_{role}_visit_branches"] = counts
        return
    if operation == "visits-reserved":
        reservations = [
            line.split()
            for line in run["log"]
            if line.startswith("VISITS reservation ")
        ]
        if len(reservations) != 1 or reservations[0][3:] != [
            "7",
            "true",
            "true",
            "true",
        ]:
            raise RuntimeError("missing nonempty reserved-cargo departure")
        if counts := branch_witnesses(
            scenario, run, role, "vehicle_visits", ["reservation"]
        ):
            result[f"{mode}_{role}_visit_branches"] = counts
        return
    required = {
        "commands": ["ORDERS backups captured 2", "ORDERS waiting true true"],
        "offline-reload": ["ORDERS backups offline 0"],
        "client-restore": [
            "ORDERS client-fixups 1 1",
            "ORDERS backups before 1",
            "ORDERS backups after 0",
        ],
        "client-shared-restore": [
            "ORDERS client-fixups 1 1",
            "ORDERS backups before 1",
            "ORDERS backups after 0",
        ],
        "active-reload": ["ORDERS backups offline 0"],
        "implicit-wrap": ["ORDERS state implicit-wrap 126 1 1 0 0 0 0 0 0 0"],
        "oversized": ["ORDERS oversized 300 0"],
    }[operation]
    for line in required:
        if line not in run["log"]:
            result["problems"].append(f"{mode}/{role}: missing {line}")
    chunk = read_save(run["snapshots"][-1])["VEHS"]
    vehicles = {
        index: decode_element(chunk, body)
        for index, body in chunk["elements"]
        if decode_element(chunk, body)["type"] == 1
    }
    # Compare the actual initialized trip values separately from #83's general mask.
    trips = {index: row[COMMON + "round_trip_time"] for index, row in vehicles.items()}
    key = mode + "_orders_trips"
    if role == "reference":
        result[key] = trips
    elif result[key] != trips:
        result["problems"].append(f"{mode}: unmasked initialized-peer trip mismatch")
    if operation in ("client-restore", "client-shared-restore") and trips[126] != 0:
        result["problems"].append(
            f"{mode}/{role}: restoration did not reset unbunching"
        )
    if operation in ("commands", "offline-reload") and trips[127] != 6000:
        result["problems"].append(
            f"{mode}/{role}: lost initialized 6000-tick peer sample"
        )
    if operation in ("commands", "offline-reload") and trips[126] != 3000:
        result["problems"].append(f"{mode}/{role}: no 3000-tick depot trip measurement")
