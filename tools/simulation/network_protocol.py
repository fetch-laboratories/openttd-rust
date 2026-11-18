"""Default two-client loopback observations through production game protocols."""

import errno
import json
import os
import re
import socket
import struct
import subprocess
import threading
import time
from collections.abc import Callable, Sequence
from dataclasses import dataclass, field
from pathlib import Path
from typing import TypedDict, cast

from . import core, multiplayer
from .console_adapter import console_adapter, prepared_console

NAME = "network-protocol-loopback"
# A rejected or dropped client leaves both processes running; without this the
# join waits last the per-game timeout. Closes and errors are expected only after
# the invalid admission, which sends one error.
JOIN_FAILURE = re.compile(
    r"SendError\(\)|Receive_SERVER_ERROR\(\)|Receive_CLIENT_ERROR\(\)"
    r"|closed connection|connection lost|has been disconnected"
)
SEND_ERROR = re.compile(
    r"SendError\(\)|Receive_SERVER_ERROR\(\)|Receive_CLIENT_ERROR\(\)"
)
INVALID_ADMISSION = re.compile(
    r"(?:\[\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\] dbg: \[net:9\] )?"
    r"client\[\d+\] SendError\(\): error=7"
)
# Each wait of a session, cleanup included, is bounded by this or the per-game
# timeout when smaller. A passing step takes about two seconds; the per-game
# timeout equals the shard job's limit, which cancelled the job before (#693).
STEP_LIMIT = 120
# The original's TCPConnecter::CheckActivity stores Status::Resolving after its
# resolver thread may already have stored Connecting; the connecter then never
# connects and reports nothing (#693). Its log shows a resolve and no attempt.
RESOLVED = "resolved in:"
SIGNATURE = "TCPConnecter stuck in Resolving (#693)"
# The race is the pair of stores in the first and the start-up lock in the second.
RACE_FILES = ("src/network/core/tcp_connect.cpp", "src/thread.h")
CALLBACK = "_Z30CcPlaySound_CONSTRUCTION_OTHER8CommandsRK11CommandCostN10StrongType7TypedefIj12TileIndexTagJNS3_7CompareENS3_7IntegerENS3_10CompatibleIiEENS8_IlEEEEE"


class Scenario(core.Scenario, total=False):
    network_protocol: bool


class Summary(TypedDict):
    command_payloads: list[str]
    callbacks: int
    replayed: int
    rejected: str
    rng: list[int]


class Result(core.Result, total=False):
    network_protocol: dict[str, Summary]
    network_protocol_retries: int


class StepExpired(RuntimeError):
    """A bounded wait of a session ended without what it waited for.
    `connecting` names the client of a connection wait that timed out while
    that client still ran: the only wait the #693 stall can explain."""

    def __init__(self, message: str, connecting: "Peer | None" = None) -> None:
        super().__init__(message)
        self.connecting = connecting


class Stalled(RuntimeError):
    """A session failed with a client stuck on the #693 signature."""

    def __init__(self, message: str, peer: str) -> None:
        super().__init__(message)
        self.peer = peer


# One address line of a resolve block: `... - 127.0.0.2:3979 (IPv4)`.
ADDRESS = re.compile(r" - \S+ \(IPv[46]\)\s*$")


def stuck_resolving(lines: Sequence[str]) -> bool:
    """The #693 signature: the log ends on a resolve block, the `resolved in:`
    line and at least one address line, with nothing after it."""
    resolved = [n for n, line in enumerate(lines) if RESOLVED in line]
    if not resolved:
        return False
    after = [line for line in lines[resolved[-1] + 1 :] if line.strip()]
    return bool(after) and all(ADDRESS.search(line) for line in after)


def step_limit(timeout: int) -> int:
    return min(STEP_LIMIT, timeout)


def scenarios(soak: bool) -> list[Scenario]:
    return [
        {
            "name": NAME,
            "kind": "save",
            "save": str(core.ROOT / "migration/saves/padhattan-ridge-2000.sav"),
            "ticks": 1,
            "short_checkpoint": True,
            "network_protocol": True,
        }
    ]


