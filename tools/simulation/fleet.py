"""Fleet hierarchy, renewal and actual replacement transactions."""

import hashlib
from pathlib import Path

from .console_adapter import prepared_console
from .core import (
    ROOT,
    WITNESS_ENVIRONMENT,
    branch_witnesses,
    capture_pair,
    compare_saves,
)

ADAPTER = ROOT / "tools/simulation/fleet-console.cpp"
# Rust construction and clone branches each transaction scenario must reach.
BRANCHES = {
    "transaction-build-road": ["road", "restore"],
    "transaction-build-aircraft": ["aircraft", "rotor", "restore"],
    "transaction-clone": ["clone_join_sell"],
    "transaction-rail": [
        "rail_wagon_join",
        "rail_normalize",
        "rail_front_trashed",
        "rail_head_transfer",
        "rail_new_front",
        "rail_move_restore",
        "rail_sell_restore",
        "rail_multihead",
        "rail_dual_heads",
        "rail_sell_transfer",
    ],
}


def scenarios(soak):
    cases = [
        {
            "name": "fleet-" + mode,
            "kind": "save",
            "save": str(
                ROOT
                / "migration/saves"
                / (
                    "padhattan-ridge-1996.sav"
                    if mode.startswith("train")
                    or mode
                    in (
                        "transaction-refit-train",
                        "transaction-mass-train",
                        "transaction-clone",
                        "transaction-rail",
                        "resolver",
                    )
                    else "padhattan-ridge-2000.sav"
                    if mode.startswith("drain") or mode == "transaction-meta"
                    else "water-ferry.sav"
                    if mode
                    in ("ship", "transaction-refit-ship", "transaction-mass-ship")
                    else "aircraft-route.sav"
                    if mode
                    in (
                        "aircraft",
                        "transaction-refit-aircraft",
                        "transaction-mass-aircraft",
                        "transaction-build-aircraft",
                    )
                    else "opus-55-167-002.sav"
                )
            ),
            "fleet": mode,
            "short_checkpoint": True,
            # The drain cases run past the scheduled witness on 2001-01-01.
            "ticks": 400 if mode.startswith("drain") else 74 if soak else 1,
            "console": ["fleet_scenario " + mode, "unpause"],
        }
        for mode in (
            "groups",
            "resolver",
            "transaction-meta",
            "transaction-mass-train",
            "transaction-mass-road",
            "transaction-mass-ship",
            "transaction-mass-aircraft",
            "transaction-refit-train",
            "transaction-refit-road",
            "transaction-refit-ship",
            "transaction-refit-aircraft",
            "transaction-build-road",
            "transaction-build-aircraft",
            "transaction-clone",
            "transaction-rail",
            "list",
            "replace",
            "cash",
            "train",
            "train-rollback",
            "train-wagons",
            "ship",
            "aircraft",
            "drain",
            "drain-cash",
        )
    ]

    cases.append(dict(cases[0], name="fleet-reload", fleet="reload"))
    resolver = next(case for case in cases if case["fleet"] == "resolver")
    cases.append(dict(resolver, name="fleet-resolver-reload", fleet="resolver-reload"))
    return cases


def game_environment(scenario):
    witnessed = scenario.get("fleet") in BRANCHES and scenario.get("witnesses")
    return WITNESS_ENVIRONMENT if witnessed else {}


def install(scenario, run_dir):
    if scenario.get("fleet", "").startswith("drain"):
        (run_dir / "fleet-drain.scr").write_text("fleet_scenario drain-inspect\n")


def adapter(build, folder, env):
    from .console_adapter import console_adapter

    return console_adapter(build, folder, env, ADAPTER, "fleet")


