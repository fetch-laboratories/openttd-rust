"""trees scenario evidence."""

import hashlib
import re
import shutil
import struct
from collections import Counter

from .core import (
    SNAPSHOT_TICKS,
    TICKS_PER_DAY,
    Reader,
    decode_element,
    read_object,
    read_save,
    run_game,
    strip_glog,
)


def tree_coverage(path):
    """Observed tile states, not an inference from requested settings."""
    chunks = read_save(path)
    states = Counter()
    for tile, kind in enumerate(chunks["MAPT"]["raw"]):
        if kind >> 4 == 4:
            ground = struct.unpack_from(">H", chunks["MAP2"]["raw"], 2 * tile)[0]
            growth = chunks["MAP5"]["raw"][tile]
            states[
                f"{kind & 3}/{ground >> 6 & 7}/{ground >> 4 & 3}/{growth >> 6}/{growth & 7}"
            ] += 1
    date = chunks["DATE"]
    pats = chunks["PATS"]
    settings = decode_element(pats, pats["elements"][0][1])
    return {
        "settings": {
            key: settings[key]
            for key in (
                "game_creation.landscape",
                "game_creation.tree_placer",
                "construction.extra_tree_placement",
            )
        },
        "zone/ground/density/count-minus-one/growth": dict(sorted(states.items())),
        "counter": decode_element(date, date["elements"][0][1])["trees_tick_counter"],
    }


