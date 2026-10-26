"""Engine lifecycle commands, preview, reliability and canonical modern reload."""

import struct
from pathlib import Path

from . import core, economy
from .console_adapter import console_adapter, prepared_console
from .core import (
    ROOT,
    WITNESS_ENVIRONMENT,
    branch_witnesses,
    compare_saves,
    witness_counts,
)

ADAPTER = ROOT / "tools/simulation/engines-console.cpp"
REQUIRED = {
    "cargo-registry": (),
    "cargo-profit": (),
    "cargo-profit-reload": (),
    "cargo-rating-reload": (),
    "types": (),
    "types-definitions": (),
    "types-reload": (),
    "monitors": (),
    "commands": ("rename", "visibility", "control"),
    "preview": ("offer", "accept", "join", "penalty"),
    "reliability": ("retire", "never_expire", "sync", "cutoff"),
    "reload": (),
    "subsidies": ("create", "award", "expire"),
    "subsidies-reload": (),
}
EXPECTED = {
    "cargo-registry": ("registry-controls true",),
    "cargo-profit": ("profit-controls true",),
    "cargo-profit-reload": ("profit-controls true",),
    "cargo-rating-reload": ("rating-controls true",),
    "types-definitions": (
        "types-case definitions-file-loaded-save true",
        "types-case definitions-reordered true",
        "types-case definitions-missing true",
    ),
    "types": (
        "types-case afterload-masks true",
        "types-case fixedpoint true",
        "types-case excluded-50 true",
        "types-case excluded-52 true",
        "types-case excluded-53 true",
        "types-case excluded-54 true",
        "types-case excluded-55 true",
        "types-case excluded-56 true",
        "types-case excluded-57 true",
        "types-case date-zero true",
        "types-case seeded-cycle true",
        "types-case inclusive-day true",
        "types-case max-date true",
        "types-case before-release true",
        "types-case company-bits true",
        "types-case all-multirail true",
        "types-case preview-owner true",
        "types-case one-year true",
        "types-case wagon-only true",
        "types-case climate true",
        "types-case lifecycle-enable true",
        "types-case lifecycle-disable true",
        "types-case monthly-publication true",
        "types-case daily-publication true",
        "types-case hidden-and-masks true",
        "types-case hidden-only-rail true",
        "types-case tram-only-mask true",
        "types-case road-only-mask true",
        "types-case company-hidden true",
        "types-case town true",
        "types-case deity true",
        "types-case editor true",
        "types-case generation true",
        "types-case restoration true",
    ),
    "types-reload": ("types-case afterload-masks true", "types-case restoration true"),
    "monitors": ("controls true",),
    "commands": (
        "company-bits true",
        "clear-special true",
        "hidden true",
        "disabled true",
    ),
    "preview": ("offered true true", "joined true true", "penalty true true"),
    "reliability": (
        "startup-life true",
        "retired true true true",
        "never-expire true true",
        "synced true",
        "cutoff true",
    ),
    "reload": ("reloaded true",),
    "subsidies": (
        "nearby true",
        "commands true true",
        "classes true true",
        "multipliers true",
        "wrap true",
        "disabled true true",
        "retries true",
        "routes true true true",
        "transfer true true",
    ),
    "subsidies-reload": ("loaded true",),
}


def scenarios(soak):
    return [
        {
            "name": "engines-" + mode,
            "kind": "save",
            "save": str(ROOT / "migration/saves/padhattan-ridge-2000.sav"),
            "engines": mode,
            "short_checkpoint": True,
            "ticks": 74 if soak else 1,
        }
        for mode in (
            "commands",
            "preview",
            "reliability",
            "reload",
            "monitors",
            "cargo-registry",
            "cargo-profit",
            "types",
            "types-reload",
            "types-definitions",
        )
    ]


def game_environment(scenario):
    env = (
        dict(WITNESS_ENVIRONMENT)
        if scenario.get("engines") and scenario.get("witnesses")
        else {}
    )
    if (
        scenario.get("engines", "").startswith("types")
        and scenario["engines"] != "types-definitions"
    ):
        env["OPENTTD_ENGINE_TYPES"] = "1"
    return env


