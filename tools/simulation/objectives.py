"""Default objectives commands, unsaved results/events, semantic chunks and reload."""

from pathlib import Path
from typing import Any, TypedDict

from . import core
from .console_adapter import console_adapter, prepared_console


class Run(TypedDict):
    exit: int
    log: list[str]


def scenarios(soak: bool) -> list[dict[str, Any]]:
    return [
        dict(
            name="objectives",
            kind="save",
            objectives=True,
            save=str(core.ROOT / "migration/saves/padhattan-ridge-2000.sav"),
            ticks=74 if soak else 1,
            short_checkpoint=True,
        )
    ]


def adapter(build: Path, folder: Path, env: dict[str, str]) -> Path:
    return console_adapter(
        build,
        folder,
        env,
        core.ROOT / "tools/simulation/objectives-console.cpp",
        "objectives",
        wraps=("_ZN4Game8NewEventEP11ScriptEvent",),
    )


def observations(run: Run) -> list[str]:
    lines = [line for line in run["log"] if line.startswith("OBJECTIVES ")]
    required = (
        "commands-complete",
        "state-complete",
        "events-complete 6",
        "result goal-global-private false",
        "result goal-destination-refused false",
        "result page-cap-refused false",
        "result button-push-test true",
        "result button-push true",
        "result button-vehicle true",
        "result button-tile true",
        "result answer-client true",
        "result question-missing-client-test false",
        "result question-missing-client-execute true",
    )
    if run["exit"] or any(
        sum(line.startswith("OBJECTIVES " + marker) for line in lines) != 1
        for marker in required
    ):
        raise RuntimeError(
            "missing required production objectives result/event evidence"
        )
    if len([line for line in lines if line.startswith("OBJECTIVES event ")]) != 6:
        raise RuntimeError("missing ordered GS goal/button event payloads")
    return lines


def prepare(
    scenario: dict[str, Any],
    binaries: dict[str, Path],
    builds: dict[str, Path],
    out: Path,
    timeout: int,
    env: dict[str, str],
    result: dict[str, Any],
) -> dict[str, Any]:
    if "objectives" not in scenario:
        return scenario
    folder = out / scenario["name"] / "input"
    source, _ = core.normalize(Path(scenario["save"]), folder / "normalized")
    scenario = dict(scenario, save=str(source))
    executables = prepared_console("objectives", adapter, binaries, builds, folder, env)
    saves = {role: folder / role / "objectives.sav" for role in executables}
    captures = core.capture_pair(
        lambda role: dict(
            scenario,
            console=[f'objectives_scenario commands "{saves[role]}"', "unpause"],
        ),
        executables,
        builds,
        folder,
        timeout,
        env,
    )
    traces = {role: observations(run) for role, run in captures.items()}
    differences = core.compare_saves(
        saves["reference"],
        saves["candidate"],
        scenario["name"],
        10,
        {"chunks": 0, "elements": 0, "masked": {}},
    )
    if traces["reference"] != traces["candidate"] or differences:
        raise RuntimeError("objectives production results, events or save state differ")
    result["objectives_state"] = [
        line
        for line in traces["reference"]
        if line.split()[1]
        in ("goal", "page", "element", "entry", "rng", "state-complete")
    ]
    return dict(
        scenario,
        executables=executables,
        role_inputs={role: {"save": str(save)} for role, save in saves.items()},
        console=["objectives_scenario inspect", "unpause"],
    )


def check(
    scenario: dict[str, Any],
    run: Run,
    mode: str,
    role: str,
    result: dict[str, Any],
) -> None:
    if "objectives" in scenario:
        lines = [line for line in run["log"] if line.startswith("OBJECTIVES ")]
        if (
            lines != result["objectives_state"]
            or "OBJECTIVES state-complete" not in lines
        ):
            raise RuntimeError(
                f"{mode}/{role}: missing or changed objectives reload state"
            )
