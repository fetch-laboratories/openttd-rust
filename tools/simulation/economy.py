"""Cargo payment/delivery evidence using the existing road corpus."""

import hashlib
import struct
import subprocess
from pathlib import Path

from . import core
from .core import ROOT, SNAPSHOT_TICKS, decode_element, field_spans, read_save, run_game
from .play_saves import DISTRIBUTIONS


def scenarios(soak):
    return [
        {
            "name": f"economy-{mode}",
            "kind": "save",
            "save": str(ROOT / "migration/saves/grok-159-001.sav"),
            "snapshot_minimum": 0 if mode == "stockpile" else 2,
            "short_checkpoint": mode == "stockpile",
            "console": (
                ["setting order.gradual_loading 0"] if mode == "stockpile" else []
            )
            + DISTRIBUTIONS["cargodist" if mode == "cargodist" else "manual"],
            "economy": mode,
            "ticks": 32
            if mode == "stockpile"
            else (12 if soak else 4) * SNAPSHOT_TICKS,
        }
        for mode in ("manual", "cargodist", "stockpile")
    ] + [
        dict(
            name="subsidies-" + mode,
            kind="save",
            save=str(ROOT / "migration/saves/padhattan-ridge-2000.sav"),
            engines="subsidies" + ("-reload" if mode == "reload" else ""),
            short_checkpoint=True,
            ticks=1,
        )
        for mode in ("control", "reload")
    ]


def rows(chunks, cid):
    return {i: decode_element(chunks[cid], body) for i, body in chunks[cid]["elements"]}


def pending(chunks):
    """Actual unloading coal vehicle with a live payment, not a synthetic packet."""
    vehicles = rows(chunks, "VEHS")
    for payment in rows(chunks, "CAPY").values():
        index = payment["front"] - 1
        v = vehicles[index]
        prefix = "roadveh[0]/common[0]/"
        if (
            v.get(prefix + "cargo_type") == 1
            and v[prefix + "cargo.action_counts"][1] > 5
        ):
            return index
    return None


