"""Real file-loaded NewGRF interpreter, failure and reset/reload comparisons."""

import calendar
import struct
from collections.abc import Callable, Sequence
from pathlib import Path
from typing import NotRequired, TypedDict, cast

from . import core
from .console_adapter import console_adapter, prepared_console


class Scenario(TypedDict):
    name: str
    kind: str
    save: str
    newgrf_loading: str
    short_checkpoint: bool
    ticks: int
    console: NotRequired[list[str]]
    executables: NotRequired[dict[str, Path]]
    role_inputs: NotRequired[dict[str, dict[str, str]]]
    loading_fixture_dir: NotRequired[str]


class Run(TypedDict):
    exit: int
    log: list[str]
    snapshots: list[Path]


class Result(TypedDict, total=False):
    loading_input: object
    problems: list[str]
    differences: list[core.Difference]


ADAPTER = core.ROOT / "tools/simulation/newgrf-loading-console.cpp"
# Reference town-name lines per scenario and mode, compared with the candidate run.
TOWN_NAMES: dict[tuple[str, str], list[str]] = {}


def scenarios(soak: bool) -> list[Scenario]:
    return [
        {
            "name": "newgrf-loading-" + mode,
            "kind": "save",
            "save": str(core.ROOT / "migration/saves/padhattan-ridge-2000.sav"),
            "newgrf_loading": mode,
            "short_checkpoint": True,
            "ticks": 74 if soak else 1,
        }
        for mode in ("v1", "v2", "bindings", "reload")
    ]


def game_args(scenario: Scenario) -> list[str]:
    return ["-d", "grf=1"] if "newgrf_loading" in scenario else []


def dword(value: int) -> bytes:
    return struct.pack("<I", value)


def param(target: int, value: int, operation: int = 0, source: int = 255) -> bytes:
    return bytes((13, target, operation, source, 255)) + dword(value)


def grf(actions: Sequence[bytes], version: int) -> bytes:
    """Construct both documented container encodings with an empty sprite section."""
    length: Callable[[int], bytes] = (
        dword if version == 2 else lambda n: struct.pack("<H", n)
    )
    body = length(4) + b"\xff" + dword(len(actions))
    body += b"".join(length(len(action)) + b"\xff" + action for action in actions)
    body += length(0)
    if version == 1:
        return body
    return (
        b"\x00\x00GRF\x82\r\n\x1a\n" + dword(len(body) + 1) + b"\x00" + body + dword(0)
    )


def info(identifier: bytes) -> bytes:
    return b"\x08\x08" + identifier + b"Interpreter fixture\x00Loading test\x00"


def action2(
    feature: int,
    setid: int,
    ranges: Sequence[tuple[int, int, int]],
    default: int = 0x7FFF,
    kind: int = 0x81,
    variable: int = 0x0C,
) -> bytes:
    width = {0x81: 1, 0x85: 2, 0x89: 4}[kind]

    def pack(value: int) -> bytes:
        return value.to_bytes(width, "little")

    return (
        bytes((2, feature, setid, kind, variable, 0))
        + pack((1 << (width * 8)) - 1)
        + bytes((len(ranges),))
        + b"".join(
            struct.pack("<H", group) + pack(low) + pack(high)
            for group, low, high in ranges
        )
        + struct.pack("<H", default)
    )


