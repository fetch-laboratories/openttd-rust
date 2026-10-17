"""Road controller and YAPF branch witnesses from the committed player saves (#121)."""

import hashlib
import json
import lzma
import shutil
from pathlib import Path

from .core import ROOT, copy_runtime, decode_element, read_save, run_game
from .disasters import patch

ROAD = "roadveh[0]/"
COMMON = ROAD + "common[0]/"
AI_FOLDER = "road-scenario-ai"

# Idle-host medians at e94b378c0f: three --benchmark 3 invocations per case.
# All 54 pairs had equal semantic state/logs; invocation spread was 0.79-1.65%.
# https://github.com/fetch-laboratories/openttd-rust/pull/279
# Each reduction exceeds both its measured invocation spread and 2%.
# Core-loop boundary exceptions follow AGENTS.md and the selected roadmap.
SPEED_BUDGETS = {
    "generate-tgp-256-1": 1.1232091690544415,
    "play-opus-55-167-002-manual": 1.2225869993434013,
    "play-grok-159-001-manual": 1.2124019957234498,
    "play-padhattan-ridge-1996-manual": 1.230443974630021,
    "play-padhattan-ridge-2000-manual": 1.2902255639097744,
    "play-padhattan-ridge-2016-manual": 1.315985130111524,
}


def speed_budget(name, ratio):
    """Report the ratchet without making noisy timings a semantic CI failure."""
    budget = SPEED_BUDGETS.get(name)
    if budget is None or ratio is None:
        return None
    return {
        "best_known_ratio": budget,
        "change_percent": (ratio / budget - 1) * 100,
        "within_three_percent": ratio <= budget * 1.03,
    }


def uses_ai(scenario):
    return scenario.get("roads") in ("level-crossing", "flooding") or scenario.get(
        "road_setup"
    )


def install(scenario, run_dir):
    if uses_ai(scenario):
        ai = run_dir / "ai/road-crossing"
        shutil.copytree(scenario["scenario_ai"], ai)
        (ai / "parameters.nut").write_text(
            f'ROAD_SETUP <- "{scenario.get("road_setup", "crossing")}";\n'
            f"ROAD_FLOOD <- {str(scenario.get('roads') == 'flooding').lower()};\n"
        )
        if scenario.get("road_setup"):
            shutil.copy2(ai / "setup.nut", ai / "main.nut")


def scenarios(soak):
    # These transient windows stay short in soak; play saves cover longer runs.
    cases = [
        {
            "name": f"roads-{operation}",
            "kind": "save",
            "short_checkpoint": True,
            "roads": operation,
            "save": str(ROOT / "migration/saves" / f"{source}.sav"),
            "console": [
                f"setting vehicle.roadveh_acceleration_model {model}",
                *(
                    [
                        "setting pf.yapf.road_curve_penalty 800",
                        "setting pf.yapf.road_slope_penalty 4000",
                        "setting pf.yapf.road_stop_penalty 1000",
                    ]
                    if operation == "yapf-penalties"
                    else []
                ),
                "unpause",
            ],
            "ticks": ticks,
        }
        for operation, source, ticks, model in (
            ("original", "opus-55-167-002", 30, 0),
            ("realistic", "opus-55-167-002", 30, 1),
            ("invalidate", "opus-55-167-002", 30, 1),
            ("no-destination", "opus-55-167-002", 30, 1),
            ("yapf-penalties", "opus-55-167-002", 30, 1),
            ("blocking-failsafe", "opus-55-167-002", 100, 1),
            ("overtake-start", "grok-159-001", 1, 1),
            ("overtake-timeout", "grok-159-001", 30, 1),
            ("reload", "grok-159-001", 30, 1),
            ("level-crossing", "road-crossing", 30, 1),
            ("flooding", "road-flooding", 300, 1),
            ("depot-service", "opus-55-167-002", 12, 1),
            ("service-retain", "opus-55-167-002", 37, 1),
            ("service-refresh", "opus-55-167-002", 37, 1),
        )
    ]
    crossing = next(case for case in cases if case["roads"] == "level-crossing")
    cases.extend(
        dict(crossing, name=f"roads-crash-{phase}", ticks=ticks, crash_expiry=phase)
        for phase, ticks in (("before-expiry", 2238), ("expiry", 2239))
    )
    return cases


