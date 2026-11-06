"""Game preparation, execution, decoding and frozen runtimes."""

import contextlib
import json
import os
import re
import shutil
import subprocess
import sys
import threading
import time
from collections.abc import Callable, Generator, Iterable, Mapping
from pathlib import Path
from typing import IO, Any, Literal, cast

from validation_execution import file_lock

from .configuration import configuration
from .models import (
    ROLES,
    ROOT,
    RUNTIME_DIRECTORIES,
    Executed,
    GameArgs,
    GameEnvironment,
    Install,
    Plan,
    Prepare,
    Result,
    Run,
    Scenario,
)
from .registry import hook_modules, sanitized


def set_option(text: str, section: str, line: str) -> str:
    """Add a line to an existing cfg section (a repeated section would be ignored)."""
    text, count = re.subn(
        rf"^\[{section}\]\s*$", f"[{section}]\n{line}", text, count=1, flags=re.M
    )
    return text if count else f"{text.rstrip()}\n[{section}]\n{line}\n"


def write_config(scenario: Scenario, build: Path, run_dir: Path) -> None:
    if scenario["kind"] == "regression":
        text = (build / "regression/regression.cfg").read_text()
    else:
        text = (
            "[misc]\nlanguage = english.lng\n"
            "[gui]\nautosave = off\n"
            "[difficulty]\nmax_no_competitors = 0\ndisasters = true\n"
            "[game_creation]\ntown_name = english\n"
        )
        if scenario["kind"] == "generate":
            text += (
                f"map_x = {scenario['map_log2']}\nmap_y = {scenario.get('map_log2_y', scenario['map_log2'])}\n"
                f"land_generator = {scenario['land_generator']}\n"
            )
    for section, settings in scenario.get("settings", {}).items():
        for name, value in settings.items():
            text = set_option(text, section, f"{name} = {value}")
    text = set_option(text, "misc", "savegame_format = none")
    # The null video driver writes save/autosave/exit.sav when it stops.
    text = set_option(text, "gui", "autosave_on_exit = true")
    (run_dir / "openttd.cfg").write_text(text)


GAME_LOCK: Path | None = None  # Set by main to the clone-wide game coordination file.


@contextlib.contextmanager
def game_slot() -> Generator[None, None, None]:
    """All games share coordination; timed games never reserve a quiet host."""
    path = GAME_LOCK
    if path is None:
        path = configuration().common_local / "simulation-game.lock"
    with file_lock(path, shared=True, label="simulation game"):
        yield


def benchmark_run(
    command: list[str],
    *,
    timeout: float,
    cwd: Path,
    env: Mapping[str, str],
    stdout: IO[Any] | int,
    stderr: IO[Any] | int,
    started: Callable[[subprocess.Popen[bytes]], None] | None = None,
) -> subprocess.CompletedProcess[bytes]:
    """Wait without polling; the watchdog kills and the caller reaps on timeout.

    `started` receives the child, so an observer can stop it early."""
    with subprocess.Popen(
        command, cwd=cwd, env=env, stdout=stdout, stderr=stderr
    ) as child:
        if started is not None:
            started(child)
        expired = threading.Event()

        def kill() -> None:
            expired.set()
            child.kill()

        watchdog = threading.Timer(timeout, kill)
        try:
            watchdog.start()
            code = child.wait()
        except BaseException:
            child.kill()
            child.wait()
            raise
        finally:
            watchdog.cancel()
            if watchdog.ident is not None:
                watchdog.join()
        if expired.is_set():
            raise subprocess.TimeoutExpired(command, timeout)
        return subprocess.CompletedProcess(command, code)


def install_ai(scenario: Scenario, run_dir: Path, name: str, parameters: str) -> Path:
    """Copy the frozen scenario AI and write its per-run parameters verbatim."""
    folder = run_dir / "ai" / name
    shutil.copytree(scenario["scenario_ai"], folder)
    (folder / "parameters.nut").write_text(parameters)
    return folder


def prepare_run_dir(scenario: Scenario, build: Path, run_dir: Path) -> None:
    """Write a game's configuration, module files and console script."""
    shutil.rmtree(run_dir, ignore_errors=True)
    run_dir.mkdir(parents=True)
    write_config(scenario, build, run_dir)
    for module in hook_modules(scenario, "install"):
        cast(Install, module.install)(scenario, run_dir)
    if "console" in scenario:
        (run_dir / "scripts").mkdir()
        (run_dir / "scripts/game_start.scr").write_text(
            "".join(f"{line}\n" for line in scenario["console"])
        )


