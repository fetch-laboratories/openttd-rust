"""Cargo owners: active-list reload, forced staging, aging and packet references."""

import hashlib
import json
import runpy
import struct
import subprocess
from pathlib import Path

from . import core
from .core import ROOT, SNAPSHOT_TICKS, field_spans, read_save
from .play_saves import DISTRIBUTIONS

PREPARE_ORDER = 6
VEHICLE = "roadveh[0]/common[0]/"


def scenarios(soak):
    return [
        {
            "name": f"cargo-storage-{mode}",
            "kind": "save",
            "save": str(ROOT / "migration/saves/grok-159-001.sav"),
            "economy": "cargodist" if mode == "cargodist" else "manual",
            "cargo_storage": mode,
            "console": DISTRIBUTIONS["cargodist" if mode == "cargodist" else "manual"],
            "ticks": (8 if soak else 4) * SNAPSHOT_TICKS,
        }
        for mode in ("manual", "cargodist", "no-unload", "transfer", "unload")
    ]


def prepare(scenario, binaries, builds, out, timeout, env, result):
    if "cargo_storage" not in scenario:
        return scenario
    mode = scenario["cargo_storage"]
    if mode in ("manual", "cargodist"):
        return scenario
    original = read_save(Path(scenario["save"]))
    source = Path(scenario["save"]).read_bytes()
    data = bytearray(source)
    patches = []

    def patch(cid, index, field, value):
        chunk = original[cid]
        body = dict(chunk["elements"])[index]
        begin, end, kind = field_spans(body, chunk["header"])[field]
        if source.count(body, *chunk["span"]) != 1:
            raise RuntimeError("ambiguous cargo input row")
        encoded = struct.pack(core.FILE_TYPES[kind & 15][0], value)
        if len(encoded) != end - begin:
            raise RuntimeError("cargo patch width mismatch")
        offset = source.index(body, *chunk["span"]) + begin
        data[offset : offset + len(encoded)] = encoded
        patches.append((cid, index, field, value))

    unload = {"unload": 1, "transfer": 2, "no-unload": 4}[mode]
    for index, orders in core.rows(original, "ORDL").items():
        # Current saves store orders as nested records inside each shared list.
        # Only station orders use these unloading flag bits.
        for field, order_type in orders.items():
            if field.endswith("/type") and order_type & 15 == 1:
                flags = field.removesuffix("type") + "flags"
                patch("ORDL", index, flags, (orders[flags] & ~7) | unload)
    for index, vehicle in core.rows(original, "VEHS").items():
        if vehicle["type"] == 1:
            flags = vehicle[VEHICLE + "current_order.flags"]
            patch("VEHS", index, VEHICLE + "current_order.flags", (flags & ~7) | unload)
    prepared = Path(scenario["save"]).with_name(f"cargo-{mode}.sav")
    prepared.write_bytes(data)
    read_save(prepared)
    result["cargo_input"] = {
        "reference_sha256": hashlib.sha256(source).hexdigest(),
        "input_sha256": hashlib.sha256(data).hexdigest(),
        "patches": patches,
    }
    scenario = dict(scenario, save=str(prepared))
    del scenario["economy"]
    return scenario


def check(scenario, run, mode, role, result):
    if "cargo_storage" not in scenario or not run["snapshots"]:
        return
    before = read_save(Path(scenario["save"]))
    saved = [read_save(path) for path in run["snapshots"]]
    packets = core.rows(before, "CAPA")
    observed = []
    action_totals = [0, 0, 0, 0]
    for chunks in saved:
        current = core.rows(chunks, "CAPA")
        for vehicle in core.rows(chunks, "VEHS").values():
            if vehicle["type"] != 1:
                continue
            actions = vehicle[VEHICLE + "cargo.action_counts"]
            references = vehicle[VEHICLE + "cargo.packets"]
            count = sum(current[index - 1]["count"] for index in references)
            if sum(actions) != count:
                result["problems"].append(f"{mode}/{role}: cargo action counts differ")
            action_totals = [a + b for a, b in zip(action_totals, actions, strict=True)]
        observed.append(
            {
                "packets": len(current),
                "items": sum(packet["count"] for packet in current.values()),
                "aged": sum(
                    index in packets
                    and packet["periods_in_transit"]
                    > packets[index]["periods_in_transit"]
                    for index, packet in current.items()
                ),
                "feeder": sum(packet["feeder_share"] for packet in current.values()),
                "flow_records": sum(
                    field.endswith("/restricted")
                    for station in core.rows(chunks, "STNN").values()
                    for field in station
                ),
                "jobs": len(chunks["LGRJ"]["elements"]),
            }
        )
    if not any(row["aged"] for row in observed):
        result["problems"].append(f"{mode}/{role}: no persistent packet aged")
    kind = scenario["cargo_storage"]
    if kind == "transfer" and not any(row["feeder"] for row in observed):
        result["problems"].append(f"{mode}/{role}: no transfer feeder share")
    result[f"{mode}_{role}_cargo"] = {"actions": action_totals, "state": observed}


def scaling_probe():
    """Witness the large-list gap: saves currently reach at most six packets."""
    migration = runpy.run_path(str(ROOT / "tools/migration.py"))
    out = ROOT / ".local/cargo-list-scaling"
    out.mkdir(parents=True, exist_ok=True)
    fixture = Path(__file__).with_name("cargo-list-scaling.cpp")
    command = [
        "g++",
        "-std=c++20",
        "-O2",
        "-I",
        str(ROOT / "src"),
        str(fixture),
        str(migration["rust_archive"](ROOT / "build-rust")),
        "-ldl",
        "-lpthread",
        "-lm",
        "-o",
        str(out / "probe"),
    ]
    env = migration["environment"]()
    subprocess.run(command, env=env, check=True)
    output = subprocess.check_output([str(out / "probe")], env=env, text=True)
    report = {
        "gap": "large cargo queues absent from the committed play corpus",
        "candidate_commit": migration["git"]("rev-parse", "HEAD"),
        "candidate_status": migration["git"]("status", "--short"),
        "fixture_sha256": hashlib.sha256(fixture.read_bytes()).hexdigest(),
        "commands": [command, [str(out / "probe")]],
        "measurements": output.splitlines(),
        "checks": "identity/order/counts; 4 reads per first-hit append and keyed load",
        "limit": "synthetic operation timings; C++ loop omits packet/services",
    }
    (out / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(output, end="")


if __name__ == "__main__":
    scaling_probe()
