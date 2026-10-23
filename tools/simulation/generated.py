"""generated scenario evidence."""

from . import core, objects
from .console_adapter import prepared_console
from .core import SNAPSHOT_TICKS, TICKS_PER_DAY


def scenarios(soak):
    scenarios = [
        {
            "name": "regression-regression",
            "kind": "regression",
            "test": "regression",
            "ticks": 30000,
        },
        {
            "name": "regression-stationlist",
            "kind": "regression",
            "test": "stationlist",
            "ticks": 30000,
        },
    ]
    seeds = (1, 12345, 777, 31337, 2024, 99) if soak else (1, 12345)
    sizes = (6, 7, 8, 9) if soak else (7, 8)
    years = 6 if soak else 2
    for generator, label in ((1, "tgp"), (0, "original")):
        for size in sizes:
            for seed in seeds:
                scenarios.append(
                    {
                        "name": f"generate-{label}-{1 << size}-{seed}",
                        "kind": "generate",
                        "seed": seed,
                        "map_log2": size,
                        "land_generator": generator,
                        "ticks": years * 365 * TICKS_PER_DAY + SNAPSHOT_TICKS,
                    }
                )
    # TGP ownership evidence: curated combinations avoid an expensive Cartesian
    # product while exercising every climate, border mode, roughness and terrain.
    generation_cases = [
        (6, 7, 1, 0, 0, 1, 0, 1, 1, 15),
        (7, 6, 12345, 1, 4, 4, 1, 2, 15, 30),
        (7, 8, 777, 2, 3, 7, 2, 3, 16, 20),
        (8, 7, 31337, 3, 5, 10, 3, 4, 5, 0),
    ]
    if soak:
        generation_cases += [
            (
                6 + i % 3,
                7 + i % 3,
                seed,
                i % 4,
                i % 6,
                i % 11,
                (i + 1) % 4,
                i % 5,
                border,
                limit,
            )
            for i, (seed, border, limit) in enumerate(
                zip(
                    (0, 99, 2024, 0xFFFFFFFE, 1, 12345, 777, 31337),
                    (2, 4, 8, 0, 15, 16, 3, 10),
                    (0, 15, 30, 40, 32, 64, 128, 255),
                    strict=True,
                )
            )
        ]
        generation_cases += [
            (10, 9, 0xFFFFFFFE, 0, 5, 1, 0, 4, 16, 255),
            (9, 10, 2024, 1, 5, 10, 3, 0, 15, 0),
            (10, 10, 99, 2, 4, 2, 2, 3, 0, 255),
            (9, 9, 1, 3, 5, 6, 1, 2, 16, 15),
            (6, 10, 777, 0, 5, 10, 2, 4, 8, 0),
            (10, 6, 12345, 3, 5, 10, 0, 0, 16, 255),
        ]
    for index, (
        mx,
        my,
        seed,
        climate,
        terrain,
        variety,
        smooth,
        sea,
        border,
        limit,
    ) in enumerate(generation_cases):
        scenarios.append(
            {
                "name": f"generate-tgp-settings-{index}-{1 << mx}x{1 << my}-{seed}",
                "kind": "generate",
                "seed": seed,
                "map_log2": mx,
                "map_log2_y": my,
                "land_generator": 1,
                "generation_owner": "snapshot",
                "ticks": 2 * SNAPSHOT_TICKS + TICKS_PER_DAY,
                "settings": {
                    "difficulty": {"terrain_type": terrain, "quantity_sea_lakes": sea},
                    "game_creation": {
                        "landscape": ("temperate", "arctic", "tropic", "toyland")[
                            climate
                        ],
                        "snow_coverage": 0,
                        "desert_coverage": 100,
                        "min_river_length": 6,
                        "variety": variety,
                        "tgen_smoothness": smooth,
                        "water_borders": border,
                        "custom_terrain_type": 1
                        if index == 16
                        else (2, 255)[index % 2],
                        "custom_sea_level": (1, 90)[index % 2],
                    },
                    "construction": {
                        "map_height_limit": limit,
                        "freeform_edges": "true" if index % 3 else "false",
                    },
                },
            }
        )
    for climate, seed in enumerate((1, 12345, 777, 31337)):
        scenarios.append(
            dict(
                name=f"generate-original-settings-{climate}",
                kind="generate",
                seed=seed,
                map_log2=7,
                map_log2_y=8 if climate == 2 else 6,
                land_generator=0,
                generation_owner="snapshot",
                ticks=2 * SNAPSHOT_TICKS + TICKS_PER_DAY,
                settings={
                    "game_creation": {
                        "landscape": ("temperate", "arctic", "tropic", "toyland")[
                            climate
                        ],
                        "snow_coverage": 100,
                        "desert_coverage": 0,
                        "amount_of_rivers": 0 if climate == 0 else 3,
                        "min_river_length": 6,
                    },
                    "construction": {
                        "freeform_edges": "false" if climate % 2 else "true"
                    },
                    "difficulty": {
                        "terrain_type": climate,
                        "quantity_sea_lakes": climate,
                    },
                },
            )
        )
    scenarios.append(
        dict(
            name="generate-river-search",
            kind="generate",
            seed=2,
            map_log2=8,
            land_generator=1,
            generation_owner="snapshot",
            ticks=2 * SNAPSHOT_TICKS + TICKS_PER_DAY,
            settings={
                "game_creation": {
                    "landscape": "temperate",
                    "amount_of_rivers": 4,
                    "min_river_length": 4,
                    "tgen_smoothness": 1,
                },
                "difficulty": {"terrain_type": 3, "quantity_sea_lakes": 0},
                "construction": {"map_height_limit": 40},
                "pf": {"yapf.max_search_nodes": 500},
            },
        )
    )
    return scenarios


