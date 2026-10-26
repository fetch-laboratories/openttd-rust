"""Real file-loaded NewGRF interpreter, failure and reset/reload comparisons."""

import struct
from pathlib import Path

from . import core
from .console_adapter import console_adapter, prepared_console

ADAPTER = core.ROOT / "tools/simulation/newgrf-loading-console.cpp"


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


def fixtures(folder, version):
    folder.mkdir(parents=True, exist_ok=True)
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
        param(20, 0xFFFFFFFF),
        bytes((6, 20, 0x84, 5, 255)),
        param(22, 1),
        bytes((14, 1)) + b"V471",
    ]
    broken = [
        info(b"B471"),
        param(0, 21),
        # The speed mutation survives the following capacity read failure.
        bytes((0, 1, 2, 1, 1, 8, 88, 15)),
    ]
    for name, actions in (
        ("main", main),
        ("broken", broken),
        ("victim", [info(b"V471"), param(0, 999)]),
    ):
        (folder / f"{name}.grf").write_bytes(grf(actions, version))


def adapter(build, folder, env):
    return console_adapter(build, folder, env, ADAPTER, "newgrf_loading")


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
            console=[command, "unpause"],
        )
    saves = {role: folder / role / "loaded.sav" for role in executables}
    captures = core.capture_pair(
        lambda role: dict(
            scenario,
            save=str(normalized),
            console=[command + f' "{saves[role]}"', "unpause"],
            ticks=1,
        ),
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


def check(scenario, run, mode, role, result):
    if "newgrf_loading" not in scenario or run["exit"]:
        return
    expected = (
        ("LOADING complete true",)
        if scenario["newgrf_loading"] != "reload"
        else ("LOADING reload true",)
    )
    for marker in expected:
        if marker not in run["log"]:
            raise RuntimeError("file-loaded GRF result missing: " + marker)


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