def fixtures(folder: Path, version: int) -> None:
    folder.mkdir(parents=True, exist_ok=True)
    seasonal = bytes(
        (month * 37 + day * 11) % 256 for month in range(12) for day in range(32)
    )
    for name, identifier, table in (
        ("snow", b"SN17", seasonal),
        ("snow-duplicate", b"SN18", bytes([17]) * 384),
    ):
        action8 = (
            bytes((8, 7 if version == 1 else 8))
            + identifier
            + b"Snow fixture\0Seasonal\0"
        )
        action0 = bytes((0, 8, 1, 1, 0, 0x10)) + table
        (folder / f"{name}.grf").write_bytes(grf([action8, action0], version))
    main = [
        info(b"R471"),
        param(0, 77),
        bytes((16, 0x42)),
        bytes((6, 0, 1, 4, 255)),
        bytes((9, 0, 1, 2, 0, 0x42)),
        param(1, 999),
        bytes((16, 0x42)),
        param(1, 123),
        param(2, 0),
        bytes((16, 0x31)),
        param(2, 1, operation=1, source=2),
        bytes((9, 2, 1, 4, 2, 0x31)),
        # A complete usable road engine definition, parsed natively at activation.
        bytes((0, 1, 5, 1, 0, 0, 0, 0, 6, 15, 8, 124, 15, 33, 4, 255)),
        # General-sprite reservation persists from reserve to activation.
        bytes((13, 3, 0, 0, 254)) + dword((3 << 16) | (8 << 8) | 255),
        # Nonfatal GRM exhaustion must not disable the file.
        bytes((13, 4, 0, 4, 254)) + dword((129 << 16) | (11 << 8) | 255),
        bytes((13, 30, 0, 0x13, 254)) + dword(0xFFFF),
        bytes((9, 0x85, 1, 0, 13, 0x66)),
        param(31, 999),
        bytes((16, 0x66)),
        param(31, 66),
        param(32, 0, operation=128),
        bytes((7, 0, 1, 2, 77, 1)),
        param(32, 55),
        bytes((13, 35, 0, 0x84, 0)),
        param(40, 0xFFFFFFFF),
        param(40, 2, operation=1, source=40),
        param(41, 0x80000000),
        param(41, 0xFFFFFFFF, operation=6, source=41),
        param(42, 0xFFFFFFF7),
        param(42, 2, operation=10, source=42),
        param(43, 17),
        param(43, 0, operation=9, source=43),
        param(20, 0xFFFFFFFF),
        bytes((6, 20, 0x84, 5, 255)),
        param(22, 1),
        bytes((14, 1)) + b"V471",
    ]
    main += [
        # The first overlapping range supplies the actual build/refit capacity.
        bytes((0, 1, 3, 1, 0, 0x17, 8, 0x1C, 32, 0x1D, 255, 255)),
        bytes((2, 1, 12, 0x80, 2, 0, 3)) + struct.pack("<HHH", 0x8037, 0x8042, 0x8037),
        bytes((1, 1, 1, 0, 0)),  # A zero-sized Action1 set needs no real sprites.
        bytes((2, 1, 13, 0, 0)),
        bytes((2, 1, 14, 1, 0, 55, 128)),
        bytes((2, 1, 15, 2, 2)) + struct.pack("<HHHH", 0x8037, 0x8037, 0x8037, 0x8037),
        bytes((2, 1, 16, 2, 2)) + struct.pack("<HHHH", 0x8037, 0x8042, 0x8037, 0x8037),
        action2(1, 17, [], default=15, kind=0x85, variable=0x1A),
        bytes((2, 1, 18, 0x81, 0x7E, 17, 0x40, 255, 0, 0, 1))
        + struct.pack("<HBBH", 16, 0, 255, 14),
        action2(
            1,
            10,
            [(18, 0x15, 0x20), (12, 1, 1), (0x8042, 0, 0xFFFFFFFF)],
            default=13,
            kind=0x89,
        ),
        bytes((3, 1, 1, 0, 0, 10, 0)),
        # A native station definition consumes the file-loaded callback graph.
        bytes((0, 4, 3, 1, 0, 8))
        + b"A485"
        + bytes((9, 8))
        + dword(0) * 8
        + bytes((11, 1)),
        action2(4, 11, [(0x8001, 0x13, 0x13)], kind=0x85),
        bytes((3, 4, 1, 0, 0, 11, 0)),
    ]
    main += [
        bytes((0, 9, 1, 1, 0, 8, 0)),
        bytes((2, 9, 20, 0x41))
        + struct.pack("<HHH", 1, 0, 2)
        + bytes((0,))
        + struct.pack("<HHH", 1, 0, 0x12)
        + bytes((0, 0, 0, 1, 1, 1, 0, 0, 0)),
        bytes((3, 9, 1, 0, 0, 20, 0)),
    ]
    for variant in range(3):
        main += [
            bytes((0, 10, 7, 1, variant, 8, 0, 10, 1))
            + dword(7)
            + bytes((0, 0, 0xFE, 0, 0, 0, 0x80))
            + bytes((0x0F, 1, 0x16, 255, 255, 255, 0x21, 4, 0x1A))
            + dword(1 << 15)
            + bytes((0x0B, 1)),
        ]
        if variant == 0:
            payload = struct.pack("<hhhHHB", -2, 0, 0, 7, 0, 0)
        elif variant == 1:
            payload = bytes((0, 0, 0, 1, 0, 0))
        else:
            payload = bytes((0, 1, 1, 1, 0))
        main += [
            bytes((2, 10, 30 + variant, variant)) + payload,
            # Store 7 into register 1 before entering the production result.
            bytes((2, 10, 40 + variant, 0x81, 0x1A, 0x20, 7, 14, 0x1A, 0, 1, 1))
            + struct.pack("<HBBH", 30 + variant, 0, 255, 0x7FFF),
            bytes((3, 10, 1, variant, 0, 40 + variant, 0)),
        ]
    broken = [
        info(b"B471"),
        param(0, 21),
        # The speed mutation survives the following capacity read failure.
        bytes((0, 1, 2, 1, 1, 8, 88, 15)),
    ]
    graph_broken = [
        info(b"B485"),
        action2(1, 30, [(0x8003, 0, 255)]),
        # A callback and partial deterministic node remain pooled; slot 30 retains
        # its prior group until loader reset erases the temporary publication map.
        bytes((2, 1, 30, 0x81, 0x0C, 0, 255, 1, 4, 128)),
    ]

    def town_names(identifier: bytes, actions: Sequence[bytes]) -> list[bytes]:
        return [info(identifier)] + [bytes((15,)) + action for action in actions]

    def text(prob: int, value: bytes) -> bytes:
        return bytes((prob,)) + value + b"\x00"

    # ActionF: <id> [<lang> <name>]... 00 <parts> {<texts> <bitstart> <bitcount> {<prob> ...}}
    names_a = town_names(
        b"TNA5",
        [
            bytes((0x01, 1, 2, 0, 8)) + text(1, b"Al") + text(127, b"Be"),
            # A repeated definition appends a second part list.
            bytes((0x01, 1, 1, 8, 4)) + text(5, b"ton"),
            bytes((0x02, 1, 3, 12, 0)) + text(0, b"X") + text(3, b"Y") + text(0, b"Z"),
            # Two localized names; bit 7 of the second language is cleared.
            bytes((0x83, 0x7F))
            + b"Alpha\x00\x81Alpha US\x00\x00"
            # A 32-bit count follows the x86 shift masking of the original GB.
            + bytes((2, 2, 16, 2, 0x81, 0x01, 0x82, 0x02, 1, 30, 32))
            + text(1, b" Hill"),
            bytes((0x84, 0x7F))
            + b"Beta\x00\x00"
            + bytes((1, 2, 0, 16, 0xFF, 0x03))
            + text(1, b"Q"),
        ],
    )
    names_bad = town_names(
        b"TNX5",
        [
            bytes((0x05, 1, 1, 0, 1)) + text(1, b"ok"),
            bytes((0x80, 0x7F))
            + b"Bad\x00\x00"
            + bytes((1, 2, 0, 1))
            + text(1, b"ok")
            + bytes((0x81, 0x10)),
        ],
    )
    names_b = town_names(
        b"TNB5",
        [
            bytes((0x85, 0x7F))
            + b"Gamma\x00\x00"
            + bytes((1, 1, 0, 1))
            + text(1, b"Gam\x9e")
        ],
    )
    # The reference byte of the last part is missing.
    names_short = town_names(
        b"TNS5",
        [
            bytes((0x81, 0x7F))
            + b"Short\x00\x00"
            + bytes((2, 1, 0, 3))
            + text(2, b"Short")
            + bytes((3, 4, 4))
            + text(1, b"ort")
            + bytes((0x82,)),
        ],
    )
    for name, actions in (
        ("main", main),
        ("names-a", names_a),
        ("names-bad", names_bad),
        ("names-b", names_b),
        ("names-short", names_short),
        ("graph-broken", graph_broken),
        ("broken", broken),
        ("victim", [info(b"V471"), param(0, 999)]),
        (
            "fatal",
            [
                info(b"F471"),
                param(0, 11),
                bytes((13, 1, 0, 0, 254)) + dword((129 << 16) | (11 << 8) | 255),
            ],
        ),
        ("before", [bytes((9, 0x84, 4, 2)) + dword(0) + b"\x00", info(b"Z471")]),
        (
            "after",
            [info(b"E471"), bytes((9, 0x88, 4, 10)) + b"NONE\x00", param(0, 999)],
        ),
    ):
        (folder / f"{name}.grf").write_bytes(grf(actions, version))