def prepare_scenario(
    scenario: Scenario,
    binaries: dict[str, Path],
    builds: dict[str, Path],
    out: Path,
    timeout: int,
    env: dict[str, str],
    result: Result,
) -> Scenario:
    """Module preparation hooks in PREPARE_ORDER, not scenario-list order."""
    for module in sorted(
        hook_modules(scenario, "prepare"), key=lambda m: getattr(m, "PREPARE_ORDER", 4)
    ):
        scenario = cast(Prepare, module.prepare)(
            scenario, binaries, builds, out, timeout, env, result
        )
    return scenario


def run_game(
    scenario: Scenario,
    binary: Path,
    build: Path,
    run_dir: Path,
    timeout: int,
    base_env: Mapping[str, str] | None = None,
    desync: bool = True,
    *,
    isolate: bool = False,
    instructions: bool = False,
    native_threads: bool = False,
) -> Run:
    """Run one binary on one scenario in an isolated personal directory.

    desync=3 writes the 32-day snapshots but also checks and rebuilds caches
    every tick and switches YAPF rail to its uncached path, so every scenario
    also has a plain run without it whose exit save is compared."""
    return read_game(
        execute_game(
            prepare_game(
                scenario,
                binary,
                build,
                run_dir,
                timeout,
                base_env,
                desync,
                isolate=isolate,
                instructions=instructions,
                native_threads=native_threads,
            )
        )
    )


def prepare_game(
    scenario: Scenario,
    binary: Path,
    build: Path,
    run_dir: Path,
    timeout: int,
    base_env: Mapping[str, str] | None = None,
    desync: bool = True,
    *,
    isolate: bool = False,
    instructions: bool = False,
    native_threads: bool = False,
) -> Plan:
    """Prepare a launch without executing or decoding a game."""
    prepare_run_dir(scenario, build, run_dir)
    if scenario["kind"] == "save":
        # A saved AI or GameScript missing from the runtime is replaced by the
        # idle dummy AI or dropped; the logs show it in both runs.
        game = ["-g", scenario["save"], "-d", "script=2"]
    elif scenario["kind"] == "regression":
        game = ["-g", f"ai/{scenario['test']}/test.sav", "-d", "script=2", "-Q"]
    else:
        game = ["-g", "-G", str(scenario["seed"])]
    for module in hook_modules(scenario, "game_args"):
        game += cast(GameArgs, module.game_args)(scenario)
    command = [
        str(binary),
        "-x",
        "-c",
        str(run_dir / "openttd.cfg"),
        *game,
        "-snull",
        "-mnull",
        scenario.get("video", f"-vnull:ticks={scenario['ticks']}"),
        *(["-d", "desync=3"] if desync else []),
    ]
    env = dict(
        base_env or os.environ,
        HOME=str(run_dir),
        XDG_DATA_HOME=str(run_dir / "xdg-data"),
        XDG_CONFIG_HOME=str(run_dir / "xdg-config"),
        XDG_CACHE_HOME=str(run_dir / "xdg-cache"),
    )
    for module in hook_modules(scenario, "game_environment"):
        env.update(cast(GameEnvironment, module.game_environment)(scenario))
    if sanitized():
        from . import sanitizers

        env.update(sanitizers.environment(run_dir))
    instruction_info = None
    if instructions:
        from . import instructions as profiler

        command += ["-r", "64x64"]
        command, instruction_info = profiler.prepare(
            scenario, command, binary, build, run_dir, env, run_dir.name == "reference"
        )
    return Plan(
        dir=run_dir,
        command=command,
        env=env,
        build=build,
        timeout=timeout,
        isolate=isolate,
        instructions=instruction_info,
        native_threads=native_threads,
    )


def execute_game(plan: Plan) -> Executed:
    """Execute an already prepared game; leave output decoding to read_game."""
    run_dir, command = plan["dir"], plan["command"]
    with (
        game_slot(),
        open(run_dir / "stdout.log", "wb") as out,
        open(run_dir / "stderr.log", "wb") as err,
    ):
        started = time.monotonic()
        code: int | Literal["timeout"]
        try:
            code = (benchmark_run if plan["isolate"] else subprocess.run)(
                command
                if plan.get("native_threads")
                else [
                    sys.executable,
                    str(ROOT / "tools/simulation/game_launcher.py"),
                    *command,
                ],
                cwd=plan["build"],
                env=plan["env"],
                stdout=out,
                stderr=err,
                timeout=plan["timeout"],
            ).returncode
        except subprocess.TimeoutExpired:
            code = "timeout"
        seconds = round(time.monotonic() - started, 3)
    return {
        "instructions": plan["instructions"],
        "dir": run_dir,
        "exit": code,
        "seconds": seconds,
        "command": command,
    }


