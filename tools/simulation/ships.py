"""Ship transport, water-region cache mutation and reference fixture construction."""

import hashlib
import json
import shutil
import struct
from pathlib import Path

from .core import (
    FILE_TYPES,
    ROOT,
    SNAPSHOT_TICKS,
    WITNESS_ENVIRONMENT,
    Reader,
    branch_witnesses,
    copy_runtime,
    decode_element,
    field_spans,
    read_save,
    read_value,
    run_game,
    save_moment,
    witness_counts,
)
from .play_saves import DISTRIBUTIONS

AI_FOLDER = "water-scenario-ai"


def uses_ai(scenario):
    return scenario.get("water") == "structures"


def game_args(scenario):
    return ["-d", "yapf=3"] if scenario.get("water") else []


def game_environment(scenario):
    witnessed = (
        scenario.get("water") or scenario.get("water_commands")
    ) and scenario.get("witnesses")
    return WITNESS_ENVIRONMENT if witnessed else {}


def install(scenario, run_dir):
    if uses_ai(scenario):
        ai = run_dir / "ai/water-scenarios"
        shutil.copytree(scenario["scenario_ai"], ai)
        (ai / "parameters.nut").write_text(
            f'WATER_SHIP <- 22;\nWATER_MODE <- "{scenario["water_operation"]}";\n'
        )


def scenarios(soak):
    scenarios = [
        dict(
            name="water-commands",
            kind="save",
            water_commands=True,
            save=str(ROOT / "migration/saves/water-structures.sav"),
            short_checkpoint=True,
            ticks=1,
            console=["water_scenario", "unpause"],
        ),
        dict(
            name="ownership-dispatch",
            kind="save",
            water_commands="ownership",
            save=str(ROOT / "migration/saves/water-structures.sav"),
            short_checkpoint=True,
            ticks=1,
            console=["owner_scenario", "unpause"],
        ),
    ]
    for distribution, commands in DISTRIBUTIONS.items():
        scenarios.append(
            {
                "name": f"water-ferry-{distribution}",
                "kind": "save",
                "water": "ferry",
                "save": str(ROOT / "migration/saves/water-ferry.sav"),
                "console": commands,
                "ticks": 200000 if soak else 100000,
            }
        )
    for operation in ("route", "mutate", "depot", "reload", "lifecycle", "class"):
        scenarios.append(
            {
                "name": f"water-structures-{operation}",
                "kind": "save",
                "water": "structures",
                "water_operation": operation,
                "save": str(ROOT / "migration/saves/water-structures.sav"),
                "console": ["unpause"],
                "ticks": 100000 if soak else 60000,
            }
        )
    return scenarios


