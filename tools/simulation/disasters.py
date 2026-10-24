"""Disaster scheduler, live targets and lifecycle witnesses (#103)."""

import hashlib
import json
import shutil
import struct
import threading
from pathlib import Path

from . import core
from .core import (
    ROOT,
    patch,
    read_save,
    rows,
    run_game,
    save_moment,
)

AI_FOLDER = "disaster-scenario-ai"
LOCK = threading.Lock()
SCHEMA = list(
    zip(
        "next subtype tile dest_tile x_pos y_pos z_pos direction owner vehstatus state sprite_cache.sprite_seq.seq[0].sprite age tick_counter image_override big_ufo_destroyer_target flags".split(),
        (6, 2, 6, 6, 5, 5, 5, 2, 2, 2, 4, 4, 5, 2, 6, 6, 2),
        strict=True,
    )
)
FAMILIES = {
    "zeppelin": (1935, 0, (0, 1)),
    "small-ufo": (1945, 1, (2, 3)),
    "airplane": (1965, 1, (4, 5)),
    "helicopter": (1995, 0, (6, 7, 8)),
    "big-ufo": (2005, 0, (9, 10)),
    "small-sub": (1945, 6, (13,)),
    "big-sub": (2005, 6, (14,)),
    "coal": (1956, 6, ()),
}


def uses_ai(scenario):
    return "disasters" in scenario


def install(scenario, run_dir):
    if uses_ai(scenario):
        ai = run_dir / "ai/disasters"
        shutil.copytree(scenario["scenario_ai"], ai)
        (ai / "parameters.nut").write_text(
            f'DISASTER_SETUP <- {str(scenario.get("setup", False)).lower()};\nDISASTER_ACTION <- "{scenario.get("action", "none")}";\nDISASTER_TARGET <- {scenario.get("target", 0)};\n'
        )


def game_args(scenario):
    return (
        ["-d", "script=2"] if uses_ai(scenario) and scenario["kind"] != "save" else []
    )


def disasters(path):
    return {
        index: {
            key.removeprefix("disaster[0]/"): value
            for key, value in row.items()
            if key != "type"
        }
        for index, row in rows(path).items()
        if row["type"] == 5
    }


def negative_probes(source, folder, main):
    """The unchanged semantic decoder must expose each Rust-private saved field."""
    baseline = folder / "negative-baseline.sav"
    patch(source, baseline, {})
    observed = disasters(source)[main]
    checked = []
    for field in ("state", "image_override", "big_ufo_destroyer_target", "flags"):
        changed = folder / f"negative-{field}.sav"
        key = f"disaster[0]/{field}"
        patch(source, changed, {"VEHS": {main: {key: observed[field] ^ 1}}})
        differences = core.compare_saves(
            baseline,
            changed,
            "disaster-negative",
            2,
            {"chunks": 0, "elements": 0, "masked": {}},
        )
        if (
            len(differences) != 1
            or differences[0]["chunk"] != "VEHS"
            or differences[0]["element"] != str(main)
            or differences[0]["field"] != key
            or "issue" in differences[0]
        ):
            raise RuntimeError(f"semantic decoder hid private disaster field {field}")
        checked.append(field)
    return checked


def calendar(year):
    return year * 365 + (year - 1) // 4 - (year - 1) // 100 + (year - 1) // 400 + 1