def read_game(run: Mapping[str, Any]) -> Run:
    """Decode a completed game's logs and discover its snapshots.

    `run` is an Executed record or a stored capture with at least its `dir`."""
    run_dir = run["dir"]
    autosave = run_dir / "save/autosave"
    snapshots = sorted(
        path
        for path in autosave.glob("dmp_cmds_*.sav")
        if not path.name.endswith("_00000000_00000000.sav")
    )  # title screen
    if (autosave / "exit.sav").is_file():
        snapshots.append(autosave / "exit.sav")
    log = log_lines(run_dir)
    sanitizer = None
    if sanitized():
        from . import sanitizers

        config = configuration()
        roots = (config.reference.resolve(), ROOT)
        builds = (config.reference_build, config.candidate_build)
        log, sanitizer = sanitizers.split_log(log, roots, builds)
        sanitizers.file_findings(run_dir, roots, sanitizer, builds)
    return cast(
        Run,
        {
            **run,
            "snapshots": snapshots,
            "log": log,
            "stdout": (run_dir / "stdout.log").read_bytes(),
            "sanitizer": sanitizer,
        },
    )


def capture_pair(
    scenario: Scenario | Callable[[str], Scenario],
    executables: Mapping[str, Path],
    builds: Mapping[str, Path],
    folder: Path,
    timeout: int,
    env: Mapping[str, str] | None,
    name: str = "capture",
) -> dict[str, Run]:
    """One plain preparation game per role in folder/role/name.

    `scenario` is a dictionary or a function of the role."""
    return {
        role: run_game(
            cast(Scenario, scenario(role) if callable(scenario) else scenario),
            executables[role],
            builds[role],
            folder / role / name,
            timeout,
            env,
            False,
        )
        for role in ROLES
    }


WITNESS_ENVIRONMENT = {"OPENTTD_WITNESS": "1"}


def witness_counts(run: Run | Executed, prefix: str) -> dict[str, int] | None:
    """A run's `prefix.*` counters, unprefixed, or None without a witness file."""
    path = run["dir"] / "branch-witnesses.json"
    if not path.is_file():
        return None
    counts = json.loads(path.read_text())
    return {
        name.removeprefix(f"{prefix}."): count
        for name, count in counts.items()
        if name.startswith(f"{prefix}.")
    }


def branch_witnesses(
    scenario: Scenario, run: Run, role: str, prefix: str, required: Iterable[str]
) -> dict[str, int] | None:
    """Return a candidate's `prefix.*` counts; a missing file or zero count fails.

    Scenario modules enable counters through game_environment when
    scenario["witnesses"] is set; the reference and --self have none."""
    if not (scenario.get("witnesses") and role == "candidate"):
        return None
    counts = witness_counts(run, prefix)
    if counts is None:
        raise RuntimeError("candidate wrote no branch-witnesses.json")
    if missing := [branch for branch in required if not counts[branch]]:
        raise RuntimeError(f"{prefix} lacks required branches {missing}: {counts}")
    return counts


def log_lines(run_dir: Path) -> list[str]:
    """Debug output (script logs, warnings) without its timestamps."""
    return [
        re.sub(r"^\[[^\]]*\] ", "", line)
        for line in (run_dir / "stderr.log")
        .read_text(errors="surrogateescape")
        .splitlines()
    ]


def copy_runtime(
    build: Path, binary: Path, destination: Path, *, candidate: bool = False
) -> Path:
    if candidate:
        with file_lock(
            Path(build).resolve() / ".migration.lock",
            shared=True,
            label="candidate capture",
        ):
            return _copy_runtime(build, binary, destination)
    return _copy_runtime(build, binary, destination)


def _copy_runtime(build: Path, binary: Path, destination: Path) -> Path:
    """Freeze the executable and its data, including symlink targets, for one run."""

    def missing_links(directory: str, names: list[str]) -> list[str]:
        # Extracted basesets can contain dangling documentation links. They were
        # unavailable in the source runtime too; resolve relative links there.
        return [
            name
            for name in names
            if (path := Path(directory) / name).is_symlink() and not path.exists()
        ]

    destination.mkdir()
    executable = destination / binary.name
    shutil.copy2(binary, executable)
    if (build / "instruction-build.json").is_file():
        shutil.copy2(build / "instruction-build.json", destination)
    for name in RUNTIME_DIRECTORIES:
        if (build / name).is_dir():
            shutil.copytree(build / name, destination / name, ignore=missing_links)
    return executable