def check_water(scenario, run, desync):
    source = Path(scenario["save"])
    record = scenario.get("water_record") or json.loads(
        source.with_suffix(".json").read_text()
    )
    if hashlib.sha256(source.read_bytes()).hexdigest() != record["normalized_sha256"]:
        raise RuntimeError("water fixture differs from its preparation record")
    ship = record["ship"]

    def observe(path):
        chunk = read_save(path)["VEHS"]
        row = decode_element(chunk, dict(chunk["elements"])[ship])
        if row["type"] != 2:
            raise RuntimeError("water witness is not a ship")
        common = "ship[0]/common[0]/"
        return {
            "tick": save_moment(path)[2],
            **{
                key: row[common + key]
                for key in (
                    "tile",
                    "motion_counter",
                    "profit_this_year",
                    "profit_last_year",
                    "cargo.action_counts",
                    "cur_real_order_index",
                )
            },
            "path_length": sum(k.startswith("ship[0]/path[") for k in row),
        }

    rows = [observe(source), *[observe(path) for path in run["snapshots"]]]
    elapsed = rows[-1]["tick"] - rows[0]["tick"]
    if (
        elapsed < 2 * SNAPSHOT_TICKS
        or rows[-1]["motion_counter"] == rows[0]["motion_counter"]
    ):
        raise RuntimeError("water fixture did not advance a moving ship")
    if not rows[0]["path_length"]:
        raise RuntimeError("water fixture lacks a saved live ship path")
    loaded = any(sum(row["cargo.action_counts"]) > 0 for row in rows)
    paid = any(
        a["profit_last_year"] == b["profit_last_year"]
        and b["profit_this_year"] > a["profit_this_year"]
        for a, b in zip(rows, rows[1:], strict=False)
    )
    if desync and scenario["water"] == "ferry" and not (loaded and paid):
        raise RuntimeError("ferry snapshots lack loaded cargo and a paid delivery")
    markers = [
        line.split("WATER ", 1)[1].split() for line in run["log"] if "WATER " in line
    ]
    operation = scenario.get("water_operation")
    if scenario["water"] == "structures":
        expected = (
            ["ship", "warm-right", "warm-left"]
            + (
                [
                    "close-lock",
                    "lock-lost",
                    "open-lock",
                    "lock-recovered",
                    "close-canal",
                    "canal-lost",
                    "open-canal",
                    "canal-recovered",
                ]
                if operation == "mutate"
                else ["send-depot", "depot-arrival", "restart", "depot-recovered"]
                if operation == "depot"
                else [
                    "build",
                    "sell",
                    "reuse",
                    "reuse-order-a",
                    "reuse-order-b",
                    "reuse-start",
                    "reuse-moved",
                ]
                if operation == "lifecycle"
                else []
            )
            + ["complete"]
        )
        if [row[0] for row in markers] != expected:
            raise RuntimeError("water command/lost/recovery witnesses are incomplete")
    lifecycle = None
    if operation == "lifecycle":
        ids = [
            int(row[1])
            for row in markers
            if row[0] in ("build", "sell", "reuse", "reuse-moved")
        ]
        if len(ids) != 4 or len(set(ids)) != 1 or ids[0] == ship:
            raise RuntimeError("ship build/sell/reuse markers lack actual ID reuse")
        chunk = read_save(run["snapshots"][-1])["VEHS"]
        reused = decode_element(chunk, dict(chunk["elements"])[ids[0]])
        if reused["type"] != 2 or reused["ship[0]/common[0]/motion_counter"] == 0:
            raise RuntimeError("rebuilt ship owner did not drive a moving ship")
        lifecycle = {
            "reused_id": ids[0],
            "motion_counter": reused["ship[0]/common[0]/motion_counter"],
        }
    return {
        "input": record,
        "lifecycle": lifecycle,
        "elapsed_ticks": elapsed,
        "observations": rows,
        "loaded": loaded,
        "paid": paid,
        "markers": markers,
    }


def adapter(build, folder, env):
    from .console_adapter import console_adapter

    return console_adapter(
        build, folder, env, ROOT / "tools/simulation/water-console.cpp", "ships"
    )


def prepare(scenario, binaries, builds, out, timeout, env, result):
    if scenario.get("water_commands"):
        import migration

        from .runtime import prepared_adapter

        executables = {}
        for role in ("reference", "candidate"):
            folder = out / scenario["name"] / role
            prepared = prepared_adapter("ships", role, builds, folder)
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
                    prepared = adapter(native, folder, env)
            executables[role] = prepared
            for name in ("lang", "baseset", "ai", "game", "regression"):
                if (builds[role] / name).is_dir():
                    (folder / name).symlink_to(
                        builds[role] / name, target_is_directory=True
                    )
        return dict(scenario, executables=executables)

    if scenario.get("water_operation") == "class":
        # MakeRiver differs from the existing flat clear canal only in m1:
        # WaterClass::River (2 << 5) and OWNER_WATER (17). Both roles load this
        # explicit source input, so the controller crosses canal/river boundaries.
        source = Path(scenario["save"])
        data, chunks = bytearray(source.read_bytes()), read_save(source)
        tile = 190 * 256 + 163
        if (
            chunks["MAPT"]["raw"][tile] != 96
            or chunks["MAP5"]["raw"][tile] != 0
            or chunks["MAPO"]["raw"][tile] != 33
        ):
            raise RuntimeError("water-class input is not the expected flat clear canal")
        offset = chunks["MAPO"]["span"][1] - len(chunks["MAPO"]["raw"]) + tile
        data[offset] = 81
        folder = out / scenario["name"] / "input"
        folder.mkdir(parents=True, exist_ok=True)
        prepared = folder / "river-input.sav"
        prepared.write_bytes(data)
        record = {
            "ship": 22,
            "reference_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
            "normalized_sha256": hashlib.sha256(data).hexdigest(),
            "changed": {"chunk": "MAPO", "tile": tile, "old": 33, "new": 81},
        }
        result["water_class_input"] = record
        return dict(scenario, save=str(prepared), water_record=record)
    if scenario.get("water_operation") != "reload":
        return scenario
    setup = dict(scenario, water_operation="route", ticks=3 * SNAPSHOT_TICKS)
    run = run_game(
        setup,
        binaries["reference"],
        builds["reference"],
        out / scenario["name"] / "prepare",
        timeout,
        env,
    )
    if run["exit"] != 0:
        raise RuntimeError("ship reload preparation failed")
    ship = json.loads(Path(scenario["save"]).with_suffix(".json").read_text())["ship"]
    fixture = next(
        (
            path
            for path in run["snapshots"]
            if any(
                key.startswith("ship[0]/path[")
                for key in decode_element(
                    read_save(path)["VEHS"],
                    dict(read_save(path)["VEHS"]["elements"])[ship],
                )
            )
        ),
        None,
    )
    if fixture is None:
        raise RuntimeError("ship reload has no saved live path")
    original, data = read_save(fixture), fixture.read_bytes()
    begin, end = original["GLOG"]["span"]
    normalized = fixture.parent / "ship-reload-input.sav"
    normalized.write_bytes(data[:begin] + data[end:])
    chunks = read_save(normalized)
    if set(chunks) != set(original) - {"GLOG"} or any(
        data[slice(*original[cid]["span"])]
        != normalized.read_bytes()[slice(*chunks[cid]["span"])]
        for cid in chunks
    ):
        raise RuntimeError("ship reload input changed outside GLOG")
    record = {
        "ship": ship,
        "reference_sha256": hashlib.sha256(data).hexdigest(),
        "normalized_sha256": hashlib.sha256(normalized.read_bytes()).hexdigest(),
        "removed_chunk": "GLOG",
    }
    result["water_reload_input"] = record
    return dict(scenario, save=str(normalized), water_record=record)