def prepare(scenario, binaries, builds, out, timeout, env, result):
    if "disasters" not in scenario:
        return scenario
    folder = out / scenario["name"] / "input"
    folder.mkdir(parents=True, exist_ok=True)

    def run(label, setup, ticks):
        cache = out / "disaster-fixtures" / label
        fixture = cache / "save/autosave/exit.sav"
        if not fixture.exists():
            trial = run_game(
                dict(setup, ticks=ticks),
                binaries["reference"],
                builds["reference"],
                cache,
                timeout,
                env,
                False,
            )
            if trial["exit"] or not fixture.exists():
                raise RuntimeError(f"disaster preparation {label} failed")
            (cache / "preparation.json").write_text(
                json.dumps(
                    {
                        "ticks": ticks,
                        "moment": save_moment(fixture),
                        "sha256": hashlib.sha256(fixture.read_bytes()).hexdigest(),
                        "log": trial["log"],
                    },
                    indent=2,
                )
                + "\n"
            )
        return fixture

    with LOCK:
        base = run(
            "world",
            {
                "kind": "generate",
                "seed": 1,
                "map_log2": 8,
                "land_generator": 1,
                "settings": {
                    "game_creation": {"landscape": "temperate", "starting_year": 1950},
                    "difficulty": {"number_industries": 4, "disasters": "false"},
                },
            },
            600,
        )
        kind = scenario["disasters"]
        station = kind.startswith(("zeppelin", "train", "road")) or kind in (
            "small-ufo-crash",
            "target-reload",
        )
        if station:
            initial = run(
                "stationlist",
                {"kind": "save", "save": str(ROOT / "regression/stationlist/test.sav")},
                1,
            )
            script = out / "disaster-fixtures/script.sav"
            patch(initial, script, {}, "observer")
            setup = dict(
                scenario,
                kind="save",
                save=str(script),
                setup=True,
                console=[
                    "setting difficulty.disasters 0",
                    "setting linkgraph.recalc_time 9000",
                    "setting linkgraph.recalc_interval 4",
                    "unpause",
                ],
            )
            base = run("rail", setup, 3000)
            if not (
                rows(base)[17]["train[0]/common[0]/tile"] == 10007
                and rows(base)[17]["train[0]/common[0]/subtype"] & 1
            ):
                raise RuntimeError(
                    "reference rail commands did not park actual front train 17 on plain rail"
                )
            if "large" in kind:
                base = run(
                    "large-airport",
                    dict(
                        scenario,
                        kind="save",
                        save=str(base),
                        action="large-airport",
                        console=["unpause"],
                    ),
                    64,
                )
                if rows(base, "STNN")[0]["normal[0]/airport.type"] != 1:
                    raise RuntimeError("actual large-airport construction failed")
        family = scenario.get(
            "family",
            "zeppelin"
            if kind.startswith("zeppelin")
            else "big-ufo"
            if kind.startswith("train")
            else "small-ufo",
        )
        year, seed, expected = FAMILIES[family]
        if family == "small-ufo" and station:
            year, seed = 1969, 4
        elif kind.startswith("train"):
            year, seed = 2099, None
        elif kind in ("disabled", "empty-year", "countdown-wrap"):
            year = 2130 if kind == "empty-year" else 1956
        date = {
            "date": calendar(year),
            "date_fract": 73,
            "economy_date_fract": 73,
            "next_disaster_start": 0 if kind == "countdown-wrap" else 1,
        }
        if kind.startswith("train"):
            date.pop("date_fract")
            date.pop("economy_date_fract")
        if seed is not None:
            date["random_state[0]"] = date["random_state[1]"] = seed
        changes = {"DATE": {0: date}}
        if kind.startswith("train"):
            changes["PLYR"] = {1: {"is_ai": 0}}
        target = folder / "input.sav"
        record = patch(
            base, target, changes, "human" if kind.startswith("train") else None
        )
        schema = next(
            field["fields"]
            for field in read_save(target)["VEHS"]["header"]
            if field["key"] == "disaster"
        )
        if schema != [{"key": key, "type": dtype} for key, dtype in SCHEMA]:
            raise RuntimeError("unexpected VEHS disaster schema")
        console = [
            f"setting difficulty.disasters {int(kind not in ('disabled', 'countdown-wrap'))}",
            "unpause",
        ]
        scenario = dict(
            scenario,
            kind="save",
            save=str(target),
            console=console,
            expected_subtypes=expected,
        )
        extra = kind in (
            "industry-reset",
            "industry-release",
            "road-release",
            "sub-expiry",
            "sub-motion",
            "target-reload",
        )
        if extra:
            natural = run(
                f"natural-{family}-{'rail' if station else 'world'}", scenario, 1
            )
            live = disasters(natural)
            subtype = expected[0]
            main = next(
                index for index, row in live.items() if row["subtype"] == subtype
            )
            record["negative_fields"] = negative_probes(natural, folder, main)
            edits = {"DATE": {0: {"next_disaster_start": 65535}}}
            params = {}
            if kind.startswith("industry"):
                industry = next(
                    index
                    for index, row in rows(natural, "INDY").items()
                    if row["type"] == (4 if family == "airplane" else 6)
                    and (row["location.tile"] // 256) * 16 + 37 == live[main]["y_pos"]
                )
                fields = {
                    "disaster[0]/state": 1,
                    "disaster[0]/dest_tile": industry,
                    "disaster[0]/age": 111 if kind == "industry-reset" else 0,
                    "disaster[0]/tick_counter": 255,
                }
                params["industry"] = industry
                if kind == "industry-release":
                    edits["CHTS"] = {0: {"magic_bulldozer.value": 1}}
                    params.update(
                        action="industry",
                        target=rows(natural, "INDY")[industry]["location.tile"],
                    )
            elif kind == "road-release":
                fields = {"disaster[0]/state": 1, "disaster[0]/dest_tile": 12}
                params.update(action="road-sale", target=12)
            elif kind == "target-reload":
                target_road = rows(natural)[21]
                fields = {
                    "disaster[0]/state": 1,
                    "disaster[0]/dest_tile": 21,
                    "disaster[0]/x_pos": target_road["roadveh[0]/common[0]/x_pos"],
                    "disaster[0]/y_pos": target_road["roadveh[0]/common[0]/y_pos"],
                    "disaster[0]/z_pos": 200,
                }
            else:
                fields = (
                    {"disaster[0]/age": 8880, "disaster[0]/tick_counter": 255}
                    if kind == "sub-expiry"
                    else {}
                )
            edits["VEHS"] = {main: fields}
            modified = folder / "live-input.sav"
            record["live_input"] = patch(natural, modified, edits)
            console = [
                "setting difficulty.disasters 0",
                *(
                    ["start_ai MigrationDisasters"]
                    if kind == "industry-release"
                    else []
                ),
                "unpause",
            ]
            scenario = dict(
                scenario,
                save=str(modified),
                console=console,
                main=main,
                before=live[main],
                **params,
            )
            if kind == "target-reload":
                reload = run("road-live-target", scenario, 16)
                saved = disasters(reload).get(main)
                if not saved or saved["state"] != 1 or saved["dest_tile"] != 21:
                    raise RuntimeError("reload input lacks live UFO/road target")
                modified = folder / "reload-input.sav"
                record["reload"] = patch(reload, modified, {})
                scenario = dict(scenario, save=str(modified), before=saved)
        record["moment"] = save_moment(Path(scenario["save"]))
        result["disaster_input"] = record
    return scenario


def check(scenario, run, mode, role, result):
    if "disasters" not in scenario or run["exit"] or not run["snapshots"]:
        return
    path = run["snapshots"][-1]
    elapsed = save_moment(path)[2] - save_moment(Path(scenario["save"]))[2]
    if elapsed != scenario["ticks"]:
        raise RuntimeError(
            f"disaster witness elapsed {elapsed}, expected {scenario['ticks']}"
        )
    live, kind = disasters(path), scenario["disasters"]
    observed = tuple(sorted(row["subtype"] for row in live.values()))
    date = rows(path, "DATE")[0]
    events = [
        line.rsplit("DISASTER-EVENT ", 1)[1]
        for line in run["log"]
        if "DISASTER-EVENT " in line
    ]
    if kind == "init" and observed != scenario["expected_subtypes"]:
        raise RuntimeError(f"natural scheduler omitted subtype family: {observed}")
    if kind in ("disabled", "empty-year", "countdown-wrap") and live:
        raise RuntimeError("ineligible scheduler created disaster")
    if kind == "countdown-wrap" and date["next_disaster_start"] != 65535:
        raise RuntimeError("disaster countdown did not underflow")
    if (
        kind in ("init", "disabled", "empty-year")
        and not 730 <= date["next_disaster_start"] <= 1241
    ):
        raise RuntimeError("scheduler delay reset omitted")
    if kind.startswith("zeppelin"):
        airport = rows(path, "STNN")[0]["normal[0]/airport.flags"]
        if scenario["ticks"] == 5000:
            if (
                not any(
                    row["subtype"] == 0 and row["state"] == 3 for row in live.values()
                )
                or airport != 4611686018427388160
                or "zeppelin-crashed 0" not in events
            ):
                raise RuntimeError("zeppelin crash, block or delivered event omitted")
        elif live or airport != 0 or "zeppelin-cleared 0" not in events:
            raise RuntimeError("zeppelin clear, removal or delivered event omitted")
    if kind == "small-ufo-crash" and not any(
        event.startswith("vehicle-crashed ") and event.split()[2] == "2"
        for event in events
    ):
        raise RuntimeError("small UFO did not crash a real road vehicle")
    if kind == "train-target" and not any(
        row["subtype"] == 9 and row["state"] == 1 and row["dest_tile"] == 10007
        for row in live.values()
    ):
        raise RuntimeError("big UFO did not select actual human front train")
    if kind == "train-landing":
        train = rows(path)[17]
        if (
            not any(row["subtype"] == 9 and row["state"] == 2 for row in live.values())
            or not any(row["subtype"] == 11 for row in live.values())
            or train["train[0]/common[0]/breakdown_ctr"] == 0
            or not 230 <= train["train[0]/common[0]/breakdown_delay"] <= 240
        ):
            raise RuntimeError(
                "big UFO landing did not break down the nearby real train"
            )
    if kind == "train-clear":
        types = read_save(path)["MAPT"]["raw"]
        if (
            observed != (11, 12)
            or any(types[tile] >> 4 != 0 for tile in (10005, 10006))
            or types[10007] >> 4 != 1
        ):
            raise RuntimeError(
                "destroyer, ground clearing or occupied-rail retention omitted"
            )
    if kind == "train-complete" and live:
        raise RuntimeError("destroyer did not expire")
    if kind == "init" and scenario["family"] == "coal":
        before_types = read_save(Path(scenario["save"]))["MAPT"]["raw"]
        after_types = read_save(path)["MAPT"]["raw"]
        changed = [
            tile
            for tile, (a, b) in enumerate(zip(before_types, after_types, strict=True))
            if a >> 4 != b >> 4
        ]
        if len(changed) != 7 or (
            len({tile // 256 for tile in changed}) != 1
            and len({tile % 256 for tile in changed}) != 1
        ):
            raise RuntimeError("coal subsidence did not clear the selected tile line")
    if kind in (
        "industry-reset",
        "industry-release",
        "road-release",
        "sub-expiry",
        "sub-motion",
        "target-reload",
    ):
        main, before = scenario["main"], scenario["before"]
        current = live.get(main)
        if kind == "sub-expiry" and current:
            raise RuntimeError("submarine did not expire after age overflow boundary")
        if kind == "sub-motion" and (
            not current
            or all(
                current[key] == before[key] for key in ("x_pos", "y_pos", "direction")
            )
        ):
            raise RuntimeError("submarine did not move or turn in water")
        if kind.startswith("industry"):
            if not current or current["state"] != (
                2 if kind == "industry-reset" else 3
            ):
                raise RuntimeError("industry destruction phase or release hook omitted")
            if kind == "industry-release" and (
                scenario["industry"] in rows(path, "INDY")
                or f"industry-close {scenario['industry']}" not in events
            ):
                raise RuntimeError(
                    "industry removal command or delivered event omitted"
                )
            if kind == "industry-reset":
                chunks = read_save(path)
                tiles = [
                    tile
                    for tile, value in enumerate(chunks["MAPT"]["raw"])
                    if value >> 4 == 8
                    and struct.unpack_from(">H", chunks["MAP2"]["raw"], tile * 2)[0]
                    == scenario["industry"]
                ]
                if not tiles or any(
                    chunks["MAPO"]["raw"][tile] & 0x8F for tile in tiles
                ):
                    raise RuntimeError(
                        "industry tile construction stages were not reset"
                    )
        if kind == "road-release" and (
            12 in rows(path)
            or not current
            or current["state"] != 0
            or "road-sale" not in " ".join(run["log"])
        ):
            raise RuntimeError("road vehicle sale did not release UFO")
        if kind == "target-reload" and (
            not current or current["state"] != 1 or current["dest_tile"] != 21
        ):
            raise RuntimeError("reloaded UFO did not retain its live road target")
    result[f"{mode}_{role}_disasters"] = {
        "elapsed_ticks": elapsed,
        "vehicles": live,
        "delay": date["next_disaster_start"],
        "events": events,
    }


def scenarios(soak):
    cases = [(f"init-{family}", "init", 1, family) for family in FAMILIES]
    cases += [
        (kind, kind, 1, "coal") for kind in ("disabled", "empty-year", "countdown-wrap")
    ]
    cases += [
        (f"{kind}-{ticks}", kind, ticks, "zeppelin")
        for kind in ("zeppelin-small", "zeppelin-large")
        for ticks in (5000, 19000)
    ]
    cases += [
        ("small-ufo-crash", "small-ufo-crash", 19000, "small-ufo"),
        ("train-target", "train-target", 12000, "big-ufo"),
        ("train-landing", "train-landing", 13564, "big-ufo"),
        ("train-clear", "train-clear", 14064, "big-ufo"),
        ("train-complete", "train-complete", 20000, "big-ufo"),
    ]
    cases += [
        (
            f"industry-{family}-{action}",
            f"industry-{action}",
            1 if action == "reset" else 64,
            family,
        )
        for family in ("airplane", "helicopter")
        for action in ("reset", "release")
    ]
    cases += [
        ("road-release", "road-release", 64, "small-ufo"),
        ("target-reload", "target-reload", 64, "small-ufo"),
    ]
    cases += [
        (f"{family}-{action}", f"sub-{action}", 1 if action == "expiry" else 64, family)
        for family in ("small-sub", "big-sub")
        for action in ("expiry", "motion")
    ]
    if soak:
        cases += [
            (f"soak-{family}", "init-soak", 9000, family)
            for family in ("small-sub", "big-sub", "airplane", "helicopter")
        ]
    return [
        {
            "name": f"disasters-{label}",
            "kind": "save",
            "disasters": kind,
            "family": family,
            "ticks": ticks,
            "short_checkpoint": True,
        }
        for label, kind, ticks, family in cases
    ]