def adapter(build, folder, env):
    economy.subsidy_reference(folder)
    return console_adapter(
        build,
        folder,
        env,
        ADAPTER,
        "engines",
        wraps=("_Z18CopyTempEngineDatav",),
        replace_objects=("saveload.cpp.o", "/newgrf.cpp.o", "/newgrf_act0_cargo.cpp.o"),
    )


def prepare(scenario, binaries, builds, out, timeout, env, result):
    if "engines" not in scenario:
        return scenario
    folder = out / scenario["name"] / "input"
    normalized, record = core.normalize(Path(scenario["save"]), folder / "source")
    result["engines_input"] = record
    scenario = dict(scenario, save=str(normalized))
    executables = prepared_console("engines", adapter, binaries, builds, folder, env)
    if scenario["engines"] == "types-definitions":
        for executable in executables.values():
            write_type_grfs(executable.parent / "newgrf")
        return definition_reload(
            scenario, executables, builds, folder, timeout, env, result
        )
    if scenario["engines"] == "monitors":
        economy.monitor_load_probes(scenario, executables, builds, folder, timeout, env)
    if scenario["engines"] not in (
        "types-reload",
        "reload",
        "subsidies-reload",
        "cargo-profit-reload",
        "cargo-rating-reload",
    ):
        return dict(
            scenario,
            executables=executables,
            console=["engines_scenario " + scenario["engines"], "unpause"],
        )
    types = scenario["engines"] == "types-reload"
    subsidy = scenario["engines"] == "subsidies-reload"
    cargo = scenario["engines"].startswith("cargo-")
    variant, prefix = (
        (scenario["engines"].removesuffix("-reload"), "CARGO ")
        if cargo
        else ("subsidies", "SUBSIDY ")
        if subsidy
        else ("commands", "ENGINE ")
    )
    if types:
        variant, prefix = "types", "ENGINE "
    saves = {role: folder / role / "engines.sav" for role in executables}
    runs = core.capture_pair(
        lambda role: dict(
            scenario,
            console=[f'engines_scenario {variant} "{saves[role]}"', "unpause"],
            ticks=1,
        ),
        executables,
        builds,
        folder,
        timeout,
        env,
    )
    captures = {}
    required = (
        EXPECTED[scenario["engines"]]
        if cargo
        else EXPECTED["types"]
        if types
        else ("transfer true true",)
        if subsidy
        else ("hidden true",)
    )
    for role, run in runs.items():
        if (
            run["exit"]
            or not saves[role].is_file()
            or any(prefix + marker not in run["log"] for marker in required)
            or (types and not type_markers(run["log"], capture=True))
        ):
            raise RuntimeError("engine reload capture failed")
        captures[role] = [line for line in run["log"] if line.startswith(prefix)]
    differences = compare_saves(
        saves["reference"],
        saves["candidate"],
        scenario["name"],
        10,
        {"chunks": 0, "elements": 0, "masked": {}},
    )
    if differences or captures["reference"] != captures["candidate"]:
        result["differences"].extend(differences)
        raise RuntimeError("engine reload capture differs")
    result["engines_capture"] = captures
    return dict(
        scenario,
        executables=executables,
        role_inputs={role: {"save": str(save)} for role, save in saves.items()},
        console=[
            "engines_scenario "
            + (
                variant
                if cargo
                else "types-inspect"
                if types
                else "subsidies-inspect"
                if subsidy
                else "inspect"
            ),
            "unpause",
        ],
    )