def prepare_trees(scenario, binaries, builds, out, timeout, env):
    setup = {k: v for k, v in scenario.items() if k not in ("trees", "console")}
    run = run_game(
        setup,
        binaries["reference"],
        builds["reference"],
        out / scenario["name"] / "prepare",
        timeout,
        env,
    )
    if run["exit"] != 0 or not run["snapshots"]:
        raise RuntimeError("tree input preparation failed")
    fixture = run["snapshots"][-1]
    chunks = read_save(fixture)
    source = fixture.read_bytes()
    data, patches = bytearray(source), []

    def patch(cid, relative, value, label):
        position = chunks[cid]["span"][0] + relative
        patches.append(
            [
                cid,
                relative,
                data[position : position + len(value)].hex(),
                value.hex(),
                label,
            ]
        )
        data[position : position + len(value)] = value

    def tile_field(cid, tile, value):
        # RIFF chunks have an eight-byte header; MAP2/MAP8 are big-endian u16.
        width = 2 if cid in ("MAP2", "MAP8") else 1
        patch(cid, 8 + width * tile, value.to_bytes(width, "big"), f"tile[{tile}]")

    types = chunks["MAPT"]["raw"]
    width = struct.unpack(">II", chunks["MAPS"]["elements"][0][1])[0]
    climate = scenario["settings"]["game_creation"]["landscape"]
    if scenario["trees"] == "growth":
        trees = [i for i, kind in enumerate(types) if kind >> 4 == 4]
        if len(trees) < 32:
            raise RuntimeError("too few tree tiles for growth/count witnesses")
        for witness, tile in enumerate(trees[:32]):
            ground = (
                (0, 1, 2, 4)[(witness + witness // 8) % 4]
                if scenario["settings"]["game_creation"]["landscape"] == "arctic"
                else witness % 2
            )
            tile_field("MAP2", tile, ground << 6 | (witness % 4) << 4)
            tile_field("MAPO", tile, chunks["MAPO"]["raw"][tile] & 0x9F | 0x60)
            tile_field("MAP5", tile, (witness // 8) << 6 | witness % 8)
        # Force death on each land ground and both single/multiple-tree shores.
        grounds = (0, 1, 2, 4) if climate == "arctic" else (0, 1)
        for ground, tile in zip(grounds, trees[32:], strict=False):
            tile_field("MAP2", tile, ground << 6 | 0x30)
            tile_field("MAPO", tile, chunks["MAPO"]["raw"][tile] & 0x9F | 0x60)
            tile_field("MAP5", tile, 6)
        shores = [
            i
            for i in trees[40:]
            if struct.unpack_from(">H", chunks["MAP2"]["raw"], 2 * i)[0] >> 6 & 7 == 3
        ]
        if len(shores) < 2:
            raise RuntimeError("missing natural tree shores")
        for count, tile in enumerate(shores[:2]):
            tile_field("MAP5", tile, count << 6 | 6)
        if climate == "tropic":
            desert = next(i for i in trees if types[i] & 3 == 1)
            tile_field("M3LO", desert, 28)  # non-cactus desert tree must die
            tile_field("MAP5", desert, 3)
        # Natural shore tiles retain their water class and real coastal slopes.
        # Freshly cleared neighboring grass provides spread rejection witnesses.
        fresh = [
            i
            for i, kind in enumerate(types)
            if kind >> 4 == 0
            and chunks["MAP5"]["raw"][i] & 0x1C == 0
            and any(
                i + offset in trees[:32]
                for offset in (
                    -width - 1,
                    -width,
                    -width + 1,
                    -1,
                    1,
                    width - 1,
                    width,
                    width + 1,
                )
            )
        ][:3]
        if len(fresh) != 3:
            raise RuntimeError("missing fresh-grass neighbors")
        for density, tile in enumerate(fresh):
            tile_field("MAP5", tile, density)
            tile_field("M3LO", tile, chunks["M3LO"]["raw"][tile] & 0xEF)
        # Flat forest next to active sea is cleared by water's nested command.
        sea = [
            i
            for i, kind in enumerate(types)
            if kind >> 4 == 6 and chunks["MAP5"]["raw"][i] == 0
        ]
        flood = next(
            i
            for i in sea
            if 1 < i % width < width - 2
            and all(i + offset in sea for offset in (-width, -1, 1, width))
        )
        for tile in (flood - width, flood - 1, flood + 1, flood + width):
            tile_field("M3LO", tile, chunks["M3LO"]["raw"][tile] & 0xFE)
        for cid, value in (
            ("MAPT", 0x40),
            ("MAPO", 0x70),
            ("MAP2", 0x30),
            (
                "M3LO",
                {"temperate": 0, "arctic": 12, "tropic": 28, "toyland": 32}[climate],
            ),
            ("MAP5", 3),
        ):
            tile_field(cid, flood, value)
        scenario = dict(scenario, tree_flood=flood)
    else:
        city = chunks["CITY"]
        towns = [decode_element(city, body)["xy"] for _, body in city["elements"]]
        anchor = next(
            i
            for i in range(width + 1, len(types) - 3 * width)
            if i % width < width - 3
            and any(
                abs(i % width - t % width) + abs(i // width - t // width) < 12
                for t in towns
            )
            and all(
                types[i + y * width + x] >> 4 in (0, 4)
                and not types[i + y * width + x] & 12
                for y in range(3)
                for x in range(3)
            )
        )
        for y in range(3):
            for x in range(3):
                tile = anchor + y * width + x
                for cid in ("M3LO", "M3HI", "MAPE", "MAP7", "MAP8", "MAP2"):
                    tile_field(cid, tile, 0)
                tile_field("MAPT", tile, types[tile] & 3)
                tile_field("MAPO", tile, 16)
                tile_field(
                    "MAP5",
                    tile,
                    (2 if x == 1 and y == 0 else 3 if x == 2 and y == 0 else 0) << 2
                    | 3,
                )
                if x == 2 and y == 0:
                    tile_field("MAP2", tile, 0xFFFF)  # unowned farm field
        species = {
            "temperate": 0,
            "arctic": 12,
            "tropic": 20
            if types[anchor] & 3 == 2
            else 27
            if types[anchor] & 3 == 1
            else 28,
            "toyland": 32,
        }[climate]
        tile_field("MAPT", anchor, 0x40 | types[anchor] & 3)
        tile_field("MAPO", anchor, 0x70)
        tile_field("MAP2", anchor, 0x30)
        tile_field("M3LO", anchor, species)
        tile_field("MAP5", anchor, 0xC3)
        water = next(
            i
            for i, kind in enumerate(types)
            if kind >> 4 == 6 and chunks["MAP5"]["raw"][i] == 0
        )
        scenario = dict(scenario, tree_anchor=anchor, tree_water=water)
    date = chunks["DATE"]
    if date["kind"] != 3 or len(date["elements"]) != 1:
        raise RuntimeError("unexpected DATE counter table")
    reader = Reader(source, date["span"][0] + 5)
    reader.take(reader.gamma() - 1)
    reader.gamma()
    for field in date["header"]:
        if field["key"] == "trees_tick_counter":
            if field["type"] != 2:
                raise RuntimeError("unexpected trees counter schema")
            patch(
                "DATE",
                reader.pos - date["span"][0],
                bytes([scenario["tree_counter"]]),
                "trees_tick_counter",
            )
            break
        read_object(reader, [field], "", {})
    else:
        raise RuntimeError("missing trees counter")
    normalized = fixture.parent / "tree-input.sav"
    strip_glog(data, normalized)
    normalized_data = normalized.read_bytes()
    return dict(scenario, kind="save", save=str(normalized)), {
        "reference": str(fixture),
        "normalized": str(normalized),
        "reference_sha256": hashlib.sha256(source).hexdigest(),
        "normalized_sha256": hashlib.sha256(normalized_data).hexdigest(),
        "removed_chunk": "GLOG",
        "patches": patches,
        "flood_tile": scenario.get("tree_flood"),
        "fresh_grass": fresh if scenario["trees"] == "growth" else [],
        "command_anchor": scenario.get("tree_anchor"),
        "coverage": tree_coverage(normalized),
    }


def scenarios(soak):
    scenarios = []
    climates = ("temperate", "arctic", "tropic", "toyland")
    for climate, label in enumerate(climates):
        for extra in range(4):
            for prepared in (False, True):
                scenarios.append(
                    {
                        "name": f"trees-{label}-{extra}-{'reload' if prepared else 'generate'}",
                        "kind": "generate",
                        "seed": 12345 if soak else 1,
                        "map_log2": 8 if soak else 7,
                        "map_log2_y": 9 if soak else 8,
                        "land_generator": 1,
                        "trees": "growth" if prepared else "generate",
                        "tree_counter": (0, 1, 255)[extra % 3],
                        "ticks": (6 if soak else 2) * SNAPSHOT_TICKS + TICKS_PER_DAY,
                        "settings": {
                            "game_creation": {
                                "landscape": label,
                                "snow_line_height": 4,
                                "tree_placer": 2 if prepared else (climate + extra) % 3,
                            },
                            "construction": {"extra_tree_placement": extra},
                            "sound": {"ambient": "false"},
                        },
                    }
                )
        for burst in (0, 1, 2, 3, 4096) if climate == 0 else (4096,):
            scenarios.append(
                {
                    "name": f"trees-{label}-commands-{burst}",
                    "kind": "generate",
                    "seed": 1,
                    "map_log2": 7,
                    "land_generator": 1,
                    "trees": "commands",
                    "tree_counter": 255,
                    "ticks": 2 * SNAPSHOT_TICKS + TICKS_PER_DAY,
                    "console": ["start_ai MigrationTrees", "unpause"],
                    "settings": {
                        "game_creation": {
                            "landscape": label,
                            "tree_placer": 2,
                            "snow_line_height": 4,
                        },
                        "construction": {
                            "extra_tree_placement": 3,
                            "tree_frame_burst": burst,
                            "tree_per_64k_frames": 0,
                        },
                    },
                }
            )
    return scenarios


def prepare(scenario, binaries, builds, out, timeout, env, result):
    if scenario.get("trees") in ("growth", "commands"):
        scenario, result["tree_input"] = prepare_trees(
            scenario, binaries, builds, out, timeout, env
        )
    return scenario


def check(scenario, run, mode, role, result):
    if "trees" in scenario:
        result[f"{mode}_{role}_trees"] = [
            tree_coverage(path) for path in run["snapshots"]
        ]
        settings = result[f"{mode}_{role}_trees"][-1]["settings"]
        expected = {
            "game_creation.landscape": (
                "temperate",
                "arctic",
                "tropic",
                "toyland",
            ).index(scenario["settings"]["game_creation"]["landscape"]),
            "game_creation.tree_placer": scenario["settings"]["game_creation"][
                "tree_placer"
            ],
            "construction.extra_tree_placement": scenario["settings"]["construction"][
                "extra_tree_placement"
            ],
        }
        if settings != expected:
            raise RuntimeError("tree scenario settings were not applied")
        if (
            "tree_flood" in scenario
            and read_save(run["snapshots"][-1])["MAPT"]["raw"][scenario["tree_flood"]]
            >> 4
            != 6
        ):
            raise RuntimeError("water did not clear the prepared forest tile")
        if scenario["trees"] == "commands":
            rows = [line for line in run["log"] if "TREE-COMMAND " in line]
            result[f"{mode}_{role}_tree_commands"] = rows
            markers = [
                line.rsplit("TREE-COMMAND-END ", 1)[1]
                for line in run["log"]
                if "TREE-COMMAND-END " in line
            ]
            labels = "rectangle add0 add1 add2 add3 full water invalid clear-full clear-added clear-field replant".split()
            order = [
                re.search(r"TREE-COMMAND (\S+) (test|execute) ", line).groups()
                for line in rows
            ]
            expected_order = [
                (label, phase) for label in labels for phase in ("test", "execute")
            ]
            if (
                order != expected_order
                or markers != ["24"]
                or any("TREE-TEST-MUTATED" in line for line in run["log"])
            ):
                raise RuntimeError(
                    f"{mode}: {role} tree commands incomplete or test mutated state"
                )


def install(scenario, run_dir):
    if scenario.get("trees") == "commands":
        ai = run_dir / "ai/tree-scenarios"
        shutil.copytree(scenario["scenario_ai"], ai)
        (ai / "parameters.nut").write_text(
            f"TREE_ANCHOR <- {scenario['tree_anchor']};\nTREE_WATER <- {scenario['tree_water']};\n"
        )


def game_args(scenario):
    return ["-d", "script=2"] if scenario.get("trees") == "commands" else []


PREPARE_ORDER = 2


AI_FOLDER = "tree-scenario-ai"


def uses_ai(scenario):
    return scenario.get("trees") == "commands"


def check_editor_groups(lines: list[str]) -> None:
    """Require full native-result, dirty-point, tile and both-stream observations."""
    observations = [line.split() for line in lines if line.startswith("GROUNDTREE ")]
    widths = {
        "cost": 16,
        "group": 10,
        "clear": 11,
        "neighbor": 11,
        "group-tile": 11,
        "dirty": 11,
        "group-rng": 7,
        "clear-rng": 7,
        "checkpoint": 7,
        "boundary": 5,
        "complete": 4,
    }
    if any(len(row) != widths.get(row[1], 0) for row in observations):
        raise RuntimeError("incomplete clear-ground/editor observation fields")
    costs = [row for row in observations if row[1] == "cost"]
    groups = [line.split() for line in lines if line.startswith("GROUNDTREE group ")]
    if (
        [int(row[2]) for row in costs] != list(range(2880))
        or [int(row[2]) for row in groups] != list(range(56))
        or "GROUNDTREE complete 2880 56" not in lines
        or sum(line.startswith("GROUNDTREE clear ") for line in lines) != 2880
        or sum(line.startswith("GROUNDTREE neighbor ") for line in lines) != 2880
        or sum(line.startswith("GROUNDTREE group-rng ") for line in lines) != 56
        or sum(row[1] == "clear-rng" for row in observations) != 1
        or sum(row[1] == "checkpoint" for row in observations) != 1
        or sum(row[1] == "dirty" for row in observations)
        != 960 + sum(int(row[9]) for row in groups)
    ):
        raise RuntimeError("missing clear-ground/editor group observations")
    random_rows = [row for row in observations if row[1] == "group-rng"]
    # The fixture's edge center is TileXY(1, 1), independently of brush output.
    width = int(random_rows[28][2]) - 1
    group_tiles: list[list[int]] = [[] for _ in groups]
    group = -1
    for row in observations:
        if row[1] == "group":
            group = int(row[2])
        elif row[1] == "group-tile":
            if group < 0:
                raise RuntimeError("editor tile observation precedes its group")
            group_tiles[group].append(int(row[2]))
    for index, row in enumerate(random_rows):
        y, x = divmod(int(row[2]), width)
        # TileArea(center) starts with zero extents. Expand(6) therefore ends
        # just before center + 6; the native constructor does not cover a tile.
        # The interior anchor leaves enough room at the south/east edges.
        expected_tiles = [
            yy * width + xx
            for yy in range(max(0, y - 6), y + 6)
            for xx in range(max(0, x - 6), x + 6)
        ]
        if group_tiles[index] != expected_tiles:
            raise RuntimeError(f"missing editor group tile observations: {index}")
    for row in costs:
        if row[9] != "true" or row[11:] != ["0", "65535", "65535", "255", "true"]:
            raise RuntimeError(f"clear-ground native result metadata differs: {row}")
    for row in groups:
        mode, planted = int(row[5]), int(row[9])
        expected = {0: 0, 1: 2, 2: 0, 3: 4, 4: 0, 5: 0, 8: 0, 9: 0, 10: 0, 12: 0}
        if mode in expected and planted != expected[mode]:
            raise RuntimeError(f"editor group branch not reached: {row}")
        if mode in (6, 7, 11) and planted == 0:
            raise RuntimeError(f"editor group planting missing: {row}")
    boundaries = [
        line.split() for line in lines if line.startswith("GROUNDTREE boundary ")
    ]
    if [int(row[2]) for row in boundaries] != [
        block + mode for block in (0, 14, 28, 42) for mode in (8, 9, 12)
    ] or any(
        row[3:] != (["0", "0"] if int(row[2]) % 14 == 12 else ["2", "0"])
        for row in boundaries
    ):
        raise RuntimeError("missing strict rainforest boundaries/type interval")