def adapter(build: Path, folder: Path, env: dict[str, str]) -> Path:
    return console_adapter(
        build,
        folder,
        env,
        ADAPTER,
        "newgrf_loading",
        replace_objects=("/newgrf/newgrf_act2.cpp.o", "/newgrf_text.cpp.o"),
        native_test_support=True,
    )


def prepare(
    scenario: Scenario,
    binaries: dict[str, Path],
    builds: dict[str, Path],
    out: Path,
    timeout: int,
    env: dict[str, str],
    result: Result,
) -> Scenario:
    if "newgrf_loading" not in scenario:
        return scenario
    folder = out / scenario["name"] / "input"
    normalized, record = core.normalize(Path(scenario["save"]), folder / "source")
    result["loading_input"] = record
    mode = scenario["newgrf_loading"]
    fixtures(folder / "grfs", 1 if mode == "v1" else 2)
    if mode in ("bindings", "reload"):
        text_fixtures(folder / "grfs")
        binding_fixtures(folder / "grfs")
    scenario = {**scenario, "loading_fixture_dir": str(folder / "grfs")}
    executables = prepared_console(
        "newgrf_loading", adapter, binaries, builds, folder, env
    )
    command = f'loading_scenario run "{folder / "grfs"}"'
    if mode != "reload":
        return {
            **scenario,
            "save": str(normalized),
            "executables": executables,
            "console": ['debug_level "grf=8"', command, "unpause"],
        }
    saves = {role: folder / role / "loaded.sav" for role in executables}
    captures = core.capture_pair(
        lambda role: cast(
            core.Scenario,
            dict(
                scenario,
                save=str(normalized),
                console=[
                    'debug_level "grf=8"',
                    command + f' "{saves[role]}"',
                    "unpause",
                ],
                ticks=1,
            ),
        ),
        executables,
        builds,
        folder,
        timeout,
        env,
    )
    for role, run in captures.items():
        seasonal_observations(
            run["log"], ("first", "reset", "reconstructed", "reload"), 8
        )
        if (
            run["exit"]
            or not saves[role].is_file()
            or "LOADING complete true" not in run["log"]
        ):
            raise RuntimeError("file-loaded GRF capture failed: " + role)
    differences = core.compare_saves(
        saves["reference"],
        saves["candidate"],
        scenario["name"],
        10,
        core.empty_stats(),
    )
    if differences or captures["reference"]["log"] != captures["candidate"]["log"]:
        result["differences"].extend(differences)
        raise RuntimeError("file-loaded GRF captures differ")
    return {
        **scenario,
        "executables": executables,
        "role_inputs": {role: {"save": str(save)} for role, save in saves.items()},
        "console": ["loading_scenario inspect", "unpause"],
    }


