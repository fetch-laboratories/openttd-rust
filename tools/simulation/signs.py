"""Default sign and depot-name commands, semantic SIGN/DEPT records and reload."""

from pathlib import Path
from typing import Any, TypedDict

from . import core
from .console_adapter import console_adapter, prepared_console


class Run(TypedDict):
    exit: int
    log: list[str]


REQUIRED = (
    "result place-test true 65535 65535",
    "result place-utf8-32 false 65535 65535",
    "result rename-deity-refused false 65535",
    "result move-deity-refused false 65535",
    "result delete-deity-refused false 65535",
    "result rename-deity-editor true",
    "result move-other true",
    "result rename-utf8-31 true",
    "result rename-utf8-32 false 65535",
    "result rename-invalid false 65535",
    "result move-invalid false 65535",
    "result delete-test true",
    "result delete-again false 65535",
    "result place-full false",
    "result depot-invalid false 65535",
    "result depot-reset-named true",
    "result depot-name-freed true",
    "filled ",
    "rng ",
    "interactive-rng ",
    "state-complete",
    "commands-complete",
)


def scenarios(soak: bool) -> list[dict[str, Any]]:
    return [
        dict(
            name="signs",
            kind="save",
            signs=True,
            save=str(core.ROOT / "migration/saves/padhattan-ridge-2000.sav"),
            ticks=74 if soak else 1,
            short_checkpoint=True,
        )
    ]


def adapter(build: Path, folder: Path, env: dict[str, str]) -> Path:
    return console_adapter(
        build, folder, env, core.ROOT / "tools/simulation/signs-console.cpp", "signs"
    )


def observations(run: Run) -> list[str]:
    lines = [line for line in run["log"] if line.startswith("SIGNS ")]
    kinds = [line for line in lines if line.startswith("SIGNS depot-kinds ")]
    if (
        run["exit"]
        or any(
            sum(line.startswith("SIGNS " + marker) for line in lines) != 1
            for marker in REQUIRED
        )
        or len(kinds) != 1
        or int(kinds[0].split()[2]) < 2
        or not any(
            line.split()[2].endswith("-duplicate")
            and line.split()[3:] not in (["false", "65535"], ["true", "65535"])
            for line in lines
            if line.startswith("SIGNS result depot-")
        )
        or not any(line.startswith("SIGNS sign ") for line in lines)
        or not any(line.startswith("SIGNS depot ") for line in lines)
    ):
        raise RuntimeError("missing required production sign/depot-name evidence")
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
    if "signs" not in scenario:
        return scenario
    folder = out / scenario["name"] / "input"
    source, _ = core.normalize(Path(scenario["save"]), folder / "normalized")
    scenario = dict(scenario, save=str(source))
    executables = prepared_console("signs", adapter, binaries, builds, folder, env)
    saves = {role: folder / role / "signs.sav" for role in executables}
    captures = core.capture_pair(
        lambda role: dict(
            scenario, console=[f'signs_scenario commands "{saves[role]}"', "unpause"]
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
        raise RuntimeError("sign/depot-name results, records or save state differ")
    result["signs_state"] = [
        line
        for line in traces["reference"]
        if line.split()[1] in ("sign", "depot", "rng", "state-complete")
    ]
    return dict(
        scenario,
        executables=executables,
        role_inputs={role: {"save": str(save)} for role, save in saves.items()},
        console=["signs_scenario inspect", "unpause"],
    )


def check(
    scenario: dict[str, Any],
    run: Run,
    mode: str,
    role: str,
    result: dict[str, Any],
) -> None:
    if "signs" not in scenario:
        return
    lines = [line for line in run["log"] if line.startswith("SIGNS ")]
    records = [line for line in lines if line.split()[1] in ("sign", "depot", "rng")]
    if (
        "SIGNS state-complete" not in lines
        or records + ["SIGNS state-complete"] != result["signs_state"]
    ):
        raise RuntimeError(f"{mode}/{role}: missing or changed sign/depot reload state")