def prepare(scenario, binaries, builds, out, timeout, env, result):
    if "fleet" not in scenario:
        return scenario

    folder = out / scenario["name"] / "input"
    if scenario["fleet"].startswith(("train", "drain")) or scenario["fleet"] in (
        "resolver",
        "resolver-reload",
        "transaction-meta",
        "transaction-refit-train",
        "transaction-mass-train",
        "transaction-clone",
        "transaction-rail",
    ):
        from .core import normalize

        source, record = normalize(Path(scenario["save"]), folder / "normalized")
        result["fleet_input"] = record
        scenario = dict(scenario, save=str(source))
    executables = prepared_console("fleet", adapter, binaries, builds, folder, env)
    if scenario["fleet"] in ("reload", "resolver-reload"):
        resolver = scenario["fleet"] == "resolver-reload"
        saves, capture, saved_resolver = {}, {}, {}
        setup = dict(
            scenario,
            console=[
                "fleet_scenario " + ("resolver" if resolver else "groups-store"),
                "unpause",
            ],
            ticks=1,
        )
        runs = capture_pair(setup, executables, builds, folder, timeout, env)
        for role, run in runs.items():
            if (
                run["exit"]
                or not run["snapshots"]
                or (
                    "FLEET resolver-checkpoint complete"
                    if resolver
                    else "FLEET groups true 1 1 true"
                )
                not in run["log"]
            ):
                raise RuntimeError("fleet reload capture lacks live groups/rules")
            saves[role] = run["snapshots"][-1]
            if resolver:
                check_override_order(run, "capture", role, result)
                check_property_boundaries(run, "capture", role, result)
                check_real_boundaries(run, "capture", role, result)
                from .core import rows

                marker = next(
                    (
                        line.split()
                        for line in run["log"]
                        if line.startswith("FLEET resolver-persist ")
                    ),
                    None,
                )
                if marker is None:
                    raise RuntimeError("missing resolver persistence input")
                index = int(marker[2])
                fields = rows(saves[role])[index]
                prefix = "train[0]/common[0]/"
                saved_resolver[role] = [
                    index,
                    fields[prefix + "random_bits"],
                    fields[prefix + "waiting_triggers"],
                ]
                if not all(saved_resolver[role][1:]):
                    raise RuntimeError("resolver save has empty random/trigger state")
            capture[role] = {
                "input_sha256": hashlib.sha256(saves[role].read_bytes()).hexdigest(),
                "witnesses": [line for line in run["log"] if line.startswith("FLEET ")],
            }
        differences = compare_saves(
            saves["reference"],
            saves["candidate"],
            scenario["name"],
            20,
            result["stats"],
        )
        if (
            differences
            or capture["reference"]["witnesses"] != capture["candidate"]["witnesses"]
        ):
            result["differences"].extend(differences)
            raise RuntimeError("fleet reload capture differs")
        result["fleet_capture"] = capture
        if resolver:
            result["resolver_saved"] = saved_resolver
        return dict(
            scenario,
            executables=executables,
            role_inputs={role: {"save": str(save)} for role, save in saves.items()},
            console=[
                "fleet_scenario " + ("resolver-inspect" if resolver else "inspect"),
                "unpause",
            ],
        )
    return dict(scenario, executables=executables)


def check_override_order(run, mode, role, result):
    # Frozen callback results from the unchanged native first-match policy.
    overrides = [
        line.split()[2:]
        for line in run["log"]
        if line.startswith("FLEET resolver-override-order ")
    ]
    expected_overrides = [
        [str(test), str(cargo), str(value)]
        for test, values in enumerate(
            ((311, 311), (419, 311), (311, 523), (523, 419), (311, 523), (523, 523))
        )
        for cargo, value in enumerate(values)
    ]
    if overrides != expected_overrides:
        result["problems"].append(f"{mode}/{role}: wagon override first-match order")
    restored_overrides = [
        line
        for line in run["log"]
        if line.startswith("FLEET resolver-override-restored ")
    ]
    if len(restored_overrides) != 1 or not restored_overrides[0].startswith(
        "FLEET resolver-override-restored true true "
    ):
        result["problems"].append(f"{mode}/{role}: wagon override restoration/RNG")


