#!/usr/bin/env python3
"""Deep-only mixed multiplayer, with a visibly fixture-matched candidate identity (#510)."""

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import threading
import time
from dataclasses import asdict, dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import TypedDict, cast

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import migration  # noqa: E402
from simulation import core  # noqa: E402
from simulation.console_adapter import console_adapter  # noqa: E402

NAME = "mixed-multiplayer-opus"
SOURCE = core.ROOT / "migration/saves/opus-55-167-002.sav"
STOP = (727960, 0, 84677)  # Calendar callback precedes economy/tick advancement.
REVISION_FIELDS = (
    "_openttd_revision",
    "_openttd_revision_tagged",
    "_openttd_newgrf_version",
)


def digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def fixture_revision(candidate: str, reference: str) -> str:
    """Replace only the three handshake identity constants, never gameplay bodies."""
    for name in REVISION_FIELDS:
        pattern = rf"^const .* {name} = .*;$"
        old, new = (
            re.findall(pattern, candidate, re.M),
            re.findall(pattern, reference, re.M),
        )
        if len(old) != 1 or len(new) != 1:
            raise RuntimeError(f"missing/ambiguous fixture identity: {name}")
        candidate = candidate.replace(old[0], new[0])
    return candidate


@dataclass
class Identity:
    original_executable: str
    fixture_executable: str
    game_objects: str
    revision: str
    original_revision: str


class CompileCommand(TypedDict):
    output: str


def game_objects(build: Path, candidate: bool) -> str:
    """Record the exact native game objects and Rust archive used by the relinker."""
    commands = cast(
        list[CompileCommand], json.loads((build / "compile_commands.json").read_text())
    )
    objects = sorted(
        Path(row["output"])
        for row in commands
        if "openttd_lib.dir" in row["output"]
        and not row["output"].endswith("generated/rev.cpp.o")
    )
    if not objects:
        raise RuntimeError("missing game objects")
    if candidate:
        objects.append(migration.rust_archive(build))
    return hashlib.sha256("".join(digest(p) for p in objects).encode()).hexdigest()


def freeze(out: Path) -> tuple[dict[str, Path], dict[str, Identity]]:
    binaries, identities = {}, {}
    migration.ensure_reference()
    for role, build in (
        ("reference", migration.REFERENCE_BUILD),
        ("candidate", migration.CANDIDATE_BUILD),
    ):
        with (
            migration.reference_lock(shared=True)
            if role == "reference"
            else migration.candidate_lock(build, shared=True)
        ):
            cache = migration.cmake_cache(build)
            if any(
                cache.get(key) != value
                for key, value in {
                    "CMAKE_BUILD_TYPE": "RelWithDebInfo",
                    "OPTION_USE_ASSERTS": "ON",
                    "OPTION_DEDICATED": "OFF",
                }.items()
            ) or any(
                "sanitize" in str(cache.get(key, ""))
                for key in (
                    "CMAKE_CXX_FLAGS",
                    "CMAKE_C_FLAGS",
                    "CMAKE_EXE_LINKER_FLAGS",
                )
            ):
                raise RuntimeError("mixed multiplayer requires ordinary native builds")
            folder = out / f"{role}-runtime"
            native = build / ("openttd" if role == "reference" else "openttd-rust")
            original, objects = digest(native), game_objects(build, role == "candidate")
            binary = core.copy_runtime(build, native, folder)
            if digest(binary) != original:
                raise RuntimeError("frozen executable differs from its build")
            revision = (build / "generated/rev.cpp").read_text()
            original_revision = revision
            if role == "candidate":
                revision = fixture_revision(revision, identities["reference"].revision)
                source = folder / "fixture-revision.cpp"
                source.write_text(
                    '#include "stdafx.h"\n#include "core/format.hpp"\n'
                    + revision
                    + '\nextern "C" void __real__Z22IConsoleStdLibRegisterv();\n'
                    'extern "C" void __wrap__Z22IConsoleStdLibRegisterv() {\n'
                    "\t__real__Z22IConsoleStdLibRegisterv();\n"
                    '\tfmt::print(stderr, "MIXED_FIXTURE revision {} tagged {} newgrf {}\\n", _openttd_revision, _openttd_revision_tagged, _openttd_newgrf_version);\n}\n'
                )
                binary = console_adapter(
                    build,
                    folder,
                    migration.environment(),
                    source,
                    "multiplayer",
                    replace_objects=("generated/rev.cpp.o",),
                )
            if (
                digest(native) != original
                or game_objects(build, role == "candidate") != objects
            ):
                raise RuntimeError(
                    "native executable/game objects changed during relink"
                )
            binaries[role] = binary
            identities[role] = Identity(
                original, digest(binary), objects, revision, original_revision
            )
    return binaries, identities


@dataclass
class Session:
    server_role: str
    directory: str
    commands: dict[str, list[str]] = field(default_factory=dict)
    exits: dict[str, int] = field(default_factory=dict)
    seconds: float = 0
    sync_packets: list[tuple[str, str]] = field(default_factory=list)
    rng: tuple[int, int] | None = None
    stop: tuple[int, int, int] | None = None


