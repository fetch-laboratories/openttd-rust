"""Reference-built aircraft transport/controller evidence (#86/#136)."""

import hashlib
import json
import shutil
import subprocess
import threading
from pathlib import Path

from . import core
from .core import (
    ROOT,
    SNAPSHOT_TICKS,
    TICKS_PER_DAY,
    copy_runtime,
    decode_element,
    patch,
    read_save,
    rows,
    run_game,
    save_moment,
)
from .effects import effect_rows
from .play_saves import DISTRIBUTIONS

AI_FOLDER = "aircraft-controller-ai"
LOCK = threading.Lock()
LANDING_SEEDS = {
    "equal": (2443390976, 1012692424),
    "above": (2443382784, 1011643848),
    "masked": (2443390968, 1012691400),
    "disabled": (2443390976, 1012692424),
    "event": (2443390976, 1012692424),
}
BREAKDOWNS = {
    "cap-smoke": (319, 15, 320, 1),
    "cap-quiet": (319, 14, 320, 0),
    "gradual": (400, 14, 396, 0),
    "landed-9": (9, 15, 9, 0),
    "landed-10": (10, 15, 10, 1),
}
# Frozen from unchanged-reference self runs, in both plain and desync modes.
BREAKDOWN_RNG = {
    "airborne": [825328816, 2815432311],
    "loading": [935562356, 3451540546],
}


def scenarios(soak):
    cases = [
        {
            "name": f"aircraft-route-{distribution}",
            "kind": "save",
            "aircraft_fixture": True,
            "save": str(ROOT / "migration/saves/aircraft-route.sav"),
            "console": commands,
            "ticks": (4 if soak else 2) * 365 * TICKS_PER_DAY + SNAPSHOT_TICKS,
        }
        for distribution, commands in DISTRIBUTIONS.items()
    ]

    cases += [
        {
            "name": f"aircraft-controller-{kind}",
            "kind": "save",
            "aircraft_control": kind,
            "ticks": (2 if soak else 1) * 365 * TICKS_PER_DAY + SNAPSHOT_TICKS
            if kind in ("traffic", "closure", "ownerless")
            else 6000,
            "short_checkpoint": True,
            "console": ["unpause"],
        }
        for kind in (
            "traffic",
            "closure",
            "zeppelin",
            "removal",
            "orphan",
            "crash",
            "reload",
            "ownerless",
        )
    ]
    cases += [
        {
            "name": f"aircraft-controller-landing-{kind}",
            "kind": "save",
            "aircraft_control": f"landing-{kind}",
            "ticks": 16 if kind == "event" else 1,
            "short_checkpoint": True,
            "console": [
                f"setting vehicle.plane_crashes {0 if kind == 'disabled' else 1}",
                "unpause",
            ],
        }
        for kind in LANDING_SEEDS
    ]
    cases += [
        {
            "name": f"aircraft-breakdown-{kind}",
            "kind": "save",
            "aircraft_control": f"breakdown-{kind}",
            "ticks": 1,
            "short_checkpoint": True,
            "console": ["unpause"],
        }
        for kind in BREAKDOWNS
    ]
    return cases


def uses_ai(scenario):
    return "aircraft_control" in scenario


def install(scenario, run_dir):
    if scenario.get("aircraft_controller_setup") or scenario.get("aircraft_action"):
        scripts = run_dir / "ai/aircraft-controller"
        shutil.copytree(scenario["scenario_ai"], scripts)
        (scripts / "parameters.nut").write_text(
            f'AIRCRAFT_ACTION <- "{scenario.get("aircraft_action", "none")}";\n'
            f"AIRCRAFT_TARGET <- {scenario.get('aircraft_action_target', 0)};\n"
            f"AIRCRAFT_COUNT <- {scenario.get('aircraft_count', 8)};\n"
            f"AIRCRAFT_AIRPORTS <- {json.dumps(scenario.get('aircraft_action_airports', []))};\n"
            f"AIRCRAFT_OILRIG <- {str(scenario.get('aircraft_oilrig', False)).lower()};\n"
        )


def game_args(scenario):
    return (
        ["-d", "script=2"]
        if scenario.get("aircraft_setup") or scenario.get("aircraft_controller_setup")
        else []
    )


