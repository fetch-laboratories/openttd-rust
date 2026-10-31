"""Real file-loaded NewGRF interpreter, failure and reset/reload comparisons."""

import calendar
import json
import shlex
import struct
import subprocess
from pathlib import Path

from . import core
from .console_adapter import console_adapter, prepared_console

ADAPTER = core.ROOT / "tools/simulation/newgrf-loading-console.cpp"
# Reference town-name lines per scenario and mode, compared with the candidate run.
TOWN_NAMES = {}


def scenarios(soak):
    return [
        {
            "name": "newgrf-loading-" + mode,
            "kind": "save",
            "save": str(core.ROOT / "migration/saves/padhattan-ridge-2000.sav"),
            "newgrf_loading": mode,
            "short_checkpoint": True,
            "ticks": 74 if soak else 1,
        }
        for mode in ("v1", "v2", "reload")
    ]


def game_args(scenario):
    return ["-d", "grf=1"] if "newgrf_loading" in scenario else []


def dword(value):
    return struct.pack("<I", value)


def param(target, value, operation=0, source=255):
    return bytes((13, target, operation, source, 255)) + dword(value)


def grf(actions, version):
    """Construct both documented container encodings with an empty sprite section."""
    length = dword if version == 2 else lambda n: struct.pack("<H", n)
    body = length(4) + b"\xff" + dword(len(actions))
    body += b"".join(length(len(action)) + b"\xff" + action for action in actions)
    body += length(0)
    if version == 1:
        return body
    return (
        b"\x00\x00GRF\x82\r\n\x1a\n" + dword(len(body) + 1) + b"\x00" + body + dword(0)
    )


def info(identifier):
    return b"\x08\x08" + identifier + b"Interpreter fixture\x00Loading test\x00"


def action2(feature, setid, ranges, default=0x7FFF, kind=0x81, variable=0x0C):
    width = {0x81: 1, 0x85: 2, 0x89: 4}[kind]

    def pack(value):
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


def fixtures(folder, version):
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

    def town_names(identifier, actions):
        return [info(identifier)] + [bytes((15,)) + action for action in actions]

    def text(prob, value):
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


def adapter(build, folder, env):
    commands = json.loads((build / "compile_commands.json").read_text())
    entry = next(
        item for item in commands if item["file"].endswith("/newgrf/newgrf_act2.cpp")
    )
    args = shlex.split(entry["command"])
    if "-DWITH_RUST" not in args:
        # Compile the candidate's portable bodies without changing the reference.
        folder.mkdir(parents=True, exist_ok=True)
        portable = folder / "action2-portable.o"
        args[args.index("-c") + 1] = str(core.ROOT / "src/newgrf/newgrf_act2.cpp")
        args[args.index("-o") + 1] = str(portable)
        subprocess.run(args, cwd=entry["directory"], env=env, check=True)
        portable.unlink()
    return console_adapter(
        build,
        folder,
        env,
        ADAPTER,
        "newgrf_loading",
        replace_objects=("/newgrf/newgrf_act2.cpp.o",),
    )


