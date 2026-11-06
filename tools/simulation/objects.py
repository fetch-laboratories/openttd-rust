"""Object ownership scenarios and named unchanged-original-body gap inputs."""

import struct
from collections.abc import Sequence
from itertools import product
from pathlib import Path
from typing import Literal, NotRequired

from . import core, generated
from .console_adapter import console_adapter, prepared_console
from .core import ROOT, WITNESS_ENVIRONMENT, branch_witnesses


class Scenario(core.Scenario):
    generation_owner: NotRequired[str]  # generated.Scenario's; it imports this module
    objects: NotRequired[
        Literal["commands", "gaps", "reload", "definitions", "definitions-reload"]
    ]
    objects_generation: NotRequired[bool]
    object_definition_grfs: NotRequired[str]


def scenarios(soak: bool) -> list[Scenario]:
    worlds = [s for s in generated.scenarios(soak) if "settings-" in s["name"]][:4]
    return [
        {**s, "name": "objects-generation-" + str(i), "objects_generation": True}
        for i, s in enumerate(worlds)
    ] + [
        Scenario(
            name="objects-" + mode,
            kind="save",
            objects=mode,
            ticks=1,
            save=str(ROOT / "migration/saves/padhattan-ridge-2000.sav"),
            short_checkpoint=True,
        )
        for mode in ("commands", "gaps", "reload", "definitions", "definitions-reload")
    ]


def game_environment(scenario: Scenario) -> dict[str, str]:
    selected = "objects" in scenario or "objects_generation" in scenario
    return WITNESS_ENVIRONMENT if selected and scenario.get("witnesses") else {}


def adapter(build: Path, folder: Path, env: dict[str, str]) -> Path:
    rust = (build / "openttd-rust").exists()
    return console_adapter(
        build,
        folder,
        env,
        ROOT / "tools/simulation/objects-console.cpp",
        "objects",
        wraps=(
            "_Z16SetModalProgressb",
            "_Z26SetGeneratingWorldProgress16GenWorldProgressj",
            "_Z31IncreaseGeneratingWorldProgress16GenWorldProgress",
            *(() if rust else ("_Z29GenerateWorldSetAbortCallbackPFvvE",)),
            "_Z13PlayTileSoundPK7GRFFiletN10StrongType7TypedefIj12TileIndexTagJNS2_7CompareENS2_7IntegerENS2_10CompatibleIiEENS7_IlEEEEE",
            "_ZN11BaseStation19SetRoadStopTileDataEN10StrongType7TypedefIj12TileIndexTagJNS0_7CompareENS0_7IntegerENS0_10CompatibleIiEENS5_IlEEEEEhb",
            "_Z19MarkTileDirtyByTileN10StrongType7TypedefIj12TileIndexTagJNS_7CompareENS_7IntegerENS_10CompatibleIiEENS4_IlEEEEEii",
            "_Z26ErrorUnknownCallbackResultjtt",
            "_Z16ShowErrorMessageO13EncodedStringS0_12WarningLeveliiS0_6PoolIDIh12CompanyIDTagLh15ELh255EE",
        ),
        replace_objects=("/src/object_cmd.cpp.o", "/src/saveload/saveload.cpp.o"),
        native_test_support=True,
    )


