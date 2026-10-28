"""Actual session entry comparisons, unsaved aliases and native exception phases."""

from pathlib import Path
from typing import NotRequired, TypedDict

from . import core, landscape
from .console_adapter import console_adapter, prepared_console

STATES = (
    "normal",
    "paused",
    "frozen-calendar",
    "newgame",
    "restart",
    "reload-random",
    "editor",
    "editor-heightmap",
    "reload-heightmap",
    "new-heightmap",
    "restart-heightmap",
    "load-scenario",
    "reload-scenario",
    "missing-load",
    "partial-load",
    "reload-save",
    "outer-editor",
    "final",
)


class Scenario(TypedDict):
    name: str
    kind: str
    save: str
    session: bool
    ticks: int
    short_checkpoint: bool
    console: NotRequired[list[str]]
    partial: NotRequired[str]
    executables: NotRequired[dict[str, Path]]
    role_inputs: NotRequired[dict[str, dict[str, str]]]


class Run(TypedDict):
    log: list[str]


class Result(TypedDict, total=False):
    problems: list[str]
    session_capture: dict[str, list[str]]


def scenarios(soak: bool) -> list[Scenario]:
    return [
        {
            "name": "session-lifecycle",
            "kind": "save",
            "session": True,
            "save": str(core.ROOT / "migration/saves/padhattan-ridge-2000.sav"),
            "ticks": 74 if soak else 1,
            "short_checkpoint": True,
        }
    ]


def adapter(build: Path, folder: Path, env: dict[str, str]) -> Path:
    return console_adapter(
        build,
        folder,
        env,
        core.ROOT / "tools/simulation/session-console.cpp",
        "session",
        wraps=("_ZN4Game8GameLoopEv",),
    )


def install(scenario: Scenario, run_dir: Path) -> None:
    if not scenario.get("session"):
        return
    landscape.install({"generation_probe": True}, run_dir)
    (run_dir / "session-failure.scr").write_text("session_throw\n")
    if partial := scenario.get("partial"):
        data = Path(partial).read_bytes()
        (run_dir / "partial.sav").write_bytes(data[: len(data) // 2])


def entry_observations(lines: list[str]) -> None:
    """Clock/RNG/credit fields distinguish skipping a phase from executing it."""
    rows = {
        parts[2]: tuple(map(int, parts[3:]))
        for line in lines
        if line.startswith("SESSION state ")
        if (parts := line.split())
    }
    before, normal, paused = (
        rows[key] for key in ("before-normal", "normal", "paused")
    )
    if normal[8] != before[8] + 1 or normal[11] <= before[11]:
        raise RuntimeError("normal entry did not advance ticks and landscaping credit")
    for field in (*range(3, 11), 12):
        if normal[field] != paused[field]:
            raise RuntimeError("paused entry advanced clocks, RNG or world")
    if paused[11] <= normal[11]:
        raise RuntimeError("paused entry did not accrue landscaping credit")
    if rows["modal-paused"][3:] != paused[3:] or rows["modal"][3:] != paused[3:]:
        raise RuntimeError("modal entry changed clocks, RNG, world or credit")
    if rows["before-editor"][3:9] != rows["editor"][3:9]:
        raise RuntimeError("editor entry advanced simulation clocks")


def prepare(
    scenario: Scenario,
    binaries: dict[str, Path],
    builds: dict[str, Path],
    out: Path,
    timeout: int,
    env: dict[str, str],
    result: Result,
) -> Scenario:
    if not scenario.get("session"):
        return scenario
    folder = out / scenario["name"] / "input"
    source, _ = core.normalize(Path(scenario["save"]), folder / "source")
    scenario = {**scenario, "save": str(source), "partial": str(source)}
    executables = prepared_console("session", adapter, binaries, builds, folder, env)
    captures: dict[str, list[str]] = {}
    saves: dict[str, Path] = {}
    for snapshots in (False, True):
        mode = "desync" if snapshots else "plain"
        for role in ("reference", "candidate"):
            states = folder / role / f"states-{mode}"
            states.mkdir(parents=True)
            capture = core.run_game(
                {
                    **scenario,
                    "console": [f'session_scenario capture "{states}"', "unpause"],
                },
                executables[role],
                builds[role],
                folder / role / mode,
                timeout,
                env,
                snapshots,
            )
            lines = [line for line in capture["log"] if line.startswith("SESSION ")]
            captures[f"{mode}-{role}"] = lines
            required = {
                "clock-exception",
                "script-exception",
                "scan-request",
                "scan-busy",
                "scan-reentry",
                "scan-replacement",
                "missing-load",
                "partial-load",
                "new-settings",
                "current-settings",
                "editor-owner",
                "heightmap-new-settings",
            }
            observed = {
                line.split()[2]
                for line in lines
                if line.startswith("SESSION check ") and line.endswith(" true")
            }
            if capture["exit"] or required - observed:
                raise RuntimeError(
                    f"{mode}: {role} incomplete session evidence: {required - observed}"
                )
            entry_observations(lines)
            if mode == "plain":
                saves[role] = states / "final.sav"
        if captures[f"{mode}-reference"] != captures[f"{mode}-candidate"]:
            raise RuntimeError(f"{mode}: actual session entry observations differ")
        for state in STATES:
            paths = [
                folder / role / f"states-{mode}" / f"{state}.sav"
                for role in ("reference", "candidate")
            ]
            if not all(path.is_file() for path in paths):
                raise RuntimeError(f"{mode}: missing {state} session save")
            differences = core.compare_saves(
                *paths, scenario["name"], 10, {"chunks": 0, "elements": 0, "masked": {}}
            )
            if differences:
                raise RuntimeError(
                    f"{mode}: {state} session state differs: {differences}"
                )
    result["session_capture"] = captures
    return {
        **scenario,
        "executables": executables,
        "role_inputs": {role: {"save": str(save)} for role, save in saves.items()},
        "console": ["session_scenario inspect unused", "unpause"],
    }


def check(scenario: Scenario, run: Run, mode: str, role: str, result: Result) -> None:
    if scenario.get("session") and "SESSION check reload-mode true" not in run["log"]:
        result["problems"].append(f"{mode}: {role} session reload observation missing")
