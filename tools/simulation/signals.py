"""Signal traversal, return classifications and transient work-set evidence."""

import hashlib
import json
from pathlib import Path

from . import core
from .console_adapter import console_adapter, prepared_console

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
    for case in cases:
        case["console"] = ["pbs_probe", *case["console"]]
    return cases


# Rail command owner (rail_cmd.rs) branches reached by the rail-commands scenario.
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


def adapter(build: Path, folder: Path, env: dict[str, str]) -> Path:
    """Include unchanged signal.cpp to expose portable private sets to inspection.

    The adapter replaces its ordinary signal object; the pinned source is never
    edited. The same input/snapshot code is used on both sides. Candidate state
    inspection calls the Rust diagnostic entry without resetting its owner.
    """
    folder.mkdir(parents=True, exist_ok=True)
    from .rails import pbs_reference, pbs_wraps

    pbs_reference(folder)
    commands = json.loads((build / "compile_commands.json").read_text())
    signal = Path(
        next(row["file"] for row in commands if row["file"].endswith("/signal.cpp"))
    )
    source = folder / "signals-source.cpp"
    source.write_text(
        f'#line 1 "{signal}"\n'
        + signal.read_text()
        + f'\n#line 1 "{source}"\n'
        + (core.ROOT / "tools/simulation/pbs-console.cpp")
        .read_text()
        .split("// PBS registration")[0]
        + ADAPTER.read_text()
        .replace(
            "../../../src/rust/signal_ffi.h", str(core.ROOT / "src/rust/signal_ffi.h")
        )
        .replace(
            '#include "roads-console.cpp"',
            (core.ROOT / "tools/simulation/roads-console.cpp").read_text(),
        )
    )
    return console_adapter(
        build,
        folder,
        env,
        source,
        "signals",
        wraps=pbs_wraps(build),
        native_test_support=True,
        replace_objects=("/src/signal.cpp.o",),
    )


def prepare(scenario, binaries, builds, out, timeout, env, result):
    if "signals" not in scenario:
        return scenario

    from .core import normalize

    folder = out / scenario["name"] / "input"
    source, record = normalize(Path(scenario["save"]), folder / "normalized")
    result["signals_input"] = record
    executables = prepared_console("signals", adapter, binaries, builds, folder, env)
    if binaries["reference"] == binaries["candidate"]:
        # Reuse the identical reference adapter in --self. Distinct output paths
        # would incorrectly make the core expect Rust-only branch witnesses.
        executables["candidate"] = executables["reference"]
    if scenario["signals"] == "reload":
        saves, logs = {}, {}
        capture = dict(
            scenario,
            save=str(source),
            ticks=32 * core.TICKS_PER_DAY + core.SNAPSHOT_TICKS,
            console=["signals_scenario ordinary", "unpause"],
        )
        runs = core.capture_pair(capture, executables, builds, folder, timeout, env)
        for role, run in runs.items():
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
        "scope": "ordinary facade entries, signal world services, direct train/reservation context and owner reads, direct occupancy GetTrack reads; returned typed records/context/scalars include padding; excludes call arguments, diagnostic snapshots/probe and witness reporting; counters disabled during timing",
    }


def check(scenario, run, mode, role, result):
    if "signals" not in scenario or run["exit"] or not run["snapshots"]:
        return
    if not any("PBS checked" in line for line in run["log"]):
        raise RuntimeError("missing default PBS transient comparison")
    if scenario["signals"] in ("2006", "rail-commands") and not any(
        "PBS ends 1 1" in line for line in run["log"]
    ):
        raise RuntimeError("missing occupied tunnel/bridge end comparisons")
    core.branch_witnesses(
        scenario,
        run,
        role,
        "pbs",
        ("follow", "loop", "lower_id", "platform_interior")
        + (
            ("other_end_train",)
            if scenario["signals"] in ("2006", "rail-commands")
            else ()
        ),
    )
    lines = [line for line in run["log"] if line.startswith("SIGNALS ")]
    if scenario["signals"] == "rail-commands":
        lines = [line for line in run["log"] if line.startswith("RAILCMD ")]
        if len(lines) < 150 or not any("live-reserved" in line for line in lines):
            result["problems"].append(f"{mode}/{role}: missing rail command results")
        core.branch_witnesses(scenario, run, role, "rail_cmd", RAIL_COMMAND_BRANCHES)
        return
    if scenario["signals"] == "reload":
        for name in ("live-states", "pending live"):
            if not any(line.startswith(f"SIGNALS {name} ") for line in lines):
                result["problems"].append(
                    f"{mode}/{role}: missing live signal reload witness"
                )
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