def check(scenario, run, mode, role, result):
    if role == "candidate" and scenario.get("witnesses"):
        counts = witness_counts(run, "engine")
        if counts is not None:
            result[mode + "_engine_crossings"] = {
                key: counts[key] for key in ("entries", "services", "bytes")
            }
    if "engines" not in scenario or run["exit"] or not run["snapshots"]:
        return
    variant = scenario["engines"]
    if variant == "types-definitions":
        trace = [line for line in run["log"] if line.startswith("ENGINE definitions-")]
        result.setdefault(mode + "_definitions_trace", {})[role] = trace
        pair = result[mode + "_definitions_trace"]
        if len(pair) == 2 and pair["reference"] != pair["candidate"]:
            raise RuntimeError("unsaved type registry/translation state differs")
    if (
        variant.startswith("types")
        and variant != "types-definitions"
        and not type_markers(run["log"], capture=variant == "types")
    ):
        raise RuntimeError("type loadphase/mask/command/restoration markers missing")
    component = (
        "cargo"
        if variant.startswith("cargo-")
        else "monitor"
        if variant == "monitors"
        else ("subsidy" if variant.startswith("subsidies") else "engine")
    )
    prefix = component.upper() + " "
    if variant == "subsidies" and not any(
        line.startswith(prefix + "retry-rng ") for line in run["log"]
    ):
        raise RuntimeError("subsidy retry RNG result missing")
    for expected in EXPECTED[variant]:
        if prefix + expected not in run["log"]:
            raise RuntimeError("engine branch result missing: " + expected)
    if counts := branch_witnesses(scenario, run, role, component, REQUIRED[variant]):
        result[mode + "_" + component + "_branches"] = counts


def type_markers(lines, capture):
    required = ["ENGINE types-loadphase true", "ENGINE types-complete true"]
    if not all(marker in lines for marker in required):
        return False
    if not any(line.startswith("ENGINE types-masks loaded ") for line in lines):
        return False
    if capture:
        for label in (
            "rail-before",
            "road-before",
            "tram-before",
            "rail-after",
            "road-after",
            "tram-after",
            "hidden-company",
            "hidden-town",
        ):
            for execute in ("false", "true"):
                if not any(
                    line.startswith(f"ENGINE types-command {label} {execute} ")
                    for line in lines
                ):
                    return False
    return True