def seasonal_scaled(raw: int, version: int, limit: int) -> int:
    """Independent fixture expectation for the documented Action0 byte encoding."""
    if raw == 255 or (version < 8 and raw >= 128):
        return 255
    return raw * (1 + limit) // (128 if version < 8 else 256)


def seasonal_observations(
    log: list[str], phases: tuple[str, ...], version: int
) -> list[str]:
    """Validate complete calendar matrices and every transient field before comparison."""
    observations = [line for line in log if line.startswith("LOADING snow ")]
    keys = [
        (phase, year, month, day)
        for phase in phases
        for year in (1999, 2000, 2001, 2100)
        for month in range(12)
        for day in range(1, calendar.monthrange(year, month + 1)[1] + 1)
    ]
    if len(observations) != len(keys):
        raise RuntimeError("seasonal snow matrix missing, truncated or duplicated")
    previous_rng: dict[str, tuple[int, ...]] = {}
    for line, (phase, year, month, day) in zip(observations, keys, strict=True):
        fields = line.split()
        if len(fields) != 17 or fields[2:6] != [phase, str(year), str(month), str(day)]:
            raise RuntimeError("seasonal snow observation key or payload malformed")
        if fields[6] != str(phase != "reset").lower():
            raise RuntimeError("seasonal snow install/reset state differs")
        value, highest, lowest, grf_version, limit, static, *rng = map(int, fields[7:])
        if grf_version != (0 if phase == "reset" else version):
            raise RuntimeError("seasonal snow fixture version missing")

        table = [
            seasonal_scaled((m * 37 + d * 11) % 256, version, limit)
            for m in range(12)
            for d in range(32)
        ]
        expected = (
            (static, static, static)
            if phase == "reset"
            else (
                seasonal_scaled((month * 37 + day * 11) % 256, version, limit),
                max(table),
                min(table),
            )
        )
        if (value, highest, lowest) != expected or not all(
            0 <= v <= 0xFFFFFFFF for v in rng
        ):
            raise RuntimeError("seasonal snow value/extrema/RNG payload differs")
        seed = tuple(rng)
        if previous_rng.setdefault(phase, seed) != seed:
            raise RuntimeError("seasonal snow observations consumed random draws")
    return observations