def rows(path):
    chunk = read_save(path)["VEHS"]
    return {
        index: fields
        for index, body in chunk["elements"]
        if (fields := decode_element(chunk, body))["type"] == 1
    }


def path_cache(row):
    return [
        (row[f"{ROAD}path[{index}]/trackdir"], row[f"{ROAD}path[{index}]/tile"])
        for index in range(sum(key.endswith("/trackdir") for key in row))
    ]


def position(row):
    return tuple(row[COMMON + name] for name in ("tile", "x_pos", "y_pos"))


def prepare(scenario, binaries, builds, out, timeout, env, result):
    if "roads" not in scenario:
        return scenario
    folder = out / scenario["name"] / "input"
    folder.mkdir(parents=True, exist_ok=True)
    source = Path(scenario["save"])
    packed = source.read_bytes()
    unpacked = folder / "player.sav"
    unpacked.write_bytes(
        packed
        if packed[:4] == b"OTTN"
        else b"OTTN" + packed[4:8] + lzma.decompress(packed[8:])
    )
    operation, changes = scenario["roads"], {}
    if operation == "no-destination":
        # The existing head is approaching a junction. Remove its order list
        # and active destination so ordinary control picks a random track.
        changes = {
            "VEHS": {
                3: {
                    COMMON + "orders": 0,
                    COMMON + "current_order.type": 0,
                    COMMON + "dest_tile": 0,
                    COMMON + "cur_implicit_order_index": 0,
                    COMMON + "cur_real_order_index": 0,
                }
            }
        }
    elif operation in ("level-crossing", "flooding"):
        frozen = json.loads(source.with_suffix(".json").read_text())
        if hashlib.sha256(packed).hexdigest() != frozen["sha256"]:
            raise RuntimeError("road crash fixture differs from preparation receipt")
        # Position existing vehicles on the command-built crossing. The stopped
        # visible train remains fixed; normal road control detects the collision.
        changes = {
            "VEHS": {
                17: {
                    "train[0]/common[0]/tile": 10005,
                    "train[0]/common[0]/x_pos": 344,
                    "train[0]/common[0]/y_pos": 632,
                    "train[0]/common[0]/vehstatus": 10,
                    "train[0]/track": 1,
                },
                21: {
                    COMMON + "tile": 10005,
                    COMMON + "x_pos": 344,
                    COMMON + "y_pos": 632,
                    COMMON + "direction": 3,
                    ROAD + "state": 1,
                },
            }
        }
        if operation == "flooding":
            # The canal barrier protects a real command-built sea-level road.
            # The AI removes one canal; ordinary water tile loops flood this bus.
            changes = {
                "VEHS": {
                    21: {
                        COMMON + "tile": 1201,
                        COMMON + "x_pos": 2840,
                        COMMON + "y_pos": 72,
                        COMMON + "z_pos": 0,
                        COMMON + "vehstatus": 10,
                        COMMON + "direction": 1,
                        ROAD + "state": 8,
                    }
                }
            }
    elif operation in ("invalidate", "yapf-penalties"):
        # At the next junction, the tail now names the wrong tile. The earlier
        # void-tile entry must also disappear, witnessing whole-cache rejection.
        changes = {
            "VEHS": {3: {ROAD + "path[0]/tile": 16383, ROAD + "path[3]/tile": 10814}}
        }
    if operation == "overtake-start":
        # Restore the naturally overtaking truck to its normal lane immediately
        # behind the existing broken-down truck 74; do not seed an active overtake.
        changes = {
            "VEHS": {
                38: {
                    ROAD + "overtaking": 0,
                    ROAD + "overtaking_ctr": 0,
                    COMMON + "x_pos": 969,
                }
            }
        }
    if operation in ("service-retain", "service-refresh"):
        # Keep vehicle 37's live blocked pose and empty cache. A prior automatic
        # service order names real depot 5; its overdue date makes servicing due.
        changes = {
            "VEHS": {
                37: {
                    COMMON + "current_order.type": 66,
                    COMMON + "current_order.flags": 1,
                    COMMON + "current_order.dest": 5,
                    COMMON + "dest_tile": 3091,
                    COMMON + "date_of_last_service": 727700,
                }
            }
        }
        if operation == "service-refresh":
            changes["DATE"] = {0: {"random_state[0]": 8, "random_state[1]": 8}}
    # Loaded overdue link jobs can spend every null-driver iteration waiting
    # for threads. Keep their relative join order, beyond these transient windows.
    jobs = read_save(unpacked)["LGRJ"]
    changes["LGRJ"] = {
        index: {"join_date": decode_element(jobs, body)["join_date"] + 32}
        for index, body in jobs["elements"]
    }
    fixture = folder / "road-input.sav"
    receipt = patch(unpacked, fixture, changes)
    receipt.update(
        committed_source=str(source),
        committed_sha256=hashlib.sha256(packed).hexdigest(),
        console=scenario["console"],
    )
    if operation == "reload":
        run = run_game(
            dict(scenario, save=str(fixture), ticks=1),
            binaries["reference"],
            builds["reference"],
            folder / "reference",
            timeout,
            env,
            False,
        )
        if run["exit"] != 0 or not run["snapshots"]:
            raise RuntimeError("road reload reference preparation failed")
        reference = run["snapshots"][-1]
        row = rows(reference)[65]
        if not path_cache(row) or not row[ROAD + "blocked_ctr"]:
            raise RuntimeError(
                "road reload lacks a live path and active blocked counter"
            )
        fixture = folder / "reload-input.sav"
        receipt["reload"] = patch(reference, fixture, {})
        receipt["reload"]["head"] = 65
    result["road_input"] = receipt
    return dict(scenario, save=str(fixture))