def prepare(scenario, binaries, builds, out, timeout, env, result):
    if "economy" not in scenario:
        return scenario
    setup = dict(scenario, ticks=8 * SNAPSHOT_TICKS)
    if scenario["economy"] == "stockpile":
        setup["console"] = DISTRIBUTIONS["manual"]
    run = run_game(
        setup,
        binaries["reference"],
        builds["reference"],
        out / scenario["name"] / "prepare",
        timeout,
        env,
    )
    candidates = [p for p in run["snapshots"] if read_save(p)["CAPY"]["elements"]]

    def witness(path):
        chunks = read_save(path)
        if scenario["economy"] == "stockpile":
            return pending(chunks) is not None
        field = (
            "visual_transfer" if scenario["economy"] == "cargodist" else "route_profit"
        )
        return any(payment[field] != 0 for payment in rows(chunks, "CAPY").values())

    fixture = next((p for p in candidates if witness(p)), None)
    if run["exit"] or fixture is None:
        raise RuntimeError("reference preparation lacks active CAPY/unloading witness")
    if scenario["economy"] == "cargodist":
        # Mature flows above came from frequent recalculation. Drain overdue
        # reference jobs before the payment test, whose cadence is deliberately
        # long; null-video loops count async reload pauses as iterations.
        drain = dict(scenario, save=str(fixture), ticks=4 * SNAPSHOT_TICKS)
        drain["console"] = [
            c for c in scenario["console"] if "linkgraph.recalc_time " not in c
        ]
        drain["console"] = ["setting linkgraph.recalc_time 9000", *drain["console"]]
        drained = run_game(
            drain,
            binaries["reference"],
            builds["reference"],
            out / scenario["name"] / "drain",
            timeout,
            env,
        )

        def future_jobs(path):
            chunks = read_save(path)
            date = core.save_moment(path)[0]
            return chunks["CAPY"]["elements"] and all(
                job["join_date"] > date for job in rows(chunks, "LGRJ").values()
            )

        fixture = next((p for p in drained["snapshots"] if future_jobs(p)), None)
        if drained["exit"] or fixture is None:
            raise RuntimeError("reference preparation did not drain overdue link jobs")
        scenario = dict(
            scenario,
            console=[
                c
                for c in scenario["console"]
                if not any(
                    key in c
                    for key in ("linkgraph.recalc_time ", "linkgraph.recalc_interval ")
                )
            ],
        )
        scenario["console"] = [
            "setting linkgraph.recalc_time 9000",
            "setting linkgraph.recalc_interval 90",
            *scenario["console"],
        ]
    original = read_save(fixture)
    source = fixture.read_bytes()
    data = bytearray(source)
    patches = []

    def patch(cid, index, field, value):
        chunk = original[cid]
        body = dict(chunk["elements"])[index]
        begin, end, kind = field_spans(body, chunk["header"])[field]
        if source.count(body, *chunk["span"]) != 1:
            raise RuntimeError("ambiguous economy input row")
        start = source.index(body, *chunk["span"]) + begin
        fmt, _ = core.FILE_TYPES[kind & 15]
        encoded = struct.pack(fmt, value)
        if len(encoded) != end - begin:
            raise RuntimeError("economy patch width mismatch")
        data[start : start + len(encoded)] = encoded
        patches.append((cid, index, field, value))

    if scenario["economy"] == "stockpile":
        # Reloaded overdue link jobs can consume the null driver's entire short
        # unload window while waiting for worker threads. Move their input join
        # dates together, retaining relative order, beyond this checkpoint.
        for index, job in rows(original, "LGRJ").items():
            patch("LGRJ", index, "join_date", job["join_date"] + 32)
        front = pending(original)
        prefix = "roadveh[0]/common[0]/"
        # Stop other transport vehicles, make this real staged unload immediate,
        # and leave exactly five units of capacity in every industry's stockpile.
        for i, v in rows(original, "VEHS").items():
            if v.get("type") != 1:
                continue
            if i == front:
                patch("VEHS", i, prefix + "load_unload_ticks", 1)
            else:
                patch("VEHS", i, prefix + "vehstatus", v[prefix + "vehstatus"] | 2)
        for i, industry in rows(original, "INDY").items():
            patch("INDY", i, "was_cargo_delivered", 0)
            for field in industry:
                if (
                    field.startswith("accepted[")
                    and field.endswith("/waiting")
                    and "/history" not in field
                ):
                    patch("INDY", i, field, 65530)
        result["stockpile_front"] = front
    # Normalize only reference-derived input history, as existing road reloads do.
    begin, end = original["GLOG"]["span"]
    normalized = fixture.parent / "economy-input.sav"
    normalized.write_bytes(data[:begin] + data[end:])
    checked = read_save(normalized)
    if set(checked) != set(original) - {"GLOG"}:
        raise RuntimeError("economy input changed the chunk set")
    result["economy_input"] = {
        "reference_sha256": hashlib.sha256(source).hexdigest(),
        "normalized_sha256": hashlib.sha256(normalized.read_bytes()).hexdigest(),
        "payments": rows(original, "CAPY"),
        "patches": patches,
    }
    return dict(scenario, save=str(normalized))


def check(scenario, run, mode, role, result):
    if "economy" not in scenario or not run["snapshots"]:
        return
    saved = [read_save(p) for p in run["snapshots"]]
    if scenario["economy"] == "stockpile":
        delivered = [
            i for i in rows(saved[-1], "INDY").values() if i["was_cargo_delivered"]
        ]
        if not delivered or not any(
            i.get("accepted[0]/waiting") == 0 for i in delivered
        ):
            result["problems"].append(f"{mode}/{role}: no near-full stockpile flush")
        # The pending coal packet has more than five pieces; only five can fit.
        before = read_save(Path(scenario["save"]))
        old = rows(before, "PLYR")
        new = rows(saved[-1], "PLYR")
        delta = sum(
            v["cur_economy[0]/delivered_cargo"][1]
            - old[i]["cur_economy[0]/delivered_cargo"][1]
            for i, v in new.items()
        )
        if delta != 5:
            result["problems"].append(
                f"{mode}/{role}: expected partial acceptance 5, saw {delta}"
            )
        result[f"{mode}_{role}_partial_acceptance"] = delta
        return
    payments = [p for c in saved for p in rows(c, "CAPY").values()]

    def delivered(chunks):
        return sum(
            sum(value)
            for company in rows(chunks, "PLYR").values()
            for field, value in company.items()
            if field.endswith("/delivered_cargo")
        )

    delivered_delta = delivered(saved[-1]) - delivered(
        read_save(Path(scenario["save"]))
    )
    if not any(p["route_profit"] != 0 for p in payments) and delivered_delta <= 0:
        result["problems"].append(f"{mode}/{role}: no final delivery payment")
    if scenario["economy"] == "cargodist":
        # In this vanilla fixture new packets have zero feeder share; split/merge
        # preserves total credit and delivery removes it. A net increase requires
        # new PayTransfer credit, rather than merely inheriting loaded shares.
        def feeder(chunks):
            return sum(p["feeder_share"] for p in rows(chunks, "CAPA").values())

        credit_states = [read_save(Path(scenario["save"])), *saved]
        if any(
            packet["feeder_share"] < 0
            for chunks in credit_states
            for packet in rows(chunks, "CAPA").values()
        ):
            result["problems"].append(
                f"{mode}/{role}: negative feeder credit invalidates transfer witness"
            )
        before = feeder(credit_states[0])
        after = max(feeder(chunks) for chunks in saved)
        result[f"{mode}_{role}_feeder_credit"] = {"before": before, "after": after}
        if after <= before:
            result["problems"].append(
                f"{mode}/{role}: no new transfer credit ({before} -> {after})"
            )
    result[f"{mode}_{role}_payment_samples"] = len(payments)
    result[f"{mode}_{role}_delivered_delta"] = delivered_delta