def townname_observations(log: list[str]) -> list[str]:
    """Town names founded through the production generator, compared across roles."""
    return [line for line in log if line.startswith("LOADING townname town ")]


def check(scenario: Scenario, run: Run, mode: str, role: str, result: Result) -> None:
    if "newgrf_loading" not in scenario or run["exit"]:
        return
    expected = (
        ("LOADING complete true",)
        if scenario["newgrf_loading"] != "reload"
        else ("LOADING reload true",)
    )
    expected += (
        "LOADING case action2-file-graph true",
        "LOADING case action2-normalized-graph true",
        "LOADING case action2-layout-consumer true",
        "LOADING case action2-production-duplicate-error true",
        "LOADING case action2-invalid-cargo-version true",
    )
    if scenario["newgrf_loading"] != "reload":
        expected += ("LOADING case townname-found true",)
    if scenario["newgrf_loading"] != "reload":
        expected += (
            "LOADING case action2-station-command true",
            "LOADING case action2-industry-command true",
            "LOADING case action2-random-trigger true",
            "LOADING case action2-production-v0 true",
            "LOADING case action2-production-v1 true",
            "LOADING case action2-production-v2 true",
        )
    else:
        expected += ("LOADING case action2-industry-reload true",)
    for marker in expected:
        if marker not in run["log"]:
            raise RuntimeError("file-loaded GRF result missing: " + marker)
    phases = (
        ("inspect",) if scenario["newgrf_loading"] == "reload" else ("first", "reload")
    )
    snow_phases = (
        ("inspect",)
        if scenario["newgrf_loading"] == "reload"
        else ("first", "reset", "reconstructed", "reload")
    )
    snow = seasonal_observations(
        run["log"], snow_phases, 7 if scenario["newgrf_loading"] == "v1" else 8
    )
    text = text_observations(
        run["log"], phases, scenario["newgrf_loading"] in ("bindings", "reload")
    )
    bindings = (
        binding_observations(run["log"], phases)
        if scenario["newgrf_loading"] in ("bindings", "reload")
        else []
    )
    names = townname_observations(run["log"]) + snow + text + bindings
    # Generated name bytes and style lookups are not saved; compare them across roles.
    key = (scenario["name"], mode)
    if role == "reference":
        TOWN_NAMES[key] = names
    elif TOWN_NAMES.pop(key, None) != names:
        raise RuntimeError("file-loaded GRF town names or seasonal snow differ")


def install(scenario: Scenario, run_dir: Path) -> None:
    if "loading_fixture_dir" not in scenario:
        return
    source = Path(scenario["loading_fixture_dir"])
    for personal in (
        run_dir / ".openttd",
        run_dir / ".openttd-rust",
        run_dir / "xdg-data/openttd",
        run_dir / "xdg-data/openttd-rust",
    ):
        target = personal / "newgrf/owner471"
        target.mkdir(parents=True, exist_ok=True)
        for path in source.glob("*.grf"):
            (target / path.name).write_bytes(path.read_bytes())


