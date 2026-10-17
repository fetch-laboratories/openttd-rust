"""Signal traversal, return classifications and transient work-set evidence (#219)."""

import hashlib
import json
from pathlib import Path

from . import core
from .console_adapter import console_adapter

ADAPTER = core.ROOT / "tools/simulation/signals-console.cpp"


def scenarios(soak):
    cases = [
        {
            "name": f"signals-{year}",
            "kind": "save",
            "save": str(core.ROOT / f"migration/saves/padhattan-ridge-{year}.sav"),
            "signals": str(year),
            "ticks": (2 if soak else 1) * 365 * core.TICKS_PER_DAY
            + core.SNAPSHOT_TICKS,
            "console": ["signals_scenario ordinary", "unpause"],
        }
        for year in (1996, 2000, 2006)
    ]

    cases.append(
        dict(
            cases[0],
            name="signals-worksets",
            signals="sets",
            short_checkpoint=True,
            ticks=1,
            console=["signals_scenario sets", "unpause"],
        )
    )
    cases.append(
        dict(
            cases[0],
            name="signals-overflow",
            signals="overflow",
            short_checkpoint=True,
            ticks=74 if soak else 1,
            console=["signals_scenario overflow", "unpause"],
        )
    )
    cases.append(
        dict(
            cases[1],
            name="signals-reload",
            signals="reload",
            short_checkpoint=True,
            ticks=74,
            console=["signals_scenario inspect-live", "unpause"],
        )
    )
    cases.append(
        dict(
            cases[2],
            name="signals-rail-commands",
            signals="rail-commands",
            ticks=45 * core.TICKS_PER_DAY + core.SNAPSHOT_TICKS,
            console=["signals_scenario rail-commands", "unpause"],
        )
    )
    return cases


# Rail command owner branches (#265) reached by the rail-commands scenario.
RAIL_COMMAND_BRANCHES = (
    "build_rail",
    "build_not_plain",
    "build_convert",
    "build_remove_signals",
    "build_crossing",
    "build_clear",
    "remove_rail",
    "remove_crossing",
    "remove_signal_charge",
    "remove_diagonal_crossing",
    "drag_build",
    "drag_remove",
    "drag_obstacle",
    "depot_build",
    "depot_rotate",
    "signal_build",
    "signal_convert",
    "signal_variant",
    "signal_cycle_type",
    "signal_cycle_side",
    "signal_copy",
    "signal_pbs_state",
    "signal_reserve",
    "signal_drag",
    "signal_drag_remove",
    "signal_minimise",
    "signal_autofill",
    "signal_wormhole",
    "signal_remove",
    "convert_rail",
    "convert_depot",
    "convert_wormhole",
    "convert_station_crossing",
    "convert_reroute",
    "clear_rail",
    "clear_depot",
    "owner_transfer",
    "owner_clear",
    "terraform_corner",
    "terraform_depot",
    "build_above_clear",
)


def install(scenario, run_dir):
    if "signals" in scenario:
        (run_dir / "signals-live.scr").write_text("signals_scenario inspect-live\n")
        (run_dir / "rail-live.scr").write_text("signals_scenario rail-live\n")


def game_environment(scenario):
    return (
        core.WITNESS_ENVIRONMENT
        if scenario.get("signals") and scenario.get("witnesses")
        else {}
    )


def adapter(build, folder, env):
    """Include unchanged signal.cpp to expose portable private sets to inspection.

    The adapter replaces its ordinary signal object; the pinned source is never
    edited. The same input/snapshot code is used on both sides. Candidate state
    inspection calls the Rust diagnostic entry without resetting its owner.
    """
    folder.mkdir(parents=True, exist_ok=True)
    commands = json.loads((build / "compile_commands.json").read_text())
    signal = Path(
        next(row["file"] for row in commands if row["file"].endswith("/signal.cpp"))
    )
    source = folder / "signals-source.cpp"
    # Only the fixture binary replaces the two occupancy slots. Its link wrapper
    # counts actual canonical GetTrack calls inside the original native queries.
    native = (
        signal.read_text()
        .replace(
            ".train_on_tile = SignalTrainOnTile,",
            ".train_on_tile = SignalCountTrainOnTile,",
        )
        .replace(
            ".train_on_tracks = SignalTrainOnTracks,",
            ".train_on_tracks = SignalCountTrainOnTracks,",
        )
    )
    native = native.replace(
        '#include "rust/signal_ffi.h"',
        '#include "rust/signal_ffi.h"\n'
        "static bool SignalCountTrainOnTile(uint32_t) noexcept;\n"
        "static bool SignalCountTrainOnTracks(uint32_t, uint8_t) noexcept;",
    )
    source.write_text(
        native
        + ADAPTER.read_text().replace(
            "../../../src/rust/signal_ffi.h", str(core.ROOT / "src/rust/signal_ffi.h")
        )
    )
    binary, receipt = console_adapter(
        build,
        folder,
        env,
        source,
        "signals",
        wraps=("openttd_rust_train_state_get_track",),
        replace_objects=("/src/signal.cpp.o",),
    )
    receipt["adapter_sha256"] = hashlib.sha256(ADAPTER.read_bytes()).hexdigest()
    receipt["signal_source_sha256"] = hashlib.sha256(signal.read_bytes()).hexdigest()
    return binary, receipt


