"""Engine lifecycle commands, preview, reliability and canonical modern reload."""

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
    "loading": (),
    "loading-shared": (),
    "loading-reload": (),
    "types": (),
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
    "loading": ("loading-complete true",),
    "loading-shared": ("loading-complete true",),
    "loading-reload": ("loading-complete true",),
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
            "loading-shared",
            "loading-reload",
            "commands",
            "preview",
            "reliability",
            "reload",
            "monitors",
            "types",
            "types-reload",
        )
    ]


def game_environment(scenario):
    env = (
        dict(WITNESS_ENVIRONMENT)
        if scenario.get("engines") and scenario.get("witnesses")
        else {}
    )
    if scenario.get("engines", "").startswith("types"):
        env["OPENTTD_ENGINE_TYPES"] = "1"
    if folder := scenario.get("engine_loading_dir"):
        env["OPENTTD_ENGINE_LOADING"] = folder
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
        replace_objects=("saveload.cpp.o",),
    )


def prepare(scenario, binaries, builds, out, timeout, env, result):
    if "engines" not in scenario:
        return scenario
    folder = out / scenario["name"] / "input"
    normalized, record = core.normalize(Path(scenario["save"]), folder / "source")
    result["engines_input"] = record
    scenario = dict(scenario, save=str(normalized))
    if scenario["engines"].startswith("loading"):
        loading_grfs(folder / "loading-grfs")
        scenario["engine_loading_dir"] = str((folder / "loading-grfs").resolve())
    executables = prepared_console("engines", adapter, binaries, builds, folder, env)
    if scenario["engines"].startswith("loading"):
        return prepare_loading(
            scenario, executables, builds, folder, timeout, env, result
        )
    if scenario["engines"] == "monitors":
        economy.monitor_load_probes(scenario, executables, builds, folder, timeout, env)
    if scenario["engines"] not in ("reload", "subsidies-reload", "types-reload"):
        return dict(
            scenario,
            executables=executables,
            console=["engines_scenario " + scenario["engines"], "unpause"],
        )
    types = scenario["engines"] == "types-reload"
    subsidy = scenario["engines"] == "subsidies-reload"
    variant, prefix = ("subsidies", "SUBSIDY ") if subsidy else ("commands", "ENGINE ")
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
        EXPECTED["types"]
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
                "types-inspect"
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
    if variant.startswith("types") and not type_markers(
        run["log"], capture=variant == "types"
    ):
        raise RuntimeError("type loadphase/mask/command/restoration markers missing")
    component = (
        "monitor"
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


def loading_grfs(folder):
    """Small v1 containers exercise the real FILE/Action0/2/3/4 loading path."""
    import struct

    folder.mkdir(parents=True, exist_ok=True)

    def word(value):
        return struct.pack("<H", value)

    def dword(value):
        return struct.pack("<I", value)

    def extended(value):
        return bytes([value]) if value < 255 else b"\xff" + word(value)

    def properties(feature, first, items, count=1):
        return (
            bytes([0, feature, len(items), count])
            + extended(first)
            + b"".join(bytes([prop]) + data for prop, data in items)
        )

    def table(prop, labels):
        return properties(8, 0, [(prop, b"".join(labels))], len(labels))

    def engine(feature, local, custom=True):
        common = [(0, word(10000)), (3, b"\xff"), (4, b"\xff"), (6, b"\x0f")]
        specific = (
            [
                (5, b"\0"),
                (9, word(80)),
                (0x0B, word(600)),
                (0x14, b"\x20"),
                (0x15, b"\xff"),
                (0x28, word(2)),
            ],
            [
                (5, b"\0"),
                (0x0F, b"\x20"),
                (0x10, b"\xff"),
                (0x15, b"\x49"),
                (0x1D, word(2)),
            ],
            [
                (9, b"\1"),
                (0x23, word(160)),
                (0x0C, b"\xff"),
                (0x0D, word(32)),
                (0x18, word(2)),
            ],
            [
                (9, b"\1"),
                (0x0C, b"\x64"),
                (0x0D, b"\x0a"),
                (0x0F, word(32)),
                (0x11, b"\x08"),
                (0x18, word(2)),
            ],
        )[feature]
        if feature == 0 and local in (301, 312):
            specific = [
                (prop, data)
                for prop, data in specific
                if prop != 0x28 and (local != 312 or prop != 0x15)
            ]
            if local == 301:
                specific = [
                    (prop, b"\1" if prop == 0x15 else data) for prop, data in specific
                ]
        result = [properties(feature, local, common + specific)]
        if custom:
            result.append(
                properties(feature, local, [((0x31, 0x28, 0x22, 0x22)[feature], b"\2")])
            )
            # Local CTT slots: include GOODS, exclude PASS, leave MAIL, fail others.
            group = bytes([2, feature, 0, 0x81, 0x18, 0, 0xFF, 3])
            group += b"".join(
                word(0x8000 | value) + bytes([slot, slot])
                for slot, value in enumerate((1, 2, 0))
            )
            dispatch = bytes([2, feature, 1, 0x85, 0x0C, 0]) + word(0xFFFF)
            dispatch += b"\1" + word(0) + word(0x163) + word(0x163) + word(0x7FFF)
            result += [
                group + word(0x7FFF),
                dispatch,
                bytes([3, feature, 1]) + extended(local) + b"\0" + word(1),
            ]
        result.append(
            bytes([4, feature, 0x7F, 1])
            + extended(local)
            + f"Loading {feature}:{local}".encode()
            + b"\0"
        )
        return result

    for which, version in ((1, 8), (2, 7), (3, 8), (4, 8)):
        sprites = [
            b"\x08"
            + bytes([version])
            + dword(0x4700A000 + which)
            + b"Engine loading fixture\0GPLv2\0"
        ]
        if which == 4:
            for local in (1, 65000):
                sprites.append(
                    b"\x04\x00\x7f\x01" + extended(local) + b"Static loading name\0"
                )
        else:
            if which == 3:
                for target in (0x4700A002, 0, 0x4700A001):
                    sprites.append(
                        properties(8, 0, [(0x11, dword(0x4700A003) + dword(target))])
                    )
            sprites += [
                table(9, [b"GOOD", b"PASS", b"MAIL", b"MISS"]),
                table(0x12, [b"RAIL", b"FAIL", b"ELRL"]),
                table(0x16, [b"ROAD", b"FAIL", b"ELRL"]),
                table(0x17, [b"ELRL", b"FAIL", b"ROAD"]),
            ]
            for feature in range(4):
                sprites += engine(feature, 300)
                if which < 3:
                    sprites += engine(feature, 0, custom=False)
            if which == 1:
                sprites.append(properties(0, 350, [(6, bytes(70))], 70))
                for local in range(301, 313):
                    sprites += engine(0, local, custom=False)
                sprites += [
                    properties(0, 301, [(0x28, word(0))]),
                    properties(0, 302, [(0x28, word(256))]),
                    properties(0, 303, [(0x34, b"\2\0\2")]),
                    properties(0, 304, [(5, b"\1")]),
                ]
                for local, parent in (
                    (305, 310),
                    (306, 306),
                    (307, 308),
                    (310, 311),
                    (311, 310),
                ):
                    sprites.append(properties(0, local, [(0x2F, word(parent))]))
                sprites += [
                    properties(
                        0, 309, [(0x28, word(0)), (0x2C, b"\1\0"), (0x2D, b"\1\0")]
                    )
                ]
                for local in (301, 302, 303, 304):
                    sprites += engine(1, local, custom=False)
                sprites += [
                    properties(1, 301, [(5, b"\xff")]),
                    properties(1, 302, [(5, b"\1")]),
                    properties(1, 303, [(0x1C, b"\1"), (5, b"\2")]),
                    properties(1, 304, [(0x1C, b"\1"), (5, b"\0")]),
                ]
        records = [dword(len(sprites))] + sprites
        data = (
            b"".join(word(len(sprite)) + b"\xff" + sprite for sprite in records)
            + b"\0\0"
        )
        (folder / f"owner470-{which}.grf").write_bytes(data)


def prepare_loading(scenario, executables, builds, folder, timeout, env, result):
    if scenario["engines"] != "loading-reload":
        return dict(
            scenario,
            executables=executables,
            console=["engines_scenario " + scenario["engines"], "unpause"],
        )
    saves = {role: folder / role / "loading.sav" for role in executables}
    runs = core.capture_pair(
        lambda role: dict(
            scenario,
            console=[f'engines_scenario loading "{saves[role]}"', "unpause"],
            ticks=1,
        ),
        executables,
        builds,
        folder,
        timeout,
        env,
    )
    captures = {}
    for role, run in runs.items():
        if (
            run["exit"]
            or not saves[role].is_file()
            or "ENGINE loading-complete true" not in run["log"]
        ):
            raise RuntimeError("file-loaded engine capture failed")
        captures[role] = [
            line for line in run["log"] if line.startswith("ENGINE loading-")
        ]
    differences = compare_saves(
        saves["reference"],
        saves["candidate"],
        scenario["name"],
        10,
        {"chunks": 0, "elements": 0, "masked": {}},
    )
    if differences or captures["reference"] != captures["candidate"]:
        result["differences"].extend(differences)
        raise RuntimeError("file-loaded engine capture differs")
    result["engine_loading_capture"] = captures
    return dict(
        scenario,
        executables=executables,
        role_inputs={role: {"save": str(save)} for role, save in saves.items()},
        console=["engines_scenario loading-inspect", "unpause"],
    )