def text_fixtures(folder: Path) -> None:
    """Saves and town-name samples cannot distinguish Action 4/13 table bytes or targets."""

    def name_action(
        feature: int,
        language: int,
        local: int,
        texts: tuple[bytes, ...],
        generic: bool = True,
    ) -> bytes:
        encoded = (
            struct.pack("<H", local)
            if generic
            else (
                b"\xff" + struct.pack("<H", local)
                if feature <= 3 or feature == 21
                else bytes((local,))
            )
        )
        return (
            bytes((4, feature, language | (0x80 if generic else 0), len(texts)))
            + encoded
            + b"\0".join(texts)
            + b"\0"
        )

    modern = [info(b"TX52")]
    for feature, prop, value in (
        (4, 8, b"ST52"),
        (7, 8, b"\0"),
        (17, 8, b"\0"),
        (21, 8, b"badge/text52\0"),
    ):
        modern.append(grf_property(feature, 256 if feature == 21 else 0, prop, value))
    for prop, value in (
        (0x13, b"\x07m\0\x08f\0\0"),
        (0x14, b"\x09nom\0\x0agen\0\0"),
        (0x15, b"\x06"),
    ):
        modern.append(grf_property(8, 7, prop, value))
    modern += [grf_property(8, 96, 0x15, b"\x01")]
    for feature in (*range(22), 0x48):
        modern.append(
            name_action(feature, 1, 0xD800 + feature, (f"feature-{feature}".encode(),))
        )
    for feature in range(4):
        modern.append(name_action(feature, 1, 0, (b"vehicle-name",), False))
    modern += [
        name_action(0x48, 7, 0xD000, (b"localized-7",)),
        name_action(21, 7, 256, (b"badge-name",), False),
        name_action(21, 7, 257, (b"undefined-badge",), False),
    ]
    for local in (
        0xC400,
        0xC500,
        0xC700,
        0xC900,
        0xC4FF,
        0xC5FF,
        0xC7FF,
        0xC9FF,
        0xC600,
    ):
        modern.append(name_action(0x48, 1, local, (b"spec-name",)))
    choice = b"\x9a\x13\x80\x9a\x11default\x9a\x10\x07male\x9a\x12"
    case = b"\x9a\x14\x9a\x11base\x9a\x10\x09nom\x9a\x15\x81\x9a\x11many\x9a\x10\x01one\x9a\x12\x9a\x12"
    modern += [
        name_action(
            0x48, 1, 0xD000, (b"Line\r\n\x80\x81\x01\xd8\x9a\x03\x34\x12\x9e",)
        ),
        name_action(
            0x48,
            7,
            0xD001,
            (
                choice,
                case,
                b"\x9a\x0e\x07\x9a\x0f\x09",
                b"\xc3\x9e\xee\x82\x80\xff\xed\xa0\x80",
            ),
        ),
        name_action(0x48, 96, 0xD005, (choice,)),
        name_action(0x48, 1, 0xFFFF, (b"wrapped", b"end")),
        name_action(0xFE, 1, 0xD000, (b"unsupported",)),
    ]
    old = [
        b"\x08\x06TX56Legacy\0Text\0",
        name_action(0x48, 28, 0xD000, (b"old-languages",)),
        name_action(0x48, 31, 0xD001, (b"old-english-first",)),
    ]
    translated = [
        info(b"TT52"),
        b"\x13TX52\x07\x02\x06\xd0translated\0second\0",
        b"\x13TX52\x01\x02\x0a\xd0\0after-empty\0",
        b"\x13NONEignored\0",
        b"\x13TX52\x01\x01\x00\xc0range\0",
    ]
    for name, actions in (
        ("translation-before", [info(b"TB52"), b"\x13TX52\x07\x01\x00\xd0before\0"]),
        ("text-modern", modern),
        ("text-old", old),
        ("text-translate", translated),
        (
            "text-translate7",
            [
                b"\x08\x07TP57Legacy translation\0Text\0",
                b"\x13TX52\x01\x02\xd0generic-translation\0",
            ],
        ),
        ("translation-bounds", [info(b"TE52"), b"\x13TX"]),
    ):
        (folder / f"{name}.grf").write_bytes(grf(actions, 2))


def text_observations(
    log: list[str], phases: tuple[str, ...], extended: bool
) -> list[str]:
    """Require table keys and switches; saves do not retain bytes or language fallbacks."""
    observations = [line for line in log if line.startswith("LOADING text ")]
    modern = int.from_bytes(b"TX52", "little")
    old = int.from_bytes(b"TX56", "little")
    expected = {(modern, 0xD800 + feature, 1) for feature in (*range(22), 0x48)}
    expected |= {
        (modern, 0xD000, 1),
        (modern, 0xD000, 7),
        (modern, 0xD005, 96),
        (modern, 0xD002, 127),
        (modern, 0xD00B, 1),
    }
    expected |= {(modern, local, 7) for local in range(0xD001, 0xD005)}
    expected |= {(modern, local, 7) for local in (0xD006, 0xD007)}
    expected |= {(old, 0xD000, language) for language in (2, 3, 4)} | {(old, 0xD001, 1)}
    for phase in phases:
        rows = [
            line.split()
            for line in observations
            if line.startswith(f"LOADING text {phase} row ")
        ]
        keys = {(int(row[4], 16), int(row[5], 16), int(row[6], 16)) for row in rows}
        if not rows or len(keys) != len(rows) or (extended and not expected <= keys):
            raise RuntimeError("GRF text table keys missing or duplicated: " + phase)
        for row in rows:
            if len(row) != 10 or not 0x20000 <= int(row[8]) < 0xA0000:
                raise RuntimeError("GRF text return ID or bytes missing")
            if row[9] != "-":
                bytes.fromhex(row[9])
        languages = [
            line.split()
            for line in observations
            if line.startswith(f"LOADING text {phase} language ")
        ]
        if len(languages) != 6 or {int(row[4]) for row in languages} != {
            0,
            1,
            2,
            7,
            96,
            127,
        }:
            raise RuntimeError("GRF text language switch missing or duplicated")
        for row in languages:
            language = int(row[4])
            mask = {2: 4, 3: 8, 4: 16}.get(language, 3)
            expected_mask = sum(1 << value for value in range(32) if value & mask)
            if (
                len(row) != 10
                or int(row[5]) != language
                or int(row[6]) != expected_mask
                or row[7:9] != ["true", "true"]
            ):
                raise RuntimeError("GRF text language state or old/new scheme differs")
        if f"LOADING text {phase} byte80 21" not in observations:
            raise RuntimeError("GRF text custom byte-80 translation missing")
    return observations


