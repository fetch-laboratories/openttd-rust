"""Industry production/history and builder evidence on existing reference inputs."""

import shutil
import struct
from pathlib import Path

from . import companies, core, economy
from .console_adapter import console_adapter, prepared_console
from .core import (
    ROOT,
    SNAPSHOT_TICKS,
    TICKS_PER_DAY,
    WITNESS_ENVIRONMENT,
    branch_witnesses,
    read_save,
    run_game,
)

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
    if source := scenario.get("industry_definition_dir"):
        for personal in (
            run_dir / ".openttd",
            run_dir / ".openttd-rust",
            run_dir / "xdg-data/openttd",
            run_dir / "xdg-data/openttd-rust",
        ):
            target = personal / "newgrf/industry-definitions"
            target.mkdir(parents=True, exist_ok=True)
            for path in Path(source).glob("*.grf"):
                (target / path.name).write_bytes(path.read_bytes())
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
    result.append(
        dict(
            name="industry-resolver",
            kind="save",
            industry="resolver",
            ticks=1,
            short_checkpoint=True,
            save=str(ROOT / "migration/saves/padhattan-ridge-2000.sav"),
        )
    )
    result.extend(
        dict(
            name="industry-" + mode,
            kind="save",
            save=str(ROOT / "migration/saves/grok-159-001.sav"),
            industry=mode,
            ticks=(8 if soak else 4) * SNAPSHOT_TICKS,
            settings={
                "construction": {"raw_industry_construction": 1},
                "economy": {"multiple_industry_per_town": True},
            },
        )
        for mode in ("definitions", "definitions-reload", "policy", "policy-reload")
    )

    return result


def game_environment(scenario):
    return (
        WITNESS_ENVIRONMENT
        if scenario.get("industry") == "resolver" and scenario.get("witnesses")
        else {}
    )


def adapter(build: Path, folder: Path, env: dict[str, str]) -> Path:
    import probe

    import migration

    source = migration.git(
        "show", migration.BASELINE["commit"] + ":src/industry_cmd.cpp"
    )
    signatures = (
        "static uint GetCurrentTotalNumberOfIndustries()",
        "static void UpdateIndustryStatistics(Industry *i)",
        "static bool CheckIndustryCloseDownProtection(IndustryType type)",
        "static void CanCargoServiceIndustry(",
        "int WhoCanServiceIndustry(Industry *ind)",
        "static void ReportNewsProductionChangeIndustry(",
        "static void ChangeIndustryProduction(Industry *i, bool monthly)",
    )
    text = "static const uint PERCENT_TRANSPORTED_60 = 153, PERCENT_TRANSPORTED_80 = 204;\n"
    text += "\n".join(probe.body(source, signature) for signature in signatures)
    for period in ("daily", "monthly"):
        start = source.index("\n{", source.index("_economy_industries_" + period)) + 1
        end = source.index("\n});", start)
        text += (
            "\nstatic void IndustryPolicy"
            + period.title()
            + "()\n"
            + source[start:end]
            + "\n}\n"
        )
    folder.mkdir(parents=True, exist_ok=True)
    (folder / "industry-policy-reference.inc").write_text(text)
    return console_adapter(
        build,
        folder,
        env,
        ROOT / "tools/simulation/industries-console.cpp",
        "industries",
    )


def prepare(scenario, binaries, builds, out, timeout, env, result):
    mode = scenario.get("industry")
    if mode in ("definitions", "definitions-reload", "policy", "policy-reload"):
        return prepare_definitions(
            scenario, binaries, builds, out, timeout, env, result
        )
    if mode == "resolver":
        folder = out / scenario["name"] / "input"
        source, _ = core.normalize(Path(scenario["save"]), folder / "normalized")
        executables = prepared_console(
            "industries", adapter, binaries, builds, folder, env
        )
        return dict(
            scenario,
            save=str(source),
            executables=executables,
            console=["industries_scenario", "unpause"],
        )
    if mode == "transfer":
        # Company preparation runs first and builds real AI assets with the
        # unchanged reference. Both roles receive these same reachable settings.
        source = Path(scenario["save"])
        industries = core.rows(read_save(source), "INDY")
        chosen = next(i for i, row in industries.items() if row["prod_level"] >= 4)
        target = source.parent / "industry-transfer.sav"
        result["industry_input"] = core.patch(
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
        industries = core.rows(read_save(source), "INDY")
        chosen = next(index for index, row in industries.items() if row["type"] == 0)
        edits = {"CHTS": {0: {"magic_bulldozer.value": int(mode == "clear")}}}
        if mode in PROSPECT_FAILURES:
            towns = core.rows(read_save(source), "CITY")
            if len(towns) != 1 or industries[chosen]["town"] != next(iter(towns)) + 1:
                raise RuntimeError(
                    "prospect exhaustion requires one town and its coal mine"
                )
            seed, _ = PROSPECT_FAILURES[mode]
            edits["DATE"] = {
                0: {f"random_state[{i}]": word for i, word in enumerate(seed)}
            }
        target = folder / "input.sav"
        result["industry_input"] = core.patch(source, target, edits)
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
        result["industry_input"] = core.patch(source, target, {})
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
    industries = core.rows(before, "INDY")
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
            for index in core.rows(before, "ITBL")
        }
        changes["ECMY"] = {0: {"industry_daily_change_counter": 512 << 16}}
    target = folder / "input.sav"
    target.parent.mkdir(parents=True, exist_ok=True)
    result["industry_input"] = core.patch(source, target, changes)
    result["industry_chosen"] = chosen
    return dict(scenario, kind="save", save=str(target), console=console)