def prepare_aircraft_save(migration, out, timeout):
    """Construct with the pinned reference; comparison never installs the setup AI."""
    runtime = out / "reference-runtime"
    with migration.reference_lock(shared=True):
        binary = copy_runtime(
            migration.REFERENCE_BUILD, migration.REFERENCE_BUILD / "openttd", runtime
        )
    scripts = ROOT / "tools/aircraft-scenario-ai"
    frozen = runtime / "ai/aircraft-setup"
    shutil.copytree(scripts, frozen)
    setup = {
        "kind": "generate",
        "aircraft_setup": True,
        "seed": 86,
        "map_log2": 7,
        "land_generator": 1,
        "ticks": 6000,
        "console": ["start_ai MigrationAircraft", "unpause"],
        "settings": {
            "game_creation": {
                "starting_year": 1980,
                "terrain_type": 0,
                "quantity_sea_lakes": 0,
                "variety": 0,
                "custom_town_number": 20,
            },
            "difficulty": {
                "number_towns": 4,
                "max_loan": 2_000_000,
            },
            "economy": {"initial_city_size": 4},
        },
    }
    run = run_game(
        setup,
        binary,
        runtime,
        out / "built",
        timeout,
        migration.environment(),
        False,
    )
    markers = [
        line.split("AIRCRAFT-SETUP-END ", 1)[1]
        for line in run["log"]
        if "AIRCRAFT-SETUP-END " in line
    ]
    if run["exit"] != 0 or len(markers) != 1:
        raise RuntimeError(
            f"aircraft construction did not complete; see {out / 'built'}"
        )
    raw = run["snapshots"][-1]
    data, chunks = raw.read_bytes(), read_save(raw)
    destination = ROOT / "migration/saves/aircraft-route.sav"
    core.strip_glog(data, destination)
    if destination.stat().st_size >= 1_000_000:
        raise RuntimeError("aircraft fixture exceeds committed-save budget")
    if core.read_value(core.Reader(chunks["GSDT"]["elements"][0][1]), 10):
        raise RuntimeError("aircraft fixture contains a GameScript")
    first, last, plane, helicopter = map(int, markers[0].split())
    record = {
        "baseline": migration.BASELINE,
        "raw_sha256": hashlib.sha256(data).hexdigest(),
        "normalized_sha256": hashlib.sha256(destination.read_bytes()).hexdigest(),
        "setup_files": {
            path.name: hashlib.sha256(path.read_bytes()).hexdigest()
            for path in sorted(frozen.glob("*.nut"))
        },
        "setup": setup,
        "airports": [first, last],
        "aircraft": {"plane": plane, "helicopter": helicopter},
        "removed_chunk": "GLOG",
        "setup_markers": markers,
        "save_moment": save_moment(raw),
    }
    destination.with_suffix(".json").write_text(json.dumps(record, indent=2) + "\n")
    print(f"Prepared {destination}; raw input and logs: {out}")


CROSSING_CAPS = {
    "aircraft-controller-traffic": 85,
    "aircraft-route-manual": 75,
    "play-padhattan-ridge-2000-manual": 110,
    "play-padhattan-ridge-2016-manual": 110,
}


def counted_scenario(scenario):
    return (
        scenario.get("aircraft_control")
        or scenario.get("aircraft_fixture")
        or scenario.get("name", "").startswith("play-")
        or "disasters" in scenario
    )


def game_environment(scenario):
    return (
        {"OPENTTD_WITNESS": "1"}
        if counted_scenario(scenario) and scenario.get("witnesses")
        else {}
    )