def grf_property(feature: int, local: int, prop: int, value: bytes) -> bytes:
    """One Action 0 property, retaining extended local IDs."""
    encoded = bytes((local,)) if local < 255 else b"\xff" + struct.pack("<H", local)
    return bytes((0, feature, 1, 1)) + encoded + bytes((prop,)) + value


def sprite_binding(
    feature: int,
    ids: Sequence[int],
    cargos: Sequence[tuple[int, int]],
    default: int,
    override: bool = False,
) -> bytes:
    """Action 3 encoding; generic callbacks intentionally use the ignored cargo byte."""
    encoded = b"".join(
        bytes((local,)) if local < 255 else b"\xff" + struct.pack("<H", local)
        for local in ids
    )
    return (
        bytes((3, feature, len(ids) | (128 if override else 0)))
        + encoded
        + bytes((len(cargos),))
        + b"".join(struct.pack("<BH", cargo, group) for cargo, group in cargos)
        + struct.pack("<H", default)
    )


def binding_fixtures(folder: Path) -> None:
    """Saves erase binding identity/order and partial parse history; earlier GRFs omit these handlers."""
    main = [
        info(b"BD52"),
        action2(0, 210, [(0x8091, 0, 255)], 0x8091),
        action2(0, 211, [(0x8092, 0, 255)], 0x8092),
    ]
    main += [
        sprite_binding(0, [0, 1], [(0, 210), (255, 211), (254, 256)], 210),
        sprite_binding(0, [27], [(0, 211)], 211, True),
        sprite_binding(0, [1], [], 211),
        sprite_binding(0, [27], [(255, 210)], 210, True),
    ]
    for feature, value in (
        (4, b"BS52"),
        (7, b"\0"),
        (9, b"\0"),
        (10, b"\0"),
        (13, b"\0"),
        (15, b"BO52"),
        (16, b"R052"),
        (17, b"\0"),
        (18, b"D052"),
        (19, b"T052"),
        (20, b"BR52"),
        (21, b"badge/bindings52\0"),
    ):
        main.append(grf_property(feature, 0, 8, value))
    for feature in (4, 5, 7, 9, 10, 11, 13, 15, 16, 17, 18, 19, 20, 21):
        main += [
            action2(feature, 210, [(0x8091, 0, 255)], 0x8091),
            action2(feature, 211, [(0x8092, 0, 255)], 0x8092),
        ]
        if feature in (4, 20):
            cargos = [(0, 210), (254, 211), (255, 210), (254, 256)]
        elif feature in (16, 18, 19, 21):
            cargos = [(0, 210), (255, 211)]
        else:
            cargos = [(255, 210), (0, 211)]
        main += [
            sprite_binding(
                feature,
                [0, 255],
                cargos,
                211 if feature in (7, 9, 10, 13, 15, 17, 21) else 210,
            ),
            sprite_binding(
                feature,
                [0],
                [(255 if feature not in (16, 18, 19, 21) else 0, 211)],
                210 if feature in (7, 9, 10, 13, 15, 17, 21) else 211,
            ),
        ]
    # Nonzero unsupported features are still marked; an out-of-range feature is not.
    main += [bytes((3, 8, 1)), bytes((3, 254, 1))]
    histories = [
        ("bindings-partial-ids", [info(b"BP52"), bytes((3, 0, 2, 2))]),
        (
            "bindings-after-ids",
            [
                info(b"BO52"),
                action2(0, 210, [(0x8094, 0, 255)], 0x8094),
                sprite_binding(0, [27], [], 210, True),
            ],
        ),
        (
            "bindings-partial-group",
            [
                info(b"BQ52"),
                action2(0, 210, [(0x8095, 0, 255)], 0x8095),
                sprite_binding(0, [3], [(0, 210)], 210)[:-2],
            ],
        ),
        (
            "bindings-after-group",
            [
                info(b"BR52"),
                action2(0, 210, [(0x8096, 0, 255)], 0x8096),
                sprite_binding(0, [27], [], 210, True),
            ],
        ),
    ]
    for name, actions in [
        ("bindings-empty", [info(b"BE52"), bytes((3, 0, 128))]),
        ("bindings-main", main),
        *histories,
        (
            "bindings-generic-a",
            [
                info(b"BG52"),
                action2(1, 210, [(0x8097, 0, 255)], 0x8097),
                bytes((3, 1, 0, 123, 210, 0)),
            ],
        ),
        (
            "bindings-generic-b",
            [
                info(b"BH52"),
                action2(1, 210, [(0x8098, 0, 255)], 0x8098),
                bytes((3, 1, 0, 99, 210, 0)),
            ],
        ),
    ]:
        (folder / f"{name}.grf").write_bytes(grf(actions, 2))


