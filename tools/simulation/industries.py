"""Industry production/history and builder evidence on existing reference inputs."""

import shutil
from pathlib import Path

from . import companies, disasters, economy
from .core import SNAPSHOT_TICKS, TICKS_PER_DAY, read_save, run_game

AI_FOLDER = "industry-scenario-ai"
# Calibrated on the pinned original with this one-town, treeless input. The
# first executed prospect enters after 216 shared draws: state (0, 123456789)
# returns FFFFFFFF (unlucky); (8, 123456789) returns 0 and exhausts all sites.
# AITestMode consumes zero draws, execution consumes 1 or 10001. These fixed
# 32-tick checkpoints include subsequent ordinary world draws in both modes.
PROSPECT_FAILURES = {
    "prospect-unlucky": ((407793002, 3937062389), (990010970, 234727308)),
    "prospect-exhausted": ((2882038248, 3636532106), (2420427660, 3018271737)),
}


def uses_ai(scenario):
    return scenario.get("industry") in (
        "lumber",
        "fund",
        "prospect",
        "prospect-unlucky",
        "prospect-exhausted",
        "clear",
        "terraform",
        "water-check",
    )


def install(scenario, run_dir):
    if uses_ai(scenario):
        folder = run_dir / "ai/industry-scenarios"
        shutil.copytree(scenario["scenario_ai"], folder)
        (folder / "parameters.nut").write_text(
            f'INDUSTRY_MODE <- "{scenario["industry"]}";\n'
            f"INDUSTRY_TARGET <- {scenario.get('industry_target', 0)};\n"
            f'INDUSTRY_WATER <- "{scenario.get("industry_water", "none")}";\n'
        )


def game_args(scenario):
    return ["-d", "script=2"] if uses_ai(scenario) else []


def scenarios(soak):
    result = [
        dict(
            scenario,
            name="industry-" + scenario["economy"],
            industry="cargo",
            ticks=scenario["ticks"]
            if scenario["economy"] == "stockpile"
            else (16 if soak else 6) * SNAPSHOT_TICKS,
        )
        for scenario in economy.scenarios(soak)
        if "economy" in scenario
    ]
    for climate in ("temperate", "arctic", "tropic", "toyland", "lumber"):
        result.append(
            {
                "name": "industry-" + climate,
                "kind": "generate",
                "seed": 777 if climate in ("tropic", "lumber") else 12345,
                "map_log2": 8,
                "land_generator": 1,
                "industry": climate,
                "industry_soak": soak,
                "ticks": (6 if soak else 2) * 365 * TICKS_PER_DAY,
                "console": ["setting difficulty.disasters 0"],
                "settings": {
                    "game_creation": {
                        "landscape": "tropic" if climate == "lumber" else climate
                    },
                    "difficulty": {
                        "industry_density": 4,
                        "max_loan": 1000000 if climate == "lumber" else 300000,
                    },
                },
            }
        )
    for mode in ("original", "frozen", "control", "closure", "farm-cleanup", "builder"):
        result.append(
            {
                "name": "industry-" + mode,
                "kind": "generate",
                "seed": 12345,
                "map_log2": 7,
                "land_generator": 1,
                "industry": mode,
                "industry_soak": soak,
                "ticks": (6 if soak else 2) * 365 * TICKS_PER_DAY,
                "settings": {"difficulty": {"industry_density": 4}},
                "console": ["setting difficulty.disasters 0"],
            }
        )
    for name, density in (("fund-only", 0), ("custom-forced", 6)):
        result.append(
            {
                "name": "industry-generation-" + name,
                "kind": "generate",
                "seed": 12345,
                "map_log2": 7,
                "land_generator": 1,
                "industry": "generation",
                "ticks": 2 * SNAPSHOT_TICKS,
                "settings": {
                    "difficulty": {"industry_density": density},
                    "game_creation": {"custom_industry_number": 0},
                },
                "console": ["setting difficulty.disasters 0"],
                "industry_generation_empty": density == 0,
            }
        )
    for mode in (
        "fund",
        "prospect",
        "prospect-unlucky",
        "prospect-exhausted",
        "clear",
        "terraform",
    ):
        result.append(
            {
                "name": "industry-command-" + mode,
                "kind": "generate",
                "seed": 12345,
                "map_log2": 7,
                "land_generator": 1,
                "industry": mode,
                "ticks": (12 if soak else 6) * SNAPSHOT_TICKS,
                "settings": {
                    "difficulty": {
                        "industry_density": 0 if mode in ("fund", "prospect") else 4,
                        "max_loan": 5000000,
                    },
                    "construction": {
                        "raw_industry_construction": 2
                        if mode.startswith("prospect")
                        else 1,
                        "autoslope": True,
                    },
                },
                "console": [
                    "setting difficulty.disasters 0",
                    "setting economy.multiple_industry_per_town "
                    + str(int(mode == "clear")),
                    "start_ai MigrationIndustries",
                ],
            }
        )
    for scenario in result:
        if scenario.get("industry") in ("prospect-unlucky", "prospect-exhausted"):
            scenario.update(ticks=32, short_checkpoint=True)
            scenario["settings"]["difficulty"].update(number_towns=4)
            scenario["settings"]["game_creation"] = {
                "custom_town_number": 1,
                "tree_placer": 0,
            }
    result.extend(
        {
            "name": "industry-water-check-" + obstruction,
            "kind": "generate",
            "seed": 12345,
            "map_log2": 7,
            "land_generator": 1,
            "industry": "water-check",
            "industry_water": obstruction,
            "ticks": 6 * SNAPSHOT_TICKS,
            "settings": {
                "game_creation": {"starting_year": 1980},
                "difficulty": {
                    "industry_density": 0,
                    "max_loan": 20000000,
                    "terrain_type": 0,
                    "quantity_sea_lakes": 3,
                },
                "construction": {"raw_industry_construction": 1},
            },
            "console": [
                "setting difficulty.disasters 0",
                "start_ai MigrationIndustries",
            ],
        }
        for obstruction in ("depot", "lock")
    )
    result.extend(
        dict(
            scenario,
            name="industry-company-" + scenario["companies"],
            industry="transfer",
        )
        for scenario in companies.scenarios(soak)
        if scenario["companies"] in ("acquire", "delete", "reuse")
    )
    return result


