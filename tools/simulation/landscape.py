"""Ground/flood scenario witnesses and unreachable scheduler comparisons."""

import hashlib
import struct
import subprocess
from pathlib import Path

from .core import ROOT, WITNESS_ENVIRONMENT, branch_witnesses, read_save
from .disasters import patch, rows
from .effects import effect_rows

# name, climate, MAPT, M3LO, M3HI, MAP5, MAP2, base height, N/W/E/S slope,
# observed chunk/value, required branches. Numeric encodings follow *_map.h.
GROUND_CASES = (
    ("grass", 0, 0, 0, 0, 226, 0, 3, 0, "MAP5", 3, ""),
    ("field", 0, 0, 8, 0, 239, 45, 3, 0, "M3LO", 96, "field_cycle field_fence"),
    ("orphan", 0, 0, 7, 0, 239, 65535, 3, 0, "MAP5", 2, "field_orphan"),
    ("snow-start", 1, 0, 0, 0, 3, 0, 10, 0, "M3LO", 16, "snow_start"),
    ("snow-increase", 1, 0, 16, 0, 0, 0, 10, 0, "MAP5", 1, "snow_change"),
    ("snow-decrease", 1, 0, 16, 0, 3, 0, 3, 0, "MAP5", 2, "snow_change"),
    ("snow-clear", 1, 0, 16, 0, 0, 0, 3, 0, "MAP5", 3, "snow_clear"),
    ("desert", 2, 1, 0, 0, 3, 0, 3, 0, "MAP5", 23, "desert"),
    ("desert-exit", 2, 0, 0, 0, 23, 0, 3, 0, "MAP5", 3, "desert"),
    ("rail-grass", 0, 16, 0, 0, 12, 0, 3, 0, "M3HI", 1, "rail_ground"),
    ("rail-fence", 0, 16, 0, 1, 1, 0, 3, 0, "M3HI", 4, "rail_ground"),
    ("rail-half-snow", 1, 16, 0, 0, 12, 0, 10, 8, "M3HI", 14, "rail_ground"),
    ("rail-snow", 1, 16, 0, 0, 12, 0, 11, 8, "M3HI", 12, "rail_ground"),
    ("dry-coast", 0, 96, 0, 0, 16, 0, 0, 7, "MAP5", 3, "dry_coast"),
    ("dry-rail", 0, 16, 0, 13, 4, 0, 0, 7, "M3HI", 10, "dry_rail"),
    ("dry-tree", 0, 64, 0, 0, 3, 240, 0, 7, "MAP2", 48, "dry_tree"),
    ("road-side", 0, 32, 0, 0, 10, 0, 3, 0, "MAPE", 8, "roadside"),
    ("road-start", 0, 32, 0, 0, 10, 0, 3, 0, "MAPE", 48, "road_work_start"),
    ("road-end", 0, 32, 0, 0, 10, 0, 3, 0, "MAPE", 8, "road_work_end"),
    ("road-partial", 0, 32, 0, 0, 10, 0, 0, 0, "MAP5", 2, "road_remove"),
    ("road-remove", 0, 32, 0, 0, 10, 0, 0, 0, "MAPT", 0, "road_remove"),
    ("half-rail", 0, 16, 0, 0, 12, 0, 0, 8, "M3HI", 13, "half_rail"),
    ("bridge-snow", 1, 144, 241, 0, 134, 0, 2, 8, "MAP7", 47, "tunnel_ground"),
    ("bridge", 0, 144, 241, 0, 134, 0, 0, 0, "MAPT", 144, "bridge_end"),
    ("airport", 0, 80, 0, 0, 23, 0, 0, 0, "MAPT", 80, "airport_member"),
    ("victims", 0, 32, 16, 0, 65, 10, 0, 0, "MAPT", 32, "crashed_skip"),
)

REMOTE = {
    "bridge": (4290, ((21, 4292, 8),)),
    "airport": (9556, ((1, 9687, 1),)),
    "victims": (10005, ((17, 10005, 0), (21, 10005, 0))),
}
REQUIRED = {
    "bridge": "bridge_start bridge_end vehicle_crash",
    "airport": "airport_member vehicle_crash",
    "victims": "vehicle_crash crashed_skip",
    "half-rail": "half_rail_remove half_rail",
}


