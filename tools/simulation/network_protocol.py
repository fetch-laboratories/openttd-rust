"""Default two-client loopback observations through production game protocols."""

import json
import os
import re
import socket
import struct
import subprocess
import threading
from dataclasses import dataclass, field
from pathlib import Path
from typing import TypedDict, cast

from . import core, multiplayer
from .console_adapter import console_adapter, prepared_console

NAME = "network-protocol-loopback"
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
    ended: bool = False
    reader: threading.Thread = field(init=False)

    def __post_init__(self) -> None:
        self.reader = threading.Thread(target=self.collect)
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

    def wait(self, marker: str, count: int = 1) -> None:
        with self.condition:
            reached = self.condition.wait_for(
                lambda: (
                    sum(marker in line for line in self.lines) >= count or self.ended
                ),
                timeout=self.timeout,
            )
            if not reached or sum(marker in line for line in self.lines) < count:
                raise RuntimeError(f"{self.folder.name}: missing {marker} of {count}")

    def trigger(self, action: str) -> None:
        pending = self.folder / "trigger.tmp"
        pending.write_text(action + "\n")
        pending.replace(self.folder / "trigger.txt")

    def finish(self) -> None:
        self.trigger("exit")
        if self.process.wait(timeout=self.timeout) != 0:
            raise RuntimeError(f"{self.folder.name}: failed process exit")
        self.reader.join()


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
    stats: core.Stats = {"chunks": 0, "elements": 0, "masked": {}}
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
            issuer.wait("NET574 ready issuer")
            server.wait("status = ACTIVE")
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
            joiner.wait(" 23 17")  # Decrypted SERVER_MAP_DONE, before its handler runs.
            issuer.trigger("issue")
            issuer.wait("NET574 callback ", 8)
            with (joiner.folder / "map-release").open("w") as release:
                release.write("release\n")
            issuer.wait("NET574 callback ", 24)
            joiner.wait("NET574 ready hold")
            server.wait("status = ACTIVE", 2)
            with socket.create_connection((host, port), timeout=timeout) as invalid:
                invalid.sendall(bytes((3, 0, 29)))
                with invalid.makefile("rb") as response:
                    header = response.read(2)
                    if len(header) != 2:
                        raise RuntimeError("missing invalid-admission response header")
                    size = struct.unpack("<H", header)[0]
                    if size != 4:
                        raise RuntimeError("invalid-admission response size differs")
                    rejected = header + response.read(size - 2)
            issuer.trigger("reconnect 1")
            issuer.wait("NET574 ready issuer", 2)
            server.wait("status = ACTIVE", 3)
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
            if server.process.wait(timeout=timeout) != 0:
                raise RuntimeError("server exit failed")
            server.reader.join()
            summary = observe(peers, rejected, source)
            return server.folder / "save/stop.sav", summary
    finally:
        for peer in peers:
            if peer.process.poll() is None:
                peer.process.kill()
            peer.process.wait()
            if peer.process.stdin is not None:
                peer.process.stdin.close()
            peer.reader.join()


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
    saves: dict[str, Path] = {}
    summaries: dict[str, Summary] = {}
    for role in ("reference", "candidate"):
        saves[role], summaries[role] = session(
            executables, source, folder / role, role, timeout, env
        )
    for key in ("command_payloads", "callbacks", "rejected", "rng"):
        if summaries["reference"][key] != summaries["candidate"][key]:
            raise RuntimeError(f"mixed server-role {key} differs")
    if core.compare_saves(
        saves["reference"],
        saves["candidate"],
        NAME,
        10,
        {"chunks": 0, "elements": 0, "masked": {}},
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