def adapter(build: Path, folder: Path, env: dict[str, str]) -> Path:
    import migration

    folder.mkdir(parents=True, exist_ok=True)
    source = core.ROOT / "tools/simulation/network_protocol-console.cpp"
    candidate = (build / "openttd-rust").is_file()
    replaced: tuple[str, ...] = ()
    if candidate:
        revision = multiplayer.fixture_revision(
            (build / "generated/rev.cpp").read_text(),
            (migration.REFERENCE_BUILD / "generated/rev.cpp").read_text(),
        )
        combined = folder / "network-protocol.cpp"
        combined.write_text(f'#include "{source}"\n' + revision)
        source = combined
        replaced = ("generated/rev.cpp.o",)
    return console_adapter(
        build,
        folder,
        env,
        source,
        "network_protocol",
        wraps=(
            CALLBACK,
            "_Z25GetRustNetworkTcpServicesv"
            if candidate
            else "_ZN23NetworkTCPSocketHandler13ReceivePacketEv",
        ),
        replace_objects=replaced,
    )


@dataclass
class Peer:
    process: subprocess.Popen[str]
    folder: Path
    timeout: int
    lines: list[str] = field(default_factory=list)
    condition: threading.Condition = field(default_factory=threading.Condition)
    step: int = field(init=False)
    ended: bool = False
    reader: threading.Thread = field(init=False)

    def __post_init__(self) -> None:
        self.step = step_limit(self.timeout)
        self.reader = threading.Thread(target=self.collect, daemon=True)
        self.reader.start()

    def collect(self) -> None:
        assert self.process.stdout is not None
        try:
            with (
                self.process.stdout as output,
                (self.folder / "stderr.log").open("w") as log,
            ):
                for line in output:
                    log.write(line)
                    log.flush()
                    with self.condition:
                        self.lines.append(line.rstrip())
                        self.condition.notify_all()
        finally:
            with self.condition:
                self.ended = True
                self.condition.notify_all()

    def wait(
        self,
        marker: str,
        count: int = 1,
        failure: re.Pattern[str] | None = None,
        allowed: re.Pattern[str] | None = None,
        connecting: bool = False,
    ) -> None:
        """Wait for `count` markers, allowing one full match of `allowed`.
        `connecting` marks a wait for this client's own connection."""

        def failed() -> list[str]:
            if failure is None:
                return []
            failures = []
            used = False
            for line in self.lines:
                if not failure.search(line):
                    continue
                if not used and allowed is not None and allowed.fullmatch(line):
                    used = True
                else:
                    failures.append(line)
            return failures

        with self.condition:
            reached = self.condition.wait_for(
                lambda: (
                    sum(marker in line for line in self.lines) >= count
                    or self.ended
                    or bool(failed())
                ),
                timeout=self.step,
            )
            if failed():
                raise RuntimeError(
                    f"{self.folder.name}: {failed()[0]} before {marker} of {count}"
                )
            if not reached or sum(marker in line for line in self.lines) < count:
                why = "the process ended" if self.ended else f"{self.step} s passed"
                raise StepExpired(
                    f"{self.folder.name}: missing {marker} of {count} ({why}); "
                    f"last line: {self.last()}",
                    self if connecting and not self.ended else None,
                )

    def last(self) -> str:
        """The peer's last log line, for a diagnostic."""
        with self.condition:
            return self.lines[-1] if self.lines else "(none)"

    def trigger(self, action: str) -> None:
        pending = self.folder / "trigger.tmp"
        pending.write_text(action + "\n")
        pending.replace(self.folder / "trigger.txt")

    def exited(self, what: str) -> int:
        """The process's status, waiting at most one step for it to exit."""
        try:
            return self.process.wait(timeout=self.step)
        except subprocess.TimeoutExpired:
            raise StepExpired(
                f"{self.folder.name}: no exit within {self.step} s of {what}; "
                f"last line: {self.last()}"
            ) from None

    def joined(self) -> bool:
        """Whether the log reader finished within one step."""
        self.reader.join(self.step)
        return not self.reader.is_alive()

    def finish(self) -> None:
        self.trigger("exit")
        if self.exited("its exit trigger") != 0:
            raise RuntimeError(f"{self.folder.name}: failed process exit")
        if not self.joined():
            raise StepExpired(
                f"{self.folder.name}: its log stayed open {self.step} s after exit; "
                f"last line: {self.last()}"
            )

    def release(self) -> None:
        """Release the held map. The FIFO opens without blocking, retried until
        the step limit, so a joiner that died cannot hold the session."""
        deadline = time.monotonic() + self.step
        while True:
            try:
                handle = os.open(
                    self.folder / "map-release", os.O_WRONLY | os.O_NONBLOCK
                )
                break
            except OSError as error:
                # ENXIO: the joiner has not opened its end, or never will.
                if error.errno != errno.ENXIO or time.monotonic() >= deadline:
                    raise StepExpired(
                        f"{self.folder.name}: map release not read within "
                        f"{self.step} s ({error.strerror}); last line: {self.last()}"
                    ) from None
                time.sleep(0.05)
        with os.fdopen(handle, "w") as release:
            release.write("release\n")


