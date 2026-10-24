"""play saves scenario evidence."""

import hashlib
import subprocess
import threading
from pathlib import Path

from .core import (
    ROOT,
    SNAPSHOT_TICKS,
    TICKS_PER_DAY,
    WITNESS_ENVIRONMENT,
    branch_witnesses,
    decode_element,
    read_save,
    run_game,
    strip_glog,
)

# Road networks built by LLM players in OpenTTD 15.1 (migration/saves/README.md),
# loaded with their scripts dropped. The first two run by default.
PLAY_SAVES = (
    "opus-55-167-002",
    "grok-159-001",
    "astra-156-003",
    "opus-55-165-002",
    "opus-5-133-009",
    "fable-152-004",
)
# Console commands run from scripts/game_start.scr after loading; the saves were
# written paused. cargodist uses short link graph intervals so jobs recur often.
DISTRIBUTIONS = {
    "manual": ["unpause"],
    "cargodist": [
        "setting linkgraph.distribution_pax 2",
        "setting linkgraph.distribution_mail 2",
        "setting linkgraph.distribution_default 1",
        "setting linkgraph.recalc_interval 4",
        "setting linkgraph.recalc_time 16",
        "unpause",
    ],
}


_SCHEDULER_LOCK = threading.Lock()
_SCHEDULER_REPORTS = {}


def scheduler_reference(out):
    """Default fixture for worker states unavailable with deterministic thread fallback."""
    import migration

    out.mkdir(parents=True, exist_ok=True)
    bodies = []
    for file, signatures, destination in (
        (
            "linkgraphschedule.cpp",
            (
                "void LinkGraphSchedule::SpawnNext()",
                "bool LinkGraphSchedule::IsJoinWithUnfinishedJobDue() const",
                "void LinkGraphSchedule::JoinNext()",
                "void LinkGraphSchedule::SpawnAll()",
                "void LinkGraphSchedule::Clear()",
                "void LinkGraphSchedule::ShiftDates(",
                "void StateGameLoop_LinkGraphPauseControl()",
                "void AfterLoad_LinkGraphPauseControl()",
                "void OnTick_LinkGraph()",
            ),
            "scheduler-original.inc",
        ),
        (
            "linkgraphschedule.h",
            ("void Queue(", "void Dequeue("),
            "scheduler-queue-original.inc",
        ),
    ):
        source = subprocess.check_output(
            ["git", "show", migration.BASELINE["commit"] + ":src/linkgraph/" + file],
            cwd=ROOT,
            text=True,
        )
        bodies.clear()
        for signature in signatures:
            start = source.index(signature)
            end = source.index("{", start) + 1
            depth = 1
            while depth:
                depth += (source[end] == "{") - (source[end] == "}")
                end += 1
            bodies.append(source[start:end])
        (out / destination).write_text("\n".join(bodies))
    build = ROOT / "build-rust"
    binary = out / "scheduler-reference"
    command = [
        "c++",
        "-std=c++20",
        "-O2",
        "-I" + str(ROOT / "src"),
        "-I" + str(out),
        str(Path(__file__).with_name("linkgraph-schedule-reference.cpp")),
        str(migration.rust_archive(build)),
        *migration.rust_configuration(build)["native_libraries"],
        "-o",
        str(binary),
    ]
    subprocess.run(
        command, env=migration.environment(), check=True, capture_output=True
    )
    run = subprocess.run([str(binary)], check=True, capture_output=True, text=True)
    if (
        not run.stdout.startswith("PASS ")
        or not run.stdout.endswith(" unchanged-reference scheduler observations\n")
        or int(run.stdout.split()[1]) == 0
    ):
        raise RuntimeError("scheduler reference fixture omitted completion marker")
    return run.stdout.strip()


def scenarios(soak):
    scenarios = []
    years = 6 if soak else 2
    for save in PLAY_SAVES if soak else PLAY_SAVES[:2]:
        for distribution, commands in DISTRIBUTIONS.items():
            scenarios.append(
                {
                    "name": f"play-{save}-{distribution}",
                    "kind": "save",
                    "console": commands,
                    "save": str(ROOT / "migration/saves" / f"{save}.sav"),
                    "ticks": years * 365 * TICKS_PER_DAY + SNAPSHOT_TICKS,
                }
            )
    # Exercise both scalers for pax/mail/armoured, asymmetric default cargo, and
    # limits of the demand/distance rules and both MCF passes. Reload reuses an original snapshot containing
    # outstanding jobs, whose input/settings/join dates are saved in LGRJ.
    # This temperate save uses cargo IDs 0/2 for passengers/mail.
    variants = [
        (
            "symmetric",
            2,
            {
                "demand_distance": 255,
                "demand_size": 100,
                "accuracy": 2,
                "short_path_saturation": 0,
            },
        ),
        (
            "asymmetric",
            1,
            {
                "demand_distance": 0,
                "demand_size": 0,
                "accuracy": 64,
                "short_path_saturation": 250,
            },
        ),
    ]
    for label, distribution, settings in variants:
        commands = [
            f"setting linkgraph.distribution_{cargo} {min(distribution, 1) if cargo == 'default' else distribution}"
            for cargo in ("pax", "mail", "armoured", "default")
        ]
        commands += [
            f"setting linkgraph.{key} {value}" for key, value in settings.items()
        ]
        commands += [
            "setting linkgraph.recalc_interval 4",
            "setting linkgraph.recalc_time 16",
            "unpause",
        ]
        scenarios.append(
            {
                "name": f"play-{PLAY_SAVES[0]}-cargodist-{label}-reload",
                "kind": "save",
                "save": str(ROOT / "migration/saves" / f"{PLAY_SAVES[0]}.sav"),
                "console": commands,
                "reload_chunk": "LGRJ",
                "reload_cargos": [0, 2],
                "ticks": years * 365 * TICKS_PER_DAY + SNAPSHOT_TICKS,
            }
        )
    return scenarios