def check(scenario, run, mode, role, result):
    if counted_scenario(scenario):
        counts = core.branch_witnesses(
            scenario,
            run,
            role,
            "aircraft",
            (
                *(
                    ("ticks", "station_resolutions")
                    if scenario.get("aircraft_control")
                    or scenario.get("aircraft_fixture")
                    or "padhattan-ridge-2000" in scenario.get("name", "")
                    or "padhattan-ridge-2016" in scenario.get("name", "")
                    else ()
                ),
                *(
                    ("disaster_entries",)
                    if scenario.get("family") in ("airplane", "helicopter")
                    and scenario.get("disasters")
                    in ("industry-reset", "industry-release", "init-soak")
                    else ()
                ),
                "services",
                "entries",
                "observation_bytes",
            ),
        )
        if counts is not None:
            result[f"{mode}_{role}_aircraft_boundary"] = dict(
                counts,
                services_per_tick=counts["services"] / counts["ticks"]
                if counts["ticks"]
                else None,
                entries_per_tick=counts["entries"] / counts["ticks"]
                if counts["ticks"]
                else None,
                observation_bytes_per_tick=counts["observation_bytes"] / counts["ticks"]
                if counts["ticks"]
                else None,
            )

            cap = CROSSING_CAPS.get(scenario["name"])
            if (
                mode == "plain"
                and cap is not None
                and counts["services"] / counts["ticks"] > cap
            ):
                raise RuntimeError(
                    f"aircraft boundary exceeds {cap} outgoing services/tick: {counts}"
                )

    if scenario.get("aircraft_control"):
        check_control(scenario, run, mode, role, result)
        return
    if not scenario.get("aircraft_fixture"):
        return
    source = Path(scenario["save"])
    record = json.loads(source.with_suffix(".json").read_text())
    if hashlib.sha256(source.read_bytes()).hexdigest() != record["normalized_sha256"]:
        raise RuntimeError("aircraft fixture differs from its preparation record")
    if not any("loading 'dummy' AI" in line for line in run["log"]) or any(
        "AIRCRAFT-BUILD" in line or "AIRCRAFT-SETUP-END" in line for line in run["log"]
    ):
        raise RuntimeError(
            "aircraft comparison did not replace setup logic with the dummy AI"
        )
    paths = [source, *run["snapshots"]]
    observations = []
    for path in paths:
        chunk = read_save(path)["VEHS"]
        observations.append(
            {index: decode_element(chunk, body) for index, body in chunk["elements"]}
        )
    witnesses = {}
    for name, head in record["aircraft"].items():
        if any(head not in snapshot for snapshot in observations):
            raise RuntimeError(f"{name} disappeared from aircraft snapshots")
        rows = [snapshot[head] for snapshot in observations]
        if any(row["type"] != 3 for row in rows):
            raise RuntimeError("aircraft witness is not an aircraft")
        prefix = "aircraft[0]/common[0]/"
        positions = {
            tuple(row[prefix + field] for field in ("x_pos", "y_pos", "z_pos"))
            for row in rows
        }
        cargo_counts = []
        for snapshot in observations:
            index, cargo = head, 0
            while index is not None:
                row = snapshot[index]
                cargo += sum(row[prefix + "cargo.action_counts"])
                index = row[prefix + "next"] - 1 if row[prefix + "next"] else None
            cargo_counts.append(cargo)
        paid = [
            b[prefix + "profit_this_year"] - a[prefix + "profit_this_year"]
            for a, b in zip(rows, rows[1:], strict=False)
            if a[prefix + "profit_last_year"] == b[prefix + "profit_last_year"]
            and b[prefix + "profit_this_year"] > a[prefix + "profit_this_year"]
        ]
        stations = sorted(
            {row[prefix + "last_station_visited"] for row in rows} - {65535}
        )
        states = sorted({row["aircraft[0]/state"] for row in rows})
        motion = [row[prefix + "motion_counter"] for row in (rows[0], rows[-1])]
        if (len(positions) < 2 and motion[0] == motion[1]) or (
            mode == "snapshots"
            and (
                not any(cargo_counts)
                or not paid
                or len(stations) < 2
                or 14 not in states
            )
        ):
            raise RuntimeError(
                f"{name} lacks aircraft movement/cargo/delivery witnesses"
            )
        witnesses[name] = {
            "head": head,
            "positions": sorted(positions),
            "motion_counter": motion,
            "stations": stations,
            "airport_states": states,
            "cargo_counts": cargo_counts,
            "positive_profit_deltas": paid,
        }
    result[f"{mode}_{role}_aircraft"] = witnesses


def finite_range_probe(folder, candidate_build, env):
    """No supplied NewGRF gives finite aircraft range: check unchanged bodies."""
    import migration

    source = (migration.REFERENCE / "src/aircraft_cmd.cpp").read_text()
    begin = source.index("static void AircraftHandleDestTooFar(")
    end = source.index("bool Aircraft::Tick()", begin)
    (folder / "aircraft-range-reference.inc").write_text(source[begin:end])
    archive = next((candidate_build / "cargo").glob("*/release/libopenttd_kernels.a"))
    probe = folder / "aircraft-range"
    command = [
        "c++",
        "-std=c++20",
        "-O2",
        "-fno-strict-overflow",
        "-I",
        str(ROOT / "src"),
        "-I",
        str(folder),
        str(ROOT / "tools/migration/aircraft-range.cpp"),
        str(archive),
        "-ldl",
        "-lpthread",
        "-lm",
        "-o",
        str(probe),
    ]
    compiled = subprocess.run(command, env=env, text=True, capture_output=True)
    (folder / "range-build.log").write_text(compiled.stdout + compiled.stderr)
    compiled.check_returncode()
    checked = subprocess.run(
        [str(probe)], env=env, text=True, capture_output=True, check=True
    )
    (folder / "range-check.log").write_text(checked.stdout + checked.stderr)
    return {
        "command": command,
        "output": checked.stdout.strip(),
        "reference_sha256": hashlib.sha256(source[begin:end].encode()).hexdigest(),
    }