def write_type_grfs(folder):
    """Real version-1 containers exercise the production reserve/activate reader."""
    folder.mkdir(parents=True, exist_ok=True)

    def prop(feature, first, number, key, value):
        return bytes((0, feature, 1, number, first, key)) + value

    def labels(feature, names):
        return prop(feature, 0, len(names), 8, b"".join(names))

    def table(key, names):
        return prop(8, 0, len(names), key, b"".join(names))

    def listing(feature, first, key, names):
        return prop(feature, first, 1, key, bytes((len(names),)) + b"".join(names))

    def write(name, grfid, actions):
        actions = [
            bytes((8, 8)) + grfid + name.encode() + b"\0Type registry fixture\0",
            *actions,
        ]
        sprites = [struct.pack("<HB", len(action), 255) + action for action in actions]
        (folder / name).write_bytes(
            struct.pack("<HBI", 4, 255, len(actions)) + b"".join(sprites) + b"\0\0"
        )

    a = [
        labels(16, [b"\0" * 4, b"R4AA", b"R4BB", b"R4DD"]),
        labels(18, [b"D4AA", b"D4DD"]),
        labels(19, [b"T4AA", b"T4DD"]),
    ]
    a += [
        table(0x12, [b"R4AA", b"R4BB", b"R4AA", b"A4AA"]),
        table(0x16, [b"D4AA", b"D4AA", b"T4AA"]),
        table(0x17, [b"T4AA", b"D4AA"]),
    ]
    a += [
        listing(16, 1, 0x1D, [b"A4AA", b"Z4AA"]),
        listing(16, 1, 0x0F, [b"R4BB", b"R4CC"]),
        listing(18, 0, 0x0F, [b"D4BB", b"T4AA"]),
        listing(19, 0, 0x0F, [b"T4BB"]),
        listing(16, 1, 0x18, [b"R4BB"]),
        listing(16, 2, 0x19, [b"R4AA"]),
        listing(18, 0, 0x18, [b"T4AA"]),
        listing(19, 0, 0x19, [b"D4AA"]),
    ]
    for feature, first in [(16, 1), (18, 0), (19, 0)]:
        a += [
            prop(feature, first, 1, 0x13, struct.pack("<H", 13)),
            prop(feature, first, 1, 0x14, struct.pack("<H", 177)),
            prop(feature, first, 1, 0x16, b"\x55"),
            prop(feature, first, 1, 0x17, struct.pack("<I", 1)),
            prop(feature, first, 1, 0x1A, b"\x33"),
            prop(feature, first, 1, 0x1C, struct.pack("<H", 19)),
            prop(feature, first, 1, 0x1E, b"\0\0"),
        ]
    a += [
        prop(16, 1, 1, 0x11, b"\x09"),
        prop(16, 1, 1, 0x12, b"\xff"),
        prop(16, 1, 1, 0x15, b"\xff"),
        prop(16, 3, 1, 0x17, struct.pack("<I", 1)),
        prop(16, 2, 1, 0x10, b"\x04"),
        prop(18, 1, 1, 0x10, b"\x08"),
        prop(0, 0, 1, 5, b"\0"),
        prop(1, 0, 1, 5, b"\0"),
        prop(1, 1, 1, 5, b"\0"),
        prop(1, 1, 1, 0x1C, b"\x01"),
    ]
    write("types-a.grf", b"T47A", a)
    b = [
        labels(16, [b"R4CC", b"R4BB", b"A4AA"]),
        labels(18, [b"D4BB"]),
        labels(19, [b"T4BB"]),
        listing(16, 0, 0x1D, [b"R4AA"]),
        listing(16, 1, 0x0F, [b"R4AA"]),
        listing(18, 0, 0x0F, [b"D4AA"]),
    ]
    write("types-b.grf", b"T47B", b)
    write(
        "types-full.grf",
        b"T47F",
        [
            labels(feature, [f"{prefix}{i:03}".encode() for i in range(count)])
            for feature, prefix, count in [(16, "X", 64), (18, "Y", 63)]
        ],
    )
    write("types-mismatch.grf", b"T47X", [labels(18, [b"T4AA"])])
    write(
        "types-missing.grf",
        b"T47A",
        [
            labels(16, [b"R4CC"]),
            listing(16, 0, 0x1D, [b"R4AA"]),
            labels(18, [b"T4AA"]),
            table(0x12, [b"R4CC"]),
            table(0x16, [b"ROAD"]),
            table(0x17, [b"ELRL"]),
            prop(0, 0, 1, 5, b"\0"),
            prop(1, 0, 1, 5, b"\0"),
            prop(1, 1, 1, 5, b"\0"),
            prop(1, 1, 1, 0x1C, b"\x01"),
        ],
    )


def definition_reload(scenario, executables, builds, folder, timeout, env, result):
    runs = core.capture_pair(
        lambda role: dict(
            scenario,
            console=["engines_scenario types-definitions-build", "unpause"],
            ticks=74,
        ),
        executables,
        builds,
        folder / "definitions",
        timeout,
        env,
    )
    for run in runs.values():
        if (
            run["exit"]
            or "ENGINE types-case definitions-built true" not in run["log"]
            or not run["snapshots"]
        ):
            raise RuntimeError("real-file type construction capture failed")
    traces = {
        role: [line for line in run["log"] if line.startswith("ENGINE definitions-")]
        for role, run in runs.items()
    }
    if traces["reference"] != traces["candidate"]:
        raise RuntimeError("loaded unsaved type registry state differs")
    saves = {role: run["snapshots"][-1] for role, run in runs.items()}
    differences = compare_saves(
        saves["reference"],
        saves["candidate"],
        scenario["name"],
        10,
        {"chunks": 0, "elements": 0, "masked": {}},
    )
    if differences:
        result["differences"].extend(differences)
        raise RuntimeError("real-file type construction differs")
    return dict(
        scenario,
        executables=executables,
        role_inputs={role: {"save": str(save)} for role, save in saves.items()},
        console=["engines_scenario types-definitions-reload", "unpause"],
    )
