"""Owner-built multimodal fixtures and rail controller evidence (#86/#179/#183/#221)."""

import hashlib
import json
import re
import shutil
import struct
import subprocess
from pathlib import Path

from . import core
from .console_adapter import console_adapter, prepared_console
from .core import (
    ROOT,
    SNAPSHOT_TICKS,
    TICKS_PER_DAY,
    decode_element,
    normalize,
    read_save,
    run_game,
)
from .play_saves import DISTRIBUTIONS

AI_FOLDER = "rail-scenario-ai"


# Exact unchanged pinned bodies: transient PBSTileInfo/train_on_res and trigger order
# are not serialized. Both adapter roles compile the same source without editing it.
def pbs_reference(folder):
    baseline = json.loads((ROOT / "migration/baseline.json").read_text())["commit"]
    original = subprocess.check_output(
        ["git", "show", f"{baseline}:src/pbs.cpp"], cwd=ROOT, text=True
    )
    names = (
        "GetReservedTrackbits",
        "SetRailStationPlatformReservation",
        "TryReserveRailTrack",
        "UnreserveRailTrack",
        "FollowReservation",
        "FollowTrainReservation",
        "GetTrainForReservation",
        "IsSafeWaitingPosition",
        "IsWaitingPositionFree",
        "HasReservedTracks",
    )
    body = "".join(f"#define {name} PbsOriginal{name}\n" for name in names)
    body += "TrackBits PbsOriginalGetReservedTrackbits(TileIndex);\nbool PbsOriginalIsSafeWaitingPosition(const Train *, TileIndex, Trackdir, bool, bool);\n"
    body += "inline bool PbsOriginalHasReservedTracks(TileIndex t, TrackBits b) { return (PbsOriginalGetReservedTrackbits(t) & b) != TRACK_BIT_NONE; }\n"
    body += (
        original[original.index("TrackBits GetReservedTrackbits") :]
        .replace("t->track", "PbsTrainTrack(t)")
        .replace("v->railtypes", "PbsTrainRailTypes(v)")
    )
    header = subprocess.check_output(
        ["git", "show", f"{baseline}:src/pathfinder/follow_track.hpp"],
        cwd=ROOT,
        text=True,
    )
    mask = header[header.index("\tinline bool MaskReservedTracks()") :]
    body += (
        "struct PbsOriginalMask : CFollowTrackRail { using CFollowTrackRail::CFollowTrackRail; "
        "static bool DoTrackMasking() { return true; }\n"
        + mask[: mask.index("\nprotected:")]
        + "};\n"
    )
    body += "".join(f"#undef {name}\n" for name in names)
    (folder / "pbs-original.hpp").write_text(body)


def adapter(build, folder, env):
    folder.mkdir(parents=True, exist_ok=True)
    pbs_reference(folder)
    source = folder / "pbs-source.cpp"
    source.write_text((ROOT / "tools/simulation/pbs-console.cpp").read_text())
    return console_adapter(build, folder, env, source, "rails", wraps=pbs_wraps(build))


PBS_WRAPS = (
    "_Z27TriggerStationRandomisationP11BaseStationN10StrongType7TypedefIj12TileIndexTagJNS1_7CompareENS1_7IntegerENS1_10CompatibleIiEENS6_IlEEEEE20StationRandomTriggerh",
    "_Z23TriggerStationAnimationP11BaseStationN10StrongType7TypedefIj12TileIndexTagJNS1_7CompareENS1_7IntegerENS1_10CompatibleIiEENS6_IlEEEEE23StationAnimationTriggerh",
)


def pbs_wraps(build):
    # Rust owners call each other directly, bypassing both native trigger facades.
    if (build / "openttd-rust").exists():
        return ("openttd_rust_station_randomisation", "openttd_rust_trigger_station")
    return PBS_WRAPS


def uses_ai(scenario):
    return "rail_control" in scenario


def install(scenario, run_dir):
    if uses_ai(scenario):
        ai = run_dir / "ai/rails"
        shutil.copytree(scenario["scenario_ai"], ai)
        (ai / "parameters.nut").write_text(
            f'RAIL_ACTION <- "{scenario["rail_control"]}";\n'
        )


# Trains, road vehicles, ships and aircraft of the 2016 save. Aircraft 26, 48
# and 57 crash in the reference (48 in both distributions) and are left out.
HEADS_2016 = tuple(
    map(
        int,
        """5 37 43
        6 11 12 18 22 41 45 46 47 52 53 54 55 56 59 60 69 71 72 83 85 87 88 90
        2 25 42 65 70
        3 7 10 28 30 44 62 73 75 76""".split(),
    )
)