def prepare(scenario, binaries, builds, out, timeout, env, result):
    mode = scenario.get("industry")
    if mode == "transfer":
        # Company preparation runs first and builds real AI assets with the
        # unchanged reference. Both roles receive these same reachable settings.
        source = Path(scenario["save"])
        industries = economy.rows(read_save(source), "INDY")
        chosen = next(i for i, row in industries.items() if row["prod_level"] >= 4)
        target = source.parent / "industry-transfer.sav"
        result["industry_input"] = disasters.patch(
            source,
            target,
            {
                "INDY": {
                    chosen: {
                        "founder": 1,
                        "exclusive_supplier": 1,
                        "exclusive_consumer": 1,
                        "ctlflags": industries[chosen]["ctlflags"] | 4,
                    }
                }
            },
        )
        result["industry_chosen"] = chosen
        return dict(scenario, save=str(target))
    if mode in ("clear", "terraform", "prospect-unlucky", "prospect-exhausted"):
        folder = out / scenario["name"] / "input"
        setup = dict(
            scenario,
            industry="input",
            ticks=2 * SNAPSHOT_TICKS,
            console=[
                line for line in scenario["console"] if not line.startswith("start_ai")
            ],
        )
        run = run_game(
            setup,
            binaries["reference"],
            builds["reference"],
            folder / "reference",
            timeout,
            env,
        )
        if run["exit"] or not run["snapshots"]:
            raise RuntimeError("industry command reference input failed")
        source = run["snapshots"][-1]
        industries = economy.rows(read_save(source), "INDY")
        chosen = next(index for index, row in industries.items() if row["type"] == 0)
        edits = {"CHTS": {0: {"magic_bulldozer.value": int(mode == "clear")}}}
        if mode in PROSPECT_FAILURES:
            towns = economy.rows(read_save(source), "CITY")
            if len(towns) != 1 or industries[chosen]["town"] != next(iter(towns)) + 1:
                raise RuntimeError(
                    "prospect exhaustion requires one town and its coal mine"
                )
            seed, _ = PROSPECT_FAILURES[mode]
            edits["DATE"] = {
                0: {f"random_state[{i}]": word for i, word in enumerate(seed)}
            }
        target = folder / "input.sav"
        result["industry_input"] = disasters.patch(source, target, edits)
        result["industry_chosen"] = chosen
        return dict(
            scenario,
            kind="save",
            save=str(target),
            industry_target=industries[chosen]["location.tile"],
            console=["setting difficulty.industry_density 0", *scenario["console"]]
            if mode.startswith("prospect-")
            else scenario["console"],
        )
    if mode == "lumber":
        setup = dict(
            scenario,
            ticks=3 * SNAPSHOT_TICKS,
            console=[*scenario["console"], "start_ai MigrationIndustries"],
        )
        run = run_game(
            setup,
            binaries["reference"],
            builds["reference"],
            out / scenario["name"] / "input" / "reference",
            timeout,
            env,
        )
        if run["exit"] or not any(
            "INDUSTRY-LUMBER ready true" in line for line in run["log"]
        ):
            raise RuntimeError("reference setup AI did not build the lumber mill")
        source = run["snapshots"][-1]
        target = out / scenario["name"] / "input" / "lumber.sav"
        result["industry_input"] = disasters.patch(source, target, {})
        return dict(scenario, kind="save", save=str(target))
    if mode not in (
        "original",
        "frozen",
        "control",
        "closure",
        "farm-cleanup",
        "builder",
    ):
        return scenario
    folder = out / scenario["name"] / "input"
    setup = dict(scenario, ticks=2 * SNAPSHOT_TICKS)
    run = run_game(
        setup,
        binaries["reference"],
        builds["reference"],
        folder / "reference",
        timeout,
        env,
    )
    if run["exit"] or not run["snapshots"]:
        raise RuntimeError("industry reference setup failed")
    source = run["snapshots"][-1]
    before = read_save(source)
    industries = economy.rows(before, "INDY")
    chosen = next(
        index
        for index, row in industries.items()
        if row.get("produced[0]/cargo", 255) < 64
    )
    changes = {}
    console = list(scenario.get("console", []))
    if mode in ("original", "frozen"):
        console.append("setting economy.type " + ("0" if mode == "original" else "2"))
    if mode == "control":
        changes["INDY"] = {index: {"ctlflags": 15} for index in industries}
    if mode in ("closure", "farm-cleanup"):
        if mode == "farm-cleanup":
            chosen = next(
                index for index, row in industries.items() if row["type"] == 9
            )
            result["industry_field_input"] = len(industry_fields(before, chosen))
            if not result["industry_field_input"]:
                raise RuntimeError("reference farm has no associated fields to remove")
        changes["INDY"] = {chosen: {"prod_level": 0}}
        console.append("setting difficulty.industry_density 0")
    if mode == "builder":
        changes["IBLD"] = {0: {"wanted_inds": 200 << 16}}
        # Disable current target selection optimizations and force an immediate
        # deficit reassessment. The reference computes all actual probabilities.
        changes["ITBL"] = {
            index: {"target_count": 0, "wait_count": 0, "max_wait": 1}
            for index in economy.rows(before, "ITBL")
        }
        changes["ECMY"] = {0: {"industry_daily_change_counter": 512 << 16}}
    target = folder / "input.sav"
    target.parent.mkdir(parents=True, exist_ok=True)
    result["industry_input"] = disasters.patch(source, target, changes)
    result["industry_chosen"] = chosen
    return dict(scenario, kind="save", save=str(target), console=console)