def start(
    binary: Path,
    folder: Path,
    source: Path,
    host: str,
    port: int,
    mode: str,
    timeout: int,
    env: dict[str, str],
) -> Peer:
    folder.mkdir(parents=True)
    core.write_config(
        # write_config reads only the kind and settings of a save game.
        cast(
            core.Scenario,
            {
                "kind": "save",
                "settings": {
                    "locale": {
                        key.removeprefix("locale."): str(value)
                        for key, value in core.rows(source, "PATS")[0].items()
                        if key.startswith("locale.")
                    },
                    "gui": {"threaded_saves": "false"},
                    "network": {
                        "server_game_type": "local",
                        "server_port": str(port),
                        "client_name": "Protocol574" + mode,
                        "participate_survey": "no",
                        "max_commands_in_queue": "128",
                        "client_secret_key": multiplayer.AUTH_SECRET,
                    },
                },
            },
        ),
        binary.parent,
        folder,
    )
    with (folder / "openttd.cfg").open("a") as config:
        config.write(f"\n[server_bind_addresses]\n{host} = \n")
    scripts = folder / "scripts"
    scripts.mkdir()
    (scripts / "game_start.scr").write_text(
        "network574 server\n" if mode == "server" else ""
    )
    (scripts / "on_client.scr").write_text(f"network574 {mode}\n")
    if mode == "hold":
        (folder / "hold-map").touch()
        os.mkfifo(folder / "map-release")
    arguments = (
        ["-D", "-g", str(source)]
        if mode == "server"
        else ["-n", f"{host}:{port}#1", "-vsdl", "-b32bpp-anim", "-r", "640x480"]
    )
    command = [
        str(binary),
        "-X",
        "-x",
        "-c",
        str(folder / "openttd.cfg"),
        "-snull",
        "-mnull",
        *arguments,
        "-d",
        "net=9,desync=1",
    ]
    (folder / "command.json").write_text(json.dumps(command))
    process = subprocess.Popen(
        command,
        cwd=folder,
        env=env
        | {
            "HOME": str(folder),
            "XDG_DATA_HOME": str(folder / "data"),
            "XDG_CONFIG_HOME": str(folder / "config"),
            "XDG_CACHE_HOME": str(folder / "cache"),
            "SDL_VIDEODRIVER": "offscreen",
            "OPENTTD_PROTOCOL_KEY": multiplayer.AUTH_PUBLIC,
            "OPENTTD_PROTOCOL_STOP": str(
                core.rows(source, "DATE")[0]["tick_counter"] + 20
            ),
        },
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        bufsize=1,
    )
    return Peer(process, folder, timeout)


def packets(peer: Peer, kind: int) -> list[bytes]:
    return [
        bytes.fromhex(parts[6])
        for line in peer.lines
        if line.startswith("NET574 packet ") and (parts := line.split())[5] == str(kind)
    ]


@dataclass
class Command:
    payload: bytes
    callback: int
    frame: int
    own: bool


def command(data: bytes) -> Command:
    # Company, u16 command/error, u16 native argument-buffer length, callback, frame, own.
    size = struct.unpack_from("<H", data, 6)[0]
    end = 8 + size
    if len(data) != end + 6:
        raise RuntimeError("unexpected server command envelope")
    return Command(
        data[1:end],
        data[end],
        struct.unpack_from("<I", data, end + 1)[0],
        bool(data[-1]),
    )