def income_gap():
    """Native-width/negative callback and Money bounds absent from road fixtures."""
    import importlib.util

    spec = importlib.util.spec_from_file_location(
        "migration", ROOT / "tools/migration.py"
    )
    migration = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(migration)
    build = ROOT / "build-rust"
    out = ROOT / ".local/cargo-income-gap"
    out.mkdir(parents=True, exist_ok=True)
    source = subprocess.check_output(
        ["git", "show", migration.BASELINE["commit"] + ":src/economy.cpp"], cwd=ROOT
    ).decode()

    def body(start, end):
        offset = source.index(start)
        return source[offset : source.index(end, offset)]

    (out / "cargo-reference.inc").write_text(
        "\n".join(
            body(start, end)
            for start, end in (
                ("static inline int32_t BigMulS(", "typedef std::vector<Industry *>"),
                ("Money GetTransportedGoodsIncome(", "/** The industries"),
                ("Money CargoPayment::PayTransfer(", "/**\n * Prepare the vehicle"),
            )
        )
    )
    flush_start = source.index("\tfor (Industry *iid : _cargo_delivery_destinations)")
    flush_end = source.index(
        "_cargo_delivery_destinations.clear();", flush_start
    ) + len("_cargo_delivery_destinations.clear();")
    (out / "cargo-destination-reference.inc").write_text(
        body(
            "/** The industries we've currently brought cargo to. */",
            "/**\n * Delivers goods to industries/towns",
        )
        + body(
            "static void TriggerIndustryProduction(Industry *i)",
            "/**\n * Makes us a new cargo payment helper.",
        )
        + "static void FlushCargoDestinationReference() {\n"
        + source[flush_start:flush_end]
        + "\n}\n"
    )
    command = [
        "c++",
        "-std=c++20",
        "-O2",
        "-DPOINTER_IS_64BIT",
        "-I" + str(ROOT / "src"),
        "-I" + str(out),
        str(ROOT / "tools/migration/cargo-income-gap.cpp"),
        str(migration.rust_archive(build)),
        "-ldl",
        "-lpthread",
        "-lm",
        "-o",
        str(out / "probe"),
    ]
    subprocess.run(command, cwd=ROOT, env=migration.environment(), check=True)
    subprocess.run([str(out / "probe")], cwd=ROOT, check=True)


if __name__ == "__main__":
    income_gap()


def subsidy_reference(folder):
    """Unchanged monthly lambda isolates wrap, disabled expiry and retry RNG gaps."""
    import migration

    source = migration.git("show", migration.BASELINE["commit"] + ":src/subsidy.cpp")
    start = source.index("\n{", source.index("_economy_subsidies_monthly")) + 1
    end = source.index("\n});", start)
    folder.mkdir(parents=True, exist_ok=True)
    (folder / "subsidy-monthly-reference.inc").write_text(
        "static void SubsidyMonthlyLoop()\n" + source[start:end] + "\n}\n"
    )