def prepare(scenario, binaries, builds, out, timeout, env, result):
    if "signals" not in scenario:
        return scenario
    import migration

    from .rails import normalize
    from .runtime import prepared_adapter

    folder = out / scenario["name"] / "input"
    source, receipt = normalize(Path(scenario["save"]), folder / "normalized")
    result["signals_input"] = receipt
    native_builds = {
        "reference": migration.REFERENCE_BUILD,
        "candidate": migration.REFERENCE_BUILD
        if binaries["reference"] == binaries["candidate"]
        else core.ROOT / "build-rust",
    }
    executables, provenance = {}, {}
    for role in ("reference", "candidate"):
        prepared = prepared_adapter("signals", role, binaries, builds, folder / role)
        if prepared is not None:
            executables[role], provenance[role] = prepared
        else:
            lock = (
                migration.reference_lock(shared=True)
                if native_builds[role] == migration.REFERENCE_BUILD
                else migration.candidate_lock(native_builds[role], shared=True)
            )
            with lock:
                executables[role], provenance[role] = adapter(
                    native_builds[role], folder / role, env
                )
        if (
            provenance[role]["ordinary_binary_sha256"]
            != hashlib.sha256(binaries[role].read_bytes()).hexdigest()
        ):
            raise RuntimeError("signal adapter differs from frozen build")
        for name in core.RUNTIME_DIRECTORIES:
            if (builds[role] / name).is_dir():
                (folder / role / name).symlink_to(
                    builds[role] / name, target_is_directory=True
                )
    if binaries["reference"] == binaries["candidate"]:
        # Reuse the identical reference adapter in --self. Distinct output paths
        # would incorrectly make the core expect Rust-only branch witnesses.
        executables["candidate"] = executables["reference"]
        provenance["candidate"] = provenance["reference"]
    result["signals_adapter"] = provenance
    if scenario["signals"] == "reload":
        saves, logs = {}, {}
        capture = dict(
            scenario,
            save=str(source),
            ticks=32 * core.TICKS_PER_DAY + core.SNAPSHOT_TICKS,
            console=["signals_scenario ordinary", "unpause"],
        )
        for role in ("reference", "candidate"):
            run = core.run_game(
                capture,
                executables[role],
                builds[role],
                folder / role / "capture",
                timeout,
                env,
                desync=False,
            )
            if run["exit"] or not run["snapshots"]:
                raise RuntimeError("signal reload capture failed")
            saves[role] = run["snapshots"][-1]
            logs[role] = [line for line in run["log"] if line.startswith("SIGNALS ")]
        differences = core.compare_saves(
            saves["reference"],
            saves["candidate"],
            scenario["name"],
            20,
            result["stats"],
        )
        if differences or logs["reference"] != logs["candidate"]:
            raise RuntimeError("signal live reload input differs")
        result["signals_capture"] = {
            role: {
                "sha256": hashlib.sha256(save.read_bytes()).hexdigest(),
                "witnesses": logs[role],
            }
            for role, save in saves.items()
        }
        normalized = {
            role: normalize(save, folder / role / "reload")[0]
            for role, save in saves.items()
        }
        return dict(
            scenario,
            executables=executables,
            role_inputs={
                role: {"save": str(save)} for role, save in normalized.items()
            },
        )
    return dict(scenario, save=str(source), executables=executables)


def boundary_profile(counts):
    if counts is None:
        return None
    total = sum(
        counts[name]
        for name in (
            "entries",
            "services",
            "context_acquisitions",
            "context_owner_reads",
            "occupancy_accessors",
        )
    )
    copied = (
        counts["bytes"]
        + counts["context_bytes"]
        + counts["context_owner_reads"]
        + counts["occupancy_accessors"]
        + counts["classification_bytes"]
    )
    return {
        "counts": counts,
        "total_crossings": total,
        "returned_bytes": copied,
        "top_level_services_per_tile": counts["services"] / counts["tiles"]
        if counts["tiles"]
        else None,
        "total_crossings_per_tile": total / counts["tiles"]
        if counts["tiles"]
        else None,
        "total_crossings_per_update": total / counts["updates"]
        if counts["updates"]
        else None,
        "returned_bytes_per_tile": copied / counts["tiles"]
        if counts["tiles"]
        else None,
        "returned_bytes_per_update": copied / counts["updates"]
        if counts["updates"]
        else None,
        "scope": "ordinary facade entries, signal world services, direct train/reservation context and owner reads, actual nested occupancy GetTrack accessors; returned typed records/context/scalars include padding; excludes call arguments, diagnostic snapshots/probe and witness reporting; counters disabled during timing",
    }


