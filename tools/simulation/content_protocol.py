"""Default content-client protocol, dependency and download observations against local servers.

A scripted TCP content server and HTTP mirror on loopback replace the live content
service. Both record every request; the console adapter records callbacks, state and
errors. Inputs are synthesized here, so no third-party content is fetched or stored.
"""

import gzip
import hashlib
import http.server
import io
import json
import socket
import struct
import tarfile
import threading
from pathlib import Path
from typing import Any, cast

from . import core
from .console_adapter import console_adapter, prepared_console

NAME = "content-protocol"


class Scenario(core.Scenario, total=False):
    content_protocol: bool


Run = core.Run

AI, SCENARIO, SOUNDS = 3, 5, 7
SERVER_INFO, SERVER_CONTENT = 4, 6
NULL_SOUNDS = int.from_bytes(b"NULL", "little")
MD5 = bytes(range(16))


def packet(kind: int, payload: bytes = b"") -> bytes:
    return struct.pack("<HB", len(payload) + 3, kind) + payload


def info_payload(
    kind: int,
    cid: int,
    size: int,
    name: bytes,
    *,
    version: bytes = b"1.0",
    url: bytes = b"",
    desc: bytes = b"",
    uid: int = 0,
    md5: bytes = MD5,
    deps: tuple[int, ...] = (),
    tags: tuple[bytes, ...] = (),
) -> bytes:
    payload = struct.pack("<BII", kind, cid, size)
    payload += b"".join(text + b"\0" for text in (name, version, url, desc))
    payload += struct.pack("<I", uid) + md5 + bytes([len(deps)])
    payload += b"".join(struct.pack("<I", d) for d in deps) + bytes([len(tags)])
    return payload + b"".join(tag + b"\0" for tag in tags)


def info(*args: Any, **kwargs: Any) -> bytes:  # noqa: ANN401
    return packet(SERVER_INFO, info_payload(*args, **kwargs))


def archive(cid: int) -> bytes:
    data = f"content 667 fixture {cid}\n".encode()
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode="w", format=tarfile.USTAR_FORMAT) as tar:
        member = tarfile.TarInfo(f"s{cid}/readme.txt")
        member.size = len(data)
        tar.addfile(member, io.BytesIO(data))
    return gzip.compress(buffer.getvalue(), mtime=0)


FILES = {cid: archive(cid) for cid in (120, 121, 122, 123, 125, 126, 128)}

LISTS = {
    AI: [
        info(AI, 100, 1000, b"alpha", deps=(101, 102), uid=0x41, tags=(b"x", b"y")),
        info(AI, 101, 500, b"beta", deps=(103,), uid=0x42),
        # Over-long name, invalid UTF-8 and CR LF in the description.
        info(AI, 102, 200, b"g" * 40, desc=b"line\r\nnext\xff", uid=0x43),
        info(
            AI,
            104,
            300,
            b"delta",
            deps=(100,),
            uid=0x44,
            url=b"https://example.invalid/d",
        ),
        info(AI, 105, 0, b"missing", uid=0x45),
    ],
    SOUNDS: [
        info(SOUNDS, 150, 10, b"here", uid=NULL_SOUNDS, md5=bytes(16)),
        info(SOUNDS, 151, 10, b"newer", uid=NULL_SOUNDS),
        info(SOUNDS, 152, 0, b"gone", uid=0x99),
    ],
    SCENARIO: [
        info(SCENARIO, cid, len(data), f"s{cid}".encode(), uid=cid)
        for cid, data in FILES.items()
    ]
    + [info(SCENARIO, 124, 10, b"s124", uid=124)],
    # Invalid content type: the client closes.
    1: [info(0, 160, 10, b"invalid")],
    # Unknown and client-only packet types, then a valid info in a later batch.
    8: [packet(9, b"\1"), packet(0), info(8, 170, 10, b"music", uid=0x170)],
    # Truncated dependencies mark the socket quit; the next packet is invalid.
    9: [
        packet(
            SERVER_INFO, info_payload(9, 180, 10, b"short", deps=(100, 101, 102))[:-9]
        ),
        info(9, 181, 10, b"after"),
    ],
}
EXTRA = {
    103: info(AI, 103, 50, b"epsilon", deps=(100,), uid=0x46),
    # Same key as 100 with an empty name: the stub keeps its address and name.
    100: info(AI, 100, 1000, b"", deps=(101, 102), uid=0x41, version=b"2.0"),
    110: info(AI, 110, 10, b"", uid=0x4141, md5=bytes(16)),
}