def observe(peers: list[Peer], rejected: bytes, source: Path) -> Summary:
    server, issuer, joiner = peers
    sent = [
        data
        for data in packets(server, 29)
        if len(data) >= 4 and struct.unpack_from("<H", data, 2)[0] == 63
    ]
    first = [
        command(data)
        for data in packets(issuer, 30)
        if struct.unpack_from("<H", data, 2)[0] == 63
    ]
    second = [
        command(data)
        for data in packets(joiner, 30)
        if struct.unpack_from("<H", data, 2)[0] == 63
    ]
    if len(sent) != 24 or len(first) != 24 or len(second) != 24:
        raise RuntimeError("missing command transport/replay records")
    payloads = [item.payload for item in first]
    if payloads != [item.payload for item in second] or payloads != [
        data[1:-1] for data in sent
    ]:
        raise RuntimeError("command bytes/order differ across production endpoints")
    frames = [item.frame for item in first]
    if frames != sorted(set(frames)) or frames != [item.frame for item in second]:
        raise RuntimeError("quota of one, replay frame or command ordering differs")
    if any(item.callback == 0 or not item.own for item in first) or any(
        item.callback != 0 or item.own for item in second
    ):
        raise RuntimeError("callback/my-command recipient differs")
    callbacks = [line for line in issuer.lines if line.startswith("NET574 callback ")]
    if len(callbacks) != 24 or any(" 63 true 0" not in line for line in callbacks):
        raise RuntimeError("missing/failed issuer callback results")
    if any(
        "NET574 callback " in line for peer in (server, joiner) for line in peer.lines
    ):
        raise RuntimeError("callback leaked to another recipient")
    # The joiner's first frame follows MAP_OK, after the held map's queued commands replay.
    replay_frame = struct.unpack_from("<I", packets(joiner, 26)[0], 1)[0]
    replayed = sum(item.frame <= replay_frame for item in second)
    if replayed < 8:
        raise RuntimeError("missing commands accumulated while the joiner held its map")
    welcomes = [struct.unpack_from("<I", data, 1)[0] for data in packets(issuer, 16)]
    if len(welcomes) != 2 or welcomes[0] == welcomes[1]:
        raise RuntimeError("reconnect did not allocate a fresh client identity")
    if len(packets(server, 40)) < 3 or rejected != bytes((4, 0, 3, 7)):
        raise RuntimeError("quit or invalid-admission reason differs")
    saves = [peer.folder / "save/stop.sav" for peer in peers]
    stats: core.Stats = core.empty_stats()
    for save in saves[1:]:
        if core.compare_saves(saves[0], save, NAME, 10, stats):
            raise RuntimeError("controlled-tick client/server semantic worlds differ")
    initial, final = core.rows(source, "DATE")[0], core.rows(saves[0], "DATE")[0]
    if final["tick_counter"] != initial["tick_counter"] + 20 or any(
        final[f"random_state[{i}]"] == initial[f"random_state[{i}]"] for i in (0, 1)
    ):
        raise RuntimeError("missing controlled simulation/RNG progress")
    if any(
        re.search(r"Sync error|sync_err|Missed frame for sync-test", line, re.I)
        for peer in peers
        for line in peer.lines
    ):
        raise RuntimeError("network sync failure")
    return {
        "command_payloads": [data.hex() for data in payloads],
        "callbacks": len(callbacks),
        "replayed": replayed,
        "rejected": rejected.hex(),
        "rng": [final[f"random_state[{i}]"] for i in (0, 1)],
    }


def invalid_admission(server: Peer, host: str, port: int) -> bytes:
    """The server's answer to a client whose first packet is
    PACKET_CLIENT_COMMAND (29) before it has joined: the original's
    Receive_CLIENT_COMMAND admission guard sends an error. Connection, send and
    every receive share one step deadline."""
    deadline = time.monotonic() + server.step

    def remaining() -> float:
        left = deadline - time.monotonic()
        if left <= 0:
            raise StepExpired(
                f"invalid admission: no full answer within {server.step} s; "
                f"server's last line: {server.last()}"
            )
        return left

    try:
        with socket.create_connection((host, port), timeout=remaining()) as invalid:
            invalid.settimeout(remaining())
            invalid.sendall(bytes((3, 0, 29)))

            def read(count: int) -> bytes:
                data = b""
                while len(data) < count:
                    invalid.settimeout(remaining())
                    chunk = invalid.recv(count - len(data))
                    if not chunk:
                        break
                    data += chunk
                return data

            header = read(2)
            if len(header) != 2:
                raise RuntimeError("missing invalid-admission response header")
            size = struct.unpack("<H", header)[0]
            if size != 4:
                raise RuntimeError("invalid-admission response size differs")
            return header + read(size - 2)
    except TimeoutError:
        raise StepExpired(
            f"invalid admission: no full answer within {server.step} s; "
            f"server's last line: {server.last()}"
        ) from None