def prepare(scenario, binaries, builds, out, timeout, env, result):
    """Reference-built airport layouts and explicitly declared branch inputs."""
    if not scenario.get("aircraft_control"):
        return scenario
    with LOCK:
        kind = scenario["aircraft_control"]
        count = 1 if kind in ("removal", "orphan", "crash", "ownerless") else 8
        folder = (
            out
            / f"aircraft-controller-fixture-{count}{'-oilrig' if kind == 'ownerless' else ''}"
        )
        source = folder / "save/autosave/exit.sav"
        if not source.exists():
            setup = {
                "kind": "generate",
                "aircraft_controller_setup": True,
                "aircraft_count": count,
                "aircraft_oilrig": kind == "ownerless",
                "scenario_ai": scenario["scenario_ai"],
                "seed": 86,
                "map_log2": 7,
                "land_generator": 1,
                "ticks": 6000,
                "console": ["start_ai MigrationAircraftController", "unpause"],
                "settings": {
                    "game_creation": {
                        "starting_year": 2000,
                        "terrain_type": 0,
                        "quantity_sea_lakes": 0,
                        "variety": 0,
                        "custom_town_number": 20,
                    },
                    "difficulty": {
                        "number_towns": 4,
                        "max_loan": 20_000_000,
                        "disasters": "false",
                    },
                    "economy": {"initial_city_size": 4, "inflation": "false"},
                    "vehicle": {"plane_crashes": 0},
                },
            }
            if kind == "ownerless":
                setup["settings"]["difficulty"]["quantity_sea_lakes"] = 2
                setup["settings"]["construction"] = {"raw_industry_construction": 1}
            trial = run_game(
                setup,
                binaries["reference"],
                builds["reference"],
                folder,
                timeout,
                env,
                False,
            )
            if trial["exit"] or not any(
                "AIRCRAFT-CONTROL-END" in line for line in trial["log"]
            ):
                raise RuntimeError(
                    f"reference aircraft controller construction failed: {folder}"
                )
        if kind == "zeppelin":
            for attempt in range(40):
                current = rows(source)
                landing = next(
                    (
                        index
                        for index, row in current.items()
                        if row["type"] == 3
                        and row["aircraft[0]/common[0]/subtype"] == 2
                        and row["aircraft[0]/state"] == 15
                        and row["aircraft[0]/pos"] == 33
                    ),
                    None,
                )
                if landing is not None:
                    break
                trial = run_game(
                    {
                        "kind": "save",
                        "save": str(source),
                        "ticks": 64,
                        "console": ["unpause"],
                    },
                    binaries["reference"],
                    builds["reference"],
                    folder / f"landing-{attempt}",
                    timeout,
                    env,
                    False,
                )
                if trial["exit"]:
                    raise RuntimeError("reference landing preparation failed")
                source = trial["snapshots"][-1]
            else:
                raise RuntimeError("reference aircraft never entered landing approach")
        original = source.read_bytes()
        ai_configuration = original[slice(*read_save(source)["AIPL"]["span"])]
        if kind in ("removal", "orphan", "crash"):
            for attempt in range(30):
                current = rows(source)
                if any(
                    row["type"] == 3
                    and row["aircraft[0]/common[0]/subtype"] == 2
                    and row["aircraft[0]/state"] == 14
                    and row["aircraft[0]/targetairport"] == 1
                    for row in current.values()
                ):
                    break
                trial = run_game(
                    {
                        "kind": "save",
                        "save": str(source),
                        "ticks": 128,
                        "console": ["unpause"],
                    },
                    binaries["reference"],
                    builds["reference"],
                    folder / f"airborne-{attempt}",
                    timeout,
                    env,
                    False,
                )
                if trial["exit"]:
                    raise RuntimeError("reference airborne removal preparation failed")
                source = trial["snapshots"][-1]
        loading = None
        if kind.startswith("breakdown-landed-"):
            cached = folder / "breakdown-loading.sav"
            if cached.exists():
                source = cached
            for attempt in range(16):
                loading = next(
                    (
                        index
                        for index, row in rows(source).items()
                        if row["type"] == 3
                        and row["aircraft[0]/common[0]/subtype"] == 2
                        and row["aircraft[0]/common[0]/current_order.type"] & 15 == 3
                        and row["aircraft[0]/state"] in range(2, 8)
                    ),
                    None,
                )
                if loading is not None:
                    if source != cached:
                        shutil.copyfile(source, cached)
                    break
                trial = run_game(
                    dict(kind="save", save=str(source), ticks=8, console=["unpause"]),
                    binaries["reference"],
                    builds["reference"],
                    folder / f"breakdown-loading-{attempt}",
                    timeout,
                    env,
                    False,
                )
                if trial["exit"]:
                    raise RuntimeError("reference loading preparation failed")
                source = trial["snapshots"][-1]
            else:
                raise RuntimeError("reference plane never started loading")
        allrows = rows(source)
        heads = [
            index
            for index, row in allrows.items()
            if row["type"] == 3 and row["aircraft[0]/common[0]/subtype"] <= 2
        ]
        stations = rows(source, "STNN")
        airports = {
            index: row
            for index, row in stations.items()
            if row.get("normal[0]/airport.type") in (4, 8, 9)
        }
        if len(heads) != 2 * count or len(airports) != (
            4 if kind == "ownerless" else 3
        ):
            raise RuntimeError(
                "controller reference input lacks all aircraft/airport layouts"
            )
        edits = {}
        flight = next(
            (
                index
                for index in heads
                if (kind == "ownerless" or allrows[index]["aircraft[0]/state"] == 14)
                and allrows[index]["aircraft[0]/common[0]/subtype"] == 2
                and (
                    kind not in ("removal", "orphan", "crash")
                    or allrows[index]["aircraft[0]/targetairport"] == 1
                )
            ),
            None,
        )
        if flight is None:
            raise RuntimeError("controller input lacks an airborne plane")
        if kind == "zeppelin":
            flight = landing
        if loading is not None:
            flight = loading
        target = allrows[flight]["aircraft[0]/targetairport"]
        action = {}
        if kind.startswith("breakdown-"):
            speed, tick, _, _ = BREAKDOWNS[kind.removeprefix("breakdown-")]
            common = "aircraft[0]/common[0]/"
            fields = {
                common + "cur_speed": speed,
                common + "subspeed": 0,
                common + "tick_counter": tick,
                common + "breakdown_ctr": 2,
            }
            if loading is not None:
                fields.update(
                    {
                        common + "current_order.flags": allrows[flight][
                            common + "current_order.flags"
                        ]
                        | 8,
                        common + "current_order.wait_time": 65535,
                        common + "current_order_time": 0,
                        common + "lateness_counter": 0,
                    }
                )
            edits["VEHS"] = {flight: fields}
        elif kind in ("closure", "removal", "orphan"):
            action = {
                "aircraft_action": kind,
                "aircraft_action_target": target
                if kind == "closure"
                else airports[target]["normal[0]/airport.tile"],
            }
        elif kind == "crash":
            action = {
                "aircraft_action": "crash",
                "aircraft_action_airports": [
                    airports[index]["normal[0]/airport.tile"]
                    for index in sorted(airports, reverse=True)
                ],
            }
        elif kind == "zeppelin":
            flags = airports[target]["normal[0]/airport.flags"]
            edits["STNN"] = {
                target: {"normal[0]/airport.flags": flags | (1 << 62) | (1 << 8)}
            }
        elif kind == "reload" or kind.startswith("landing-"):
            # A second reference run produces a live rotor/FTA reload, rather than
            # changing candidate fields or expectations together.
            resumed = folder / "reload"
            trial = run_game(
                {
                    "kind": "save",
                    "save": str(source),
                    "ticks": 411,
                    "console": ["unpause"],
                },
                binaries["reference"],
                builds["reference"],
                resumed,
                timeout,
                env,
                False,
            )
            if trial["exit"]:
                raise RuntimeError("reference live aircraft reload failed")
            source = trial["snapshots"][-1]
            if kind.startswith("landing-"):
                flight = 2
                target = rows(source)[flight]["aircraft[0]/targetairport"]
                seeds = LANDING_SEEDS[kind.removeprefix("landing-")]
                edits["DATE"] = {
                    0: {f"random_state[{i}]": seed for i, seed in enumerate(seeds)}
                }
                if kind == "landing-event":
                    action = {"aircraft_action": "landing-event"}
        destination = out / scenario["name"] / "input.sav"
        destination.parent.mkdir(parents=True, exist_ok=True)
        record = patch(source, destination, edits)
        if action:
            # Reference stepping installed a dummy AI. Restore only the setup's
            # exact AIPL configuration so both roles execute the frozen command.
            data = destination.read_bytes()
            begin, end = read_save(destination)["AIPL"]["span"]
            destination.write_bytes(data[:begin] + ai_configuration + data[end:])
            record["assignments"].append(
                ["AIPL", "configuration", "reference setup AI restored"]
            )
            record["sha256"] = hashlib.sha256(destination.read_bytes()).hexdigest()
        if "range_probe" not in scenario and not (folder / "range-check.log").exists():
            import migration

            result["aircraft_range_gap"] = finite_range_probe(
                folder, migration.ROOT / "build-rust", env
            )
        result["aircraft_input"] = record
        result["aircraft_input"]["heads"] = heads
        result["aircraft_input"]["airports"] = sorted(airports)
        result["aircraft_input"]["selected"] = flight
        return dict(
            scenario,
            save=str(destination),
            aircraft_heads=heads,
            aircraft_selected=flight,
            aircraft_airports=sorted(airports),
            aircraft_target=target,
            **action,
        )