def scenarios(soak):
    saves = {"airport": "aircraft-route", "victims": "road-crossing"}
    return [
        dict(
            name="landscape-" + case[0],
            kind="save",
            ground=case,
            save=str(
                ROOT
                / "migration/saves"
                / (saves.get(case[0], "road-flooding") + ".sav")
            ),
            short_checkpoint=True,
            ticks=1,
            console=["unpause"],
        )
        for case in GROUND_CASES
    ]


def game_environment(scenario):
    selected = "ground" in scenario or scenario.get("roads") == "flooding"
    return WITNESS_ENVIRONMENT if selected and scenario.get("witnesses") else {}


def prepare(scenario, binaries, builds, out, timeout, env, result):
    if "ground" not in scenario:
        return scenario
    case = scenario["ground"]
    folder = out / scenario["name"] / "input"
    folder.mkdir(parents=True, exist_ok=True)
    source = Path(scenario["save"])
    chunks = read_save(source)
    width = struct.unpack(">II", chunks["MAPS"]["elements"][0][1])[0]
    types = chunks["MAPT"]["raw"]
    tile = next(
        t
        for t in range(2 * width + 2, len(types) - 3 * width)
        if 2 <= t % width < width - 3
        and all(
            types[t + y * width + x] >> 4 == 0
            for y in range(-2, 3)
            for x in range(-2, 3)
        )
    )
    tile, victims = REMOTE.get(
        case[0], (4290 if case[0] == "bridge-snow" else tile, ())
    )
    sea = tile + width if case[0] == "half-rail" else tile - 1 if victims else None
    fixture = folder / "ground.sav"
    changes = {
        "DATE": {0: {"cur_tileloop_tile": sea if sea is not None else tile}},
        "PATS": {
            0: {
                "game_creation.landscape": case[1],
                "game_creation.snow_line_height": 0 if case[0] == "bridge-snow" else 10,
                "economy.mod_road_rebuild": 1,
            }
        },
    }
    if case[0] == "road-start":
        changes["CITY"] = {0: {"xy": tile, "road_build_months": 1}}
        changes["DATE"][0].update({"random_state[0]": 8, "random_state[1]": 0})
    for vehicle, target, z in victims:
        kind = "train" if vehicle == 17 else "aircraft" if vehicle == 1 else "roadveh"
        x = target % width * 16 + (0 if case[0] == "bridge" else 8)
        x += (vehicle == 21 and case[0] == "victims") - 4 * (vehicle == 17)
        values = (target, x, target // width * 16 + 8, z, 10)
        changes.setdefault("VEHS", {})[vehicle] = {
            f"{kind}[0]/common[0]/{key}": value
            for key, value in zip(
                ("tile", "x_pos", "y_pos", "z_pos", "vehstatus"), values, strict=True
            )
        }
        if vehicle != 1:
            leaf = "track" if vehicle == 17 else "state"
            value = 8 if case[0] == "bridge" else 1
            changes["VEHS"][vehicle][f"{kind}[0]/{leaf}"] = value
    if case[0] == "victims":
        for v in (18, 19):
            changes["VEHS"][v] = dict(changes["VEHS"][17])
            changes["VEHS"][v]["train[0]/common[0]/x_pos"] += 8 * (v - 17)
            changes["VEHS"][v]["train[0]/common[0]/tile"] += v == 19
    receipt = patch(source, fixture, changes)
    data = bytearray(fixture.read_bytes())

    def put(cid, t, value):
        size = 2 if cid in ("MAP2", "MAP8") else 1
        position = chunks[cid]["span"][0] + 8 + size * t
        data[position : position + size] = value.to_bytes(size, "big")
        receipt.setdefault("ground_map", []).append([cid, t, value])

    if case[0] in REMOTE:
        edge = 5 if case[0] == "airport" else 2
        for y in range(-3, edge + 4):
            for x in range(-3, edge + 4):
                t = tile + y * width + x
                distance = max(-x, x - edge, -y, y - edge, 0)
                put("MAPH", t, min(chunks["MAPH"]["raw"][t], distance))
    area = () if case[0] in REMOTE or case[0] == "bridge-snow" else range(-2, 3)
    for y in area:
        for x in range(-2, 3):
            t = tile + y * width + x
            for cid in ("MAP2", "M3LO", "M3HI", "MAPE", "MAP7", "MAP8"):
                put(cid, t, 0)
            put("MAPT", t, case[2] & 3)
            put("MAPO", t, 0 if t == tile and case[2] == 16 else 16)
            put("MAP5", t, 3)
            put("MAPH", t, case[7])
    for cid, value in zip(
        ("MAPT", "M3LO", "M3HI", "MAP5", "MAP2"), case[2:7], strict=True
    ):
        put(cid, tile, value)
    if case[0] == "victims":
        for cid in ("MAPT", "MAPO", "MAP2", "M3LO", "M3HI", "MAP5", "MAPE"):
            put(cid, tile + 1, {"MAPT": 16, "MAPO": 1, "MAP5": 1}.get(cid, 0))
    for offset, bit in zip((0, 1, width, width + 1), (8, 1, 4, 2), strict=True):
        put("MAPH", tile + offset, case[7] + bool(case[8] & bit))
    if case[0].startswith("road-"):
        start = case[0] in ("road-start", "road-side")
        put("MAP8", tile, 4032)  # Road type 0, no tram; MAPE[3:6] is roadside.
        put("MAPE", tile, 0 if start else 48)
        put("MAP7", tile, 0 if start else 14)
    water = [sea] if sea is not None else []
    if case[0] in ("road-partial", "road-remove"):
        water += [tile - 1] if case[0] == "road-partial" else [tile - 1, tile + 1]
    for t in water:
        for cid, value in (("MAPT", 96), ("MAPO", 17), ("MAP5", 0), ("M3LO", 0)):
            put(cid, t, value)
    fixture.write_bytes(data)
    receipt["sha256"] = hashlib.sha256(data).hexdigest()
    result["ground_input"] = receipt
    return dict(scenario, save=str(fixture), ground_tile=tile)


def check(scenario, run, mode, role, result):
    if scenario.get("roads") == "flooding":
        required = "flood_success flood_fail vehicle_crash wakeup".split()
        result[f"{mode}_{role}_flood"] = branch_witnesses(
            scenario, run, role, "landscape", required
        )
        return
    if "ground" not in scenario:
        return
    case = scenario["ground"]
    counts = branch_witnesses(
        scenario, run, role, "landscape", REQUIRED.get(case[0], case[11]).split()
    )
    chunks = read_save(run["snapshots"][-1])
    size = 2 if case[9] == "MAP2" else 1
    at = scenario["ground_tile"] * size
    actual = int.from_bytes(chunks[case[9]]["raw"][at : at + size], "big")
    if actual != case[10]:
        raise RuntimeError(f"{case[0]} tile endpoint {actual} != {case[10]}")
    result[f"{mode}_{role}_ground"] = {"endpoint": actual, "branches": counts}
    if case[0] in REMOTE:
        vehicles = rows(run["snapshots"][-1])
        heads = tuple(v for v, _, _ in REMOTE[case[0]][1])
        for head in heads:
            while head is not None:
                row = vehicles[head]
                prefix = next(
                    k.removesuffix("vehstatus") for k in row if k.endswith("/vehstatus")
                )
                if not row[prefix + "vehstatus"] & 128:
                    raise RuntimeError("flood left a live consist member")
                head = row[prefix + "next"] - 1 if row[prefix + "next"] else None
        effects = effect_rows(run["snapshots"][-1])
        if sum(r["subtype"] == 5 for r in effects.values()) < len(heads):
            raise RuntimeError("flood victim explosions missing")


def scheduler_gap():
    """Compare unreachable map phases and commit-after-batch with unchanged C++."""
    import migration

    root = migration.ROOT
    out = migration.LOCAL / "landscape-loop-gap"
    out.mkdir(parents=True, exist_ok=True)
    source = (migration.REFERENCE / "src/landscape.cpp").read_text()
    start = source.index("void RunTileLoop()")
    end = source.index("\nvoid InitializeLandscape()", start)
    (out / "landscape-loop-reference.inc").write_text(source[start:end])
    binary = out / "landscape-loop-gap"
    subprocess.run(
        [
            "c++",
            "-std=c++20",
            "-O2",
            "-I" + str(root / "src"),
            "-I" + str(out),
            str(root / "tools/simulation/landscape-loop-reference.cpp"),
            str(migration.rust_archive(root / "build-rust")),
            "-ldl",
            "-lpthread",
            "-lm",
            "-o",
            str(binary),
        ],
        cwd=root,
        env=migration.environment(),
        check=True,
    )
    subprocess.run([str(binary)], cwd=root, check=True)