def check_property_boundaries(run, mode, role, result):
    # Explicit native callback-36 results; failure preserves the full int default.
    expected = []
    for value, signed in (
        (0, 0),
        (1, 1),
        (0x3FFE, 16382),
        (0x3FFF, 16383),
        (0x4000, -16384),
        (0x4001, -16383),
        (0x7FFE, -2),
        (0x7FFF, -1),
        (0xFFFF, None),
    ):
        for fallback in (-(2**31), -17, 0, 17, 2**31 - 1):
            for is_signed in (False, True):
                output = fallback if signed is None else signed if is_signed else value
                expected.append(
                    list(
                        map(
                            str,
                            (value, fallback, str(is_signed).lower(), output, output),
                        )
                    )
                )
    actual = [
        line.split()[2:]
        for line in run["log"]
        if line.startswith("FLEET resolver-property-boundary ")
    ]
    if actual != expected:
        result["problems"].append(f"{mode}/{role}: signed property boundary/fallback")
    restored = [
        line
        for line in run["log"]
        if line.startswith("FLEET resolver-property-restored ")
    ]
    if len(restored) != 1 or not restored[0].startswith(
        "FLEET resolver-property-restored true true "
    ):
        result["problems"].append(f"{mode}/{role}: property restoration/RNG")


def check_real_boundaries(run, mode, role, result):
    # Real-group policy is observed through normal callback/property entries.
    # Explicit boundary results distinguish empty-vector fallback and clamps.
    purchases = [
        line.split()[2:]
        for line in run["log"]
        if line.startswith("FLEET resolver-real-purchase ")
    ]
    if purchases != [
        [str(i), str(value)] for i, value in enumerate((17, 101, 211, 211))
    ]:
        result["problems"].append(f"{mode}/{role}: real-group purchase fallback")
    boundaries = (
        (0, 0, 0),
        (0, 1, 2),
        (8, 0, 0),
        (8, 1, 0),
        (8, 2, 0),
        (8, 3, 1),
        (8, 5, 1),
        (8, 6, 2),
        (8, 7, 2),
        (8, 8, 2),
        (8, 9, 2),
    )
    expected = []
    for mask in range(4):
        for state in range(5):
            moving = state in (0, 2)
            for cap, count, index in boundaries:
                value = (
                    (101 if moving else 211) + index
                    if mask & (1 if moving else 2)
                    else 65535
                )
                expected.append(list(map(str, (mask, state, cap, count, count, value))))
    selections = [
        line.split()[2:]
        for line in run["log"]
        if line.startswith("FLEET resolver-real-selection ")
    ]
    if selections != expected:
        result["problems"].append(f"{mode}/{role}: real-group cargo/order selection")
    restored = [
        line for line in run["log"] if line.startswith("FLEET resolver-real-restored ")
    ]
    if (
        len(restored) != 1
        or len(restored[0].split()) != 8
        or restored[0].split()[2:4] != ["true", "true"]
        or any(not word.isdecimal() for word in restored[0].split()[4:])
    ):
        result["problems"].append(f"{mode}/{role}: real-group cargo restoration/RNG")
    orders = [
        line.split()[2:]
        for line in run["log"]
        if line.startswith("FLEET resolver-real-order ")
    ]
    # NoLoad is 4, NoUnload is 4; MakeLoading(false) keeps default flags zero.
    if orders != [
        list(map(str, (mask, state, load, unload)))
        for mask in range(4)
        for state, (load, unload) in enumerate(((0, 0), (0, 0), (4, 4), (4, 0), (0, 4)))
    ]:
        result["problems"].append(f"{mode}/{role}: real-group order flag inputs")