def check_breakdown(scenario, run, mode, role, result):
    """Declared boundary inputs enter ordinary HandleBreakdown in both passes."""
    kind = scenario["aircraft_control"].removeprefix("breakdown-")
    speed, tick, final_speed, smoke_count = BREAKDOWNS[kind]
    source, final = Path(scenario["save"]), run["snapshots"][-1]
    before, after = rows(source), rows(final)
    selected = scenario["aircraft_selected"]
    a, b = before[selected], after[selected]
    plane, common = "aircraft[0]/", "aircraft[0]/common[0]/"
    landed = kind.startswith("landed-")
    if (
        a[common + "cur_speed"] != speed
        or a[common + "tick_counter"] != tick
        or a[common + "breakdown_ctr"] != 2
        or a[common + "vehstatus"] != 8
        or a[common + "engine_type"] != 238
        or rows(source, "PATS")[0]["vehicle.plane_speed"] != 4
        or any(
            before[head][common + "breakdown_ctr"]
            for head in scenario["aircraft_heads"]
            if head != selected
        )
        or (a[common + "current_order.type"] & 15 == 3) != landed
    ):
        raise RuntimeError("breakdown input lacks ordinary visible plane boundary")
    if not landed and [
        a[plane + field] for field in ("state", "pos", "previous_pos", "targetairport")
    ] != [14, 38, 37, 0]:
        raise RuntimeError("breakdown input lacks maximum-speed international FTA")
    if rows(source, "STNN")[a[plane + "targetairport"]]["normal[0]/airport.type"] != 4:
        raise RuntimeError("breakdown input lacks the selected international airport")
    if a[common + "current_order.max_speed"] != 65535:
        raise RuntimeError("breakdown input has an independent order speed limit")
    if landed and (
        a[common + "current_order.flags"] & 8 == 0
        or a[common + "current_order.wait_time"] != 65535
        or a[common + "current_order_time"] != 0
        or a[common + "lateness_counter"] != 0
        or b[common + "current_order.type"] & 15 != 3
        or any(
            a[common + key] != b[common + key] for key in ("x_pos", "y_pos", "z_pos")
        )
    ):
        raise RuntimeError("breakdown loading boundary moved or departed")
    # Stock engine238 (aircraft 23) is Dinger 100: max speed (74*128)/10=947.
    # International position38 has NoSpeedClamp/SlowTurn, so neither limit
    # selects 320 itself.
    if (
        b[common + "tick_counter"] != tick + 1
        or save_moment(final)[2] != save_moment(source)[2] + 1
    ):
        raise RuntimeError("breakdown checkpoint did not advance exactly one tick")
    cleared = kind == "landed-9"
    if (
        b[common + "cur_speed"] != final_speed
        or b[common + "breakdown_ctr"] != (0 if cleared else 1)
        or b[common + "vehstatus"] != (8 if cleared else 72)
        or b[common + "breakdowns_since_last_service"]
        != a[common + "breakdowns_since_last_service"] + 1
    ):
        raise RuntimeError("breakdown speed/clear/native entry witness differs")
    old_effects, new_effects = effect_rows(source), effect_rows(final)
    smoke = {
        i: row
        for i, row in new_effects.items()
        if i not in old_effects and row["subtype"] == 10
    }
    dx, dy = ((5, 5), (6, 0), (5, -5), (0, -6), (-5, -5), (-6, 0), (-5, 5), (0, 6))[
        a[common + "direction"]
    ]
    position = [
        a[common + "x_pos"] + dx,
        a[common + "y_pos"] + dy,
        a[common + "z_pos"] + 2,
    ]
    if len(smoke) != smoke_count or any(
        [row[key] for key in ("x_pos", "y_pos", "z_pos")] != position
        for row in smoke.values()
    ):
        raise RuntimeError("breakdown smoke cadence/count/relative position differs")
    rng = [rows(final, "DATE")[0][f"random_state[{i}]"] for i in (0, 1)]
    if rng != BREAKDOWN_RNG["loading" if landed else "airborne"]:
        raise RuntimeError("breakdown shared RNG differs from frozen reference")
    result[f"{mode}_{role}_breakdown"] = {
        "selected": selected,
        "input_speed": speed,
        "final_speed": b[common + "cur_speed"],
        "tick_counter": b[common + "tick_counter"],
        "counter": b[common + "breakdown_ctr"],
        "status": b[common + "vehstatus"],
        "smoke": smoke,
        "shared_rng": rng,
    }