def prepare(
    scenario: Scenario,
    binaries: dict[str, Path],
    builds: dict[str, Path],
    out: Path,
    timeout: int,
    env: dict[str, str],
    result: core.Result,
) -> Scenario:
    if "objects" not in scenario:
        return scenario
    folder = out / scenario["name"] / "input"
    source, _ = core.normalize(Path(scenario["save"]), folder / "normalized")
    scenario = {**scenario, "save": str(source)}
    executables = prepared_console("objects", adapter, binaries, builds, folder, env)
    if scenario["objects"].startswith("definitions"):
        return prepare_definitions(
            scenario, executables, builds, folder, timeout, env, result
        )
    if scenario["objects"] == "gaps":
        for kind in ("valid", "header", "partial", "excess", "fixed", "riff"):
            logs: dict[str, list[str]] = {}
            probe: Scenario = {**scenario, "console": ["objects_scenario anit-" + kind]}
            runs = core.capture_pair(
                probe, executables, builds, folder, timeout, env, "anit-" + kind
            )
            for role, run in runs.items():
                logs[role] = [x for x in run["log"] if x.startswith("OBJECT ")]
                if (
                    run["exit"]
                    or len(logs[role]) != 2
                    or logs[role][-1:] != ["OBJECT animation-load-complete " + kind]
                ):
                    raise RuntimeError(f"{role}: missing actual ANIT {kind} evidence")
            if logs["reference"] != logs["candidate"]:
                raise RuntimeError(f"actual ANIT {kind} differs: {logs}")
    if scenario["objects"] != "reload":
        return {
            **scenario,
            "executables": executables,
            "console": ["objects_scenario " + scenario["objects"], "unpause"],
        }
    saves = {role: folder / role / "objects.sav" for role in executables}

    def commands(role: str) -> Scenario:
        console = [f'objects_scenario commands "{saves[role]}"', "unpause"]
        return {**scenario, "console": console}

    runs = core.capture_pair(
        commands,
        executables,
        builds,
        folder,
        timeout,
        env,
    )
    for role, capture in runs.items():
        if (
            capture["exit"]
            or not saves[role].is_file()
            or "OBJECT id-reused true" not in capture["log"]
        ):
            raise RuntimeError("object reload capture failed")
        state = [x for x in capture["log"] if x.startswith("OBJECT animation-reload ")]
        if len(state) != 1 or (
            role == "candidate" and state != core.findings(result)["saved_animation"]
        ):
            raise RuntimeError("nonempty animation reload capture differs/missing")
        core.findings(result)["saved_animation"] = state
    differences = core.compare_saves(
        saves["reference"],
        saves["candidate"],
        scenario["name"],
        10,
        core.empty_stats(),
    )
    if differences:
        raise RuntimeError(f"object reload capture differs: {differences}")
    return {
        **scenario,
        "executables": executables,
        "role_inputs": {r: {"save": str(s)} for r, s in saves.items()},
        "console": ["objects_scenario inspect", "unpause"],
    }


