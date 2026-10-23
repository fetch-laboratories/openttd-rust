"""effects scenario evidence."""

import hashlib
import json
import struct
import threading
from pathlib import Path

from .core import (
    FILE_TYPES,
    ROOT,
    Reader,
    decode_element,
    field_spans,
    read_save,
    run_game,
    save_moment,
)
from .play_saves import PLAY_SAVES

EFFECT_PREPARE_LOCK = threading.Lock()

EFFECT_SCHEMA = list(
    zip(
        "subtype tile x_pos y_pos z_pos sprite_cache.sprite_seq.seq[0].sprite progress vehstatus animation_state animation_substate spritenum".split(),
        (2, 6, 5, 5, 5, 4, 2, 2, 4, 2, 2),
        strict=True,
    )
)
EFFECT_SPRITES = (
    3701,
    3079,
    3073,
    3084,
    2040,
    3709,
    3737,
    3725,
    1416,
    4751,
    2040,
    2040,
)


def effect_rows(path):
    chunk = read_save(path)["VEHS"]
    rows = {}
    for index, body in chunk["elements"]:
        fields = decode_element(chunk, body)
        if fields is None:
            raise RuntimeError("undecodable VEHS evidence")
        if fields["type"] == 4:
            rows[index] = {
                k.removeprefix("effect[0]/"): v
                for k, v in fields.items()
                if k != "type"
            }
    return rows