# Successful builds per kind over both areas, and each bankrupt tile's (type, owner).
OWNER_BUILDS = dict(rail=8, road=20, stop=8, station=10, water=6, bridge=4, tunnel=4)
OWNER_BANKRUPT = {
    **dict.fromkeys(["drive-bus", "drive-truck", "road-waypoint", "crossing"], (2, 16)),
    **dict.fromkeys(["rail", "bay", "rail-station", "airport", "dock"], (0, 16)),
    **dict.fromkeys(
        [
            "dock-water",
            "buoy",
            "lock",
            "lock-upper",
            "lock-lower",
            "canal",
            "depot",
            "depot-other",
        ],
        (6, 16),
    ),
    **dict.fromkeys(["rail-bridge", "rail-tunnel"], (0, 16)),
    **dict.fromkeys(["road-bridge", "road-tunnel"], (9, 16)),
    "foreign-crossing": (2, 0),
}
OWNER_INDUSTRIES = {
    "merge 0": "0 0 0",
    "merge 1": "16 0 255",
    "bankrupt 2": "16 255 255",
    "bankrupt 3": "16 255 255",
}


def check_ownership(scenario, run, role, markers):
    for kind, count in OWNER_BUILDS.items():
        if sum(row.startswith(f"cost owner-{kind} true ") for row in markers) != count:
            raise RuntimeError("ownership fixture did not build " + kind)
    tiles = {}
    for row in markers:
        if row.startswith("owner "):
            _, phase, name, kind, m1 = row.split()[:5]
            tiles.setdefault(phase, {})[name] = (int(kind), int(m1) & 31)
    if any(
        tiles["merge"][name] != (kind, 0) for name, (kind, _) in tiles["built"].items()
    ):
        raise RuntimeError("merger did not transfer every owned tile")
    if any(tiles["bankrupt"][name] != want for name, want in OWNER_BANKRUPT.items()):
        raise RuntimeError("bankruptcy left unexpected tiles: " + str(tiles))
    industries = {
        " ".join(row.split()[1:3]): " ".join(row.split()[3:])
        for row in markers
        if row.startswith("owner-industry ") and row.split()[1] != "built"
    }
    if industries != OWNER_INDUSTRIES:
        raise RuntimeError("industry ownership roles differ: " + str(industries))
    for prefix, required in (
        ("station_command", ["change_owner", "remove_road_tile", "clear"]),
        ("road_commands", ["owner"]),
        ("rail_cmd", ["owner_transfer", "owner_clear", "remove_crossing"]),
        ("water_cmd", ["owner"]),
    ):
        branch_witnesses(scenario, run, role, prefix, required)