def session(
    binaries: dict[str, Path],
    source: Path,
    folder: Path,
    server_role: str,
    timeout: int,
    env: dict[str, str],
) -> tuple[Path, Summary]:
    peers: list[Peer] = []
    host = "127.0.0.2"  # Host HTTP/TLS probes use 127.0.0.1 (#629).
    with socket.socket() as probe:
        probe.bind((host, 0))
        port = probe.getsockname()[1]
    client_role = "candidate" if server_role == "reference" else "reference"
    try:
        with core.game_slot():
            server = start(
                binaries[server_role],
                folder / "server",
                source,
                host,
                port,
                "server",
                timeout,
                env,
            )
            peers.append(server)
            server.wait("NET574 ready server")
            server.wait("NET574 allowed")
            issuer = start(
                binaries[client_role],
                folder / "issuer",
                source,
                host,
                port,
                "issuer",
                timeout,
                env,
            )
            peers.append(issuer)
            issuer.wait("NET574 ready issuer", failure=JOIN_FAILURE, connecting=True)
            server.wait("status = ACTIVE", failure=JOIN_FAILURE)
            joiner = start(
                binaries[client_role],
                folder / "joiner",
                source,
                host,
                port,
                "hold",
                timeout,
                env,
            )
            peers.append(joiner)
            # Decrypted SERVER_MAP_DONE, before its handler runs.
            joiner.wait(" 23 17", failure=JOIN_FAILURE, connecting=True)
            issuer.trigger("issue")
            issuer.wait("NET574 callback ", 8)
            joiner.release()
            issuer.wait("NET574 callback ", 24)
            joiner.wait("NET574 ready hold", failure=JOIN_FAILURE)
            server.wait("status = ACTIVE", 2, JOIN_FAILURE)
            rejected = invalid_admission(server, host, port)
            issuer.trigger("reconnect 1")
            issuer.wait("NET574 ready issuer", 2, SEND_ERROR, connecting=True)
            server.wait("status = ACTIVE", 3, SEND_ERROR, allowed=INVALID_ADMISSION)
            assert server.process.stdin is not None
            server.process.stdin.write("unpause\n")
            server.process.stdin.flush()
            for peer in peers:
                peer.wait("NET574 saved ")
            issuer.finish()
            joiner.finish()
            server.wait("Receive_CLIENT_QUIT()", 3)
            server.process.stdin.write("exit\n")
            server.process.stdin.flush()
            if server.exited("its exit command") != 0:
                raise RuntimeError("server exit failed")
            if not server.joined():
                raise StepExpired(
                    f"server: its log stayed open {server.step} s after exit; "
                    f"last line: {server.last()}"
                )
            summary = observe(peers, rejected, source)
    except Exception as error:
        raise failure(error, peers) from error
    except BaseException:
        clean_up(peers)
        raise
    if problems := clean_up(peers):
        raise StepExpired("cleanup: " + "; ".join(problems))
    return server.folder / "save/stop.sav", summary


def failure(error: Exception, peers: list[Peer]) -> Exception:
    """The session's error, with any cleanup problem in its message. Only a
    connection wait that timed out while its own client still runs, with that
    client's log ending on a resolve block, is the #693 stall; and a session
    whose cleanup also failed is never retried."""
    client = error.connecting if isinstance(error, StepExpired) else None
    stuck = (
        client is not None
        and client.process.poll() is None
        and stuck_resolving(client.lines)
    )
    if problems := clean_up(peers):
        return RuntimeError(f"{error}; cleanup: " + "; ".join(problems))
    if stuck and client is not None:
        name = client.folder.name
        return Stalled(f"{error}; {name}: {SIGNATURE}", name)
    return error