def check(scenario, run, mode, role, result):
    kind = scenario.get("industry")
    if kind == "water-check":
        for branch in ("plain", scenario["industry_water"], "fund"):
            marker = f"INDUSTRY-WATER {branch} ready true"
            if not any(marker in line for line in run["log"]):
                result["problems"].append(
                    f"{mode}/{role}: water-check {branch} branch missing"
                )
        if not run["snapshots"] or not any(
            row["type"] == 5 and row["founder"] < 15
            for row in economy.rows(read_save(run["snapshots"][-1]), "INDY").values()
        ):
            result["problems"].append(f"{mode}/{role}: no player-funded oil rig")
        return
    if kind == "transfer" and not run["snapshots"]:
        raise RuntimeError("industry ownership transfer witness has no output save")
    if kind is None or not run["snapshots"]:
        return
    saved = [read_save(path) for path in run["snapshots"]]
    rows = [economy.rows(chunks, "INDY") for chunks in saved]
    facts = {
        "industries": len(rows[-1]),
        "produced_history": max(
            (
                value
                for sample in rows
                for row in sample.values()
                for key, value in row.items()
                if key.endswith("/production")
            ),
            default=0,
        ),
        "transported_history": max(
            (
                value
                for sample in rows
                for row in sample.values()
                for key, value in row.items()
                if key.endswith("/transported")
            ),
            default=0,
        ),
        "accepted_history": max(
            (
                value
                for sample in rows
                for row in sample.values()
                for key, value in row.items()
                if key.startswith("accepted[") and key.endswith("/accepted")
            ),
            default=0,
        ),
        "valid_history": max(
            (row["valid_history"] for sample in rows for row in sample.values()),
            default=0,
        ),
        "builder_backoff": max(
            row["max_wait"]
            for chunks in saved
            for row in economy.rows(chunks, "ITBL").values()
        ),
    }
    facts["year_history"] = bool(facts["valid_history"] & (1 << 42))
    if kind == "transfer":
        chosen = result["industry_chosen"]
        expected = (2, 2, 2) if scenario["companies"] == "acquire" else (16, 255, 255)
        final = rows[-1].get(chosen, {})
        facts["ownership"] = tuple(
            final.get(field)
            for field in ("founder", "exclusive_supplier", "exclusive_consumer")
        )
        if facts["ownership"] != expected:
            result["problems"].append(
                f"{mode}/{role}: industry ownership transfer missing: {facts['ownership']} != {expected}"
            )
    if kind in ("temperate", "arctic"):

        def fields(chunks):
            return {
                tile
                for tile, value in enumerate(chunks["MAPT"]["raw"])
                if value >> 4 == 0 and (chunks["MAP5"]["raw"][tile] >> 2) & 7 == 3
            }

        facts["new_field_tiles"] = (
            len(set.union(*(fields(c) for c in saved[1:])) - fields(saved[0]))
            if len(saved) > 1
            else 0
        )
        facts["field_tiles"] = len(fields(saved[-1]))
        if len(saved) > 1 and facts["new_field_tiles"] == 0:
            result["problems"].append(f"{mode}/{role}: no new farm-field tiles")
    if kind == "lumber":
        facts["lumber_harvest"] = any(
            row["type"] == 25
            and row.get("produced[0]/rate") == 0
            and any(
                value > 0
                for key, value in row.items()
                if key.startswith("produced[0]/history[")
                and key.endswith("/production")
            )
            for sample in rows
            for row in sample.values()
        )
        if not facts["lumber_harvest"]:
            result["problems"].append(f"{mode}/{role}: no actual lumber harvest")
    if kind in ("fund", "prospect", "clear", "terraform", *PROSPECT_FAILURES):
        marker = f"INDUSTRY-COMMAND {kind} ready true"
        facts["command_completed"] = any(marker in line for line in run["log"])
        if not facts["command_completed"]:
            result["problems"].append(f"{mode}/{role}: {kind} command branch missing")
        if kind in ("fund", "prospect") and not any(
            row["type"] == 0 and row["founder"] < 15
            for sample in rows
            for row in sample.values()
        ):
            result["problems"].append(f"{mode}/{role}: no player-funded raw industry")
        if kind in PROSPECT_FAILURES:
            before = economy.rows(read_save(Path(scenario["save"])), "INDY")

            def identity(sample):
                return {
                    index: tuple(
                        row[key] for key in ("type", "town", "location.tile", "founder")
                    )
                    for index, row in sample.items()
                }

            facts["unchanged_identity"] = all(
                identity(sample) == identity(before) for sample in rows
            )
            date = economy.rows(saved[-1], "DATE")[0]
            facts["random_state"] = tuple(date[f"random_state[{i}]"] for i in (0, 1))
            facts["checkpoint"] = (date["tick_counter"], date["economy_date_fract"])
            facts["company_restored"] = (
                economy.rows(saved[-1], "PLYR")[1]["name"] == "Industry " + kind
            )
            if not facts["unchanged_identity"] or not facts["company_restored"]:
                result["problems"].append(
                    f"{mode}/{role}: prospect changed identity/company context"
                )
            if facts["random_state"] != PROSPECT_FAILURES[kind][1] or facts[
                "checkpoint"
            ] != (6048, 32):
                result["problems"].append(
                    f"{mode}/{role}: {kind} RNG checkpoint missing: {facts['random_state']} at {facts['checkpoint']}"
                )
        if kind == "clear":
            before = economy.rows(read_save(Path(scenario["save"])), "INDY")
            chosen = result["industry_chosen"]
            facts["reused_id"] = (
                chosen in rows[-1]
                and rows[-1][chosen]["founder"] < 15
                and before[chosen]["founder"] == 16
            )
            if not facts["reused_id"]:
                result["problems"].append(
                    f"{mode}/{role}: industry ID not reused after clear"
                )
    result[f"{mode}_{role}_industry"] = facts
    if kind == "generation":
        if bool(facts["industries"] == 0) != scenario["industry_generation_empty"]:
            result["problems"].append(
                f"{mode}/{role}: generation density branch missing"
            )
    if (
        kind
        not in (
            "cargo",
            "generation",
            "fund",
            "prospect",
            *PROSPECT_FAILURES,
            "clear",
            "terraform",
            "transfer",
        )
        and facts["produced_history"] == 0
    ):
        result["problems"].append(f"{mode}/{role}: no industry production history")
    if scenario.get("industry_soak") and not facts["year_history"]:
        result["problems"].append(f"{mode}/{role}: no yearly industry history")
    if kind == "cargo" and scenario["economy"] != "stockpile":
        if facts["transported_history"] == 0 or facts["accepted_history"] == 0:
            result["problems"].append(f"{mode}/{role}: no transported/accepted history")
    if kind in ("closure", "farm-cleanup"):
        chosen = result["industry_chosen"]
        if chosen in rows[-1]:
            result["problems"].append(f"{mode}/{role}: selected industry did not close")
        if kind == "farm-cleanup":
            facts["remaining_associated_fields"] = len(
                industry_fields(saved[-1], chosen)
            )
            if facts["remaining_associated_fields"]:
                result["problems"].append(
                    f"{mode}/{role}: farm fields remained associated"
                )
    if kind == "control":
        before = economy.rows(read_save(Path(scenario["save"])), "INDY")
        for index, row in rows[-1].items():
            if index in before and row["prod_level"] != before[index]["prod_level"]:
                result["problems"].append(
                    f"{mode}/{role}: controlled production changed"
                )
    if kind == "builder":
        before = economy.rows(read_save(Path(scenario["save"])), "INDY")
        facts["new_industries"] = len(set(rows[-1]) - set(before))
        if facts["new_industries"] == 0 or facts["builder_backoff"] <= 1:
            result["problems"].append(f"{mode}/{role}: no builder success/backoff")