def game_environment(scenario):
    return (
        core.WITNESS_ENVIRONMENT
        if scenario.get("generation_owner") and scenario.get("witnesses")
        else {}
    )


def prepare(scenario, binaries, builds, out, timeout, env, result):
    if not scenario.get("generation_owner"):
        return scenario
    executables = prepared_console(
        "objects",
        objects.adapter,
        binaries,
        builds,
        out / scenario["name"] / "input",
        env,
    )
    return dict(
        scenario,
        executables=executables,
        console=["generation_scenario " + scenario["generation_owner"]],
    )


def check(scenario, run, mode, role, result):
    if not scenario.get("generation_owner"):
        return
    lines = [line for line in run["log"] if line.startswith("GENERATION ")]
    if not any(line.startswith("GENERATION map early ") for line in lines):
        raise RuntimeError("missing early map/RNG snapshot")
    required = ["initialize", "fix_slopes", "watering"]
    required += ["tgp" if scenario["land_generator"] else "original"]
    climate = scenario["settings"]["game_creation"]["landscape"]
    if climate == "arctic":
        required += ["snow"]
    if climate == "tropic":
        required += ["coverage_edge", "rainforest"]
    if not scenario["land_generator"]:
        required += ["template_ne", "template_se", "template_sw", "template_nw"]
    branches = {
        "generate-original-settings-1": "sea_flatten sea_edge long_attempt short_attempt flow_recurse yapf_open_update",
        "generate-original-settings-2": "wetland sea_cutoff",
        "generate-tgp-settings-2-128x256-777": "desert",
        "generate-tgp-settings-3-256x128-31337": "widen widen_terraform widen_slope",
        "generate-river-search": "lake wetland wetland_trees yapf_found yapf_limit widen",
    }
    required += branches.get(scenario["name"], "").split()
    counts = core.branch_witnesses(scenario, run, role, "generation", required)
    if counts is not None:
        result.setdefault("generation_witnesses", {})[mode] = counts


def game_args(scenario):
    return ["-d", "yapf=3"] if scenario["name"] == "generate-river-search" else []