def check(
    scenario: Scenario,
    run: core.Run,
    mode: core.Mode,
    role: core.Role,
    result: core.Result,
) -> None:
    if "objects_generation" in scenario:
        branches: Sequence[str] = ("generate",)
        if scenario["name"].endswith("-1"):
            branches += ("lighthouse", "transmitter")
        branch_witnesses(scenario, run, role, "object", branches)
    if "objects" not in scenario:
        return
    if scenario["objects"].startswith("definitions"):
        check_definitions(scenario, run, mode, role, result)
        return
    for marker in (
        "invocation-roots",
        "invocation-parent",
        "invocation-animation",
        "invocation-complete",
        "invocation-rng",
    ):
        if sum(x.startswith("OBJECT " + marker + " ") for x in run["log"]) != 1:
            raise RuntimeError(f"{mode}/{role}: missing {marker}")
    for name in (
        "invocation-slope-reject-test",
        "invocation-slope-reject-execute",
        "invocation-autoslope-disabled",
    ):
        if sum(x.startswith("OBJECT cost " + name + " ") for x in run["log"]) != 1:
            raise RuntimeError(f"{mode}/{role}: missing real callback result {name}")
    admissions = [
        tuple(x.split()[2:5])
        for x in run["log"]
        if x.startswith("OBJECT invocation-eligibility ")
    ]
    expected_admissions = set(
        product(("1", "2"), ("false", "true"), map(str, range(8)))
    )
    if len(admissions) != 32 or set(admissions) != expected_admissions:
        raise RuntimeError(
            f"{mode}/{role}: missing actual availability admission boundaries"
        )
    required = {
        "commands": ("hq-cargo 5 2 1", "id-reused true", "water-clear 1 16 2 17 0 17"),
        "gaps": (
            "count-wrap true|scratch-growth true true|callback-before-clear true|"
            "cleared-before-bridge true|callback-colour true|"
            "cancellation 0 true|cancellation 1 true|query-animation-order true|"
            "animation-binding house true|animation-binding industry true|"
            "animation-binding station true|animation-binding airport true|"
            "animation-binding roadstop true|animation-binding object true|"
            "animation-road-zero 0 1 0 true"
        ).split("|"),
        "reload": ("afterload-counts true",),
    }[scenario["objects"]]
    for text in required:
        if "OBJECT " + text not in run["log"]:
            raise RuntimeError(f"{mode}/{role}: missing object evidence: {text}")
    lines = [line for line in run["log"] if line.startswith("OBJECT ")]
    if (
        scenario["objects"] == "gaps"
        and sum(line.startswith("OBJECT query ") for line in lines) != 96
    ):
        raise RuntimeError(f"{mode}/{role}: missing object query results")
    if scenario["objects"] == "gaps":
        for marker, expected in (
            ("animation-step", 240),
            ("animation-speed", 144),
            ("animation-property-speed", 90),
            ("animation-traversal", 72),
            ("animation-airport", 5),
            ("animation-diagnostic", 6),
            ("animation-visit station", 46),
            ("animation-visit roadstop", 21),
            ("animation-visit airport", 9),
        ):
            if (
                sum(line.startswith("OBJECT " + marker + " ") for line in lines)
                != expected
            ):
                raise RuntimeError(f"{mode}/{role}: missing {marker} boundary cases")
        features = "house industry station airport roadstop object".split()
        results = "65535 509 510 511 259 263 32771".split()
        rejections = "cached nonmember nullspec disabled accepted".split()
        lifecycle = "house-replace house-clear industry-clear industry-terraform rail-remove roadstop-remove airport-remove oilrig-convert object-clear object-preserve object-remove station-preserve station-remove".split()
        booleans = ["false", "true"]
        matrix_axes = (["station", "roadstop"], booleans, booleans, results)
        inventories = (
            ("matrix", "-", matrix_axes),
            ("version", " ", (features, ["7", "8"], ["255", "256", "65535"])),
            ("rejection", " ", (rejections, ["before", "after"])),
            ("diagnostic", " ", ([f"{f}-8-256" for f in features],)),
            ("lifecycle", " ", (lifecycle, ["before", "after"])),
        )
        for marker, separator, axes in inventories:
            keys = {separator.join(x) for x in product(*axes)}
            width = 1 if separator == "-" else len(axes)
            rows = [
                x.split()[2:]
                for x in lines
                if x.startswith("OBJECT animation-" + marker + " ")
            ]
            if len(rows) != len(keys) or {" ".join(x[:width]) for x in rows} != keys:
                raise RuntimeError(f"missing/duplicate keyed animation {marker}")
        traces = [
            x.split()[2:] for x in lines if x.startswith("OBJECT animation-trace ")
        ]
        for event, count in {
            "sound-enter": 20,
            "sound-exit": 20,
            "setter-enter": 20,
            "dirty": 20,
            "callback": 56,
            "end": 56,
        }.items():
            if sum(x[2] == event for x in traces) != count:
                raise RuntimeError(
                    f"original animation event inventory differs: {event}"
                )
    if scenario["objects"] == "reload":
        state = [x for x in lines if x.startswith("OBJECT animation-reload ")]
        if state != core.findings(result)["saved_animation"]:
            raise RuntimeError(
                "animation queue/frame/bits/records/RNG changed on reload"
            )

    branches = {
        "commands": "build clear hq_grow hq_relocate area cargo hq_pass hq_mail owner colours autoslope build_above".split(),
        "gaps": ("slope_callback",),
        "reload": (),
    }[scenario["objects"]]
    branch_witnesses(scenario, run, role, "object", branches)
    if scenario["objects"] == "commands":  # Object clearing redispatches to water.
        branch_witnesses(scenario, run, role, "water_cmd", ("owner",))


def definition_fixtures(folder: Path) -> None:
    """Owned synthetic GRFs reach production Action0, strings, Action3 and finalisation."""
    from .newgrf_loading import action2, grf, grf_property, info, sprite_binding

    folder.mkdir(parents=True, exist_ok=True)

    def prop(local: int, code: int, value: bytes) -> bytes:
        return grf_property(15, local, code, value)

    first = [
        info(b"OA64"),
        prop(2, 0x0D, b"\x17"),
        prop(0, 8, b"A001"),
        prop(1, 8, b"A002"),
        prop(0, 9, struct.pack("<H", 0xD000)),
        prop(0, 8, b"A003"),
        prop(0, 10, struct.pack("<H", 0xD001)),
        prop(1, 10, struct.pack("<H", 0xD002)),
        bytes((4, 15, 255, 3, 0, 208))
        + b"Harbour class\0First object\0Second object\0",
    ]
    for local, view in ((0, 2), (1, 4)):
        first += [
            prop(local, 11, b"\x0f"),
            prop(local, 12, bytes((0x11 + local * 0x11,))),
            prop(local, 13, bytes((77 + local,))),
            prop(local, 14, struct.pack("<I", 0)),
            prop(local, 15, struct.pack("<I", 0x7FFFFFFF)),
            prop(local, 23, bytes((view,))),
        ]
    first += [action2(15, 0, [], 0x8001), sprite_binding(15, [0, 1], [], 0)]
    second = [
        info(b"OB64"),
        prop(0, 8, b"A003"),
        prop(0, 11, b"\x0f"),
        prop(0, 10, struct.pack("<H", 0xD000)),
        bytes((4, 15, 255, 1, 0, 208)) + b"Other object\0",
        action2(15, 0, [], 0x8002),
        sprite_binding(15, [0], [], 0),
    ]
    for name, actions in (("objects-a", first), ("objects-b", second)):
        (folder / (name + ".grf")).write_bytes(grf(actions, 2))