def check(scenario, run, mode, role, result):
    if scenario.get("water_commands") == "ownership":
        markers = [
            line.split("WATER-CMD ", 1)[1]
            for line in run["log"]
            if "WATER-CMD " in line
        ]
        if not markers or markers[-1] != "complete":
            raise RuntimeError("ownership fixture did not finish")
        check_ownership(scenario, run, role, markers)
        key = mode + "_ownership"
        if role == "reference":
            result[key] = markers
        elif result[key] != markers:
            raise RuntimeError("ownership dispatch results differ")
        return
    if scenario.get("water_commands"):
        markers = [
            line.split("WATER-CMD ", 1)[1]
            for line in run["log"]
            if "WATER-CMD " in line
        ]
        if not markers or markers[-1] != "complete":
            raise RuntimeError("water command fixture did not finish")
        for command, count in {
            "depot": 2,
            "ship": 2,
            "lock": 12,
            "remove-lock": 12,
            "force": 2,
            "bankrupt-occupied": 2,
            "native-object": 1,
            "buoy-build": 3,
            "buoy-bankrupt": 3,
            "buoy-reuse": 3,
            "buoy-remove": 3,
        }.items():
            if sum(row.startswith(f"cost {command} true ") for row in markers) != count:
                raise RuntimeError("water fixture did not exercise " + command)
        if (
            sum(row.startswith("cost above-lock-int-min false ") for row in markers)
            != 12
        ):
            raise RuntimeError("water fixture did not exercise wrapping bridge errors")
        required = {
            f"restored-owner {kind} {mismatch} {bankrupt}"
            for kind in (
                "clear",
                "sea",
                "elevated-sea-canal",
                "canal",
                "river",
                "sloped-canal-land",
                "sloped-river-land",
            )
            for mismatch in ("false", "true")
            for bankrupt in ("false", "true")
        }
        if {
            " ".join(row.split()[:4])
            for row in markers
            if row.startswith("restored-owner ")
        } != required:
            raise RuntimeError("water fixture missed restored-owner cases")
        captured = {
            f"captured {viewer} {owner} {tile}"
            for viewer in (0, 1)
            for owner in (0, 1, 15)
            for tile in (0, 10280)
            if owner != 15 or tile != 0
        }
        if {row for row in markers if row.startswith("captured ")} != captured:
            raise RuntimeError("water fixture missed captured-owner checks")
        if sum(row.startswith("parameter captured-owner ") for row in markers) != 3:
            raise RuntimeError("water fixture missed encoded ownership errors")
        if not any(row.startswith("cost bankrupt-depot true ") for row in markers):
            raise RuntimeError("water fixture missed live depot bankruptcy")
        for name, kind in (
            ("native-object-transfer", 10),
            ("native-object-bankrupt", 0),
        ):
            if not any(
                row.startswith(f"state {name} 10280 {kind} ") for row in markers
            ):
                raise RuntimeError("water fixture missed " + name)
        for row in (
            "cost low-bridge-lock false 0 4138 ",  # bridge too low over the lower part
            # The height comes from the middle tile: 150 m, not the lower part's 100 m.
            "parameter low-bridge-lock \ue054\ue04eBridge is 150\xa0m too low for lock",
            "cost sloped-end-lock false 0 3930 ",
            "cost edge-lock false 0 3927 ",
            "cost edge-clear false 0 3927 ",
            "cost edge-lock-freeform true ",
            "cost edge-clear-freeform true ",
            "state editor-sea-zero 41120 6 17 ",
        ):
            if not any(marker.startswith(row) for marker in markers):
                raise RuntimeError("water fixture missed " + row)
        if (
            sum(row.startswith("cost water-remove-lock false ") for row in markers)
            != 12
        ):
            raise RuntimeError("water fixture missed OWNER_WATER lock clears")
        if "docking true true" not in markers:
            raise RuntimeError("water restoration did not preserve docking membership")
        if sum(row.startswith("cost buoy-occupied false ") for row in markers) != 3:
            raise RuntimeError("water fixture missed occupied buoy refusals")
        branch_witnesses(
            scenario,
            run,
            role,
            "waypoint_command",
            ["build_buoy", "remove_buoy", "bankrupt", "reuse"],
        )
        key = mode + "_water_commands"
        if role == "reference":
            result[key] = markers
        elif result[key] != markers:
            raise RuntimeError("water command/state/RNG results differ")
        branches = branch_witnesses(
            scenario,
            run,
            role,
            "water_cmd",
            "restore depot remove_depot force lock remove_lock canal skip clear owner desert".split(),
        )
        if branches is not None:
            result[f"{mode}_{role}_water_command_profile"] = branches
        return

    if "water" in scenario:
        if scenario.get("water_operation") == "route":
            result[f"{mode}_{role}_movement"] = branch_witnesses(
                scenario,
                run,
                role,
                "movement",
                ["ship_bridge_enter", "ship_bridge_exit"],
            )
        required = ["economy_day", "path_cache", "reverse", "rotate", "buoy"]
        required += (
            ["loading", "auto_service", "rotation_reload"]
            if scenario["water"] == "ferry"
            else ["lock_up", "lock_down", "aqueduct"]
        )
        if scenario.get("water_operation") == "depot":
            required += ["depot_search", "depot_leave", "depot_enter"]
        if scenario.get("water_operation") == "class":
            required += ["water_change"]
        if scenario.get("water_operation") == "lifecycle":
            required += ["build", "depot_leave"]
        branches = branch_witnesses(scenario, run, role, "ship", required)
        if branches is not None:
            if scenario.get("water_operation") == "lifecycle" and branches["build"] < 2:
                raise RuntimeError(
                    "ship controller did not initialize both build/reuse owners"
                )
            result[f"{mode}_{role}_ship_control_profile"] = branches
        result[f"{mode}_{role}_water"] = check_water(scenario, run, mode == "snapshots")
        if (profile := witness_counts(run, "water")) is not None:
            result[f"{mode}_{role}_water_profile"] = profile
        required = [
            "region_nodes",
            "track_nodes",
            "cache_truncations",
            "final_region_clears",
            "reverse_chosen",
        ]
        required += (
            ["intermediate", "alternate_docking"]
            if scenario["water"] == "ferry"
            else ["retries", "lost", "random_draws", "blocked_calls"]
        )
        branches = branch_witnesses(scenario, run, role, "ship_yapf", required)
        if branches is not None:
            result[f"{mode}_{role}_ship_yapf_profile"] = branches