def scenarios(soak):
    cases = [
        {
            "name": f"play-padhattan-ridge-1996-{distribution}",
            "kind": "save",
            "rail_fixture": True,
            "console": commands,
            "save": str(ROOT / "migration/saves/padhattan-ridge-1996.sav"),
            "ticks": (6 if soak else 2) * 365 * TICKS_PER_DAY + SNAPSHOT_TICKS,
        }
        for distribution, commands in DISTRIBUTIONS.items()
    ]
    cases.extend(
        dict(
            name="movement-" + route,
            kind="save",
            save=str(ROOT / "migration/saves/padhattan-ridge-1996.sav"),
            movement_structure=route,
            ticks=10000,
            console=["movement_structures " + route, "unpause"],
        )
        for route in ("train-bridge", "road-tunnel")
    )
    cases.append(
        dict(
            cases[0],
            name="rail-movement-depot-3",
            movement_depot=True,
            ticks=1200,
            short_checkpoint=True,
        )
    )
    cases.append(
        dict(
            cases[0],
            name="rail-padhattan-ridge-1996-realistic",
            console=["setting vehicle.train_acceleration_model 1", "unpause"],
        )
    )
    for forbid90 in (0, 1):
        cases.append(
            dict(
                cases[0],
                name=f"rail-reservation-90-{forbid90}",
                console=[
                    f"setting pf.forbid_90_deg {forbid90}",
                    "setting pf.reserve_paths 1",
                    "setting pf.yapf.rail_look_ahead_max_signals 1",
                    "unpause",
                ],
            )
        )
    cases.append(dict(cases[-2], name="rail-reservation-reload", rail_reload=True))
    for action in ("reverse", "service", "crossing", "collision", "reservation"):
        cases.append(
            dict(
                cases[0],
                name=f"rail-controller-{action}",
                rail_control=action,
                ticks=(60000 if soak else 18000) + SNAPSHOT_TICKS,
                console=[
                    "setting difficulty.disasters 0",
                    "setting pf.reserve_paths 1",
                    "unpause",
                ],
            )
        )
    # The blocked train waits wait_for_pbs_path days, then reverses (stuck_reverse).
    cases.append(
        dict(
            cases[-1],
            name="rail-controller-reservation-reverse",
            console=[*cases[-1]["console"][:-1], "setting pf.reverse_at_signals 1"]
            + ["unpause"],
        )
    )
    for distribution, commands in DISTRIBUTIONS.items():
        cases.append(
            {
                "name": f"play-padhattan-ridge-2000-{distribution}",
                "kind": "save",
                "rail_fixture": True,
                "vehicle_heads": (8, 14, 37, 2, 25, 42, 28, 30, 44, 48),
                "crossing_tiles": (4668, 4671),
                "barred_crossings": (4668,) if distribution == "cargodist" else (),
                "console": commands,
                "save": str(ROOT / "migration/saves/padhattan-ridge-2000.sav"),
                "ticks": (6 if soak else 2) * 365 * TICKS_PER_DAY + SNAPSHOT_TICKS,
            }
        )
    for distribution, commands in DISTRIBUTIONS.items():
        cases.append(
            {
                "name": f"play-padhattan-ridge-2006-{distribution}",
                "kind": "save",
                "rail_fixture": True,
                "owner_structures": True,
                "vehicle_heads": (
                    37,
                    43,
                    45,
                    2,
                    25,
                    42,
                    65,
                    3,
                    28,
                    30,
                    44,
                    48,
                    57,
                    62,
                ),
                "console": commands,
                "save": str(ROOT / "migration/saves/padhattan-ridge-2006.sav"),
                "ticks": (6 if soak else 2) * 365 * TICKS_PER_DAY + SNAPSHOT_TICKS,
            }
        )
    base = cases[-2]  # Manual 2006 input; both distribution plays remain intact.
    for name, ticks, tile, y, z in (
        ("canal", 30, 12192, 1529, 8),
        ("lock-lowering", 120, 12448, 1560, 3),
        ("lock-exit", 240, 12576, 1579, 0),
    ):
        cases.append(
            dict(
                base,
                name=f"rail-owner-2006-{name}",
                owner_structure_reload=True,
                structure_checkpoint=(tile, y, z),
                ticks=ticks,
                short_checkpoint=True,
            )
        )
    for distribution, commands in DISTRIBUTIONS.items():
        cases.append(
            {
                "name": f"play-padhattan-ridge-2016-{distribution}",
                "kind": "save",
                "rail_fixture": True,
                "owner_structures": True,
                "vehicle_heads": HEADS_2016,
                "console": commands,
                "save": str(ROOT / "migration/saves/padhattan-ridge-2016.sav"),
                "ticks": (6 if soak else 2) * 365 * TICKS_PER_DAY + SNAPSHOT_TICKS,
            }
        )
    for case in cases:
        if case["name"].startswith(("rail-reservation-", "rail-controller-")):
            case["pbs_probe"] = True
            case["console"] = ["pbs_probe", *case["console"]]
    return cases


def game_args(scenario):
    return ["-d", "yapf=3"] if scenario.get("rail_fixture") else []


def game_environment(scenario):
    witnessed = (
        scenario.get("rail_fixture") or scenario.get("movement_structure")
    ) and scenario.get("witnesses")
    return core.WITNESS_ENVIRONMENT if witnessed else {}