def check(scenario, run, mode, role, result):
    if "roads" not in scenario:
        return
    if not run["snapshots"]:
        raise RuntimeError("road witness has no output save")
    source, final = Path(scenario["save"]), run["snapshots"][-1]
    before, after = rows(source), rows(final)
    chunks = read_save(source)
    operation = scenario["roads"]
    witnesses = {}

    def require(condition, message):
        if not condition:
            raise RuntimeError(f"road {operation}: {message}")

    def observe(index):
        keys = [
            ROAD + key
            for key in ("state", "frame", "blocked_ctr", "overtaking", "overtaking_ctr")
        ] + [
            COMMON + key
            for key in (
                "tile",
                "x_pos",
                "y_pos",
                "direction",
                "z_pos",
                "cur_speed",
                "breakdown_ctr",
                "current_order.type",
                "current_order.flags",
                "current_order.dest",
                "dest_tile",
                "day_counter",
                "date_of_last_service",
                "reliability",
                "breakdowns_since_last_service",
            )
        ]
        keys.append(ROAD + "gv_flags")
        witnesses[index] = {
            "before": {key.removeprefix(ROAD): before[index][key] for key in keys},
            "after": {key.removeprefix(ROAD): after[index][key] for key in keys},
            "path_before": path_cache(before[index]),
            "path_after": path_cache(after[index]),
        }
        return before[index], after[index]

    def map_tile(tile):
        values = {cid: chunks[cid]["raw"][tile] for cid in ("MAPT", "MAP5")}
        witnesses.setdefault("map", {})[tile] = values
        return values["MAPT"] >> 4, values["MAP5"]

    pats = read_save(final)["PATS"]
    model = decode_element(pats, pats["elements"][0][1])[
        "vehicle.roadveh_acceleration_model"
    ]
    require(
        model == (0 if operation == "original" else 1), "acceleration setting missing"
    )
    witnesses["acceleration_model"] = model

    if operation in ("service-retain", "service-refresh"):
        a, b = observe(37)
        refresh = operation == "service-refresh"
        require(
            a[COMMON + "current_order.type"] == b[COMMON + "current_order.type"] == 66
            and a[COMMON + "current_order.flags"]
            == b[COMMON + "current_order.flags"]
            == 1
            and a[COMMON + "current_order.dest"] == 5
            and a[COMMON + "dest_tile"] == 3091
            and a[COMMON + "date_of_last_service"] + a[COMMON + "service_interval"]
            < 727929
            and a[COMMON + "day_counter"] == 31
            and b[COMMON + "day_counter"] == 32
            and not path_cache(a)
            and not path_cache(b)
            and position(a) == position(b)
            and a[ROAD + "blocked_ctr"] == 185
            and b[ROAD + "blocked_ctr"] == 222,
            "overdue non-stop service order or daily blocked-vehicle witness missing",
        )
        depot, tile = (2, 6206) if refresh else (5, 3091)
        depots = chunks["DEPT"]
        depot_row = decode_element(depots, dict(depots["elements"])[depot])
        require(
            depot_row["xy"] == tile
            and b[COMMON + "current_order.dest"] == depot
            and b[COMMON + "dest_tile"] == tile
            and a[ROAD + "gv_flags"] == 0
            and b[ROAD + "gv_flags"] == (4 if refresh else 0),
            "service routing decision or implicit-order suppression differs",
        )
        date = read_save(final)["DATE"]
        fields = decode_element(date, date["elements"][0][1])
        rng = tuple(fields[f"random_state[{i}]"] for i in (0, 1))
        require(
            rng == ((2072642750, 3270530758) if refresh else (2876180153, 3044549951)),
            f"service routing shared RNG differs: {rng}",
        )
        witnesses["service_routing"] = {"refresh": refresh, "depot": depot, "rng": rng}

    if operation == "no-destination":
        a, b = observe(3)
        require(
            a[COMMON + "orders"] == b[COMMON + "orders"] == 0
            and a[COMMON + "dest_tile"] == b[COMMON + "dest_tile"] == 0,
            "vehicle retained orders or a destination",
        )
        tile, roadbits = map_tile(10815)
        require(tile == 2 and roadbits & 15 not in (5, 10), "choice is not a junction")
        require(
            b[COMMON + "tile"] == 10815 and position(a) != position(b),
            "vehicle did not enter its no-destination junction",
        )
        require(
            path_cache(a) == path_cache(b), "no-destination choice consumed cached path"
        )
        witnesses["random_track"] = b[ROAD + "state"]
        date = read_save(final)["DATE"]
        fields = decode_element(date, date["elements"][0][1])
        witnesses["shared_rng"] = [fields[f"random_state[{i}]"] for i in range(2)]
        # Frozen unchanged-reference result; removing this site's Random draw
        # changes both DATE seeds and chooses state 5 instead of 10.
        require(
            witnesses["random_track"] == 10
            and witnesses["shared_rng"] == [15484628, 1889006306],
            "no-destination track/shared RNG differs from the original",
        )

    if operation == "level-crossing":
        a = before[21]
        tile, roadbits = map_tile(10005)
        require(tile == 2 and roadbits & 192 == 64, "tile is not a level crossing")
        train = next(
            decode_element(chunks["VEHS"], body)
            for index, body in chunks["VEHS"]["elements"]
            if index == 17
        )
        require(
            train["train[0]/common[0]/tile"] == a[COMMON + "tile"] == 10005
            and train["train[0]/common[0]/vehstatus"] == 10
            and train["train[0]/common[0]/x_pos"] == a[COMMON + "x_pos"]
            and train["train[0]/common[0]/y_pos"] == a[COMMON + "y_pos"]
            and abs(train["train[0]/common[0]/z_pos"] - a[COMMON + "z_pos"]) <= 6,
            "input lacks visible train within collision distance",
        )
        require(
            not a[COMMON + "vehstatus"] & 128 and a[ROAD + "crashed_ctr"] == 0,
            "crossing input already contains a crashed road vehicle",
        )
        events = [
            line.split("ROAD-CROSSING-EVENT ", 1)[1]
            for line in run["log"]
            if "ROAD-CROSSING-EVENT " in line
        ]
        require(
            events == ["21 10005 1 3"],
            "crossing crash event/victims missing or repeated",
        )
        witnesses["crash_event"] = events[0]
        expiry = scenario.get("crash_expiry")
        if expiry:
            final_chunks = read_save(final)
            date = final_chunks["DATE"]
            tick = decode_element(date, date["elements"][0][1])["tick_counter"]
            require(
                a[COMMON + "next"] == 0
                and a[COMMON + "tick_counter"] == 129
                and tick == 15780 + scenario["ticks"],
                "single-vehicle crash expiry input or endpoint differs",
            )
            witnesses["expiry_tick"] = tick
        if expiry == "expiry":
            require(
                21 not in dict(final_chunks["VEHS"]["elements"]),
                "crashed head survived its deletion tick",
            )
            witnesses["deleted_head"] = 21
        else:
            a, b = observe(21)
            require(
                b[COMMON + "vehstatus"] & 128 and b[ROAD + "crashed_ctr"] > 0,
                "crossing did not crash the road vehicle",
            )
            witnesses["crashed_ctr"] = b[ROAD + "crashed_ctr"]
            if expiry == "before-expiry":
                # Frozen unchanged-reference endpoint: the per-vehicle u8 tick
                # wraps to 63, one tick before the 32-tick deletion boundary.
                require(
                    b[ROAD + "crashed_ctr"] == 2239
                    and b[COMMON + "tick_counter"] == 63
                    and b[COMMON + "next"] == 0,
                    "crashed head did not survive until just before deletion",
                )

    if operation == "flooding":
        a, b = observe(21)
        require(
            position(a) == position(b) == (1201, 2840, 72)
            and a[COMMON + "z_pos"] == b[COMMON + "z_pos"] == 0
            and a[COMMON + "vehstatus"] == 10
            and b[COMMON + "vehstatus"] == 138
            and a[ROAD + "crashed_ctr"] == 0
            and b[ROAD + "crashed_ctr"] == 2032
            and a[COMMON + "cargo.action_counts"]
            == b[COMMON + "cargo.action_counts"]
            == (0, 0, 18, 0),
            "sea-level loaded bus did not enter the flooded crash countdown",
        )
        require(map_tile(1201) == (2, 10), "input bus is not on a straight road")
        final_chunks = read_save(final)
        require(
            final_chunks["MAPT"]["raw"][1201] >> 4 == 2
            and final_chunks["MAP5"]["raw"][1201] == 10,
            "failed flooded clear did not preserve the occupied road",
        )
        require(
            chunks["MAPT"]["raw"][945] >> 4 == 6
            and chunks["MAPO"]["raw"][945] == 33
            and final_chunks["MAPT"]["raw"][945] >> 4 == 6
            and final_chunks["MAPO"]["raw"][945] == 17,
            "canal breach did not become flooding sea",
        )
        events = [
            line.split("ROAD-FLOOD-EVENT ", 1)[1]
            for line in run["log"]
            if "ROAD-FLOOD-EVENT " in line
        ]
        require(events == ["21 1201 5 14"], "flooded event/victims missing or repeated")
        require(
            sum("ROAD-FLOOD-BREACH true" in line for line in run["log"]) == 1,
            "ordinary canal-removal command missing",
        )
        date = final_chunks["DATE"]
        fields = decode_element(date, date["elements"][0][1])
        rng = [fields[f"random_state[{i}]"] for i in range(2)]
        require(
            fields["tick_counter"] == 16080 and rng == [1209897734, 2054087316],
            "flooding checkpoint/shared RNG differs from the original",
        )
        witnesses.update(crash_event=events[0], crashed_ctr=2032, shared_rng=rng)

    if operation in ("original", "realistic", "invalidate"):
        a, b = observe(3)
        old, new = path_cache(a), path_cache(b)
        tile, roadbits = map_tile(10815)
        require(
            tile == 2 and roadbits & 15 not in (5, 10),
            "cache choice is not a road junction",
        )
        require(
            b[COMMON + "tile"] == 10815 and b[ROAD + "state"] == 10,
            "cached junction turn missing",
        )
        if operation == "invalidate":
            require(
                old[-1][1] != 10815 and old[0][1] == 16383, "stale input cache missing"
            )
            require(
                new and all(tile not in (16383, 10814) for _, tile in new),
                "stale cache entries survived",
            )
        else:
            require(
                old[-1] == (10, 10815) and new == old[:-1],
                "cached route was not consumed at its junction",
            )

    if operation == "yapf-penalties":
        a, b = observe(3)
        old, new = path_cache(a), path_cache(b)
        require(old[0][1] == 16383 and old[-1][1] == 10814, "forced cache miss absent")
        require(
            new and all(tile not in (16383, 10814) for _, tile in new),
            "forced miss did not reconstruct",
        )
        settings = decode_element(pats, pats["elements"][0][1])
        require(
            settings["pf.yapf.road_curve_penalty"] == 800
            and settings["pf.yapf.road_slope_penalty"] == 4000
            and settings["pf.yapf.road_stop_penalty"] == 1000,
            "competing route penalties absent",
        )
        witnesses["search"] = {
            "path": new,
            "vehicle_flags": b[COMMON + "vehicle_flags"],
        }

    if operation in ("original", "realistic"):
        # Disabling trimming retains three near-target choices (5043/5040/5037)
        # before 3090. The input starts without a cache; this is a fresh search.
        a, b = observe(64)
        require(
            not path_cache(a) and a[COMMON + "current_order.dest"] == 17,
            "fresh station route absent",
        )
        require(map_tile(4787) == (5, 5), "target is not a drive-through road stop")
        require(
            path_cache(b) == [(1, 3090), (5, 2706), (8, 2703), (4, 2700)]
            and not b[COMMON + "vehicle_flags"] & 128,
            "station-area cache prefix was not trimmed",
        )
        witnesses["station_trim"] = {"destination": 4787, "path": path_cache(b)}

    if operation in ("original", "realistic", "depot-service"):
        a, b = observe(90)
        tile, roadtype = map_tile(a[COMMON + "tile"])
        require(tile == 2 and roadtype >> 6 == 2, "service arrival is not at a depot")
        require(a[COMMON + "current_order.type"] & 15 == 2, "input lacks depot order")
        require(
            b[COMMON + "date_of_last_service"] > a[COMMON + "date_of_last_service"]
            and b[COMMON + "reliability"] > a[COMMON + "reliability"]
            and a[COMMON + "breakdowns_since_last_service"] == 1
            and b[COMMON + "breakdowns_since_last_service"] == 0
            and b[COMMON + "current_order.type"] & 15 == 1
            and b[ROAD + "state"] != 254
            and b[COMMON + "cur_speed"] > 0,
            "depot service/reset/departure missing",
        )

    if operation in ("original", "realistic"):
        a, b = observe(5)
        leader, _ = observe(6)
        require(
            position(a) == position(b)
            and b[COMMON + "cur_speed"] == 0
            and b[ROAD + "blocked_ctr"] > a[ROAD + "blocked_ctr"]
            and leader[COMMON + "x_pos"] - a[COMMON + "x_pos"] == 8
            and leader[COMMON + "y_pos"] == a[COMMON + "y_pos"]
            and leader[COMMON + "z_pos"] == a[COMMON + "z_pos"]
            and leader[COMMON + "direction"] == a[COMMON + "direction"] == 5,
            "nearby same-lane vehicle did not block movement",
        )

    if operation == "blocking-failsafe":
        a, b = observe(17)
        leader, last_leader = observe(36)
        require(
            0 < a[ROAD + "blocked_ctr"] <= 1480 < b[ROAD + "blocked_ctr"]
            and position(a) != position(b)
            and b[COMMON + "cur_speed"] > 0
            and position(leader) == position(last_leader)
            and leader[COMMON + "y_pos"] == a[COMMON + "y_pos"] - 8
            and leader[COMMON + "x_pos"] == a[COMMON + "x_pos"]
            and leader[COMMON + "z_pos"] == a[COMMON + "z_pos"]
            and leader[COMMON + "direction"] == a[COMMON + "direction"] == 7,
            "1480-count blocking escape missing",
        )

    if operation in ("overtake-start", "overtake-timeout", "reload"):
        a, b = observe(38)
        tile, roadbits = map_tile(a[COMMON + "tile"])
        require(tile == 2 and roadbits == 5, "overtake is not on a straight road")
        if operation == "overtake-start":
            obstacle, last_obstacle = observe(74)
            require(
                map_tile(a[COMMON + "tile"] + 128) == (2, 5),
                "next overtake tile is not a straight road",
            )
            require(
                a[ROAD + "overtaking"] == 0
                and a[ROAD + "overtaking_ctr"] == 0
                and b[ROAD + "overtaking"] == 16
                and b[ROAD + "overtaking_ctr"] == 17
                and obstacle[COMMON + "breakdown_ctr"] != 0
                and obstacle[COMMON + "x_pos"] == a[COMMON + "x_pos"]
                and obstacle[COMMON + "z_pos"] == a[COMMON + "z_pos"]
                and obstacle[COMMON + "direction"] == a[COMMON + "direction"] == 3
                and 0 < obstacle[COMMON + "y_pos"] - a[COMMON + "y_pos"] < 8
                and position(obstacle) == position(last_obstacle),
                "broken-down neighbour did not initiate overtaking",
            )
        else:
            require(
                a[ROAD + "overtaking"] == 16
                and 0 < a[ROAD + "overtaking_ctr"] < 35
                and b[ROAD + "overtaking"] == 0
                and b[ROAD + "overtaking_ctr"] == 35
                and position(a) != position(b),
                "active overtaking did not reach its straight-road timeout",
            )
        if operation == "reload":
            a, b = observe(65)
            require(
                path_cache(a)
                and path_cache(a) == path_cache(b)
                and 0 < a[ROAD + "blocked_ctr"] < b[ROAD + "blocked_ctr"]
                and position(a) == position(b),
                "reference reload did not retain a live path and advance the blocked counter",
            )
    result[f"{mode}_{role}_roads"] = witnesses