def industry_fields(chunks, industry):
    """Canonical packed field association, exposed by the existing save decoder."""
    import struct

    return {
        tile
        for tile, value in enumerate(chunks["MAPT"]["raw"])
        if value >> 4 == 0
        and (chunks["MAP5"]["raw"][tile] >> 2) & 7 == 3
        and struct.unpack_from(">H", chunks["MAP2"]["raw"], tile * 2)[0] == industry
    }


def production_gap():
    """Absent NewGRFs: unchanged version, repetition and native arithmetic policy."""
    import importlib.util
    import subprocess

    from .core import ROOT

    spec = importlib.util.spec_from_file_location(
        "migration", ROOT / "tools/migration.py"
    )
    migration = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(migration)
    out = ROOT / ".local/industry-production-gap"
    out.mkdir(parents=True, exist_ok=True)
    source = subprocess.check_output(
        ["git", "show", migration.BASELINE["commit"] + ":src/newgrf_industries.cpp"],
        cwd=ROOT,
    ).decode()
    start = source.index("void IndustryProductionCallback(Industry *ind, int reason)")
    end = source.index("/**\n * Check whether an industry temporarily", start)
    (out / "industry-production-reference.inc").write_text(source[start:end])
    binary = out / "probe"
    subprocess.run(
        [
            "c++",
            "-std=c++20",
            "-O2",
            "-fno-strict-overflow",
            "-DPOINTER_IS_64BIT",
            "-I" + str(ROOT / "src"),
            "-I" + str(out),
            str(ROOT / "tools/migration/industry-production-gap.cpp"),
            str(migration.rust_archive(ROOT / "build-rust")),
            "-ldl",
            "-lpthread",
            "-lm",
            "-o",
            str(binary),
        ],
        cwd=ROOT,
        env=migration.environment(),
        check=True,
    )
    subprocess.run([str(binary)], cwd=ROOT, check=True)