def check_landing(scenario, run, mode, role, result):
    """Ordinary valid-station MaybeCrash: threshold, mask, draw and side effects."""
    kind = scenario["aircraft_control"].removeprefix("landing-")
    source, final = Path(scenario["save"]), run["snapshots"][-1]
    before, after = rows(source), rows(final)
    plane, common = "aircraft[0]/", "aircraft[0]/common[0]/"
    a, b = before[2], after[2]
    target = a[plane + "targetairport"]
    station_a, station_b = rows(source, "STNN")[target], rows(final, "STNN")[target]
    if (
        [
            a[plane + field]
            for field in ("state", "pos", "previous_pos", "targetairport")
        ]
        != [16, 34, 33, 0]
        or a[common + "cur_speed"] != 293
        or a[common + "vehstatus"] != 8
        or a[plane + "crashed_counter"] != 0
        or station_a["normal[0]/airport.type"] != 4
    ):
        raise RuntimeError("reference reload lacks the actual landing brake boundary")
    shadow = a[common + "next"] - 1
    for head in (2, shadow):
        if not before[head][common + "cargo.packets"]:
            raise RuntimeError("landing input lacks plane passenger/mail cargo")
    cargo = {
        key: value
        for key, value in station_a.items()
        if "/cargo[" in key and key.endswith("/second")
    }
    if not all(station_a[f"normal[0]/goods[{i}]/cargo[0]/second"] for i in (0, 2)):
        raise RuntimeError("landing input lacks target-station passenger/mail cargo")
    crash = kind in ("equal", "masked", "event")
    if b[common + "vehstatus"] != (138 if crash else 8) or b[
        plane + "crashed_counter"
    ] != (93 if kind == "event" else 3 if crash else 0):
        raise RuntimeError(f"landing {kind} crash status/counter differs from original")
    for head in (2, shadow):
        for field in ("cargo.packets", "cargo.action_counts"):
            expected = (
                (() if field.endswith("packets") else (0,) * 4)
                if crash
                else before[head][common + field]
            )
            if after[head][common + field] != expected:
                raise RuntimeError(f"landing {kind} passenger/mail cleanup differs")
    if crash:
        if any("/cargo[" in key for key in station_b):
            raise RuntimeError("landing crash did not clear all target-station cargo")
        for i in range(64):
            # MaybeCrash sets every rating to 1, then CrashAirplane's surrounding
            # station penalty lowers ratings with nonzero goods status to zero.
            if station_b[f"normal[0]/goods[{i}]/rating"] != (
                0 if i in (0, 2, 5) else 1
            ):
                raise RuntimeError("landing crash rating reset/nearby penalty differs")
    elif any(station_b.get(key) != value for key, value in cargo.items()) or any(
        station_b[f"normal[0]/goods[{i}]/rating"]
        != station_a[f"normal[0]/goods[{i}]/rating"]
        for i in range(64)
    ):
        raise RuntimeError("surviving landing changed target-station cargo/ratings")
    mask = station_b["normal[0]/airport.flags"]
    if mask != (2304 if kind == "event" else 3328):
        raise RuntimeError("landing airport reservation mask differs")
    rng = [rows(final, "DATE")[0][f"random_state[{i}]"] for i in (0, 1)]
    expected_rng = {
        "equal": [3963465306, 766842487],
        "above": [1772663753, 1476991553],
        "masked": [3996593850, 770979447],
        "disabled": [3995363677, 21],
        "event": [3455259752, 4266961540],
    }
    if rng != expected_rng[kind]:
        raise RuntimeError(f"landing {kind} shared RNG {rng} differs from original")
    events = [
        line.split("LANDING-EVENT ", 1)[1]
        for line in run["log"]
        if "LANDING-EVENT " in line
    ]
    if events != (["2 10064 3 10"] if kind == "event" else []):
        raise RuntimeError(
            "landing crash event/site/reason/victims missing or repeated"
        )
    result[f"{mode}_{role}_landing"] = {
        "crashed": crash,
        "counter": b[plane + "crashed_counter"],
        "status": b[common + "vehstatus"],
        "shared_rng": rng,
        "airport_blocks": mask,
        "events": events,
    }


