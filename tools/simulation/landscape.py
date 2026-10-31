"""Ground/flood scenario witnesses and unreachable scheduler comparisons."""

import struct
import subprocess
import zlib
from pathlib import Path

from . import core, objects
from .console_adapter import prepared_console
from .core import ROOT, WITNESS_ENVIRONMENT, branch_witnesses, patch, read_save, rows
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
    return (
        [
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
        + [
            dict(
                name="landscape-commands-" + mode,
                kind="save",
                landscape_commands=mode,
                save=str(ROOT / "migration/saves/padhattan-ridge-2000.sav"),
                ticks=1,
                short_checkpoint=True,
            )
            for mode in ("control", "reload")
        ]
        + [
            dict(
                name="landscape-generic-callbacks",
                kind="save",
                generic_callbacks=True,
                save=str(ROOT / "migration/saves/padhattan-ridge-2000.sav"),
                short_checkpoint=True,
                ticks=1,
            )
        ]
        + [
            dict(
                name="generate-heightmap-and-cancellation",
                kind="generate",
                seed=1,
                map_log2=6,
                map_log2_y=7,
                land_generator=1,
                generation_probe=True,
                ticks=1,
                short_checkpoint=True,
            )
        ]
    )


def game_environment(scenario):
    selected = (
        scenario.get("generation_probe")
        or "ground" in scenario
        or "landscape_commands" in scenario
        or scenario.get("roads") == "flooding"
    )
    return WITNESS_ENVIRONMENT if selected and scenario.get("witnesses") else {}


def prepare(scenario, binaries, builds, out, timeout, env, result):
    if scenario.get("generic_callbacks"):
        folder = out / scenario["name"] / "input"
        source, record = core.normalize(Path(scenario["save"]), folder / "normalized")
        result["generic_input"] = record
        scenario = dict(scenario, save=str(source))
        executables = prepared_console(
            "objects", objects.adapter, binaries, builds, folder, env
        )
        return dict(
            scenario,
            executables=executables,
            console=["generic_scenario probe", "unpause"],
        )
    if scenario.get("generation_probe"):
        executables = prepared_console(
            "objects",
            objects.adapter,
            binaries,
            builds,
            out / scenario["name"] / "input",
            env,
        )
        return dict(
            scenario, executables=executables, console=["generation_scenario probe"]
        )

    if "landscape_commands" in scenario:
        folder = out / scenario["name"] / "input"
        source, record = core.normalize(Path(scenario["save"]), folder / "normalized")
        result["landscape_input"] = record
        scenario = dict(scenario, save=str(source))
        executables = prepared_console(
            "objects", objects.adapter, binaries, builds, folder, env
        )
        if scenario["landscape_commands"] == "control":
            return dict(
                scenario,
                executables=executables,
                console=["landscape_scenario control", "unpause"],
            )
        saves = {
            role: folder / role / "landscape.sav" for role in ("reference", "candidate")
        }
        runs = core.capture_pair(
            lambda role: dict(
                scenario,
                console=[f'landscape_scenario control "{saves[role]}"', "unpause"],
            ),
            executables,
            builds,
            folder,
            timeout,
            env,
        )
        for role, run in runs.items():
            save = saves[role]
            if (
                run["exit"]
                or not save.is_file()
                or not any(line.startswith("LANDSCAPE final ") for line in run["log"])
            ):
                raise RuntimeError("landscape reload setup failed")
        if core.compare_saves(
            saves["reference"],
            saves["candidate"],
            scenario["name"],
            10,
            {"chunks": 0, "elements": 0, "masked": {}},
        ):
            raise RuntimeError("landscape reload setup differs")
        return dict(
            scenario,
            executables=executables,
            role_inputs={r: {"save": str(s)} for r, s in saves.items()},
            console=["landscape_scenario inspect", "unpause"],
        )
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
    patch(source, fixture, changes)
    data = bytearray(fixture.read_bytes())

    def put(cid, t, value):
        size = 2 if cid in ("MAP2", "MAP8") else 1
        position = chunks[cid]["span"][0] + 8 + size * t
        data[position : position + size] = value.to_bytes(size, "big")

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
    return dict(scenario, save=str(fixture), ground_tile=tile)


def check(scenario, run, mode, role, result):
    if scenario.get("generic_callbacks"):
        lines = [line for line in run["log"] if line.startswith("GENERIC ")]
        required = (
            "fallback-duplicates-registers priority-v7-v8 "
            "industry-sentinels-narrowing unavailable-all-fail-reset programs-complete ambient-unavailable-all-fail tree-ambient-reread"
        ).split()
        if any(lines.count(f"GENERIC {name} true") != 2 for name in required):
            raise RuntimeError("missing generic registry/AI real-group witnesses")
        if "GENERIC reinstalled-after-native-reload true" not in lines:
            raise RuntimeError("missing generic transient reload reattachment")
        expected = {
            (str(t), str(v), str(m))
            for t in (0, 4, 6)
            for v in (7, 8)
            for m in range(4)
        }
        ambient = [
            line.split() for line in lines if line.startswith("GENERIC ambient ")
        ]
        if len(ambient) != 48 or {tuple(p[2:5]) for p in ambient} != expected:
            raise RuntimeError("missing direct ambient tile/gate/version witnesses")
        if any(
            p[5] != str(int(p[4] == "3")) or p[7] != str(p[4] == "3").lower()
            for p in ambient
        ):
            raise RuntimeError("ambient playback/file outcome missing")
        return
    if scenario.get("generation_probe"):
        lines = [line for line in run["log"] if line.startswith("GENERATION ")]
        expected = [
            f"GENERATION cancel {i} true {i + 1} {int(i != 0)}" for i in range(3)
        ]
        if (
            any(line not in lines for line in expected)
            or "GENERATION missing false false" not in lines
        ):
            raise RuntimeError("missing cancellation/heightmap failure evidence")
        if (
            sum(
                line.startswith("GENERATION heightmap ") and line.endswith("true")
                for line in lines
            )
            != 8
        ):
            raise RuntimeError("missing heightmap rotation/aspect/edge cases")
        successful = [
            line.split() for line in lines if line.startswith("GENERATION success ")
        ]
        if (
            len(successful) != 4
            or {line[2] for line in successful} != {"0", "1", "2", "3"}
            or any(
                line[3] != "true"
                or int(line[4]) != (15 if line[2] == "2" else 3)
                or int(line[5]) < 5
                for line in successful
            )
            or lines.count("GENERATION progress set 1 3") != 3
            or lines.count("GENERATION progress set 1 15") != 1
            or sum(line.startswith("GENERATION progress set 2 ") for line in lines) != 4
        ):
            raise RuntimeError(
                "missing successful heightmap landscape/climate/river progress"
            )
        check_heightmap_inputs(lines)
        # The two outer missing-file loads also report their decoded errors.
        expected_errors = 2 + sum(
            int(size is None)
            + int(size is None or name in ("truncated.bmp", "mono-odd.bmp"))
            for name, (_, size) in heightmap_inputs().items()
        )
        if (
            sum(line.startswith("GENERATION error ") for line in lines)
            != expected_errors
        ):
            raise RuntimeError("missing heightmap error messages")
        if any(
            len(line.split()) != 10
            for line in lines
            if line.startswith("GENERATION map ")
        ):
            raise RuntimeError("missing map or simulation/interactive RNG observations")
        if sum(line.startswith("GENERATION map ") for line in lines) != 42 + len(
            heightmap_inputs()
        ):
            raise RuntimeError("missing early/heightmap/flat map and RNG snapshots")
        counts = branch_witnesses(
            scenario,
            run,
            role,
            "generation",
            [
                "heightmap_ccw",
                "heightmap_cw",
                "fix_slopes",
                "watering",
                "flat",
                "load_failure",
                "tgp_abort",
                "snow",
                "coverage_edge",
                "rainforest",
                "long_attempt",
                "short_attempt",
            ],
        )
        if counts is not None:
            # One initial world + eight direct loads + two slope passes for
            # each complete heightmap world + FlatEmptyWorld. In particular,
            # a no-op second slope pass must not hide an early continuation exit.
            if counts["fix_slopes"] < 18 or counts["watering"] < 11:
                raise RuntimeError("missing downstream heightmap slope/watering passes")
            result.setdefault("generation_witnesses", {})[mode] = counts
        return

    if "landscape_commands" in scenario:
        lines = [line for line in run["log"] if line.startswith("LANDSCAPE ")]
        if len(lines) < (35 if scenario["landscape_commands"] == "control" else 1):
            raise RuntimeError("missing landscape command output")
        if scenario["landscape_commands"] == "control":
            costs = {
                parts[1]: parts for line in lines if len(parts := line.split()) == 7
            }
            names = "raise lower recursive level-test level-exec terraform-limit level-limit clear-test clear-exec clear-limit clear-cash level-cash object-two-pass partial-clear object-remove editor edge void too-high bridge-build bridge-damage bridge-remove tunnel-build tunnel-damage tunnel-remove occupied-clear occupied-limit final".split()
            if (
                set(names) - {line.split()[1] for line in lines}
                or "LANDSCAPE editor-restored true" not in lines
            ):
                raise RuntimeError("landscape command/restoration evidence missing")
            if any(int(costs[name][5]) <= 0 for name in ("clear-cash", "level-cash")):
                raise RuntimeError("landscape insufficient-cash branch missing")
        required = (
            ("recursive", "two_pass", "indirect", "partial")
            if scenario["landscape_commands"] == "control"
            else ()
        )
        branch_witnesses(scenario, run, role, "landscape_cmd", required)
        return
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

    # Movement gaps: impossible foundation outcomes and off-map signed extrema.
    # Every algorithm/helper body below is extracted from the unchanged reference.
    def function(text, name):
        import re

        import probe

        match = re.search(
            r"^(?:static )?(?:inline )?[\w:<>, \t*&]+\s+" + name + r"\([^;{}]*\)\s*\{",
            text,
            re.M,
        )
        return probe.body(text, match[0]) + "\n"

    fragments = (
        (migration.REFERENCE / "src/slope_func.h")
        .read_text()
        .replace('#include "tile_type.h"', "")
    )
    track = (migration.REFERENCE / "src/track_func.h").read_text()
    fragments += function(track, "TracksOverlap") + function(track, "CornerToTrackBits")
    data = (migration.REFERENCE / "src/table/track_data.h").read_text()
    begin = data.index("extern const TrackBits _corner_to_trackbits")
    fragments += data[begin : data.index("};", begin) + 2] + "\n"
    for file, table, name in (
        (
            "rail_cmd.cpp",
            "static const TrackBits _valid_tracks_without",
            "GetRailFoundation",
        ),
        (
            "road_cmd.cpp",
            "static const RoadBits _invalid_tileh_slopes_road",
            "GetRoadFoundation",
        ),
    ):
        text = (migration.REFERENCE / "src" / file).read_text()
        begin = text.index(table)
        end = text.index("/**", text.index("};", begin))
        fragments += text[begin:end]
        if file == "rail_cmd.cpp":
            begin = text.index(
                "static const TrackBits _valid_tracks_on_leveled_foundation"
            )
            fragments += text[begin : text.index("};", begin) + 2] + "\n"
        fragments += function(text, name)
    fragments += function(source, "GetPartialPixelZ")
    (out / "movement-foundation-reference.inc").write_text(fragments)
    void = (migration.REFERENCE / "src/void_cmd.cpp").read_text()
    terrain = (migration.REFERENCE / "src/tile_map.cpp").read_text()
    (out / "movement-height-reference.inc").write_text(
        function(terrain, "GetTileSlopeGivenHeight")
        + function(terrain, "GetTilePixelSlopeOutsideMap")
    )
    (out / "movement-coordinate-reference.inc").write_text(
        function(void, "GetSlopePixelZ_Void")
        + function(source, "GetSlopePixelZOutsideMap")
    )
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


def png_chunk(kind: bytes, data: bytes) -> bytes:
    body = kind + data
    return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))