class ContentServer:
    """One connection at a time, logging each client packet in order."""

    def __init__(self) -> None:
        self.listener = socket.create_server(("127.0.0.1", 0))
        self.port = self.listener.getsockname()[1]
        self.log: list[str] = []
        self.thread = threading.Thread(target=self.serve, daemon=True)
        self.thread.start()

    def serve(self) -> None:
        index = 0
        while True:
            try:
                conn, _ = self.listener.accept()
            except OSError:
                return
            with conn:
                self.handle(conn, index)
            index += 1

    def handle(self, conn: socket.socket, index: int) -> None:
        buffer = b""
        while True:
            try:
                data = conn.recv(65536)
            except OSError:
                data = b""
            if not data:
                self.log.append(f"tcp {index} closed")
                return
            buffer += data
            while (
                len(buffer) >= 2 and len(buffer) >= struct.unpack_from("<H", buffer)[0]
            ):
                size = struct.unpack_from("<H", buffer)[0]
                request, buffer = buffer[:size], buffer[size:]
                self.log.append(f"tcp {index} {request.hex()}")
                try:
                    conn.sendall(self.respond(request))
                except OSError:
                    pass

    def respond(self, request: bytes) -> bytes:
        kind, body = request[2], request[3:]
        if kind == 0:
            return b"".join(LISTS.get(body[0], []))
        if kind == 1:
            count = struct.unpack_from("<H", body)[0]
            ids = struct.unpack_from(f"<{count}I", body, 2)
            return b"".join(EXTRA.get(i, info(AI, i, 0, b"")) for i in ids)
        if kind in (2, 3):
            width = 21 if kind == 3 else 5
            keys = [body[1 + i * width : 6 + i * width] for i in range(body[0])]
            return b"".join(
                EXTRA[110]
                for key in keys
                if key == bytes([AI]) + struct.pack("<I", 0x4141)
            )
        if kind == 5:
            count = struct.unpack_from("<H", body)[0]
            return b"".join(
                self.content(i) for i in struct.unpack_from(f"<{count}I", body, 2)
            )
        return b""

    def content(self, cid: int) -> bytes:
        if cid == 124:
            return packet(SERVER_CONTENT, struct.pack("<BII", 0, cid, 10) + b"s124\0")
        data = FILES[cid]
        out = packet(
            SERVER_CONTENT,
            struct.pack("<BII", SCENARIO, cid, len(data)) + f"s{cid}\0".encode(),
        )
        chunks = [data[i : i + 1000] for i in range(0, len(data), 1000)]
        if cid == 128:
            # The rest never arrives; the client cancels with the file open.
            return out + packet(SERVER_CONTENT, chunks[0])
        return (
            out
            + b"".join(packet(SERVER_CONTENT, chunk) for chunk in chunks)
            + packet(SERVER_CONTENT)
        )

    def close(self) -> None:
        self.listener.close()