def check_control(scenario, run, mode, role, result):
    if scenario["aircraft_control"].startswith("breakdown-"):
        check_breakdown(scenario, run, mode, role, result)
        return
    if scenario["aircraft_control"].startswith("landing-"):
        check_landing(scenario, run, mode, role, result)
        return
    if scenario.get("aircraft_action") and not any(
        f"AIRCRAFT-ACTION {scenario['aircraft_action']} true" in line
        for line in run["log"]
    ):
        raise RuntimeError("aircraft airport command did not execute")
    observations = [
        rows(Path(scenario["save"])),
        *[rows(path) for path in run["snapshots"]],
    ]
    stations = [
        rows(Path(scenario["save"]), "STNN"),
        *[rows(path, "STNN") for path in run["snapshots"]],
    ]
    states, pads, groups = set(), set(), set()
    rotor_states, shadow_positions = set(), set()
    service, plane_service = False, False
    waiting_details = []
    occupied_wait = 0
    blocked = []
    closed_circling = 0
    ownerless_landing = 0
    for sample, snapshot in enumerate(observations):
        for head in scenario["aircraft_heads"]:
            if head not in snapshot:
                continue
            row = snapshot[head]
            state = row["aircraft[0]/state"]
            states.add(state)
            target = row["aircraft[0]/targetairport"]
            if (
                state in (8, 9, 17, 18, 21)
                and target in stations[sample]
                and stations[sample][target]["normal[0]/airport.type"] == 9
                and stations[sample][target]["normal[0]/base[0]/owner"] == 16
            ):
                ownerless_landing += 1
            if (
                scenario["aircraft_control"] == "closure"
                and head == scenario["aircraft_selected"]
                and state == 14
                and row["aircraft[0]/targetairport"] == scenario["aircraft_target"]
                and stations[sample][scenario["aircraft_target"]][
                    "normal[0]/airport.flags"
                ]
                & (1 << 63)
            ):
                closed_circling += 1
            if state in (8, 9, 21):
                pads.add(state)
            if state in range(2, 8):
                groups.add(0 if state < 5 else 1)
            prefix = "aircraft[0]/common[0]/"
            if row[prefix + "cur_speed"] == 0 and state in (1, 10, 16, 18):
                target = row["aircraft[0]/targetairport"]
                station = stations[sample][target]
                mask = station["normal[0]/airport.flags"]
                position = row["aircraft[0]/pos"]
                waiting_details.append(
                    [head, state, position, station["normal[0]/airport.type"], mask]
                )
                # Unchanged international FTA roots: hangar0 -> entrance block13;
                # hangar1 -> hangar2-area block16. These source HasBlock paths
                # reset speed while another aircraft owns the actual next block.
                if (
                    state == 1
                    and station["normal[0]/airport.type"] == 4
                    and position in (0, 1)
                ):
                    bit = 13 if position == 0 else 16
                    if mask & (1 << bit):
                        occupied_wait += 1
                        blocked.append((sample, head, target, position, bit))
                # Helistation buffer16 -> hangar-entry23 takes Hangar2Area16;
                # an aircraft headed to HANGAR waits here for its owner to leave.
                if (
                    state == 1
                    and station["normal[0]/airport.type"] == 8
                    and position == 16
                ):
                    if mask & (1 << 16):
                        occupied_wait += 1
                        blocked.append((sample, head, target, position, 16))
            shadow = snapshot[row[prefix + "next"] - 1]
            shadow_positions.add(
                tuple(shadow[prefix + field] for field in ("x_pos", "y_pos", "z_pos"))
            )
            if row[prefix + "subtype"] == 2:
                plane_service |= (
                    row[prefix + "date_of_last_service"]
                    != observations[0][head][prefix + "date_of_last_service"]
                )
            if row[prefix + "subtype"] == 0:
                rotor = snapshot[shadow[prefix + "next"] - 1]
                rotor_states.add(rotor["aircraft[0]/state"])
                service |= (
                    row[prefix + "date_of_last_service"]
                    != observations[0][head][prefix + "date_of_last_service"]
                )
    masks = [
        snapshot[station]["normal[0]/airport.flags"]
        for snapshot in stations
        for station in scenario["aircraft_airports"]
        if station in snapshot
    ]
    released_wait = sum(
        any(
            head in snapshot
            and target in stations[later]
            and snapshot[head]["aircraft[0]/pos"] != position
            and not stations[later][target]["normal[0]/airport.flags"] & (1 << bit)
            for later, snapshot in enumerate(observations)
            if later > sample
        )
        for sample, head, target, position, bit in blocked
    )
    kind = scenario["aircraft_control"]
    if (
        kind == "traffic"
        and mode == "snapshots"
        and not (
            groups == {0, 1}
            and pads
            and occupied_wait
            and released_wait
            and service
            and plane_service
            and len(set(masks)) > 3
            and len(rotor_states) > 1
            and len(shadow_positions) > 10
        )
    ):
        raise RuntimeError(
            f"controller branches missing: groups={groups}, pads={pads}, waits={waiting_details}, occupied={occupied_wait}, heli-service={service}, plane-service={plane_service}, rotors={rotor_states}"
        )
    if kind == "crash" and scenario["aircraft_selected"] in observations[-1]:
        raise RuntimeError("crashed aircraft did not complete animation/deletion")
    if kind == "removal" and any(
        snapshot.get(1, {}).get("normal[0]/airport.tile", 0xFFFFFFFF) != 0xFFFFFFFF
        for snapshot in stations[1:]
    ):
        raise RuntimeError("airborne airport removal did not persist")
    if kind == "removal":
        final = observations[-1].get(scenario["aircraft_selected"])
        if (
            final is None
            or final["aircraft[0]/targetairport"] == scenario["aircraft_target"]
        ):
            raise RuntimeError("aircraft did not divert from its removed airport")
    # HandleMissingAircraftOrders sends an orderless plane, whose target lost
    # its airport, to a hangar (source DepotCommandFlag{} is Service).
    if kind == "orphan" and not any(
        snapshot.get(scenario["aircraft_selected"], {}).get("aircraft[0]/state") == 1
        for snapshot in observations[1:]
    ):
        raise RuntimeError("orderless aircraft was not sent to a hangar")
    if kind == "zeppelin":
        initial = observations[0][scenario["aircraft_selected"]]
        if initial["aircraft[0]/state"] != 15 or not any(
            snapshot[scenario["aircraft_selected"]]["aircraft[0]/state"] == 14
            for snapshot in observations[1:]
        ):
            raise RuntimeError("zeppelin did not abort an actual aircraft landing")
    if kind == "closure" and not any(mask & (1 << 63) for mask in masks):
        raise RuntimeError("airport closure bit disappeared")
    if kind == "closure" and mode == "snapshots" and closed_circling < 3:
        raise RuntimeError("closed-airport aircraft did not continue circling")
    if kind == "ownerless" and mode == "snapshots" and not ownerless_landing:
        raise RuntimeError("helicopter did not land at the real ownerless oilrig")
    if kind == "zeppelin" and not any(mask & (1 << 62) for mask in masks):
        raise RuntimeError("zeppelin reservation bit disappeared")
    result[f"{mode}_{role}_controller"] = {
        "states": sorted(states),
        "terminal_groups": sorted(groups),
        "pads": sorted(pads),
        "waiting": waiting_details,
        "occupied_next_block_wait": occupied_wait,
        "waited_block_released": released_wait,
        "closed_airport_circling": closed_circling,
        "ownerless_oilrig_landing": ownerless_landing,
        "plane_service": plane_service,
        "rotor_states": sorted(rotor_states),
        "shadow_positions": len(shadow_positions),
        "service": service,
        "block_masks": sorted(set(masks)),
    }