def clean_up(peers: list[Peer]) -> list[str]:
    """Kill what is left; a cleanup wait that expires is reported, not repeated."""
    problems = []
    for peer in peers:
        if peer.process.poll() is None:
            peer.process.kill()
        try:
            peer.process.wait(timeout=peer.step)
        except subprocess.TimeoutExpired:
            problems.append(
                f"{peer.folder.name}: still running {peer.step} s after kill; "
                f"last line: {peer.last()}"
            )
        if peer.process.stdin is not None:
            try:
                peer.process.stdin.close()
            except OSError:
                pass
        if not peer.joined():
            problems.append(
                f"{peer.folder.name}: log still open {peer.step} s after kill; "
                f"last line: {peer.last()}"
            )
    return problems


def connecter_differs() -> str | None:
    """Why the candidate's connecter source is not the pinned original's, or
    None when both race files are identical. CI's checkout is shallow, so the
    pinned commit is fetched when this clone lacks it."""
    commit = json.loads((core.ROOT / "migration/baseline.json").read_text())["commit"]

    def git(*args: str) -> subprocess.CompletedProcess[bytes]:
        return subprocess.run(
            ["git", *args], cwd=core.ROOT, capture_output=True, timeout=STEP_LIMIT
        )

    try:
        if git("cat-file", "-e", f"{commit}^{{commit}}").returncode != 0:
            git("fetch", "--quiet", "--depth=1", "--no-tags", "origin", commit)
        shown = {path: git("show", f"{commit}:{path}") for path in RACE_FILES}
    except (OSError, subprocess.SubprocessError) as error:
        return f"the pinned original could not be read ({error})"
    for path in RACE_FILES:
        pinned = shown[path]
        if pinned.returncode != 0:
            return f"the pinned original's {path} could not be read"
        current = core.ROOT / path
        try:
            if current.read_bytes() != pinned.stdout:
                return f"{path} differs from the pinned original's"
        except OSError as error:
            return f"the candidate's {path} could not be read ({error})"
    return None


def sessions(
    play: Callable[[Path, str], tuple[Path, Summary]], folder: Path, result: Result
) -> tuple[dict[str, Path], dict[str, Summary]]:
    """Both server roles' sessions. A session that stalls on the #693 signature
    runs once more in a fresh folder, at most once per scenario run and only
    while both race files equal the pinned original's; the retry is a warning
    annotation and a count in the report."""
    saves: dict[str, Path] = {}
    summaries: dict[str, Summary] = {}
    result["network_protocol_retries"] = 0
    for role in ("reference", "candidate"):
        try:
            saves[role], summaries[role] = play(folder / role, role)
            continue
        except Stalled as stall:
            if result["network_protocol_retries"]:
                raise RuntimeError(f"{stall}; a second stall in this run") from stall
            if reason := connecter_differs():
                raise RuntimeError(f"{stall}; no retry: {reason}") from stall
            result["network_protocol_retries"] = 1
            print(
                f"::warning::{NAME}: {stall.peer} of the {role}-server session stalled "
                f"with {SIGNATURE}; running that session once more",
                flush=True,
            )
        saves[role], summaries[role] = play(folder / f"{role}-retry", role)
    return saves, summaries


def prepare(
    scenario: Scenario,
    binaries: dict[str, Path],
    builds: dict[str, Path],
    out: Path,
    timeout: int,
    env: dict[str, str],
    result: Result,
) -> Scenario:
    if not scenario.get("network_protocol"):
        return scenario
    folder = out / NAME / "input"
    source, _ = core.normalize(Path(scenario["save"]), folder / "source")
    executables = prepared_console(
        "network_protocol", adapter, binaries, builds, folder, env
    )
    saves, summaries = sessions(
        lambda where, role: session(executables, source, where, role, timeout, env),
        folder,
        result,
    )
    for key in ("command_payloads", "callbacks", "rejected", "rng"):
        if summaries["reference"][key] != summaries["candidate"][key]:
            raise RuntimeError(f"mixed server-role {key} differs")
    if core.compare_saves(
        saves["reference"],
        saves["candidate"],
        NAME,
        10,
        core.empty_stats(),
    ):
        raise RuntimeError("mixed server-role semantic worlds differ")
    result["network_protocol"] = summaries
    (folder / "observations.json").write_text(json.dumps(summaries, indent=2) + "\n")
    # The actual binaries record their own revisions again on the subsequent reload.
    for role, save in saves.items():
        saves[role], _ = core.normalize(save, folder / role / "reload-input")
    return {
        **scenario,
        "role_inputs": {role: {"save": str(save)} for role, save in saves.items()},
    }