def png_file(
    width: int,
    height: int,
    depth: int,
    colour: int,
    samples: list[list[int]],
    palette: list[tuple[int, int, int]] | None = None,
) -> bytes:
    """A filter-0 PNG; each row lists its samples, packed to `depth` bits."""
    rows = b""
    for row in samples:
        if depth == 16:
            packed = b"".join(struct.pack(">H", v) for v in row)
        elif depth == 8:
            packed = bytes(row)
        else:
            bits = "".join(format(v, f"0{depth}b") for v in row)
            bits += "0" * (-len(bits) % 8)
            packed = int(bits, 2).to_bytes(len(bits) // 8, "big")
        rows += b"\0" + packed
    header = struct.pack(">IIBBBBB", width, height, depth, colour, 0, 0, 0)
    chunks = png_chunk(b"IHDR", header)
    if palette is not None:
        chunks += png_chunk(b"PLTE", b"".join(bytes(c) for c in palette))
    chunks += png_chunk(b"IDAT", zlib.compress(rows)) + png_chunk(b"IEND", b"")
    return b"\x89PNG\r\n\x1a\n" + chunks


def bmp_file(
    width: int,
    height: int,
    bpp: int,
    indices: list[list[int]],
    palette: list[tuple[int, int, int]],
    truncate: int = 0,
) -> bytes:
    """A bottom-up uncompressed BMP with a BGRA palette."""
    pixels = b""
    for row in reversed(indices):
        if bpp == 24:
            line = b"".join(bytes((v, (v * 3) % 256, (v * 7) % 256)) for v in row)
        else:
            bits = "".join(format(v, f"0{bpp}b") for v in row)
            bits += "0" * (-len(bits) % 8)
            line = int(bits, 2).to_bytes(len(bits) // 8, "big")
        pixels += line + bytes(-len(line) % 4)
    table = b"".join(bytes((b, g, r, 0)) for r, g, b in palette)
    offset = 54 + len(table)
    header = struct.pack("<2sIHHI", b"BM", offset + len(pixels), 0, 0, offset)
    header += struct.pack(
        "<IiiHHIIiiII", 40, width, height, 1, bpp, 0, len(pixels), 0, 0, len(palette), 0
    )
    data = header + table + pixels
    return data[: len(data) - truncate] if truncate else data


def heightmap_inputs() -> dict[str, tuple[bytes | None, tuple[int, int] | None]]:
    """Name, file contents (None: missing) and expected dimensions (None: error)."""
    w, h = 9, 5

    def grid(limit: int) -> list[list[int]]:
        return [[(x * 7 + y * 13) % limit for x in range(w)] for y in range(h)]

    grey16 = [(17 * i, 17 * i, 17 * i) for i in range(16)]
    mono = [(0, 0, 0), (255, 255, 255)]
    colour16 = [(255 - 16 * i, 40 + 8 * i, (90 * i) % 256) for i in range(16)]
    colour256 = [(i, (i * 5) % 256, (255 - i)) for i in range(256)]
    return {
        "grey8.png": (png_file(w, h, 8, 0, grid(256)), (w, h)),
        "grey16.png": (
            png_file(w, h, 16, 0, [[v * 257 for v in r] for r in grid(256)]),
            (w, h),
        ),
        "grey1.png": (png_file(w, h, 1, 0, grid(2)), (w, h)),
        "grey-alpha.png": (
            png_file(
                w, h, 8, 4, [sum(([v, 255 - v] for v in r), []) for r in grid(256)]
            ),
            (w, h),
        ),
        "rgb.png": (
            png_file(
                w,
                h,
                8,
                2,
                [sum(([v, 255 - v, v // 2] for v in r), []) for r in grid(256)],
            ),
            (w, h),
        ),
        "rgba16.png": (
            png_file(
                w,
                h,
                16,
                6,
                [
                    sum(([v * 257, 0, 65535 - v * 257, 4096] for v in r), [])
                    for r in grid(256)
                ],
            ),
            (w, h),
        ),
        "palette-grey16.png": (png_file(w, h, 4, 3, grid(16), grey16), (w, h)),
        "palette-colour16.png": (png_file(w, h, 4, 3, grid(16), colour16), (w, h)),
        "palette256.png": (png_file(w, h, 8, 3, grid(256), colour256), (w, h)),
        "palette2.png": (
            png_file(w, h, 1, 3, grid(2), [(10, 20, 30), (200, 210, 220)]),
            (w, h),
        ),
        "corrupt.png": (b"\x89PNG\r\n\x1a\n" + b"\0" * 24, None),
        "wide.png": (png_file(8193, 1, 1, 0, [[x % 2 for x in range(8193)]]), None),
        "missing.png": (None, None),
        "mono.bmp": (bmp_file(8, h, 1, [r[:8] for r in grid(2)], mono), (8, h)),
        # BmpRead1 pads 1-bit lines by 4 - width / 8, so this reads past the end.
        "mono-odd.bmp": (bmp_file(w, h, 1, grid(2), mono), (w, h)),
        "colour16.bmp": (bmp_file(w, h, 4, grid(16), colour16), (w, h)),
        "grey16.bmp": (bmp_file(w, h, 4, grid(16), grey16), (w, h)),
        "colour256.bmp": (bmp_file(w, h, 8, grid(256), colour256), (w, h)),
        "rgb.bmp": (bmp_file(w, h, 24, grid(256), []), (w, h)),
        "header.bmp": (b"BM" + b"\0" * 20, None),
        "wide.bmp": (bmp_file(8193, 1, 1, [[0] * 8193], [(0, 0, 0), (9, 9, 9)]), None),
        # The header is valid, so only the load (not the dimensions) fails.
        "truncated.bmp": (bmp_file(w, h, 8, grid(256), colour256, truncate=20), (w, h)),
        "missing.bmp": (None, None),
    }


def check_heightmap_inputs(lines: list[str]) -> None:
    """Every input reports dimensions, load result and one error per failed read."""
    rows = {
        parts[2]: parts[3:]
        for line in lines
        if line.startswith("GENERATION input ") and len(parts := line.split()) == 9
    }
    for name, (_, size) in heightmap_inputs().items():
        loads = size is not None and name not in ("truncated.bmp", "mono-odd.bmp")
        width, height = size if size else (4294967295, 4294967295)
        expected = [str(size is not None).lower(), str(width), str(height)]
        expected += [str(int(size is None)), str(loads).lower(), str(int(not loads))]
        if rows.get(name) != expected:
            raise RuntimeError(
                f"heightmap input {name}: {rows.get(name)} != {expected}"
            )


def install(scenario, run_dir):
    if not scenario.get("generation_probe"):
        return
    for name, width, height in (("wide", 17, 9), ("tall", 9, 17)):
        pixels = b"".join(
            b"".join(
                bytes([(0, 1, 254, 255)[(x + 2 * y) % 4]]) * 3 for x in range(width)
            )
            + bytes((-width * 3) % 4)
            for y in range(height)
        )
        header = struct.pack("<2sIHHI", b"BM", 54 + len(pixels), 0, 0, 54)
        header += struct.pack(
            "<IiiHHIIiiII", 40, width, height, 1, 24, 0, len(pixels), 0, 0, 0, 0
        )
        (run_dir / f"{name}.bmp").write_bytes(header + pixels)
    names = []
    for name, (data, _) in heightmap_inputs().items():
        names.append(name)
        if data is not None:
            (run_dir / "heightmap-inputs").mkdir(exist_ok=True)
            (run_dir / "heightmap-inputs" / name).write_bytes(data)
    (run_dir / "heightmap-inputs.txt").write_text("\n".join(names) + "\n")