def prepare_effects(scenario, binaries, builds, out, timeout, env):
    setup = dict(scenario, ticks=600)
    if scenario["effects"] == "breakdown":
        setup.update(
            ticks=6000,
            kind="save",
            save=str(ROOT / "migration/saves" / f"{PLAY_SAVES[0]}.sav"),
            console=[
                f"setting linkgraph.distribution_{cargo} 0"
                for cargo in ("pax", "mail", "default", "armoured")
            ]
            + [
                "setting linkgraph.recalc_time 9000",
                "setting linkgraph.recalc_interval 4",
                "unpause",
            ],
        )
    # The scenarios share three immutable reference outputs; each prepares its
    # own patched input below. Serial creation avoids races and repeated road runs.
    folder = (
        "road"
        if scenario["effects"] == "breakdown"
        else setup["settings"]["game_creation"]["landscape"]
    )
    cache = out / "effect-fixtures" / folder
    fixture = cache / "save/autosave/exit.sav"
    preparation = cache / "preparation.json"
    with EFFECT_PREPARE_LOCK:
        if not preparation.is_file():
            attempts = []
            for iterations in (
                (6000, 12000, 24000) if scenario["effects"] == "breakdown" else (600,)
            ):
                run = run_game(
                    dict(setup, ticks=iterations),
                    binaries["reference"],
                    builds["reference"],
                    cache,
                    timeout,
                    env,
                    False,
                )
                if run["exit"] != 0 or not fixture.is_file():
                    raise RuntimeError("effect reference preparation failed")
                prepared = read_save(fixture)
                deadlines = [
                    decode_element(prepared["LGRJ"], body)["join_date"]
                    for _, body in prepared.get("LGRJ", {}).get("elements", [])
                ]
                attempts.append(
                    {
                        "loop_iterations": iterations,
                        "saved_moment": save_moment(fixture),
                        "job_deadlines": deadlines,
                        "sha256": hashlib.sha256(fixture.read_bytes()).hexdigest(),
                    }
                )
                # Old loaded jobs can stall null-driver iterations. Drain them
                # through the unchanged game; never patch their dates or state.
                if scenario["effects"] != "breakdown" or all(
                    decode_element(prepared["LGRJ"], body)["join_date"]
                    > save_moment(fixture)[0] + 56
                    for _, body in prepared["LGRJ"]["elements"]
                ):
                    break
            else:
                raise RuntimeError(
                    "short road fixture still has a job due within 4099 ticks"
                )
            preparation.write_text(json.dumps(attempts, indent=2) + "\n")
    chunks, source = read_save(fixture), fixture.read_bytes()
    vehicles = chunks["VEHS"]
    effect_header = next(
        f["fields"] for f in vehicles["header"] if f["key"] == "effect"
    )
    if vehicles["kind"] != 4 or effect_header != [
        {"key": key, "type": kind} for key, kind in EFFECT_SCHEMA
    ]:
        raise RuntimeError("unexpected VEHS effect schema")
    natural = effect_rows(fixture)
    if scenario["effects"] == "chimney" and not any(
        row["subtype"] == 0
        and 3701 <= row["sprite_cache.sprite_seq.seq[0].sprite"] <= 3708
        and row["progress"] <= 7
        for row in natural.values()
    ):
        raise RuntimeError("natural chimney construction was not observed")
    if scenario["effects"] == "bubble" and {
        row["animation_substate"] for row in natural.values() if row["subtype"] == 9
    } != {0, 1, 2, 3}:
        raise RuntimeError("natural industry bubble caller omitted direction bytes")
    data, patches, assignments = bytearray(source), [], []
    road_position = None
    variants = []
    if scenario["effects"] in ("controllers", "reload"):
        variants = [
            (
                f"subtype-{i}",
                {
                    "subtype": i,
                    "progress": 255,
                    "animation_state": 0,
                    "animation_substate": 1,
                    "spritenum": 0,
                    "sprite_cache.sprite_seq.seq[0].sprite": sprite,
                },
            )
            for i, sprite in enumerate(EFFECT_SPRITES)
        ]
        for subtype, last, progress in (
            (1, 3083, 3),
            (2, 3078, 0),
            (3, 3089, 2),
            (4, 2044, 3),
            (5, 3724, 3),
            (7, 3736, 3),
            (10, 2044, 3),
            (11, 2044, 3),
        ):
            variants.append(
                (
                    f"expiry-{subtype}",
                    {
                        "subtype": subtype,
                        "progress": progress,
                        "sprite_cache.sprite_seq.seq[0].sprite": last,
                    },
                )
            )
        variants += [
            (
                f"countdown-{count}",
                {
                    "subtype": 6,
                    "progress": 255,
                    "animation_state": count,
                    "sprite_cache.sprite_seq.seq[0].sprite": 3737,
                },
            )
            for count in (0, 1, 65535)
        ]
        variants += [
            (
                "bulldozer-end",
                {
                    "subtype": 8,
                    "progress": 7,
                    "animation_state": 19,
                    "animation_substate": 6,
                },
            )
        ]
        variants += [
            (
                f"bubble-direction-{direction}-{z}",
                {
                    "subtype": 9,
                    "progress": 3,
                    "animation_state": 3,
                    "spritenum": direction,
                    "z_pos": z,
                },
            )
            for direction in range(1, 5)
            for z in (180, 181)
        ]
        variants += [
            (
                f"bubble-generate-{direction}",
                {
                    "subtype": 9,
                    "progress": 3,
                    "animation_substate": direction,
                    "spritenum": 0,
                    "sprite_cache.sprite_seq.seq[0].sprite": 4753,
                },
            )
            for direction in (0, 1)
        ]
        variants += [
            (
                "bubble-burst-end",
                {"subtype": 9, "progress": 3, "animation_state": 3, "spritenum": 5},
            ),
            (
                "bubble-absorb",
                {"subtype": 9, "progress": 3, "animation_state": 77, "spritenum": 6},
            ),
            (
                "bubble-absorb-end",
                {"subtype": 9, "progress": 3, "animation_state": 83, "spritenum": 6},
            ),
            (
                "chimney-removed",
                {"subtype": 0, "progress": 0, "x_pos": 16, "y_pos": 16},
            ),
        ]
        width = struct.unpack(">II", chunks["MAPS"]["elements"][0][1])[0]
        animation = chunks["ANIT"]
        animated = decode_element(animation, animation["elements"][0][1])["tiles"]
        catcher = next(
            i
            for i, kind in enumerate(chunks["MAPT"]["raw"])
            if kind >> 4 == 8
            and chunks["MAP5"]["raw"][i] == 162
            and not chunks["MAPE"]["raw"][i] & 4
            and i not in animated
        )
        for label, values in variants:
            if label in ("subtype-0", "bubble-absorb"):
                values.update(
                    x_pos=(catcher % width) * 16 + 7,
                    y_pos=(catcher // width) * 16 + 7,
                    progress=0 if label == "subtype-0" else 3,
                )
    if variants and len(effect_rows(fixture)) < len(variants):
        raise RuntimeError("too few natural effects for controller witnesses")
    reader = Reader(source, vehicles["span"][0] + 5)
    reader.take(reader.gamma() - 1)
    pending = iter(variants)
    road_chosen = False
    for index, body in vehicles["elements"]:
        length = reader.gamma() - 1
        element_begin = reader.pos
        if reader.gamma() != index:
            raise RuntimeError("unexpected sparse VEHS numbering")
        begin = reader.pos
        if reader.take(length - (begin - element_begin)) != body:
            raise RuntimeError("unexpected sparse VEHS body")
        fields = decode_element(vehicles, body)
        label, values = "", {}
        if fields["type"] == 4:
            label, values = next(pending, ("", {}))
            values = {"effect[0]/" + key: value for key, value in values.items()}
        elif (
            scenario["effects"] == "breakdown"
            and not road_chosen
            and fields["type"] == 1
            and fields["roadveh[0]/common[0]/cur_speed"] > 0
            and not fields["roadveh[0]/common[0]/vehstatus"] & 0x83
        ):
            label, road_chosen = "road-breakdown", True
            road_position = [
                fields[f"roadveh[0]/common[0]/{key}"] + delta
                for key, delta in (("x_pos", 4), ("y_pos", 4), ("z_pos", 5))
            ]
            values = {
                "roadveh[0]/common[0]/breakdown_ctr": 2,
                "roadveh[0]/common[0]/breakdown_delay": 37,
            }
        spans = field_spans(body, vehicles["header"])
        for key, value in values.items():
            start, end, kind = spans[key]
            raw = struct.pack(FILE_TYPES[kind & 15][0], value)
            position = begin + start
            if end - start != len(raw):
                raise RuntimeError("effect patch width mismatch")
            patches.append(
                [
                    "VEHS",
                    position - vehicles["span"][0],
                    data[position : position + len(raw)].hex(),
                    raw.hex(),
                    f"{index}/{key}",
                ]
            )
            data[position : position + len(raw)] = raw
        if label:
            assignments.append([index, label, values])
    if scenario["effects"] == "breakdown" and not road_chosen:
        raise RuntimeError("no moving visible road vehicle for actual breakdown caller")
    begin, end = chunks["GLOG"]["span"]
    normalized = out / scenario["name"] / "prepare/effect-input.sav"
    normalized.parent.mkdir(parents=True, exist_ok=True)
    normalized.write_bytes(data[:begin] + data[end:])
    check, normalized_data = read_save(normalized), normalized.read_bytes()
    if set(check) != set(chunks) - {"GLOG"}:
        raise RuntimeError("effect input changed chunk inventory")
    for cid in check:
        expected = bytearray(source[slice(*chunks[cid]["span"])])
        for patched, offset, old, new, _ in patches:
            if patched == cid:
                if expected[offset : offset + len(bytes.fromhex(old))] != bytes.fromhex(
                    old
                ):
                    raise RuntimeError("effect patch old-byte preparation mismatch")
                expected[offset : offset + len(bytes.fromhex(new))] = bytes.fromhex(new)
        if expected != normalized_data[slice(*check[cid]["span"])]:
            raise RuntimeError("effect input changed unrelated bytes")
    record = {
        "reference": str(fixture),
        "normalized": str(normalized),
        "reference_sha256": hashlib.sha256(source).hexdigest(),
        "normalized_sha256": hashlib.sha256(normalized_data).hexdigest(),
        "removed_chunk": "GLOG",
        "patches": patches,
        "assignments": assignments,
        "natural_effects": natural,
        "preparation": json.loads(preparation.read_text()),
        "road_position": road_position,
    }
    if scenario["effects"] == "reload":
        # Capture still-live private state through the unchanged reference only.
        live = run_game(
            dict(scenario, kind="save", save=str(normalized), ticks=16),
            binaries["reference"],
            builds["reference"],
            out / scenario["name"] / "reload",
            timeout,
            env,
            False,
        )
        fixture = live["snapshots"][-1]
        if save_moment(fixture)[2] - save_moment(normalized)[2] != 16 or not any(
            r["animation_state"] for r in effect_rows(fixture).values()
        ):
            raise RuntimeError(
                "reload preparation lacks live private state at 16 ticks"
            )
        original = fixture.read_bytes()
        chunk = read_save(fixture)["GLOG"]
        normalized = fixture.parent / "effect-reload-input.sav"
        normalized.write_bytes(
            original[: chunk["span"][0]] + original[chunk["span"][1] :]
        )
        loaded, reloaded = read_save(fixture), read_save(normalized)
        reload_data = normalized.read_bytes()
        if set(reloaded) != set(loaded) - {"GLOG"} or any(
            original[slice(*loaded[cid]["span"])]
            != reload_data[slice(*reloaded[cid]["span"])]
            for cid in reloaded
        ):
            raise RuntimeError("effect reload input changed unrelated chunks")
        record["reload"] = {
            "reference": str(fixture),
            "reference_sha256": hashlib.sha256(original).hexdigest(),
            "normalized_sha256": hashlib.sha256(normalized.read_bytes()).hexdigest(),
            "effects": effect_rows(normalized),
        }
    return dict(
        scenario,
        kind="save",
        save=str(normalized),
        effect_assignments=assignments,
        effect_road=road_position,
    ), record


def check_effects(scenario, path):
    before, after = effect_rows(Path(scenario["save"])), effect_rows(path)
    elapsed = save_moment(path)[2] - save_moment(Path(scenario["save"]))[2]
    if elapsed != scenario["ticks"]:
        raise RuntimeError(
            f"effect checkpoint advanced {elapsed}, expected {scenario['ticks']} actual ticks"
        )
    changed = [index for index, row in before.items() if after.get(index) != row]
    if not changed:
        raise RuntimeError("effect checkpoint observed no transition")

    def animated_tiles(save):
        chunk = read_save(save)["ANIT"]
        return decode_element(chunk, chunk["elements"][0][1])["tiles"]

    added_animation = sorted(
        set(animated_tiles(path)) - set(animated_tiles(Path(scenario["save"])))
    )
    lifecycle = None
    if scenario["effects"] == "controllers" and elapsed == 1:
        prior = animated_tiles(Path(scenario["save"]))
        current = animated_tiles(path)
        if len(set(prior) & set(current)) < 2:
            raise RuntimeError("catcher insertion lacks two live ANIT survivors")
        lifecycle = ["effects-catcher", prior, current]
    labels = {label: index for index, label, _ in scenario["effect_assignments"]}
    if scenario["effects"] == "controllers" and elapsed == 1:
        if {before[labels[f"subtype-{i}"]]["subtype"] for i in range(12)} != set(
            range(12)
        ) or any(labels[f"subtype-{i}"] not in changed for i in range(12)):
            raise RuntimeError("controller checkpoint omitted a subtype transition")
        for label, index in labels.items():
            if (
                label.startswith("expiry-")
                or label.endswith("-end")
                or label in ("countdown-1", "chimney-removed")
            ):
                if index in after:
                    raise RuntimeError(f"effect expiry witness {label} remained live")
            elif label in ("countdown-0", "countdown-65535"):
                if after[index]["animation_state"] != (
                    65535 if label == "countdown-0" else 65534
                ):
                    raise RuntimeError(
                        "breakdown countdown witness did not wrap/decrement"
                    )
            elif label.startswith("bubble-direction-") and label.endswith("-181"):
                if after[index]["spritenum"] != 5:
                    raise RuntimeError("high bubble did not burst")
        if (
            after[labels["bubble-generate-0"]]["spritenum"] != 6
            or after[labels["bubble-generate-1"]]["spritenum"] not in range(1, 5)
            or after[labels["bubble-absorb"]]["animation_state"] != 79
        ):
            raise RuntimeError(
                "bubble generation/absorption witness missed its transition"
            )
        row = after[labels["bubble-absorb"]]
        maps = read_save(path)["MAPS"]
        width = struct.unpack(">II", maps["elements"][0][1])[0]
        if (row["y_pos"] // 16) * width + row["x_pos"] // 16 not in added_animation:
            raise RuntimeError(
                "bubble absorption did not add its natural catcher tile to ANIT"
            )
    created = sorted(set(after) - set(before))
    if scenario["effects"] == "bubble" and elapsed >= 16 and not created:
        raise RuntimeError("natural industry loop created no new bubble")
    if (
        scenario["effects"] == "breakdown"
        and elapsed == 1
        and not any(
            row["subtype"] == 6
            and row["animation_state"] == 73
            and row["progress"] == 1
            and [row[key] for key in ("x_pos", "y_pos", "z_pos")]
            == scenario["effect_road"]
            for index, row in after.items()
            if index in created
        )
    ):
        raise RuntimeError(
            "road breakdown constructor/duration setter was not observed"
        )
    date = read_save(path)["DATE"]
    fields = decode_element(date, date["elements"][0][1])
    return {
        "elapsed_ticks": elapsed,
        "added_animated_tiles": added_animation,
        "animated_tile_order": animated_tiles(path),
        "animation_lifecycle": lifecycle,
        "rng": [fields[f"random_state[{i}]"] for i in range(2)],
        "before": before,
        "after": after,
        "changed_ids": changed,
        "expired_ids": sorted(set(before) - set(after)),
        "created_ids": created,
    }


def scenarios(soak):
    scenarios = []
    # Exit checkpoints observe effects that disappear before the 32-day hook.
    for kind in ("controllers", "reload", "chimney", "bubble", "breakdown"):
        horizons = (
            (1, 4, 16, 80, 1024)
            if kind == "controllers"
            else (1, 16, 259 if kind == "chimney" else 256)
        )
        if soak:
            horizons += (4099,)
        for ticks in horizons:
            scenarios.append(
                {
                    "name": f"effects-{kind}-{ticks}",
                    "kind": "generate",
                    "effects": kind,
                    "ticks": ticks,
                    "seed": 1,
                    "map_log2": 9,
                    "land_generator": 1,
                    "settings": {
                        "game_creation": {
                            "landscape": "temperate"
                            if kind == "chimney"
                            else "toyland",
                            "starting_year": 1950,
                        },
                        "difficulty": {"number_industries": 4},
                    },
                }
            )
    return scenarios


def prepare(scenario, binaries, builds, out, timeout, env, result):
    if "effects" in scenario:
        scenario, result["effect_input"] = prepare_effects(
            scenario, binaries, builds, out, timeout, env
        )
    return scenario


def check(scenario, run, mode, role, result):
    if "effects" in scenario:
        result[f"{mode}_{role}_effects"] = check_effects(scenario, run["snapshots"][-1])


PREPARE_ORDER = 0