def check(scenario, run, mode, role, result):
    kind = scenario.get("industry")
    if kind in ("definitions", "definitions-reload"):
        check_definitions(scenario, run, mode, role, result)
        return
    if kind in ("policy", "policy-reload"):
        markers = [
            "INDUSTRY-POLICY complete true",
            "INDUSTRY-POLICY forests",
            "INDUSTRY-POLICY rng",
        ]
        if kind == "policy":
            markers += [
                f"INDUSTRY-POLICY servicing {outcome} true" for outcome in range(3)
            ]
        if run["exit"] or any(marker not in run["log"] for marker in markers):
            raise RuntimeError("industry policy observations missing: " + role)
        return
    if kind == "resolver":
        branch_witnesses(
            scenario,
            run,
            role,
            "industry_resolver",
            (
                "variable",
                "tile_variable",
                "count",
                "filtered_count",
                "location",
                "root",
                "tile_root",
                "town",
                "refusal",
                "probability",
            ),
        )
        for marker in (
            "var67 0",
            "var68 0",
            "refusal true",
            "location 0 2271560481",
            "checkpoint true",
            "unavailable-fallback true",
            "count-wrap true",
            "tile-identities true",
            "production-params true",
            "production 0 12",
            "callbacks true",
            "randomized true",
            "complete true",
        ):
            if not any("INDUSTRY-RESOLVER " + marker in line for line in run["log"]):
                raise RuntimeError(
                    f"{mode}/{role}: missing industry resolver evidence: {marker}"
                )
        return
    if kind == "water-check":
        for branch in ("plain", scenario["industry_water"], "fund"):
            marker = f"INDUSTRY-WATER {branch} ready true"
            if not any(marker in line for line in run["log"]):
                result["problems"].append(
                    f"{mode}/{role}: water-check {branch} branch missing"
                )
        if not run["snapshots"] or not any(
            row["type"] == 5 and row["founder"] < 15
            for row in core.rows(read_save(run["snapshots"][-1]), "INDY").values()
        ):
            result["problems"].append(f"{mode}/{role}: no player-funded oil rig")
        return
    if kind == "transfer" and not run["snapshots"]:
        raise RuntimeError("industry ownership transfer witness has no output save")
    if kind is None or not run["snapshots"]:
        return
    saved = [read_save(path) for path in run["snapshots"]]
    rows = [core.rows(chunks, "INDY") for chunks in saved]
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
            for row in core.rows(chunks, "ITBL").values()
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
            before = core.rows(read_save(Path(scenario["save"])), "INDY")

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
            date = core.rows(saved[-1], "DATE")[0]
            facts["random_state"] = tuple(date[f"random_state[{i}]"] for i in (0, 1))
            facts["checkpoint"] = (date["tick_counter"], date["economy_date_fract"])
            facts["company_restored"] = (
                core.rows(saved[-1], "PLYR")[1]["name"] == "Industry " + kind
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
            before = core.rows(read_save(Path(scenario["save"])), "INDY")
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
        before = core.rows(read_save(Path(scenario["save"])), "INDY")
        for index, row in rows[-1].items():
            if index in before and row["prod_level"] != before[index]["prod_level"]:
                result["problems"].append(
                    f"{mode}/{role}: controlled production changed"
                )
    if kind == "builder":
        before = core.rows(read_save(Path(scenario["save"])), "INDY")
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

    import probe

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
        bodies.append(probe.body(source, signature))
    (out / "industry-lifecycle-reference.inc").write_text("\n\n".join(bodies))
    tile_source = subprocess.check_output(
        ["git", "show", migration.BASELINE["commit"] + ":src/industry_cmd.cpp"],
        cwd=ROOT,
        text=True,
    )
    (out / "industry-acceptance-reference.inc").write_text(
        probe.body(tile_source, "static void AddAcceptedCargo_Industry(")
    )
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
        animation.append(probe.body(tile_source, "static void " + name + "("))
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


def definition_grfs(folder):
    """Write real GRF containers with independent property and layout expectations."""
    folder.mkdir(parents=True, exist_ok=True)

    def action(feature, first, properties, count=1):
        return bytes((0, feature, len(properties), count, first)) + b"".join(
            bytes((prop,)) + data for prop, data in properties
        )

    def byte(value):
        return bytes((value,))

    def word(value):
        return struct.pack("<H", value)

    def dword(value):
        return struct.pack("<I", value)

    def layout(*tables):
        data = b"".join(tables)
        # A short declared size is warning-only; the table terminators control parsing.
        return byte(len(tables)) + dword(0) + data

    custom = bytes((0, 0, 0xFE)) + word(0) + bytes((0, 0x80))
    duplicate = bytes((0, 0, 0, 0, 0, 1, 0, 0x80))
    imported = bytes((0xFE, 0, 0))
    water = bytes((0xFF, 1, 0xFF, 0, 0, 0, 0, 0x80))
    for name, identifier, properties in (
        (
            "main",
            b"I475",
            [
                action(8, 0, [(0x18, b"owner475/main\0")]),
                # Eighty-two early reservations cross the 0xFF water-marker hole.
                action(9, 0, [(8, bytes(82))], count=82),
                action(
                    9,
                    0,
                    [
                        (8, byte(5)),
                        (9, byte(250)),
                        (9, byte(0)),
                        (0x0F, bytes((3, 0))),
                        (0x10, byte(4)),
                        (0x11, byte(0)),
                        (0x12, byte(2)),
                        (0x13, bytes((2, 1, 8, 0, 0xFC))),
                        (0x14, word(3) + word(0) + word(0) + word(99)),
                    ],
                ),
                action(
                    10,
                    17,
                    [
                        (8, byte(0)),
                        (8, byte(3)),
                        (9, byte(250)),
                        (9, byte(0)),
                        (0x0A, layout(custom, duplicate, imported)),
                        (0x0F, byte(1)),
                        (0x14, byte(0)),
                        (0x16, bytes((255, 255, 255))),
                        (0x19, byte(77)),
                        (0x1A, dword(0)),
                        (0x20, dword(0xFEDCBA98)),
                        (0x21, byte(0)),
                        (0x22, byte(0)),
                        (0x23, dword(12345)),
                        (0x24, word(0)),
                        (0x25, bytes((1, 1))),
                        (0x26, byte(0)),
                        (0x27, bytes((1, 80))),
                        (
                            0x28,
                            bytes((2, 2))
                            + word(256)
                            + word(512)
                            + word(768)
                            + word(1024),
                        ),
                    ],
                ),
                action(10, 17, [(0x29, word(1) + word(0))]),
                action(9, 90, [(0x0F, bytes((3, 0)))]),
                action(10, 90, [(0x0A, byte(0)), (0x29, word(0)), (0x0F, byte(12))]),
                action(
                    10,
                    18,
                    [(8, byte(3)), (0x0A, layout(water)), (0x15, bytes((2, 3, 4)))],
                ),
            ],
        ),
        (
            "conflict",
            b"C475",
            [
                action(9, 0, [(8, byte(1)), (9, byte(0))]),
                action(10, 17, [(8, byte(0)), (9, byte(0))]),
            ],
        ),
        # The first animation byte is published even though the second read fails.
        ("partial", b"P475", [action(9, 0, [(8, byte(2)), (0x0F, byte(7))])]),
        # Scalar industry mutation likewise survives the later truncated dword.
        (
            "broken",
            b"B475",
            [action(10, 19, [(8, byte(3)), (0x0F, byte(93)), (0x20, word(1))])],
        ),
        (
            "badges",
            b"Q475",
            [
                action(
                    8, 0, [(0x18, b"owner475/partial1\0owner475/partial2\0")], count=2
                ),
                action(
                    9,
                    0,
                    [
                        (8, byte(2)),
                        (0x14, word(1) + word(1)),
                        (0x14, word(2) + word(0)),
                    ],
                ),
            ],
        ),
        (
            "invalid",
            b"X475",
            [action(10, 20, [(8, byte(3)), (0x0A, layout(bytes((0xFE, 255, 0))))])],
        ),
    ):
        actions = [
            b"\x08\x07" + identifier + b"Industry definitions\0Ownership fixture\0",
            *properties,
        ]
        payload = word(4) + b"\xff" + dword(len(actions))
        payload += b"".join(word(len(data)) + b"\xff" + data for data in actions)
        (folder / (name + ".grf")).write_bytes(payload + word(0))


def prepare_definitions(scenario, binaries, builds, out, timeout, env, result):
    folder = out / scenario["name"] / "input"
    source, _ = core.normalize(Path(scenario["save"]), folder / "normalized")
    policy = scenario["industry"].startswith("policy")
    if not policy:
        definition_grfs(folder / "grfs")
    executables = prepared_console("industries", adapter, binaries, builds, folder, env)
    command = (
        "industry_policy run"
        if policy
        else f'industry_definitions run "{folder / "grfs"}"'
    )
    marker = "INDUSTRY-POLICY" if policy else "INDUSTRY-DEFINITIONS"
    scenario = dict(
        scenario,
        executables=executables,
        save=str(source),
    )
    if policy:
        scenario.update(ticks=1, short_checkpoint=True)
    else:
        scenario["industry_definition_dir"] = str(folder / "grfs")
    if not scenario["industry"].endswith("-reload"):
        return dict(
            scenario,
            console=[command, "setting difficulty.industry_density 0", "unpause"],
        )
    saves = {role: folder / role / "definitions.sav" for role in executables}
    captures = core.capture_pair(
        lambda role: dict(
            scenario,
            console=[command + f' "{saves[role]}"', "unpause"],
            ticks=1,
            short_checkpoint=True,
        ),
        executables,
        builds,
        folder,
        timeout,
        env,
    )
    for role, run in captures.items():
        if (
            run["exit"]
            or not saves[role].is_file()
            or marker + " complete true" not in run["log"]
        ):
            raise RuntimeError("industry capture failed: " + role)
    differences = core.compare_saves(
        saves["reference"],
        saves["candidate"],
        scenario["name"],
        10,
        {"chunks": 0, "elements": 0, "masked": {}},
    )
    if differences or captures["reference"]["log"] != captures["candidate"]["log"]:
        result["differences"].extend(differences)
        raise RuntimeError("industry captures differ")
    return dict(
        scenario,
        role_inputs={role: {"save": str(save)} for role, save in saves.items()},
        console=[
            "industry_policy inspect" if policy else "industry_definitions inspect",
            "setting difficulty.industry_density 0",
            "unpause",
        ],
    )


def check_definitions(scenario, run, mode, role, result):
    if run["exit"]:
        return
    marker = "INDUSTRY-DEFINITIONS complete true"
    if marker not in run["log"]:
        raise RuntimeError(f"{mode}/{role}: missing real-GRF industry definitions")
    saves = [read_save(path) for path in run["snapshots"]]
    if not saves:
        raise RuntimeError("industry definition scenario produced no saves")
    identity = int.from_bytes(b"I475", "little")
    for saved in saves:
        industries = core.rows(saved, "INDY")
        mapping = core.rows(saved, "IIDS")
        tiles = core.rows(saved, "TIDS")
        if not any(
            row["grfid"] == identity and row["entity_id"] == 18
            for row in mapping.values()
        ):
            raise RuntimeError("industry mapping identity was not retained")
        custom = [row for row in tiles.values() if row["grfid"] == identity]
        if len(custom) != 82 or 255 in tiles or tiles[256]["entity_id"] != 80:
            raise RuntimeError("early tile reservations or 0xFF exclusion changed")
        coal = [row for row in industries.values() if row["type"] == 0]
        if len(coal) < 2:
            raise RuntimeError("industry funding did not create a second coal mine")
    final = core.rows(saves[-1], "INDY")
    funded = [
        row
        for row in final.values()
        if row["type"] == 0 and row["location.w"] == 1 and row["location.h"] == 1
    ]
    if len(funded) != 1:
        raise RuntimeError("funded custom layout identity was not retained")
    for field in ("production", "transported"):
        if not any(
            value > 0
            for key, value in funded[0].items()
            if key.startswith("produced[0]/history[") and key.endswith("/" + field)
        ):
            raise RuntimeError("funded custom industry has no cargo " + field)