def prepare_crossing(migration, out, operation="crossing"):
    """Freeze reference-built road crash fixtures through ordinary commands."""
    runtime = out / "reference-runtime"
    with migration.reference_lock(shared=True):
        binary = copy_runtime(
            migration.REFERENCE_BUILD, migration.REFERENCE_BUILD / "openttd", runtime
        )
    scripts = out / AI_FOLDER
    shutil.copytree(ROOT / "tools" / AI_FOLDER, scripts)
    source = ROOT / "migration/saves/water-ferry.sav"
    preparation = None
    fixture_source = source
    if operation == "flooding":
        # Keep the existing 18 passengers aboard during construction. Postpone
        # loaded link jobs exactly as the short road-controller scenarios do.
        chunks = read_save(source)
        fixture_source = out / "setup-input.sav"
        preparation = patch(
            source,
            fixture_source,
            {
                "VEHS": {21: {COMMON + "vehstatus": 10}},
                "LGRJ": {
                    index: {
                        "join_date": decode_element(chunks["LGRJ"], body)["join_date"]
                        + 32
                    }
                    for index, body in chunks["LGRJ"]["elements"]
                },
            },
        )
    scenario = {
        "kind": "save",
        "save": str(fixture_source),
        "ticks": 1000,
        "road_setup": operation,
        "scenario_ai": str(scripts),
        "console": ["unpause"],
    }
    run = run_game(
        scenario,
        binary,
        runtime,
        out / "built",
        1200,
        migration.environment(),
        False,
    )
    marker = "ROAD-FLOOD-BUILT " if operation == "flooding" else "ROAD-CROSSING-BUILT "
    markers = [line.split(marker, 1)[1] for line in run["log"] if marker in line]
    expected = ["1201"] if operation == "flooding" else ["10005 1"]
    if run["exit"] != 0 or markers != expected or not run["snapshots"]:
        raise RuntimeError("reference road crash construction failed")
    destination = ROOT / "migration/saves" / f"road-{operation}.sav"
    receipt = patch(run["snapshots"][-1], destination, {})
    if destination.stat().st_size >= 1_000_000:
        raise RuntimeError("road crossing exceeds committed-save budget")
    receipt["raw_sha256"] = receipt["source_sha256"]
    receipt.update(
        baseline=migration.BASELINE,
        committed_source="migration/saves/water-ferry.sav",
        source_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),
        reference_binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
        setup_files={
            p.name: hashlib.sha256(p.read_bytes()).hexdigest()
            for p in sorted(scripts.glob("*.nut"))
        },
        setup={
            key: value
            for key, value in scenario.items()
            if key not in ("scenario_ai", "save")
        },
        setup_markers=markers,
    )
    receipt.pop("source", None)
    if preparation is not None:
        preparation["source"] = "migration/saves/water-ferry.sav"
        receipt["preparation"] = preparation
    destination.with_suffix(".json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(f"Prepared {destination}; construction evidence: {out}")


if __name__ == "__main__":
    import sys

    import migration

    from . import core

    if sys.argv[1:] not in (["--prepare-crossing"], ["--prepare-flooding"]):
        raise SystemExit(
            "usage: PYTHONPATH=tools python3 -m simulation.roads --prepare-{crossing,flooding}"
        )
    operation = sys.argv[1].removeprefix("--prepare-")
    out = migration.LOCAL / f"road-{operation}-preparation"
    if out.exists():
        raise SystemExit(f"retain or remove previous preparation first: {out}")
    out.mkdir(parents=True)
    migration.COMMON_LOCAL.mkdir(parents=True, exist_ok=True)
    core.GAME_LOCK = migration.COMMON_LOCAL / "simulation-game.lock"
    prepare_crossing(migration, out, operation)