def check(scenario, run, mode, role, result):
    if "fleet" not in scenario or run["exit"] or not run["snapshots"]:
        return
    required = {
        "resolver": [
            "FLEET resolver-checkpoint complete",
            "FLEET resolver-parent true",
            "FLEET resolver-relative true",
            "FLEET resolver-relative-uncached true",
            "FLEET resolver-caller-grf true true true",
            "FLEET resolver-wide-badge true true",
            "FLEET resolver-reentry true true true",
            "FLEET resolver-property true true",
            "FLEET resolver-probability true true true",
            "FLEET resolver-probability-failed true true",
            "FLEET resolver-purchase-real true",
            "FLEET resolver-unavailable true",
            "FLEET resolver-anynew true",
            "FLEET resolver-zero-reseed true true",
            "FLEET resolver-reinstalled true true",
            *[f"FLEET resolver-trigger {trigger} true" for trigger in (0, 1, 2, 4)],
        ],
        "resolver-reload": [],
        **{
            "transaction-mass-" + kind: [
                "FLEET mass-test true true",
                "FLEET mass true true true true",
                "FLEET mass-empty true",
            ]
            for kind in ("train", "road", "ship", "aircraft")
        },
        **{
            "transaction-refit-" + kind: ["FLEET refit-loaded true true true"]
            for kind in ("train", "road", "ship", "aircraft")
        },
        **{
            "transaction-build-" + kind: ["FLEET build-restored true true"]
            for kind in ("road", "aircraft")
        },
        "transaction-clone": ["FLEET clone-join false true true true"],
        "transaction-rail": [
            "FLEET rail-checks true true true true true true true true true"
        ],
        "transaction-meta": [
            f"FLEET transaction-meta {kind} true 50 true true" for kind in range(4)
        ],
        "reload": ["FLEET reloaded true 1 true"],
        "list": [
            "FLEET cost list-test true 0 65535",
            "FLEET cost list-wide-id false 0 65535",
            "FLEET cost list-invalid false 0 65535",
            "FLEET list-rejected true true true true",
            "FLEET list-group true true true",
        ],
        "train-wagons": [
            "FLEET free-replaced true true true true",
            "FLEET wagons-replaced true true true 6 true 0",
        ],
        "groups": [
            "FLEET rule all true true",
            "FLEET rule protected true false",
            "FLEET rule inherited true true",
            "FLEET groups true 1 1 true",
            "FLEET deleted true true true",
        ],
        **{
            operation: [
                "FLEET test-rng true original true",
                "FLEET test-cargo true",
                "FLEET replaced true",
                "FLEET transferred true true",
            ]
            for operation in ("replace", "train", "ship", "aircraft")
        },
        # Frozen from the unchanged reference: the running vehicle entered its
        # service depot, was renewed (new ID 53, age 2 days) and left running;
        # with only the reserve it restarted unrenewed with money-limit news.
        "drain": [
            "FLEET drain input 12 139 191152",
            "FLEET drain old false false -1 money 176297",
            "FLEET drain new 53 139 2 false false",
        ],
        "drain-cash": [
            "FLEET drain input 12 139 100000",
            "FLEET drain old true false 5860 money 101673",
            "FLEET drain news 12 Autorenew failed on Road Vehicle #17 (money limit)",
        ],
        "cash": [
            "FLEET test-rng true original true",
            "FLEET rollback true true true true",
        ],
        "train-rollback": [
            "FLEET test-rng true original true",
            "FLEET rollback true true true true",
        ],
    }[scenario["fleet"]]
    for line in required:
        if line not in run["log"]:
            result["problems"].append(f"{mode}/{role}: missing {line}")
    if scenario["fleet"] == "resolver":
        check_override_order(run, mode, role, result)
        check_property_boundaries(run, mode, role, result)
        check_real_boundaries(run, mode, role, result)
        if (
            len(
                [line for line in run["log"] if line.startswith("FLEET resolver-real ")]
            )
            != 2
        ):
            result["problems"].append(
                f"{mode}/{role}: missing loaded/loading real groups"
            )
        for label, count in (("relative-matrix", 40), ("var61-policy", 24)):
            lines = [
                line.split()
                for line in run["log"]
                if line.startswith(f"FLEET resolver-{label} ")
            ]
            if len(lines) != 1 or int(lines[0][2]) != count:
                result["problems"].append(
                    f"{mode}/{role}: missing complete resolver {label}"
                )
        for label in ("cache", "translation", "var61", "grf-reload"):
            lines = [
                line
                for line in run["log"]
                if line.startswith(f"FLEET resolver-{label} ")
            ]
            if len(lines) != 1:
                result["problems"].append(f"{mode}/{role}: missing resolver {label}")
            elif label == "grf-reload" and not lines[0].startswith(
                "FLEET resolver-grf-reload true true true "
            ):
                result["problems"].append(
                    f"{mode}/{role}: resolver nonempty native reload invariant"
                )
            elif label in ("translation", "var61") and not lines[0].endswith(" true"):
                result["problems"].append(f"{mode}/{role}: resolver {label} invariant")
    if scenario["fleet"] == "resolver-reload":
        expected = "FLEET resolver-saved " + " ".join(
            map(str, result["resolver_saved"][role])
        )
        if expected not in run["log"]:
            result["problems"].append(
                f"{mode}/{role}: resolver saved random/trigger state changed"
            )
    if scenario["fleet"] in (
        "resolver",
        "resolver-reload",
        "transaction-build-road",
        "transaction-build-aircraft",
        "ship",
    ):
        probes = [
            line.split()
            for line in run["log"]
            if line.startswith("FLEET resolver-variables ")
        ]
        if not probes or any(int(probe[4]) != 169 for probe in probes):
            result["problems"].append(
                f"{mode}/{role}: missing complete resolver variable probe"
            )
    if scenario["fleet"] == "transaction-build-aircraft":
        probes = [
            line.split()
            for line in run["log"]
            if line.startswith("FLEET resolver-aircraft ")
        ]
        if len(probes) != 1 or int(probes[0][2]) < 512:
            result["problems"].append(
                f"{mode}/{role}: missing aircraft state compatibility probe"
            )
    if scenario["fleet"] == "transaction-rail":
        cases = [(7, value) for value in (0, 0xFD, 0xFE, 0xFF, 0x100)]
        cases += [
            (8, value) for value in (0, 0x400, 0x401, 0x40F, 0x402, 0x403, 0xFFFF)
        ]
        lines = [
            line.split() for line in run["log"] if line.startswith("FLEET attachment ")
        ]
        expected = {
            (str(v), str(r), execute) for v, r in cases for execute in ("false", "true")
        }
        if (
            len(lines) != len(expected)
            or {tuple(line[2:5]) for line in lines} != expected
            or any(
                line[5] != "true" or int(line[6]) < 1 or line[7:] != ["true"] * 3
                for line in lines
            )
        ):
            raise RuntimeError(
                "missing real attachment callback outcome/state/RNG evidence"
            )
    if scenario["fleet"] in ("transaction-build-road", "transaction-rail"):
        kind = "road" if scenario["fleet"] == "transaction-build-road" else "rail"
        probes = [line for line in run["log"] if line.startswith("FLEET ground-probe ")]
        if len(probes) != 1 or probes[0].split()[3] != "30":
            result["problems"].append(f"{mode}/{role}: missing ground arithmetic probe")
        result[f"{mode}_{role}_ground"] = branch_witnesses(
            scenario,
            run,
            role,
            "ground",
            [kind + "_power", kind + "_weight", "full_weight"],
        )
    if scenario["fleet"] == "transaction-build-road":
        if not {
            "FLEET occupancy-probe 18 live-next true",
            "FLEET occupancy-live-near 5",
        }.issubset(run["log"]):
            raise RuntimeError("missing native occupancy arithmetic/live-advance probe")
        branch_witnesses(
            scenario,
            run,
            role,
            "occupancy",
            ["same_bucket", "remove", "alias_filter", "reset", "near"],
        )
    if scenario["fleet"] in BRANCHES:
        result[mode + "_" + role + "_branches"] = branch_witnesses(
            scenario, run, role, "vehicle_transactions", BRANCHES[scenario["fleet"]]
        )