def check(scenario, run, mode, role, result):
    if "signals" not in scenario or run["exit"] or not run["snapshots"]:
        return
    lines = [line for line in run["log"] if line.startswith("SIGNALS ")]
    if scenario["signals"] == "rail-commands":
        lines = [line for line in run["log"] if line.startswith("RAILCMD ")]
        if len(lines) < 150 or not any("live-reserved" in line for line in lines):
            result["problems"].append(f"{mode}/{role}: missing rail command results")
        key = mode + "_rail_commands"
        if role == "reference":
            result[key] = lines
        elif result[key] != lines:
            result["problems"].append(f"{mode}: rail command results differ")
        core.branch_witnesses(scenario, run, role, "rail_cmd", RAIL_COMMAND_BRANCHES)
        return
    if scenario["signals"] == "reload":
        for name in ("live-states", "pending live"):
            if not any(line.startswith(f"SIGNALS {name} ") for line in lines):
                result["problems"].append(
                    f"{mode}/{role}: missing live signal reload witness"
                )
        key = mode + "_signals_reload"
        if role == "reference":
            result[key] = lines
        elif result[key] != lines:
            result["problems"].append(f"{mode}: live signal reload state differs")
        counts = core.branch_witnesses(
            scenario,
            run,
            role,
            "signal",
            ("updates", "tiles", "train", "reserved_skip"),
        )
        if counts is not None:
            result[mode + "_signals_profile"] = boundary_profile(counts)
        return
    if scenario["signals"] == "overflow":
        for line in (
            "SIGNALS overflow 1",
            "SIGNALS follow-up 0",
            "SIGNALS owner-barrier 0",
        ):
            if line not in lines:
                result["problems"].append(f"{mode}/{role}: missing {line}")
        for name in ("overflow-reset", "overflow-follow-up"):
            if not any(
                line.startswith(f"SIGNALS pending {name} 0 0 0 false false false ")
                for line in lines
            ):
                result["problems"].append(
                    f"{mode}/{role}: missing reset/follow-up state"
                )
        key = mode + "_signals_overflow"
        if role == "reference":
            result[key] = lines
        elif result[key] != lines:
            result["problems"].append(f"{mode}: signal overflow input/state differs")
        counts = core.branch_witnesses(
            scenario,
            run,
            role,
            "signal",
            (
                "tbu_overflow",
                "reset",
                "owner_barrier",
                "neighbour_continue",
                "multi_exit",
                "multi_green",
                "combo_red",
            ),
        )
        if counts is not None:
            result[mode + "_signals_profile"] = boundary_profile(counts)
        return
    if scenario["signals"] == "sets":
        for size in (64, 256, 128):
            if not any(line.startswith(f"SIGNALS sets {size} ") for line in lines):
                result["problems"].append(
                    f"{mode}/{role}: missing unchanged-body fixed-set probe {size}"
                )
        key = mode + "_signals_worksets"
        if role == "reference":
            result[key] = lines
        elif result[key] != lines:
            result["problems"].append(f"{mode}: fixed-set source probe differs")
        return
    for name in (
        "initial",
        "duplicates",
        "public-flush",
        "below-threshold",
        "forced-flush",
        "private-flush",
        "commands",
    ):
        if not any(line.startswith(f"SIGNALS pending {name} ") for line in lines):
            result["problems"].append(
                f"{mode}/{role}: missing signal pending-state witness {name}"
            )
    if "SIGNALS rng true" not in lines or not any(
        line.startswith("SIGNALS states ") for line in lines
    ):
        result["problems"].append(f"{mode}/{role}: missing signal return/RNG witness")
    live = [line for line in lines if line.startswith("SIGNALS live-states ")]
    if not live or any(int(count) == 0 for count in live[0].split()[2:]):
        result["problems"].append(
            f"{mode}/{role}: missing free/occupied/PBS return classifications during live play"
        )
    key = mode + "_signals_witnesses"
    if role == "reference":
        result[key] = lines
    elif result[key] != lines:
        result["problems"].append(
            f"{mode}: native signal state/command witnesses differ"
        )
    from .rails import vehicle_rows

    positions = {}
    for path in [Path(scenario["save"]), *run["snapshots"]]:
        for vehicle, row in vehicle_rows(path).items():
            if row["type"] == 0:
                positions.setdefault(vehicle, set()).add(
                    (row["tile"], row["x_pos"], row["y_pos"])
                )
    moving = [vehicle for vehicle, poses in positions.items() if len(poses) > 1]
    if not moving:
        result["problems"].append(
            f"{mode}/{role}: signal fixture has no actual train movement"
        )
    result[f"{mode}_{role}_signals_moving"] = moving
    counts = core.branch_witnesses(
        scenario,
        run,
        role,
        "signal",
        (
            "updates",
            "tiles",
            "forced_flush",
            "changed",
            "train",
            "depot",
            "split",
            "pbs",
            "multi_enter",
            "exit",
            "green_exit",
            "station",
            "reserved_skip",
            "combo",
            "propagate",
            *(("crossing",) if scenario["signals"] in ("2000", "2006") else ()),
            *(("wormhole",) if scenario["signals"] == "2006" else ()),
        ),
    )
    if counts is not None:
        result[mode + "_signals_profile"] = boundary_profile(counts)