def lifecycle_gap():
    """Absent custom industry GRFs: unchanged tile/parent RNG and animation."""
    import importlib.util
    import subprocess

    from .core import ROOT

    spec = importlib.util.spec_from_file_location(
        "migration", ROOT / "tools/migration.py"
    )
    migration = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(migration)
    out = ROOT / ".local/industry-lifecycle-gap"
    out.mkdir(parents=True, exist_ok=True)
    source = subprocess.check_output(
        ["git", "show", migration.BASELINE["commit"] + ":src/newgrf_industrytiles.cpp"],
        cwd=ROOT,
        text=True,
    )
    signatures = (
        "static bool DoTriggerIndustryTileAnimation(",
        "bool TriggerIndustryTileAnimation_ConstructionStageChanged(",
        "bool TriggerIndustryTileAnimation(",
        "bool TriggerIndustryAnimation(",
        "static void DoTriggerIndustryTileRandomisation(",
        "static void DoReseedIndustry(",
        "void TriggerIndustryTileRandomisation(",
        "void TriggerIndustryRandomisation(",
    )
    bodies = []
    for signature in signatures:
        start = source.index(signature)
        end = source.index("{", start) + 1
        depth = 1
        while depth:
            if source[end] == "{":
                depth += 1
            elif source[end] == "}":
                depth -= 1
            end += 1
        bodies.append(source[start:end])
    (out / "industry-lifecycle-reference.inc").write_text("\n\n".join(bodies))
    tile_source = subprocess.check_output(
        ["git", "show", migration.BASELINE["commit"] + ":src/industry_cmd.cpp"],
        cwd=ROOT,
        text=True,
    )
    start = tile_source.index("static void AddAcceptedCargo_Industry(")
    end = tile_source.index("{", start) + 1
    depth = 1
    while depth:
        if tile_source[end] == "{":
            depth += 1
        elif tile_source[end] == "}":
            depth -= 1
        end += 1
    (out / "industry-acceptance-reference.inc").write_text(tile_source[start:end])
    animation = []
    for name in (
        "AnimateSugarSieve",
        "AnimateToffeeQuarry",
        "AnimateBubbleCatcher",
        "AnimatePowerPlantSparks",
        "AnimateToyFactory",
        "AnimatePlasticFountain",
        "AnimateOilWell",
        "AnimateMineTower",
        "AnimateTile_Industry",
    ):
        start = tile_source.index("static void " + name + "(")
        end = tile_source.index("{", start) + 1
        depth = 1
        while depth:
            if tile_source[end] == "{":
                depth += 1
            elif tile_source[end] == "}":
                depth -= 1
            end += 1
        animation.append(tile_source[start:end])
    drawing = subprocess.check_output(
        ["git", "show", migration.BASELINE["commit"] + ":src/table/industry_land.h"],
        cwd=ROOT,
        text=True,
    )
    start = drawing.index("static const uint8_t _industry_anim_offs_toffee[]")
    end = drawing.index("};", start) + 2
    animation.insert(0, drawing[start:end])
    (out / "industry-animation-reference.inc").write_text("\n\n".join(animation))
    graphics = subprocess.check_output(
        ["git", "show", migration.BASELINE["commit"] + ":src/industry_map.h"],
        cwd=ROOT,
        text=True,
    )
    start = graphics.index("enum IndustryGraphics :")
    end = graphics.index("};", start) + 2
    (out / "industry-graphics-reference.inc").write_text(graphics[start:end])
    binary = out / "probe"
    subprocess.run(
        [
            "c++",
            "-std=c++20",
            "-O2",
            "-fno-strict-overflow",
            "-DPOINTER_IS_64BIT",
            "-I" + str(ROOT / "src"),
            "-I" + str(out),
            str(ROOT / "tools/migration/industry-lifecycle-gap.cpp"),
            str(migration.rust_archive(ROOT / "build-rust")),
            "-ldl",
            "-lpthread",
            "-lm",
            "-o",
            str(binary),
        ],
        cwd=ROOT,
        env=migration.environment(),
        check=True,
    )
    subprocess.run([str(binary)], cwd=ROOT, check=True)


if __name__ == "__main__":
    production_gap()
    lifecycle_gap()