def mirror_handler(log: list[str]) -> type[http.server.BaseHTTPRequestHandler]:
    def line(cid: int, url: str, size: int | str | None = None) -> str:
        return f"{cid},{SCENARIO},{len(FILES[cid]) if size is None else size},{url}\n"

    class Mirror(http.server.BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.0"

        def log_message(self, format: str, *args: Any) -> None:  # noqa: A002, ANN401
            pass

        def reply(self, status: int, body: bytes, length: int | None = None) -> None:
            self.send_response(status)
            self.send_header(
                "Content-Length", str(len(body) if length is None else length)
            )
            self.end_headers()
            self.wfile.write(body)
            self.wfile.flush()

        def do_POST(self) -> None:
            body = self.rfile.read(int(self.headers["Content-Length"]))
            log.append(f"post {body.decode()!r}")
            ids = [int(i) for i in body.split()]
            port = cast(http.server.HTTPServer, self.server).server_port
            files = f"http://127.0.0.1:{port}/files"
            if ids in ([121], [128]):
                return self.reply(500, b"")
            lines = {
                120: line(120, f"{files}/s120.tar.gz"),
                122: line(122, "ottd://fallback"),
                123: line(123, f"{files}/stall/s123.tar.gz"),
                125: line(125, f"{files}/s125.tar.gz", "abc"),
                126: line(126, f"{files}/s126tar"),
            }
            self.reply(200, "".join(lines[i] for i in ids if i in lines).encode())

        def do_GET(self) -> None:
            log.append(f"get {self.path}")
            name = self.path.rsplit("/", 1)[-1]
            data = FILES[int(name[1:4])]
            if "/stall/" in self.path:
                # Half the body, then silence until the client gives up.
                self.reply(200, data[: len(data) // 2], len(data))
                threading.Event().wait(5)
                return
            self.reply(200, data)

    return Mirror


STEPS = [
    "listen",
    # Metadata, stubs and dependency resolution.
    f"update {AI}",
    "wait info 4",
    "state",
    "select 100",
    "idle 600",
    "wait info 1",
    "state",
    "tree 103",
    "tree 100",
    "select 104",
    "state",
    "unselect 100",
    "state",
    "toggle 102",
    "toggle 101",
    "state",
    "selectall",
    "state",
    "unselectall",
    "state",
    "extid 1 3:4141:00000000000000000000000000000000 3:4242:000102",
    "wait info 1",
    "extid 0 3:4141:00 4:5151:00",
    "wait info 1",
    "ids 100 999",
    "wait info 1",
    "idle 300",
    "state",
    f"update {SOUNDS}",
    "wait info 2",
    "upgrade",
    "state",
    # Downloads: HTTP, mirror failure, fallback URL, bad lines and cancellation.
    "unselectall",
    f"update {SCENARIO}",
    "wait info 8",
    *[
        step
        for cid in (120, 121, 122, 125, 126)
        for step in ("unselectall", f"select {cid}", "download 0", "wait complete 1")
    ],
    "unselectall",
    "select 124",
    "download 1",
    "wait disconnect 1",
    "state",
    "unselectall",
    "select 123",
    "mark",
    "download 0",
    "wait progress 1",
    "cancel",
    "idle 3000",
    "state",
    # Malformed packets.
    "update 1",
    "wait disconnect 1",
    "update 8",
    "wait info 1",
    "idle 300",
    "update 9",
    "wait info 1",
    "idle 600",
    "close",
    "wait disconnect 1",
    "state",
    # Fallback cancellation leaves the file open, as the original does.
    "unselectall",
    "select 128",
    "mark",
    "download 0",
    "wait progress 1",
    "cancel",
    "idle 300",
    "state",
    "clear",
    "state",
]


def scenarios(soak: bool) -> list[Scenario]:
    return [
        Scenario(
            name=NAME,
            kind="save",
            content_protocol=True,
            save=str(core.ROOT / "migration/saves/padhattan-ridge-2000.sav"),
            ticks=1,
            short_checkpoint=True,
        )
    ]


def adapter(build: Path, folder: Path, env: dict[str, str]) -> Path:
    return console_adapter(
        build,
        folder,
        env,
        core.ROOT / "tools/simulation/content_protocol-console.cpp",
        "content_protocol",
    )


def observe(run: Run, server: ContentServer, mirror: list[str]) -> list[str]:
    """Ordered callbacks, state, errors and requests; HTTP chunk sizes become totals."""
    lines = [
        line
        for line in run["log"]
        if line.startswith("CONTENT ") or "[tcp/content]" in line or "integer" in line
    ]
    totals: dict[str, int] = {}
    trace = []
    for line in lines:
        if line.startswith("CONTENT progress "):
            _, _, cid, amount = line.split()
            if int(amount) > 0:
                totals[cid] = totals.get(cid, 0) + int(amount)
            continue
        trace.append(line)
    trace += [f"bytes {cid} {total}" for cid, total in sorted(totals.items())]
    return trace + server.log + mirror


REQUIRED = (
    "CONTENT event connect true",
    "CONTENT event complete 120",
    "get /files/s120.tar.gz",
    "get /files/stall/s123.tar.gz",
    "CONTENT event complete 121",
    "CONTENT event complete 122",
    "CONTENT event complete 125",
    "CONTENT event complete 126",
    "CONTENT event progress-failed 123",
    "dbg: [net:0] [tcp/content] Received illegal packet",
    "dbg: [net:0] [tcp/content] Received illegal packet type 0",
    "dbg: [net:0] [tcp/content] Received invalid packet type 0",
)


def prepare(
    scenario: Scenario,
    binaries: dict[str, Path],
    builds: dict[str, Path],
    out: Path,
    timeout: int,
    env: dict[str, str],
    result: core.Result,
) -> Scenario:
    if "content_protocol" not in scenario:
        return scenario
    folder = out / scenario["name"] / "input"
    source, _ = core.normalize(Path(scenario["save"]), folder / "normalized")
    scenario = {**scenario, "save": str(source)}
    executables = prepared_console(
        "content_protocol", adapter, binaries, builds, folder, env
    )
    traces: dict[str, list[str]] = {}
    for role in core.ROLES:
        server = ContentServer()
        mirror: list[str] = []
        httpd = http.server.ThreadingHTTPServer(
            ("127.0.0.1", 0), mirror_handler(mirror)
        )
        httpd.daemon_threads = True
        address = f"http://127.0.0.1:{httpd.server_address[1]}"
        threading.Thread(target=httpd.serve_forever, daemon=True).start()
        try:
            capture = cast(
                core.Scenario,
                {**scenario, "console": [f"content_probe {step}" for step in STEPS]},
            )
            run = core.run_game(
                capture,
                executables[role],
                builds[role],
                folder / role / "capture",
                timeout,
                {
                    **env,
                    "OTTD_CONTENT_SERVER_CS": f"127.0.0.1:{server.port}",
                    "OTTD_CONTENT_MIRROR_URI": address + "/mirror",
                },
                False,
                # The original HTTP queue requires its native HttpThread worker.
                native_threads=True,
            )
        finally:
            httpd.shutdown()
            httpd.server_close()
            server.close()
        traces[role] = observe(run, server, mirror)
        (folder / role / "protocol-trace.json").write_text(
            json.dumps(traces[role], indent=2) + "\n"
        )
        missing = [marker for marker in REQUIRED if marker not in traces[role]]
        if (
            run["exit"]
            or missing
            or any("CONTENT timeout" in line for line in traces[role])
        ):
            raise RuntimeError(f"{role}: missing content protocol evidence {missing}")
    if traces["reference"] != traces["candidate"]:
        first = core.first_difference(traces["reference"], traces["candidate"])
        raise RuntimeError(f"content protocol traces differ at {first}")
    core.findings(result)["content_protocol"] = len(traces["reference"])
    core.findings(result)["content_protocol_sha256"] = hashlib.sha256(
        json.dumps(traces["reference"]).encode()
    ).hexdigest()
    return {**scenario, "executables": executables}