# train.* witness branches each candidate run must reach, by longest name prefix.
# Unreached (#156): wormhole_swap, unequal_before/after, red_twoway,
# force_signal, free_wagon_delete, articulated_move, depot_reentry and
# extension_opposing_red/extension_restore.
TICKS = ("tick_first", "tick_second")
PATHS = (*TICKS, "first_tile_rail", "controller_extension", "free_path")
PATHS += ("choose_track", "order_restore")
ROUTES = (*PATHS, "reversal", "depot_start", "red_oneway", "stuck_retry")
ROUTES += ("order_lookahead",)
CROSSINGS = ("cross_bar", "cross_unbar")
BLOCKED = (*TICKS, "choose_track", "controller_extension", "stuck_retry")
BLOCKED += ("extension_fail", "extension_rollback")
TRAIN_BRANCHES = {
    "play-padhattan-ridge-1996": ROUTES,
    "rail-padhattan-ridge-1996": ROUTES,
    "rail-reservation-": ROUTES,
    "play-padhattan-ridge-2000": (*ROUTES, *CROSSINGS),
    "play-padhattan-ridge-2006": (*PATHS, "reversal", "depot_start", *CROSSINGS)
    + ("depot_service", "wormhole_exit"),
    "rail-owner-2006": (*PATHS, "wormhole_exit"),
    "play-padhattan-ridge-2016": (*TICKS, "first_tile_rail", "free_path")
    + ("choose_track", "order_restore", "reversal", "depot_start", *CROSSINGS)
    + ("wormhole_exit",),
    "rail-controller-reverse": ROUTES,
    "rail-controller-service": ROUTES,
    "rail-controller-crossing": (*ROUTES, *CROSSINGS),
    "rail-controller-collision": (*TICKS, "free_path", "collision", "crash_delete"),
    "rail-controller-reservation": BLOCKED,
    "rail-controller-reservation-reverse": (*BLOCKED, "stuck_reverse", "reversal"),
}


def train_required(scenario):
    name = scenario["name"]
    if scenario.get("movement_depot"):
        return []
    return TRAIN_BRANCHES[
        max((k for k in TRAIN_BRANCHES if name.startswith(k)), key=len)
    ]


def prepare(scenario, binaries, builds, out, timeout, env, result):
    if scenario.get("movement_structure"):
        from . import signals

        prepared = signals.prepare(
            dict(scenario, signals="movement"),
            binaries,
            builds,
            out,
            timeout,
            env,
            result,
        )
        prepared.pop("signals")
        return prepared
    if not scenario.get("rail_fixture"):
        return scenario
    directory = out / scenario["name"] / "prepare"
    normalized, record = normalize(Path(scenario["save"]), directory)
    result["rail_input"] = record
    if scenario.get("owner_structures"):
        result["owner_structure_inventory"] = structure_inventory(normalized)
        vehicles = vehicle_rows(normalized)
        followers = {row["next"] - 1 for row in vehicles.values() if row["next"]}
        result["owner_transport_heads"] = {
            kind: sorted(
                index
                for index, row in vehicles.items()
                if row["type"] == kind and index not in followers
            )
            for kind in range(4)
        }
    if scenario.get("movement_depot"):
        warm = run_game(
            dict(scenario, save=str(normalized), ticks=1000),
            binaries["reference"],
            builds["reference"],
            directory / "depot-warm",
            timeout,
            env,
        )
        normalized, _ = normalize(
            warm["snapshots"][-1], directory / "depot-warm-normalized"
        )
        from .core import rows

        if rows(normalized)[14]["train[0]/common[0]/current_order.type"] & 15 == 3:
            raise RuntimeError("depot staging still has a station loading owner")
        normalized = depot_input(normalized, directory)
    scenario = dict(scenario, save=str(normalized))
    if "rail_control" in scenario:
        normalized, record = controller_input(normalized, directory, scenario)
        result["rail_controller_input"] = record
        scenario = dict(scenario, save=str(normalized))
    if scenario.get("owner_structure_reload"):
        # Reach this actual upper-lock pose from the immutable owner save.
        # No position, height, map, RNG or expected-result input is patched.
        warm = run_game(
            dict(scenario, ticks=47000),
            binaries["reference"],
            builds["reference"],
            directory / "warm",
            timeout,
            env,
        )
        fixture = next(
            (
                path
                for path in warm["snapshots"]
                if all(
                    vehicle_rows(path)[65][key] == value
                    for key, value in (("tile", 12064), ("y_pos", 1515), ("z_pos", 8))
                )
            ),
            None,
        )
        if warm["exit"] != 0 or fixture is None:
            raise RuntimeError("owner structure reload lacks reference upper-lock pose")
        normalized, record = normalize(fixture, directory / "reload")
        result["owner_structure_reload_input"] = record
        scenario = dict(scenario, save=str(normalized))
    if scenario.get("rail_reload"):
        setup = dict(scenario, ticks=8 * SNAPSHOT_TICKS)
        run = run_game(
            setup,
            binaries["reference"],
            builds["reference"],
            directory / "warm",
            timeout,
            env,
        )
        if run["exit"] != 0:
            raise RuntimeError("rail reload preparation failed")
        fixture = next(
            (
                p
                for p in run["snapshots"]
                if reservation_rows(p)["rail"] and reservation_rows(p)["platforms"]
            ),
            None,
        )
        if fixture is None:
            raise RuntimeError("rail reload lacks live track/platform reservations")
        normalized, record = normalize(fixture, directory / "reload")
        result["rail_reload_input"] = record
        scenario = dict(scenario, save=str(normalized))
    if scenario.get("pbs_probe"):
        executables = prepared_console(
            "rails", adapter, binaries, builds, directory / "pbs", env
        )
        scenario = dict(scenario, executables=executables)
    return scenario