def prepare(scenario, binaries, builds, out, timeout, env, result):
    if "reload_chunk" in scenario:
        if binaries["candidate"] != binaries["reference"]:
            with _SCHEDULER_LOCK:
                if out not in _SCHEDULER_REPORTS:
                    _SCHEDULER_REPORTS[out] = scheduler_reference(
                        out / "scheduler-reference"
                    )
                result["scheduler_reference"] = _SCHEDULER_REPORTS[out]
        # The fixture comes from the unchanged reference only. Outstanding
        # jobs recompute from their saved graph/settings on load in both.
        setup = dict(scenario, ticks=3 * SNAPSHOT_TICKS)
        prepared = run_game(
            setup,
            binaries["reference"],
            builds["reference"],
            out / scenario["name"] / "prepare",
            timeout,
            env,
            True,
        )
        if prepared["exit"] != 0:
            raise RuntimeError(f"reload preparation exited with {prepared['exit']}")
        chunk = scenario["reload_chunk"]
        fixture = None
        for path in prepared["snapshots"]:
            chunks = read_save(path)
            saved = chunks.get(chunk, {})
            queues = decode_element(chunks["LGRS"], chunks["LGRS"]["elements"][0][1])
            graph_ids = {index + 1 for index, _ in chunks["LGRP"]["elements"]}
            job_ids = {index + 1 for index, _ in saved.get("elements", [])}
            if not (
                queues["schedule"]
                and queues["running"]
                and set(queues["schedule"]) <= graph_ids
                and set(queues["running"]) <= job_ids
            ):
                continue
            cargos = {
                fields["linkgraph[0]/cargo"]
                for _, body in saved.get("elements", [])
                if (fields := decode_element(saved, body)) is not None
            }
            if (
                saved.get("elements")
                and set(scenario.get("reload_cargos", [])) <= cargos
            ):
                fixture = path
                break
        if fixture is None:
            raise RuntimeError(
                f"reload preparation saved no outstanding {chunk} records for required cargo IDs"
            )
        original = read_save(fixture)
        result["reload_queues"] = queues
        result["reload_records"] = len(original[chunk]["elements"])
        result["reload_cargos"] = sorted(
            {
                fields["linkgraph[0]/cargo"]
                for _, body in original[chunk]["elements"]
                if (fields := decode_element(original[chunk], body)) is not None
            }
        )
        # GLOG history includes settings, GRF and cheat events.
        data = fixture.read_bytes()
        normalized = fixture.parent / "reload-input.sav"
        strip_glog(data, normalized)
        normalized_data = normalized.read_bytes()
        result["reload_input"] = {
            "reference": str(fixture),
            "normalized": str(normalized),
            "reference_sha256": hashlib.sha256(data).hexdigest(),
            "normalized_sha256": hashlib.sha256(normalized_data).hexdigest(),
            "removed_chunk": "GLOG",
        }
        scenario = dict(scenario, save=str(normalized))
    return scenario


PREPARE_ORDER = 3


def counted_road_save(scenario):
    return any(
        scenario.get("name", "").startswith(f"play-{save}-")
        for save in (*PLAY_SAVES, "padhattan-ridge-2000")
    )


def game_environment(scenario):
    return (
        WITNESS_ENVIRONMENT
        if counted_road_save(scenario) and scenario.get("witnesses")
        else {}
    )


def check(scenario, run, mode, role, result):
    if not counted_road_save(scenario):
        return
    if "reload_chunk" in scenario:
        result[f"{mode}_{role}_schedule"] = branch_witnesses(
            scenario,
            run,
            role,
            "schedule",
            ("queue", "spawn", "join", "requeue", "reload_job", "import"),
        )
    counts = branch_witnesses(
        scenario, run, role, "road", ("ticks", "services", "entries")
    )
    if counts is None:
        return
    crossings = counts["services"] + counts["visitors"]
    density = crossings / counts["ticks"]
    result[f"{mode}_{role}_road_crossings"] = dict(
        counts,
        services_and_visitors_per_tick=density,
        controller_entries_per_tick=counts["entries"] / counts["ticks"],
        observed_controller_crossings_per_tick=(crossings + counts["entries"])
        / counts["ticks"],
    )
    # Plain mode budgets 13 outgoing services plus visitor callbacks per tick.
    # Snapshot desync checks additionally rebuild/check caches; their calls and
    # selected controller ingress are reported separately.
    if (
        mode == "plain"
        and scenario["name"] == "play-padhattan-ridge-2000-manual"
        and density > 13
    ):
        raise RuntimeError(f"road boundary exceeds 13 crossings/tick: {density:.3f}")