def prepare(scenario, binaries, builds, out, timeout, env, result):
    if "newgrf_loading" not in scenario:
        return scenario
    folder = out / scenario["name"] / "input"
    normalized, record = core.normalize(Path(scenario["save"]), folder / "source")
    result["loading_input"] = record
    mode = scenario["newgrf_loading"]
    fixtures(folder / "grfs", 1 if mode == "v1" else 2)
    scenario = dict(scenario, loading_fixture_dir=str(folder / "grfs"))
    executables = prepared_console(
        "newgrf_loading", adapter, binaries, builds, folder, env
    )
    command = f'loading_scenario run "{folder / "grfs"}"'
    if mode != "reload":
        return dict(
            scenario,
            save=str(normalized),
            executables=executables,
            console=['debug_level "grf=8"', command, "unpause"],
        )
    saves = {role: folder / role / "loaded.sav" for role in executables}
    captures = core.capture_pair(
        lambda role: dict(
            scenario,
            save=str(normalized),
            console=['debug_level "grf=8"', command + f' "{saves[role]}"', "unpause"],
            ticks=1,
        ),
        executables,
        builds,
        folder,
        timeout,
        env,
    )
    for role, run in captures.items():
        seasonal_observations(
            run["log"], ("first", "clear", "reconstructed", "reload"), 8
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
        {"chunks": 0, "elements": 0, "masked": {}},
    )
    if differences or captures["reference"]["log"] != captures["candidate"]["log"]:
        result["differences"].extend(differences)
        raise RuntimeError("file-loaded GRF captures differ")
    return dict(
        scenario,
        executables=executables,
        role_inputs={role: {"save": str(save)} for role, save in saves.items()},
        console=["loading_scenario inspect", "unpause"],
    )


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
        if fields[6] != str(phase != "clear").lower():
            raise RuntimeError("seasonal snow install/reset state differs")
        value, highest, lowest, grf_version, limit, static, *rng = map(int, fields[7:])
        if grf_version != version:
            raise RuntimeError("seasonal snow fixture version missing")

        table = [
            seasonal_scaled((m * 37 + d * 11) % 256, version, limit)
            for m in range(12)
            for d in range(32)
        ]
        expected = (
            (static, static, static)
            if phase == "clear"
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


def townname_observations(log: list[str], phases: tuple[str, ...]) -> list[str]:
    """Require each generator lookup and seed sample before comparing roles."""
    names = [line for line in log if line.startswith("LOADING townname ")]
    seeds = (0, 1, 0x80000000, 0xFFFFFFFF) + tuple(
        (k * 0x9E3779B9) & 0xFFFFFFFF for k in range(4, 40)
    )
    for phase in phases:
        prefix = f"LOADING townname {phase} "
        sizes = [line for line in names if line.startswith(prefix + "styles ")]
        if len(sizes) != 1:
            raise RuntimeError("file-loaded GRF style count missing: " + phase)
        count = int(sizes[0].split()[-1])
        expected = {
            (str(gen), f"{seed:08X}") for gen in range(count + 1) for seed in seeds
        }
        actual = {
            tuple(line.split()[4:6])
            for line in names
            if line.startswith(prefix + "name ")
        }
        styles = {
            line.split()[4] for line in names if line.startswith(prefix + "style ")
        }
        if actual != expected or styles != {str(gen) for gen in range(count + 1)}:
            raise RuntimeError(
                "file-loaded GRF town-name observations missing: " + phase
            )
    return names


def check(scenario, run, mode, role, result):
    if "newgrf_loading" not in scenario or run["exit"]:
        return
    expected = (
        ("LOADING complete true",)
        if scenario["newgrf_loading"] != "reload"
        else ("LOADING reload true",)
    )
    expected += (
        "LOADING case action2-file-graph true",
        "LOADING case action2-partial-read true",
        "LOADING case action2-normalized-graph true",
        "LOADING case action2-layout-consumer true",
        "LOADING case action2-partial-allocation-order true",
        "LOADING case action2-legacy-ffxx true",
        "LOADING case action2-modern-ffxx true",
        "LOADING case action2-partial-layout true",
        "LOADING case action2-production-count-error true",
        "LOADING case action2-production-duplicate-error true",
        "LOADING case action2-invalid-cargo-version true",
        "LOADING case action2-layout-flag-error true",
    )
    if scenario["newgrf_loading"] != "reload":
        expected += tuple(
            f"LOADING case townname-{case} true"
            for case in (
                "style-order",
                "fallback",
                "repeated-definition",
                "recursive-final",
                "second-grf",
                "invalid-reference-deleted",
                "invalid-reference",
                "partial-read",
                "found",
            )
        )
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
        else ("first", "clear", "reconstructed", "reload")
    )
    snow = seasonal_observations(
        run["log"], snow_phases, 7 if scenario["newgrf_loading"] == "v1" else 8
    )
    names = townname_observations(run["log"], phases) + snow
    # Generated name bytes and style lookups are not saved; compare them across roles.
    key = (scenario["name"], mode)
    if role == "reference":
        TOWN_NAMES[key] = names
    elif TOWN_NAMES.pop(key, None) != names:
        raise RuntimeError("file-loaded GRF town names differ")


def install(scenario, run_dir):
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