def depot_input(source, directory):
    """Stage the existing consist north of real depot 5; ordinary ticks enter/leave."""
    from .core import patch

    common, depot = "train[0]/common[0]/", 8280
    changes = {"VEHS": {}}
    for index in (14, 15, 16):
        y = 1016 - (index - 14) * 8
        changes["VEHS"][index] = {
            common + "tile": (y // 16) * 128 + 88,
            common + "x_pos": 1416,
            common + "y_pos": y,
            common + "z_pos": 48,
            common + "direction": 3,
            common + "vehstatus": 8,
            "train[0]/track": 2,
        }
    changes["VEHS"][14].update(
        {
            common + "current_order.type": 66,
            common + "current_order.flags": 0,
            common + "current_order.dest": 5,
            common + "dest_tile": depot,
            common + "cur_speed": 32,
        }
    )
    target = directory / "depot-input.sav"
    patch(source, target, changes)
    data, chunks = bytearray(target.read_bytes()), read_save(target)
    # rail_map.h: retain the canonical depot pool entry, rotate its door to NW,
    # and declare a flat, unreserved straight approach; no output is patched.
    for tile in (depot, depot - 128, depot - 256, depot - 384):
        for cid, value in {
            "MAPT": 16,
            "MAPO": 0,
            "M3LO": 0,
            "M3HI": 1,
            "MAP5": 0xC3 if tile == depot else 2,
            "MAPE": 0,
        }.items():
            data[chunks[cid]["span"][0] + 8 + tile] = value
        struct.pack_into(">H", data, chunks["MAP2"]["span"][0] + 8 + tile * 2, 0)
    target.write_bytes(data)
    return target


def controller_input(source, directory, scenario):
    """Declare company/AI inputs and source-derived placement; no expected edits."""
    from .core import patch, rows

    changes = {"PLYR": {0: {"is_ai": 1}}}
    if scenario["rail_control"] == "collision":
        vehicles = rows(source)
        common = "train[0]/common[0]/"
        changes["VEHS"] = {}
        # Place the opposing source consist on the first consist's occupied
        # track. Coordinates/track are copied from actual reference vehicles;
        # no crash state or expected output is injected.
        for moving, occupied in ((14, 8), (15, 9), (16, 10)):
            changes["VEHS"][moving] = {
                common + key: vehicles[occupied][common + key]
                for key in ("tile", "x_pos", "y_pos", "z_pos", "direction")
            }
            changes["VEHS"][moving]["train[0]/track"] = vehicles[occupied][
                "train[0]/track"
            ]
    if scenario["rail_control"] == "reservation":
        vehicles = rows(source)
        common = "train[0]/common[0]/"
        changes["VEHS"] = {}
        # Translate the real second consist along its existing straight line,
        # then park it. All parts retain their relative geometry and direction.
        for index in (14, 15, 16):
            original = vehicles[index]
            changes["VEHS"][index] = {
                common + "tile": original[common + "tile"] - 29 * 128,
                common + "y_pos": original[common + "y_pos"] - 29 * 16,
            }
        changes["VEHS"][14][common + "vehstatus"] = (
            vehicles[14][common + "vehstatus"] | 2
        )
        # The first consist starts past the depot junction so controller
        # extension itself reaches the parked train, without delegating at the
        # earlier choice to YAPF's distinct reservation search/rollback.
        for index, model in ((8, 14), (9, 15), (10, 16)):
            original = vehicles[model]
            changes["VEHS"][index] = {
                common + "tile": original[common + "tile"] - 44 * 128,
                common + "y_pos": original[common + "y_pos"] - 44 * 16,
                common + "direction": original[common + "direction"],
            }
    target = directory / "controller-input.sav"
    record = patch(source, target, changes)
    if scenario["rail_control"] == "reservation":
        data, chunks = target.read_bytes(), read_save(target)
        work, allowed = bytearray(data), set()
        # rail_map.h reservation bits8..11; copy TRACK_Y's real reservation
        # from the owner's already reserved approach tile onto occupied tiles.
        reserved = (
            int.from_bytes(chunks["MAP2"]["raw"][2007 * 2 : 2007 * 2 + 2], "big")
            & 0xF00
        )
        begin = chunks["MAP2"]["span"][0] + 8
        for tile in (3415, 3543, 5207, 5335, 5463):
            before = int.from_bytes(
                chunks["MAP2"]["raw"][tile * 2 : tile * 2 + 2], "big"
            )
            struct.pack_into(">H", work, begin + tile * 2, (before & ~0xF00) | reserved)
            allowed.update((begin + tile * 2, begin + tile * 2 + 1))
            record["assignments"].append(
                ["MAP2", tile, "reserved_track", before, (before & ~0xF00) | reserved]
            )
        if any(
            a != b and index not in allowed
            for index, (a, b) in enumerate(zip(data, work, strict=True))
        ):
            raise RuntimeError("rail reservations changed undeclared input bytes")
        target.write_bytes(work)
    if dict(read_save(target)["AIPL"]["elements"])[0] != core.EMPTY_AI:
        raise RuntimeError("rail controller source company already has an AI")
    config = core.ai_config(b"MigrationRails") * 2 + b"\0"
    prepared = core.set_ai_slots(target.read_bytes(), {0: config})
    target.write_bytes(prepared)
    record["assignments"].append(["AIPL", 0, "configuration", "MigrationRails"])
    record["sha256"] = hashlib.sha256(prepared).hexdigest()
    return target, record


def reservation_rows(path):
    """Read current-save reservation fields; comparator still checks every byte."""
    chunks = read_save(path)
    tile_types = chunks["MAPT"]["raw"]
    m2 = chunks["MAP2"]["raw"]
    m5, m6 = chunks["MAP5"]["raw"], chunks["MAPE"]["raw"]
    # rail_map.h: plain railway m5 top bits 00/01, m2 bits8..11.
    # station_map.h: rail station/waypoint subtype0/7, m6 bit2.
    rail = [
        (tile, int.from_bytes(m2[tile * 2 : tile * 2 + 2], "big") >> 8 & 15)
        for tile, kind in enumerate(tile_types)
        if kind >> 4 == 1
        and m5[tile] & 0x80 == 0
        and int.from_bytes(m2[tile * 2 : tile * 2 + 2], "big") >> 8 & 7
    ]
    platforms = [
        tile
        for tile, kind in enumerate(tile_types)
        if kind >> 4 == 5 and m6[tile] >> 3 & 15 in (0, 7) and m6[tile] & 4
    ]
    return {"rail": rail, "platforms": platforms}


SEARCH = re.compile(
    r"\[YAPFt\]([!-])\s*(\d+) - (\d+) rounds - (\d+) open - "
    r"(\d+) closed - CHR\s*([\d.]+)% - C (-?\d+) D (-?\d+)"
)


def search_witnesses(scenario, run, mode):
    searches = [match.groups() for line in run["log"] if (match := SEARCH.search(line))]
    if not searches:
        raise RuntimeError("rail fixture lacks native YAPFt search witnesses")
    cache_hits = sum(float(row[5]) > 0 for row in searches)
    if mode == "plain" and "rail-reservation" in scenario["name"] and not cache_hits:
        raise RuntimeError("plain rail fixture lacks reused cached segment costs")
    reservations = [
        reservation_rows(p) for p in [Path(scenario["save"]), *run["snapshots"]]
    ]
    if "rail-reservation" in scenario["name"] and not any(
        row["rail"] and row["platforms"] for row in reservations[1:]
    ):
        raise RuntimeError("rail fixture lacks saved track/platform reservations")
    return {
        "searches": len(searches),
        "cache_hit_searches": cache_hits,
        "failed_searches": sum(row[0] == "!" for row in searches),
        "max_closed_nodes": max(int(row[4]) for row in searches),
        "reservations": reservations,
    }


def vehicle_rows(path):
    chunk = read_save(path)["VEHS"]
    rows = {}
    for index, body in chunk["elements"]:
        fields = decode_element(chunk, body)
        if fields["type"] not in (0, 1, 2, 3):
            continue
        kind = {0: "train", 1: "roadveh", 2: "ship", 3: "aircraft"}[fields["type"]]
        prefix = f"{kind}[0]/common[0]/"
        rows[index] = {
            name: fields[prefix + name]
            for name in (
                "next",
                "tile",
                "x_pos",
                "y_pos",
                "z_pos",
                "last_station_visited",
                "motion_counter",
                "cargo.packets",
                "cargo.action_counts",
                "profit_this_year",
                "profit_last_year",
            )
        }
        rows[index]["type"] = fields["type"]
        if kind == "roadveh":
            rows[index]["state"] = fields["roadveh[0]/state"]
        if kind == "train":
            rows[index]["track"] = fields["train[0]/track"]
        if kind == "aircraft":
            rows[index]["airport_state"] = fields["aircraft[0]/state"]
    return rows


def structure_inventory(path):
    """Decode native map accessors; presence is independent of vehicle use."""
    chunks = read_save(path)
    types, m5 = chunks["MAPT"]["raw"], chunks["MAP5"]["raw"]
    m1, m8 = chunks["MAPO"]["raw"], chunks["MAP8"]["raw"]
    # rail_map.h: m8 bits0..5; tunnelbridge_map.h: m5 bit7/type bits2..3.
    # water_map.h: m5 subtype bits4..7, m1 waterclass bits5..6.
    return {
        "monorail": [
            tile
            for tile, kind in enumerate(types)
            if kind >> 4 == 1
            and int.from_bytes(m8[2 * tile : 2 * tile + 2], "big") & 63 == 2
        ],
        "bridges": [
            tile for tile, kind in enumerate(types) if kind >> 4 == 9 and m5[tile] & 128
        ],
        "tunnels": [
            tile
            for tile, kind in enumerate(types)
            if kind >> 4 == 9 and not m5[tile] & 128
        ],
        "canals": [
            tile
            for tile, kind in enumerate(types)
            if kind >> 4 == 6 and m5[tile] >> 4 == 0 and m1[tile] >> 5 & 3 == 1
        ],
        "locks": [
            tile
            for tile, kind in enumerate(types)
            if kind >> 4 == 6 and m5[tile] >> 4 == 2
        ],
    }


def structure_witnesses(paths, observations, mode):
    """Record actual poses on structures, separately from saved inventory."""
    inventory = structure_inventory(paths[0])
    if any(not tiles for tiles in inventory.values()):
        raise RuntimeError("owner save lacks declared structure inventory")
    witnessed = {name: [] for name in inventory}
    for checkpoint, (path, vehicles) in enumerate(
        zip(paths, observations, strict=True)
    ):
        chunks = read_save(path)
        for vehicle, row in vehicles.items():
            tile = row["x_pos"] // 16 + 128 * (row["y_pos"] // 16)
            for name in ("monorail", "canals", "locks"):
                if tile in inventory[name] and row["type"] == (
                    0 if name == "monorail" else 2
                ):
                    witnessed[name].append([checkpoint, vehicle, tile, row["z_pos"]])
            # Native train wormhole track0x40 plus a tunnel-entry tile proves
            # tunnel occupancy; coordinates advance while the tile stays at entry.
            if (
                row["type"] == 0
                and row["track"] == 64
                and row["tile"] in inventory["tunnels"]
            ):
                witnessed["tunnels"].append(
                    [checkpoint, vehicle, row["tile"], row["x_pos"], row["y_pos"]]
                )
            # Road bridges use state255; compare physical position with the
            # saved bridge-above axis bits (tile_map/bridge_map.h).
            if (
                row["type"] == 1
                and row["state"] == 255
                and chunks["MAPT"]["raw"][tile] & 12
            ):
                witnessed["bridges"].append([checkpoint, vehicle, tile, row["z_pos"]])
    if mode == "snapshots" and any(
        not witnessed[name] for name in ("monorail", "bridges", "tunnels", "locks")
    ):
        raise RuntimeError(f"owner save lacks reachable structure poses: {witnessed}")
    return witnessed


def controller_witnesses(scenario, run, mode):
    from .core import rows

    if run["exit"] != 0 or not run["snapshots"]:
        raise RuntimeError("rail controller game failed or omitted checkpoints")
    common = "train[0]/common[0]/"
    observations = [rows(path) for path in [Path(scenario["save"]), *run["snapshots"]]]
    heads = {
        head: [
            {
                key: snapshot[head][common + key]
                for key in ("tile", "direction", "vehstatus", "date_of_last_service")
            }
            if head in snapshot and snapshot[head]["type"] == 0
            else None
            for snapshot in observations
        ]
        for head in (8, 14)
    }
    crossings = [
        {
            "crossing": chunks["MAPT"]["raw"][4183] >> 4 == 2
            and chunks["MAP5"]["raw"][4183] >> 6 == 1,
            "barred": bool(chunks["MAP5"]["raw"][4183] & 32),
        }
        for path in [Path(scenario["save"]), *run["snapshots"]]
        for chunks in [read_save(path)]
    ]
    events = [line for line in run["log"] if "RAIL-" in line]
    action = scenario["rail_control"]
    if action == "collision":
        for head in (8, 14):
            if (
                not any(f"RAIL-CRASH {head} 0 " in line for line in events)
                or heads[head][-1] is not None
            ):
                raise RuntimeError("rail collision lacks actual crash and deletion")
            if mode == "snapshots" and not any(
                row and row["vehstatus"] & 128 for row in heads[head][1:]
            ):
                raise RuntimeError("rail collision lacks saved crashed train")
    elif action == "reservation":
        if not all(
            any(f"RAIL-COMMAND {command}=true" in line for line in events)
            for command in ("skip", "remove-signal", "pbs", "opposing-pbs")
        ):
            raise RuntimeError("rail reservation setup command failed")
        stopped = (
            observations[-1][8]["train[0]/flags"] & 256
            and heads[8][-1]["tile"] == heads[8][0]["tile"]
        )
        if "reverse" in scenario["name"]:
            if heads[8][-1]["direction"] == heads[8][0]["direction"]:
                raise RuntimeError("blocked rail controller did not reverse")
        elif not stopped:
            raise RuntimeError(
                "rail controller did not stop before occupied reservation"
            )
        for path in run["snapshots"]:
            reserved = dict(reservation_rows(path)["rail"])
            if any(tile in reserved for tile in range(3671, 5207, 128)):
                raise RuntimeError(
                    "rail controller retained partially extended reservation"
                )
    else:
        if not any(f"RAIL-COMMAND {action}=true" in line for line in events):
            raise RuntimeError(f"rail controller {action} command failed")
        if action == "service" and not (
            any("RAIL-DEPOT occupied=true" in line for line in events)
            and any("RAIL-DEPOT occupied=false" in line for line in events)
            and heads[8][-1]["date_of_last_service"]
            > heads[8][0]["date_of_last_service"]
        ):
            raise RuntimeError("rail servicing lacks depot entry, exit and service")
        if action == "crossing":
            if not all(
                any(f"RAIL-CROSSING occupied={occupied}" in line for line in events)
                for occupied in ("true", "false")
            ):
                raise RuntimeError("rail crossing lacks actual train entry and exit")
            if mode == "snapshots" and {
                row["barred"] for row in crossings if row["crossing"]
            } != {True, False}:
                raise RuntimeError("rail crossing lacks saved bar and release")
    return {
        "heads": heads,
        "events": events,
        "crossings": crossings,
    }


def check(scenario, run, mode, role, result):
    if route := scenario.get("movement_structure"):
        markers = [
            line.split() for line in run["log"] if line.startswith("MOVEMENT route ")
        ]
        if len(markers) != 1 or not run["snapshots"]:
            raise RuntimeError("movement route lacks its vehicle/map fixture")
        _, _, name, vehicle, depot, station, entrance, exit_tile = markers[0]
        if name != route:
            raise RuntimeError("movement route marker differs")
        vehicle, depot, station, entrance, exit_tile = map(
            int, (vehicle, depot, station, entrance, exit_tile)
        )
        observations = [vehicle_rows(path)[vehicle] for path in run["snapshots"]]
        if not any(
            row["tile"] != depot and row["motion_counter"] > 0 for row in observations
        ):
            raise RuntimeError("movement vehicle did not leave its depot")
        if not any(row["last_station_visited"] != 65535 for row in observations):
            raise RuntimeError("movement vehicle did not visit its ordered station")
        chunks = read_save(run["snapshots"][-1])
        transport = 0 if route == "train-bridge" else 1
        for tile in (entrance, exit_tile):
            if (
                chunks["MAPT"]["raw"][tile] >> 4 != 9
                or (chunks["MAP5"]["raw"][tile] >> 2) & 3 != transport
                or bool(chunks["MAP5"]["raw"][tile] & 128) != (transport == 0)
            ):
                raise RuntimeError("movement route lost its actual structure head")
        if mode == "snapshots" and not any(
            station <= row["tile"] < entrance for row in observations
        ):
            raise RuntimeError("movement vehicle did not reach the far side")
        prefix = "train_bridge" if route == "train-bridge" else "road_tunnel"
        result[f"{mode}_{role}_movement"] = core.branch_witnesses(
            scenario, run, role, "movement", [prefix + "_enter", prefix + "_exit"]
        )
        result[f"{mode}_{role}_route"] = observations
        return
    if not scenario.get("rail_fixture"):
        return
    ground = ["rail_power", "rail_weight"]
    if scenario["name"] == "rail-padhattan-ridge-1996-realistic":
        ground += ["speed", "accelerate", "brake", "depot"]
    if scenario.get("rail_control") == "collision":
        ground += ["crash"]
        core.branch_witnesses(
            scenario,
            run,
            role,
            "occupancy",
            ["insert", "same_bucket", "remove", "reset", "near"],
        )
    result[f"{mode}_{role}_ground"] = core.branch_witnesses(
        scenario, run, role, "ground", ground
    )
    if scenario.get("pbs_probe"):
        if not any("PBS checked" in line for line in run["log"]):
            raise RuntimeError("missing default PBS transient comparison")
        result[f"{mode}_{role}_pbs"] = core.branch_witnesses(
            scenario,
            run,
            role,
            "pbs",
            (
                "follow",
                "loop",
                "second_direction",
                "stray",
                "lower_id",
                "platform_interior",
                "overlap",
                "oneway_stop",
                "search_oneway_skip",
                "safe_opposing_pbs",
            ),
        )
    required = ["pixel_z", "track_rail"]
    if scenario.get("movement_depot"):
        required += ["train_depot_enter_3", "train_depot_leave_3"]
    if scenario["name"] == "play-padhattan-ridge-2016-manual":
        required += (
            "coast lock ship_depot train_tunnel_enter train_tunnel_exit "
            "train_tunnel_sound road_bridge_enter road_bridge_exit station_stop "
            "station_slow bridge_up bridge_down"
        ).split()
        required += [
            f"train_depot_{phase}_{direction}"
            for phase in ("enter", "leave")
            for direction in range(3)
        ]
        required += [
            f"train_tunnel_{phase}_{direction}"
            for phase in ("enter", "exit")
            for direction in range(4)
        ]
        required += [f"road_depot_enter_{direction}" for direction in range(4)]
    if scenario.get("rail_control") in ("crossing", "service"):
        required += ["signal_directions", "station_stop"]
    result[f"{mode}_{role}_movement_profile"] = core.branch_witnesses(
        scenario, run, role, "movement", required
    )
    counts = core.branch_witnesses(
        scenario, run, role, "train", train_required(scenario)
    )
    if counts is not None:
        result[f"{mode}_{role}_train_profile"] = counts
    if scenario.get("movement_depot"):
        return
    if scenario.get("owner_structure_reload"):
        if run["exit"] != 0 or not run["snapshots"]:
            raise RuntimeError("owner structure window lacks exit checkpoint")
        before = vehicle_rows(Path(scenario["save"]))[65]
        after = vehicle_rows(run["snapshots"][-1])[65]
        actual = tuple(after[key] for key in ("tile", "y_pos", "z_pos"))
        if actual != scenario["structure_checkpoint"]:
            raise RuntimeError(
                f"owner structure checkpoint {actual} != {scenario['structure_checkpoint']}"
            )
        result[f"{mode}_{role}_owner_structure_window"] = {
            "before": before,
            "after": after,
        }
        return
    if "rail_control" in scenario:
        # Controller cases can intentionally stop or delete the original heads.
        result[f"{mode}_{role}_rail_control"] = controller_witnesses(
            scenario, run, mode
        )
        return
    result[f"{mode}_{role}_rail_search"] = search_witnesses(scenario, run, mode)
    if (profile := core.witness_counts(run, "rail")) is not None:
        result[f"{mode}_{role}_rail_profile"] = profile
    paths = [Path(scenario["save"]), *run["snapshots"]]
    observations = [vehicle_rows(path) for path in paths]
    witnesses = {}
    for head in scenario.get("vehicle_heads", (8, 14, 2)):
        rows = [snapshot[head] for snapshot in observations]
        positions = {(row["tile"], row["x_pos"], row["y_pos"]) for row in rows}
        loaded = []
        for snapshot in observations:
            index, cargo = head, 0
            while index is not None:
                row = snapshot[index]
                cargo += sum(row["cargo.action_counts"])
                if row["cargo.packets"] and not cargo:
                    raise RuntimeError(f"vehicle {index} has packets without cargo")
                index = row["next"] - 1 if row["next"] else None
            loaded.append(cargo)
        paid = [
            b["profit_this_year"] - a["profit_this_year"]
            for a, b in zip(rows, rows[1:], strict=False)
            if a["profit_last_year"] == b["profit_last_year"]
            and b["profit_this_year"] > a["profit_this_year"]
        ]
        stations = sorted({row["last_station_visited"] for row in rows} - {65535})
        # Plain mode has only the exit checkpoint: a cyclic route can return
        # to its exact starting position, while motion_counter records travel.
        # Delivery revenue is witnessed by the
        # intermediate desync snapshots, which the semantic comparator checks.
        moved = (
            len(positions) > 1
            or rows[-1]["motion_counter"] != rows[0]["motion_counter"]
        )
        if not moved or (
            mode == "snapshots" and (not any(loaded) or not paid or len(stations) < 2)
        ):
            raise RuntimeError(
                f"vehicle {head} lacks movement/cargo/delivery witnesses"
            )
        witnesses[head] = {
            "positions": sorted(positions),
            "motion_counter": [rows[0]["motion_counter"], rows[-1]["motion_counter"]],
            "stations": stations,
            "cargo_counts": loaded,
            "positive_profit_deltas": paid,
        }
        if "airport_state" in rows[0]:
            states = sorted({row["airport_state"] for row in rows})
            if mode == "snapshots" and 14 not in states:
                raise RuntimeError(f"aircraft {head} lacks flying-state witness")
            witnesses[head]["airport_states"] = states
    result[f"{mode}_{role}_rail"] = witnesses
    if scenario.get("owner_structures"):
        result[f"{mode}_{role}_owner_structures"] = structure_witnesses(
            paths, observations, mode
        )
    if "crossing_tiles" in scenario:
        chunks = [read_save(path) for path in paths]
        crossings = {
            tile: [
                bool(chunk["MAP5"]["raw"][tile] & 32)
                for chunk in chunks
                if chunk["MAPT"]["raw"][tile] >> 4 == 2
                and chunk["MAP5"]["raw"][tile] >> 6 == 1
            ]
            for tile in scenario["crossing_tiles"]
        }
        if any(len(states) != len(paths) for states in crossings.values()):
            raise RuntimeError("owner-built crossing disappeared")
        if mode == "snapshots" and any(
            set(crossings[tile]) != {True, False}
            for tile in scenario["barred_crossings"]
        ):
            raise RuntimeError("owner-built crossing lacks saved bar and release")
        result[f"{mode}_{role}_crossings"] = crossings
        # Offers/awards and reservations are observations, not witnesses of a
        # particular subsidy multiplier or PBS search/control-flow branch.
        result[f"{mode}_{role}_subsidies"] = [
            {
                index: decode_element(chunk["SUBS"], body)
                for index, body in chunk["SUBS"]["elements"]
            }
            for chunk in chunks
        ]
