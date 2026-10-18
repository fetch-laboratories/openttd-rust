"""Base scheduler and maintenance evidence over actual transport saves (#213)."""

from pathlib import Path

from . import core, rails
from .disasters import patch, rows

OPUS = "play-opus-55-167-002-manual"
PREFIXES = (
    "play-",
    "roads-",
    "rail-",
    "water-ferry-",
    "aircraft-",
    "effects-",
    "fleet-",
    "vehicles-",
)


COMMON = "roadveh[0]/common[0]/"
MAINTENANCE = {
    "age-value": (0, ("age", "decay", "profit_age", "value", "check", "days")),
    "breakdown-start": (0, ("started", "smoke", "delay", "ended")),
    "breakdown-countdown": (0, ("countdown", "started", "ended")),
    "breakdown-normal": (2, ("check", "trigger")),
    "breakdown-reduced": (1, ("check", "trigger")),
    "service-percent": (0, ("service", "percent")),
}


def scenarios(soak):
    # Short typed input boundaries supplement the unchanged multi-year saves.
    return [
        {
            "name": "vehicles-" + kind,
            "kind": "save",
            "save": str(core.ROOT / "migration/saves/opus-55-167-002.sav"),
            "vehicle_maintenance": kind,
            "short_checkpoint": True,
            "ticks": 148,
            "console": [
                f"setting difficulty.vehicle_breakdowns {breakdowns}",
                "setting order.no_servicing_if_no_breakdowns false",
                "unpause",
            ],
        }
        for kind, (breakdowns, _) in MAINTENANCE.items()
    ]


def prepare(scenario, binaries, builds, out, timeout, env, result):
    if "vehicle_maintenance" not in scenario:
        return scenario
    kind = scenario["vehicle_maintenance"]
    folder = out / scenario["name"] / "input"
    source, normalization = rails.normalize(Path(scenario["save"]), folder)
    original = rows(source)[3]
    if original["type"] != 1 or original[COMMON + "vehstatus"] & 130:
        raise RuntimeError("maintenance fixture is not an active road head")
    changes = {}
    if kind == "age-value":
        changes = {
            "age": 3659,
            "max_age": 3660,
            "economy_age": 730,
            "reliability_spd_dec": 257,
            "day_counter": 7,
            "value": 65535,
        }
    elif kind in ("breakdown-start", "breakdown-countdown"):
        changes = {
            "breakdown_ctr": 2 if kind.endswith("start") else 4,
            "breakdown_delay": 2,
            "breakdowns_since_last_service": 254,
        }
    elif kind in ("breakdown-normal", "breakdown-reduced"):
        changes = {
            "breakdown_ctr": 0,
            "reliability": 0,
            "breakdown_chance": 255,
        }
    elif kind == "service-percent":
        changes = {
            "vehicle_flags": original[COMMON + "vehicle_flags"] | (1 << 8) | (1 << 9),
            "service_interval": 50,
            "reliability": 0,
        }
    destination = folder / "maintenance.sav"
    receipt = patch(
        source,
        destination,
        {"VEHS": {3: {COMMON + key: value for key, value in changes.items()}}},
    )
    result["vehicle_maintenance_input"] = {
        "normalization": normalization,
        "patch": receipt,
    }
    return dict(scenario, save=str(destination))


def selected(scenario):
    return scenario.get("name", "").startswith(PREFIXES)


def game_environment(scenario):
    return (
        core.WITNESS_ENVIRONMENT
        if selected(scenario) and scenario.get("witnesses")
        else {}
    )


def check(scenario, run, mode, role, result):
    if not selected(scenario):
        return
    if scenario.get("vehicle_maintenance") == "age-value":
        final = rows(run["snapshots"][-1])[3]
        if not (
            final[COMMON + "age"] >= 3660
            and final[COMMON + "economy_age"] > 730
            and final[COMMON + "reliability_spd_dec"] == 514
            and final[COMMON + "value"] == 65280
        ):
            raise RuntimeError("maintenance age/value boundary was not reached")
    required = ["outer_visits", "crossings", "observed_bytes", "scalar_read_bytes"]
    if scenario.get("name") == OPUS:
        required += [
            "visits.road",
            "days",
            "stations",
            "motion",
            "post_tick",
            "cargo_owner",
            "packet_lookups",
            "running_sound",
            "periodic_sound",
            "descriptor_bytes",
            "scalar_write_bytes",
        ]
    counts = core.branch_witnesses(scenario, run, role, "vehicle", required)
    if counts is None:
        return
    visits = counts["outer_visits"]
    # Post-Tick type is sampled only after survival; deleted visits remain in
    # the actual outer denominator without dereferencing a destroyed shell.
    surviving = sum(
        counts[f"visits.{kind}"]
        for kind in (
            "train",
            "road",
            "ship",
            "aircraft",
            "effect",
            "disaster",
            "end",
            "invalid",
        )
    )
    if surviving + counts["deleted"] != visits:
        raise RuntimeError("scheduler survival accounting")
    maintenance_required = (
        ["entries", "crossings", "bytes"]
        if scenario.get("vehicle_maintenance") or scenario["name"] == OPUS
        else []
    )
    if kind := scenario.get("vehicle_maintenance"):
        maintenance_required += list(MAINTENANCE[kind][1])
    maintenance = core.branch_witnesses(
        scenario, run, role, "vehicle_maintenance", maintenance_required
    )
    if maintenance is not None:
        result[f"{mode}_{role}_vehicle_maintenance"] = {
            "counts": maintenance,
            # Native entries plus both selected owners' outgoing calls. Existing
            # controller/fleet internals retain their separate instrumentation.
            "selected_boundary_calls": counts["crossings"]
            + maintenance["crossings"]
            + maintenance["entries"],
            "selected_boundary_calls_per_outer_visit": (
                counts["crossings"] + maintenance["crossings"] + maintenance["entries"]
            )
            / visits,
        }
    result[f"{mode}_{role}_vehicle_scheduler"] = {
        "outer_visits": visits,
        "counts": counts,
        "crossings_per_outer_visit": counts["crossings"] / visits,
        "observed_bytes_per_outer_visit": counts["observed_bytes"] / visits,
        "scalar_read_bytes_per_outer_visit": counts["scalar_read_bytes"] / visits,
        "scalar_write_bytes_per_outer_visit": counts["scalar_write_bytes"] / visits,
    }