def definition_rows(log: list[str], phase: str) -> list[str]:
    prefix = "OBJECT definition " + phase + " "
    return [line[len(prefix) :] for line in log if line.startswith(prefix)]


def prepare_definitions(
    scenario: Scenario,
    executables: dict[str, Path],
    builds: dict[str, Path],
    folder: Path,
    timeout: int,
    env: dict[str, str],
    result: core.Result,
) -> Scenario:
    definition_fixtures(folder / "grfs")
    scenario = {**scenario, "object_definition_grfs": str(folder / "grfs")}
    command = f'objects_scenario definitions "{folder / "grfs"}"'
    if scenario["objects"] == "definitions":
        return {**scenario, "executables": executables, "console": [command, "unpause"]}
    saves = {role: folder / role / "objects-definitions.sav" for role in executables}

    def capture(role: str) -> Scenario:
        return {**scenario, "console": [command + f' "{saves[role]}"', "unpause"]}

    captures = core.capture_pair(
        capture,
        executables,
        builds,
        folder,
        timeout,
        env,
    )
    for role, run in captures.items():
        if (
            run["exit"]
            or not saves[role].is_file()
            or "OBJECT definition complete true" not in run["log"]
        ):
            raise RuntimeError("object definition capture failed: " + role)
    original = [
        line
        for line in captures["reference"]["log"]
        if line.startswith("OBJECT definition ")
    ]
    candidate = [
        line
        for line in captures["candidate"]["log"]
        if line.startswith("OBJECT definition ")
    ]
    differences = core.compare_saves(
        saves["reference"], saves["candidate"], scenario["name"], 10, core.empty_stats()
    )
    if differences or original != candidate:
        raise RuntimeError("object definition publication captures differ")
    core.findings(result)["object_definition_saved"] = definition_rows(
        original, "reordered"
    )
    return {
        **scenario,
        "executables": executables,
        "role_inputs": {role: {"save": str(save)} for role, save in saves.items()},
        "console": ["objects_scenario definitions-inspect", "unpause"],
    }


def check_definitions(
    scenario: Scenario,
    run: core.Run,
    mode: core.Mode,
    role: core.Role,
    result: core.Result,
) -> None:
    rows = [line for line in run["log"] if line.startswith("OBJECT definition ")]
    if scenario["objects"] == "definitions-reload":
        if (
            definition_rows(rows, "inspect")
            != core.findings(result)["object_definition_saved"]
        ):
            raise RuntimeError(
                "object definition fields/classes/identity/RNG changed on saved reload"
            )
        return
    partial = {
        line.split()[3]
        for line in rows
        if line.startswith("OBJECT definition partial ")
    }
    expected = set(
        "limit ignore-byte ignore-word ignore-dword ignore-badges allocation-before-read default class class-name reclass object-name unknown-grow identity size views build clear build-replace date end two flags-replace animation-partial speed triggers badges badges-partial values badge-state names".split()
    )
    if partial != expected or "OBJECT definition complete true" not in rows:
        raise RuntimeError("production object parser observations missing")
    for phase in ("first", "reordered"):
        state = definition_rows(rows, phase)
        if "count 8" not in state or sum(row.startswith("spec ") for row in state) != 8:
            raise RuntimeError("production object publication records missing")
    key = "object_definitions_" + str(mode)
    if role == "reference":
        core.findings(result)[key] = rows
    elif core.findings(result).get(key) != rows:
        raise RuntimeError(
            "object parser partial effects or publication/class order differ"
        )


def install(scenario: Scenario, run_dir: Path) -> None:
    if "object_definition_grfs" in scenario:
        from .newgrf_loading import install_grfs

        install_grfs(Path(scenario["object_definition_grfs"]), run_dir)
