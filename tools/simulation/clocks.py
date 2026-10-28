"""Clock ownership: semantic DATE state and unchanged-original subscription probes."""

import subprocess
from pathlib import Path
from typing import NotRequired, TypedDict

from . import core
from .console_adapter import console_adapter, prepared_console
from .core import ROOT


class Scenario(TypedDict):
    name: str
    kind: str
    save: str
    clocks: str
    ticks: int
    short_checkpoint: bool
    console: NotRequired[list[str]]
    executables: NotRequired[dict[str, Path]]
    role_inputs: NotRequired[dict[str, dict[str, str]]]


class Run(TypedDict):
    exit: int
    log: list[str]
    snapshots: list[Path]


class Result(TypedDict, total=False):
    problems: list[str]
    differences: list[str]
    clocks_capture: dict[str, list[str]]


def scenarios(soak: bool) -> list[Scenario]:
    return [
        {
            "name": "clocks",
            "kind": "save",
            "save": str(ROOT / "migration/saves/padhattan-ridge-2000.sav"),
            "clocks": "reload",
            "ticks": 74 if soak else 1,
            "short_checkpoint": True,
        }
    ]


def adapter(build: Path, folder: Path, env: dict[str, str]) -> Path:
    return console_adapter(
        build, folder, env, ROOT / "tools/simulation/clocks-console.cpp", "clocks"
    )


def native_probes(folder: Path, env: dict[str, str]) -> None:
    """Native aliases, registration threads and exceptions do not appear in saves."""
    import probe

    out = probe.output("clock-native", folder)
    expected: bytes | None = None
    for assertions in (False, True):
        for label, source in probe.LABELS:
            target = out / f"{label}-{assertions}"
            target.mkdir(exist_ok=True)
            bodies = []
            if label == "candidate":
                for clock in ("Calendar", "Economy"):
                    text = (
                        source / f"src/timer/timer_game_{clock.lower()}.cpp"
                    ).read_text()
                    for prefix, function in (
                        (f"TimerGame{clock}::YearMonthDay ", "ConvertDateToYMD"),
                        (f"TimerGame{clock}::Date ", "ConvertYMDToDate"),
                        ("void ", "SetDate"),
                    ):
                        bodies.append(
                            probe.body(text, f"{prefix}TimerGame{clock}::{function}")
                        )
                    if clock == "Economy":
                        bodies.append(
                            probe.body(
                                text, "bool TimerGameEconomy::UsingWallclockUnits"
                            )
                        )
                    for timer in ("IntervalTimer", "TimeoutTimer"):
                        bodies.append(
                            "template <>\n"
                            + probe.body(
                                text, f"void {timer}<TimerGame{clock}>::Elapsed"
                            )
                        )
                    bodies.append(
                        "template <>\n"
                        + probe.body(
                            text, f"bool TimerManager<TimerGame{clock}>::Elapsed"
                        )
                    )
            else:
                for clock in ("common", "calendar", "economy"):
                    text = (source / f"src/timer/timer_game_{clock}.cpp").read_text()
                    start = (
                        "#define M("
                        if clock == "common"
                        else f"TimerGame{clock.title()}::Year TimerGame"
                    )
                    bodies.append(
                        text[text.index(start) :].removesuffix(
                            "\n#endif /* WITH_RUST */\n"
                        )
                    )
            (target / "clock-bodies.inc").write_text("\n".join(bodies))
            command = [
                probe.compiler(),
                "-std=c++20",
                "-O2",
                *probe.target_flags(),
                "-DWITH_ASSERT" if assertions else "-DNDEBUG",
                "-I" + str(source / "src"),
                "-I" + str(target),
                str(ROOT / "tools/simulation/clocks-native.cpp"),
                "-pthread",
                "-o",
                str(target / "probe"),
            ]
            probe.compile(
                command + (probe.rust() if label == "candidate" else []),
                target / "build.log",
                env,
            )
            output = probe.execute([target / "probe"], target / "run", env=env)
            if expected is None:
                expected = output
            probe.same(expected, output, label, target)
            duplicate = subprocess.run(
                [target / "probe", "duplicate"],
                env=env,
                capture_output=True,
                check=False,
            )
            (target / "duplicate.log").write_bytes(duplicate.stdout + duplicate.stderr)
            if (duplicate.returncode != 0) != assertions:
                raise RuntimeError(
                    f"{label}: native duplicate-priority assertion policy differs"
                )
    if expected is None or b"CLOCK exception-snapshot-cleanup true" not in expected:
        raise RuntimeError("clock native comparison did not run")


def prepare(
    scenario: Scenario,
    binaries: dict[str, Path],
    builds: dict[str, Path],
    out: Path,
    timeout: int,
    env: dict[str, str],
    result: Result,
) -> Scenario:
    if "clocks" not in scenario:
        return scenario
    folder = out / scenario["name"] / "input"
    native_probes(folder / "native", env)
    executables = prepared_console("clocks", adapter, binaries, builds, folder, env)
    saves = {role: folder / role / "fractions.sav" for role in executables}
    runs = core.capture_pair(
        lambda role: dict(
            scenario,
            console=[f'clocks_scenario capture "{saves[role]}"', "unpause"],
            ticks=1,
        ),
        executables,
        builds,
        folder,
        timeout,
        env,
    )
    traces = {}
    for role, run in runs.items():
        traces[role] = [line for line in run["log"] if line.startswith("CLOCK ")]
        if (
            run["exit"]
            or not saves[role].is_file()
            or "CLOCK saved-fractions true" not in traces[role]
        ):
            raise RuntimeError(f"{role}: clock capture failed")
    differences = core.compare_saves(
        saves["reference"],
        saves["candidate"],
        scenario["name"],
        10,
        {"chunks": 0, "elements": 0, "masked": {}},
    )
    if differences or traces["reference"] != traces["candidate"]:
        raise RuntimeError("clock capture state or trace differs")
    result["clocks_capture"] = traces
    return {
        **scenario,
        "executables": executables,
        "role_inputs": {role: {"save": str(save)} for role, save in saves.items()},
        "console": ["clocks_scenario inspect", "unpause"],
    }


def check(scenario: Scenario, run: Run, mode: str, role: str, result: Result) -> None:
    if "clocks" in scenario and "CLOCK reloaded-fractions true" not in run["log"]:
        result["problems"].append(f"{mode}: {role} clock reload observation missing")