def prepare_water_save(layout, migration, out, timeout):
    """Build inputs with reference commands; only funding/AI identity are patched."""
    runtime = out / "reference-runtime"
    with migration.reference_lock(shared=True):
        binary = copy_runtime(
            migration.REFERENCE_BUILD, migration.REFERENCE_BUILD / "openttd", runtime
        )
    scripts = out / "water-ai"
    shutil.copytree(ROOT / "tools/water-scenario-ai", scripts)
    scenario = {
        "kind": "save",
        "save": str(migration.REFERENCE / "regression/stationlist/test.sav"),
        "ticks": 1,
        "console": ["unpause"],
    }
    initial = run_game(
        scenario,
        binary,
        runtime,
        out / "initial",
        timeout,
        migration.environment(),
        False,
    )
    if initial["exit"] != 0 or not initial["snapshots"]:
        raise RuntimeError("water preparation could not emit the original save")
    emitted = initial["snapshots"][-1]
    data, chunks = bytearray(emitted.read_bytes()), read_save(emitted)

    def element_offset(chunk, wanted):
        reader = Reader(data, chunk["span"][0] + 5)
        reader.take(reader.gamma() - 1)
        for index, body in chunk["elements"]:
            size = reader.gamma() - 1
            begin = reader.pos
            if chunk["kind"] == 4 and reader.gamma() != index:
                raise RuntimeError("water fixture sparse index mismatch")
            offset = reader.pos
            if reader.take(size - (offset - begin)) != body:
                raise RuntimeError("water fixture table body mismatch")
            if index == wanted:
                return offset, body
        raise RuntimeError("water fixture company disappeared")

    funding = None
    if layout == "structures":
        company = chunks["PLYR"]
        offset, body = element_offset(company, 1)
        begin, end, kind = field_spans(body, company["header"])["money"]
        old = struct.unpack(FILE_TYPES[kind & 15][0], body[begin:end])[0]
        replacement = struct.pack(FILE_TYPES[kind & 15][0], 10_000_000)
        if kind != 7 or len(replacement) != end - begin:
            raise RuntimeError("water fixture money schema changed")
        data[offset + begin : offset + end] = replacement
        funding = {"company": 1, "field": "money", "old": old, "new": 10_000_000}
    funded = out / "funded-input.sav"
    funded.write_bytes(data)
    # Check every field and chunk around the sole optional funding edit.
    funded_chunks = read_save(funded)
    for cid, chunk in chunks.items():
        if cid == "PLYR" and funding:
            for (index, before), (other, after) in zip(
                chunk["elements"], funded_chunks[cid]["elements"], strict=True
            ):
                a, b = decode_element(chunk, before), decode_element(chunk, after)
                if index == 1:
                    a["money"] = funding["new"]
                if index != other or a != b:
                    raise RuntimeError("funding changed another company field")
        elif (
            emitted.read_bytes()[slice(*chunk["span"])]
            != data[slice(*funded_chunks[cid]["span"])]
        ):
            raise RuntimeError("funding changed another chunk")
    shutil.copy2(scripts / "setup.nut", runtime / "ai/stationlist/main.nut")
    (runtime / "ai/stationlist/parameters.nut").write_text(
        f'WATER_SETUP <- "{layout}";\n'
    )
    scenario.update(save=str(funded), ticks=6000)
    built = run_game(
        scenario,
        binary,
        runtime,
        out / "built",
        timeout,
        migration.environment(),
        False,
    )
    markers = [
        line.split("WATER-SETUP-END ", 1)[1]
        for line in built["log"]
        if "WATER-SETUP-END " in line
    ]
    if (
        built["exit"] != 0
        or len(markers) != 1
        or not markers[0].startswith(layout + " ")
    ):
        raise RuntimeError(
            "water construction did not complete; see retained preparation logs"
        )
    raw = built["snapshots"][-1]
    data, chunks = bytearray(raw.read_bytes()), read_save(raw)
    chunk = chunks["AIPL"]
    offset, body = element_offset(chunk, 1)
    if chunk["header"] != [
        {"key": "name", "type": 26},
        {"key": "settings", "type": 26},
        {"key": "version", "type": 6},
    ]:
        raise RuntimeError("water AI configuration schema changed")
    reader, renamed = Reader(body), []
    for key, kind in (
        ("name", 10),
        ("settings", 10),
        ("version", 6),
        ("running_name", 10),
    ):
        begin = reader.pos
        value = read_value(reader, kind)
        if key in ("name", "running_name"):
            if value.casefold() != "stationlist":
                raise RuntimeError(
                    "water preparation did not use its isolated setup AI"
                )
            # Equal-length names preserve every length and all AI saved-data bytes.
            data[offset + begin + 1 : offset + reader.pos] = b"WaterScenes"
            renamed.append([offset + begin + 1, value, "WaterScenes"])
    glog = chunks["GLOG"]["span"]
    destination = ROOT / "migration/saves" / f"water-{layout}.sav"
    destination.write_bytes(data[: glog[0]] + data[glog[1] :])
    normalized = read_save(destination)
    if set(normalized) != set(chunks) - {"GLOG"}:
        raise RuntimeError("water normalization changed chunk inventory")
    for cid, chunk in normalized.items():
        if (
            bytes(data[slice(*chunks[cid]["span"])])
            != destination.read_bytes()[slice(*chunk["span"])]
        ):
            raise RuntimeError("water normalization changed unrelated bytes")
    if destination.stat().st_size >= 1_000_000:
        raise RuntimeError("water fixture exceeds the committed-save budget")
    record = {
        "baseline": migration.BASELINE,
        "layout": layout,
        "ship": int(markers[0].split()[1]),
        "source_sha256": hashlib.sha256(
            Path(initial["snapshots"][-1]).read_bytes()
        ).hexdigest(),
        "raw_sha256": hashlib.sha256(raw.read_bytes()).hexdigest(),
        "normalized_sha256": hashlib.sha256(destination.read_bytes()).hexdigest(),
        "setup_sha256": hashlib.sha256(
            (scripts / "setup.nut").read_bytes()
        ).hexdigest(),
        "funding": funding,
        "ai_renames": renamed,
        "removed_chunk": "GLOG",
        "setup_markers": markers,
        "elapsed_ticks": save_moment(raw)[2] - save_moment(funded)[2],
    }
    destination.with_suffix(".json").write_text(json.dumps(record, indent=2) + "\n")
    print(f"Prepared {destination}; raw inputs and logs: {out}")