def session(
    binaries: dict[str, Path],
    source: Path,
    folder: Path,
    server_role: str,
    timeout: int,
) -> Session:
    started = time.monotonic()
    result = Session(server_role, str(folder))
    envs = {}
    # Sessions serialize under the deep lock; a bind failure remains a retained failure.
    port = 41051
    for side, role in (
        ("server", server_role),
        ("client", "candidate" if server_role == "reference" else "reference"),
    ):
        run = folder / side
        run.mkdir(parents=True)
        core.write_config(
            {
                "kind": "save",
                "settings": {
                    "gui": {"threaded_saves": "false"},
                    "network": {
                        "server_game_type": "local",
                        "server_port": str(port),
                        "client_name": "Mixed510",
                        "participate_survey": "no",
                    },
                },
            },
            binaries[role].parent,
            run,
        )
        with (run / "openttd.cfg").open("a") as config:
            config.write("\n[server_bind_addresses]\n127.0.0.1 = \n")
        scripts = run / "scripts"
        scripts.mkdir()
        start = "script console.log\nschedule on-next-calendar-month scripts/stop.scr\n"
        (scripts / "game_start.scr").write_text(start + "echo MIXED_READY\n")
        (scripts / "on_client.scr").write_text(start)
        (scripts / "stop.scr").write_text(
            "save stop\necho MIXED_STOP\n" + ("exit\n" if side == "client" else "")
        )
        args = (
            ["-D", "-g", str(source)]
            if side == "server"
            else [
                "-n",
                f"127.0.0.1:{port}#255",
                "-vsdl",
                "-b32bpp-anim",
                "-r",
                "640x480",
            ]
        )
        # -D resets net debug; ACTIVE must remain observable after the initial ACK.
        result.commands[side] = [
            str(binaries[role]),
            "-X",
            "-x",
            "-c",
            str(run / "openttd.cfg"),
            "-snull",
            "-mnull",
            *args,
            "-d",
            "net=9,desync=1,driver=1",
        ]
        envs[side] = migration.environment() | {
            "HOME": str(run),
            "XDG_DATA_HOME": str(run / "data"),
            "XDG_CONFIG_HOME": str(run / "config"),
            "XDG_CACHE_HOME": str(run / "cache"),
            "SDL_VIDEODRIVER": "offscreen",
        }
    ready = threading.Event()
    errors: list[str] = []
    with (
        core.game_slot(),
        subprocess.Popen(
            result.commands["server"],
            cwd=folder / "server",
            env=envs["server"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            bufsize=1,
        ) as server,
    ):
        assert server.stdin is not None and server.stdout is not None
        writer, output = server.stdin, server.stdout
        watchdog = threading.Timer(2 * timeout, server.kill)

        def collect() -> None:
            active = False
            try:
                with (folder / "server/stderr.log").open("w") as log:
                    for line in output:
                        log.write(line)
                        log.flush()
                        if line.strip() == "MIXED_READY":
                            ready.set()
                        if "status = ACTIVE" in line and not active:
                            active = True
                            # One line avoids dedicated std::getline/select read-ahead.
                            writer.write("unpause\n")
                            writer.flush()
            except OSError as error:
                errors.append(str(error))
            finally:
                ready.set()

        reader = threading.Thread(target=collect)
        watchdog.start()
        reader.start()
        try:
            if not ready.wait(timeout):
                raise RuntimeError("server readiness timeout")
            with (folder / "client/stderr.log").open("w") as log:
                client = core.benchmark_run(
                    result.commands["client"],
                    cwd=folder / "client",
                    env=envs["client"],
                    stdout=log,
                    stderr=subprocess.STDOUT,
                    timeout=timeout,
                )
            result.exits["client"] = client.returncode
            writer.write("exit\n")
            writer.flush()
            result.exits["server"] = server.wait(timeout=30)
        finally:
            if server.poll() is None:
                server.kill()
            server.wait()
            reader.join()
            watchdog.cancel()
            result.seconds = round(time.monotonic() - started, 3)
            (folder / "execution.json").write_text(
                json.dumps(asdict(result), indent=2) + "\n"
            )
    if errors:
        raise RuntimeError("; ".join(errors))
    return result


def check(result: Session, source: Path) -> None:
    folder = Path(result.directory)
    server, client = (
        (folder / side / "stderr.log").read_text() for side in ("server", "client")
    )
    sent = re.findall(r"SendSync\(\), frame_counter=(\d+), sync_seed_1=(\d+)", server)
    received = re.findall(
        r"Receive_SERVER_SYNC\(\): sync_frame=(\d+), sync_seed_1=(\d+)", client
    )
    if (
        result.exits != {"server": 0, "client": 0}
        or not sent
        or sent != received
        or len(sent) < 2
    ):
        raise RuntimeError("missing/failed exits or sync evidence")
    if (
        "MIXED_READY" not in server
        or "Receive_CLIENT_ACK()" not in server
        or "Receive_CLIENT_QUIT()" not in server
        or "Client::SendQuit()" not in client
        or "using thread for game-loop" not in client
        or any(log.count("status = ACTIVE") != 1 for log in (server, client))
    ):
        raise RuntimeError("missing readiness, ACTIVE, thread or normal quit evidence")
    fixture_log = client if result.server_role == "reference" else server
    if (
        len(
            re.findall(r"MIXED_FIXTURE revision \S+ tagged \d+ newgrf \d+", fixture_log)
        )
        != 1
    ):
        raise RuntimeError("missing fixture executable identity")
    if re.search(
        r"Sync error|sync_err|Missed frame for sync-test|SendError\(\)|Receive_SERVER_ERROR\(\)|connection lost|has been disconnected",
        server + client,
        re.I,
    ):
        raise RuntimeError("desync, skipped sync test or unexpected network disconnect")
    initial = core.rows(source, "DATE")[0]
    dates = []
    for side in ("server", "client"):
        run, save = folder / side, folder / side / "save/stop.sav"
        if (
            not save.is_file()
            or core.save_moment(save) != STOP
            or (run / "console.log").read_text().splitlines().count("MIXED_STOP") != 1
        ):
            raise RuntimeError("missing save/stop marker or incorrect stop moment")
        dates.append(core.rows(save, "DATE")[0])
    if (
        dates[0] != dates[1]
        or any(
            dates[0][f"random_state[{i}]"] == initial[f"random_state[{i}]"]
            for i in (0, 1)
        )
        or dates[0]["tick_counter"] <= initial["tick_counter"]
        or (dates[0]["economy_date"], dates[0]["economy_date_fract"])
        != (STOP[0] - 1, 73)
    ):
        raise RuntimeError("RNG/state mismatch or missing simulation progress")
    result.sync_packets, result.stop = sent, STOP
    result.rng = (dates[0]["random_state[0]"], dates[0]["random_state[1]"])


class Stats(TypedDict):
    chunks: int
    elements: int
    masked: dict[str, int]


@dataclass
class Result:
    scenario: str = NAME
    seconds: float = 0
    passed: bool = False
    problems: list[str] = field(default_factory=list)
    differences: list[dict[str, object]] = field(default_factory=list)
    sessions: list[Session] = field(default_factory=list)
    comparisons: dict[str, Stats] = field(default_factory=dict)


def compare(result: Result) -> None:
    if [s.server_role for s in result.sessions] != ["reference", "candidate"]:
        raise RuntimeError("both role orders are required")
    for side in ("server", "client"):
        saves = [Path(s.directory) / side / "save/stop.sav" for s in result.sessions]
        stats: Stats = {"chunks": 0, "elements": 0, "masked": {}}
        # Network clients omit AI configuration; equivalent roles compare AIPL too.
        result.differences.extend(core.compare_saves(*saves, NAME, 20, stats))
        result.comparisons[side] = stats
    result.passed = not result.problems and not result.differences


@dataclass
class Report:
    candidate_commit: str
    candidate_status: str
    identity: dict[str, Identity]
    results: list[Result]
    passed: bool


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("names", nargs="*")
    parser.add_argument("--jobs", type=int, default=1)
    parser.add_argument("--timeout", type=int, default=1200)
    args = parser.parse_args()
    if any(name not in NAME for name in args.names) or args.timeout < 1:
        parser.error("no mixed multiplayer input matches selectors")
    if migration.SANITIZE or migration.TSAN:
        parser.error("mixed multiplayer requires ordinary native runtimes")
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    out = migration.LOCAL / "simulation" / f"{stamp}-multiplayer-{os.getpid()}"
    out.mkdir(parents=True)
    commit, status = (
        migration.git("rev-parse", "HEAD"),
        migration.git("status", "--short"),
    )
    result, identities = Result(), {}
    started = time.monotonic()
    try:
        binaries, identities = freeze(out)
        source, _ = core.normalize(SOURCE, out / "input")
        for role in ("reference", "candidate"):
            run = session(binaries, source, out / f"{role}-server", role, args.timeout)
            result.sessions.append(run)
            check(run, source)
        compare(result)
    except (
        OSError,
        RuntimeError,
        ValueError,
        KeyError,
        core.Corrupt,
        subprocess.SubprocessError,
    ) as error:
        result.problems.append(str(error))
    result.seconds = round(time.monotonic() - started, 3)
    report = Report(commit, status, identities, [result], result.passed)
    path = out / "report.json"
    path.write_text(json.dumps(asdict(report), indent=2) + "\n")
    print(
        f"{'ok' if result.passed else 'FAIL'} {NAME}: {result.problems}; {len(result.differences)} differences",
        flush=True,
    )
    print(f"Report: {path}", flush=True)
    return 0 if result.passed else 1


if __name__ == "__main__":
    sys.exit(main())