def binding_observations(log: list[str], phases: tuple[str, ...]) -> list[str]:
    """Reject missing handlers/order/flags before comparing normalized roots and RNG."""
    observations = [line for line in log if line.startswith("LOADING binding ")]
    expected = {("standard", str(feature)) for feature in (7, 9, 10, 13, 15, 17)}
    expected |= {("station", str(feature)) for feature in (4, 20)}
    expected |= {("engine", str(local)) for local in (0, 1, 2, 3, 27)}
    expected |= {("override", str(index)) for index in range(6)}
    expected |= {("selected", str(local)) for local in range(4)}
    expected |= {("leaf", str(feature)) for feature in (5, 11, 16, 18, 19)}
    expected |= {
        ("flags", grfid)
        for grfid in ("BE52", "BD52", "BG52", "BH52", "BP52", "BO52", "BQ52", "BR52")
    }
    expected |= {("badge", ""), ("generic", ""), ("rng", "")}
    lengths = {
        "standard": 9,
        "station": 10,
        "engine": 10,
        "override": 9,
        "selected": 6,
        "leaf": 7,
        "flags": 7,
        "badge": 6,
        "generic": 6,
        "rng": 8,
    }
    overrides = [
        (0, 146, "0,1,"),
        (64, 146, "0,1,"),
        (65, 145, "1,"),
        (64, 145, "1,"),
        (64, 148, "2,0,"),
        (64, 150, "3,"),
    ]
    features = sum(
        1 << feature
        for feature in (0, 4, 5, 7, 8, 9, 10, 11, 13, 15, 16, 17, 18, 19, 20, 21)
    )
    for phase in phases:
        rows = [
            line.split()
            for line in observations
            if line.startswith(f"LOADING binding {phase} ")
        ]
        keys: set[tuple[str, str]] = set()
        for row in rows:
            kind = row[3]
            if kind not in lengths or len(row) != lengths[kind]:
                raise RuntimeError("malformed binding observation")
            key = (
                kind,
                row[5]
                if kind == "override"
                else ""
                if kind in ("badge", "generic", "rng")
                else row[4],
            )
            if key in keys:
                raise RuntimeError("duplicate binding observation")
            keys.add(key)
            if kind == "override":
                if (
                    row[4] != "27"
                    or (int(row[6]), int(row[7]), row[8]) != overrides[int(row[5])]
                ):
                    raise RuntimeError(
                        "wagon override publication/history order differs"
                    )
            elif kind == "flags":
                status = 1 if row[4] in ("BE52", "BP52", "BQ52") else 4
                flags = (
                    0
                    if row[4] in ("BG52", "BH52")
                    else features
                    if row[4] == "BD52"
                    else 1
                )
                if int(row[5]) != status or int(row[6], 16) != flags:
                    raise RuntimeError(
                        "generic/unsupported/partial feature flags differ"
                    )
            elif kind == "leaf" and int(row[6]) != 146:
                raise RuntimeError("native binding leaf did not replace its group")
            elif kind == "engine":
                groups = {
                    0: (145, 145, 146),
                    1: (145, 146, 146),
                    2: (-1, -1, -1),
                    3: (149, -1, -1),
                    27: (-1, -1, -1),
                }
                if tuple(map(int, row[7:])) != groups[int(row[4])]:
                    raise RuntimeError(
                        "engine cargo/default/purchase partial publication differs"
                    )
            elif kind == "rng" and not all(
                0 <= int(value) <= 0xFFFFFFFF for value in row[4:]
            ):
                raise RuntimeError("binding RNG observation missing")
        if keys != expected:
            raise RuntimeError(
                "binding handlers, override order or flags missing: " + phase
            )
    return observations
