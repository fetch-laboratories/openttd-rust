#!/usr/bin/env python3
"""Compare public auth endpoints, wire bytes, fallback and prescribed entropy.

The immutable Rust known answers cover primitive internals. This probe compares
public protocol behavior against the pinned original, including provider calls,
malformed packets, low-order peers, password callback reentry and stream retry.
Nonempty-short Recv_bytes remains outside its original precondition (#41).
Environmental exceptions and physical secret erasure are not compared.
"""

import json
import subprocess
from dataclasses import dataclass
from pathlib import Path

import probe

SECRET_CLIENT = bytes(range(32, 64)).hex()
Transcript = list[tuple[str, str, str]]


def hex_bytes(value: bytes) -> str:
    return value.hex() or "-"


def wire(payload: bytes) -> str:
    return ((len(payload) + 3).to_bytes(2, "little") + b"\x00" + payload).hex()


class Endpoint:
    def __init__(
        self, binary: Path, role: str, transcript: Transcript, env: dict[str, str]
    ) -> None:
        self.role, self.transcript = role, transcript
        self.process = subprocess.Popen(
            [str(binary)],
            env=env,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )

    def call(self, command: str) -> str:
        assert self.process.stdin is not None and self.process.stdout is not None
        self.process.stdin.write(command + "\n")
        self.process.stdin.flush()
        result = self.process.stdout.readline().rstrip("\n")
        if not result or result.startswith("error "):
            raise RuntimeError(f"{self.role} command {command}: {result}")
        self.transcript.append((self.role, command, result))
        return result

    def close(self) -> None:
        assert self.process.stdin is not None and self.process.stderr is not None
        self.process.stdin.close()
        error = self.process.stderr.read()
        code = self.process.wait()
        if code or error:
            raise RuntimeError(f"{self.role} endpoint exited {code}: {error}")


@dataclass(frozen=True)
class Case:
    name: str
    password: bytes | None = None
    keys: str | None = None
    client_password: bytes = b""
    methods: tuple[int, ...] = (0,)
    success: bool = True
    mask: int = 7
    failures: bool = False
    reentry: bool = False
    low_order: bool = False
    damage: int | None = None


CASES = (
    Case("no-password", failures=True),
    Case("empty-providers", password=b"", keys=""),
    Case("password", password=b"password", client_password=b"password", methods=(1,)),
    Case(
        "binary-password",
        password=b"a\x00b\xff",
        client_password=b"a\x00b\xff",
        methods=(1,),
    ),
    Case(
        "wrong-password",
        password=b"password",
        client_password=b"wrong",
        methods=(1,),
        success=False,
    ),
    Case("authorized", keys="valid", methods=(2,)),
    Case("unauthorized", keys="invalid", methods=(2,), success=False),
    Case("key-before-password", password=b"password", keys="valid", methods=(2,)),
    Case(
        "password-fallback",
        password=b"password",
        keys="invalid",
        client_password=b"password",
        methods=(2, 1),
    ),
    Case(
        "fallback-exhausted",
        password=b"password",
        keys="invalid",
        client_password=b"wrong",
        methods=(2, 1),
        success=False,
    ),
    Case(
        "no-supported-methods",
        password=b"password",
        keys="valid",
        mask=0,
        methods=(),
        success=False,
    ),
    Case("only-plain-supported", password=b"password", keys="valid", mask=1),
    Case(
        "only-password-supported",
        password=b"password",
        keys="valid",
        client_password=b"password",
        mask=2,
        methods=(1,),
    ),
    Case(
        "only-key-supported", password=b"password", keys="valid", mask=4, methods=(2,)
    ),
    Case(
        "password-reentry",
        password=b"password",
        client_password=b"password",
        methods=(1,),
        reentry=True,
    ),
    Case("low-order-server", low_order=True, success=False),
    Case("damaged-response-key", damage=3, success=False),
    Case("damaged-response-mac", damage=35, success=False),
)


def scenario(
    server_binary: Path,
    client_binary: Path,
    env: dict[str, str],
    case: Case,
) -> Transcript:
    transcript: Transcript = []
    server = Endpoint(server_binary, "server", transcript, env)
    client = Endpoint(client_binary, "client", transcript, env)
    try:
        public = client.call(f"public {SECRET_CLIENT}")
        keys = (
            public
            if case.keys == "valid"
            else "00" * 32
            if case.keys == "invalid"
            else case.keys
        )
        password = "none" if case.password is None else hex_bytes(case.password)
        assert (
            server.call(
                f"server {password} {(keys or '-') if keys is not None else 'none'} {case.mask} 73"
            )
            == "ok"
        )
        assert (
            client.call(
                f"client {SECRET_CLIENT} {hex_bytes(case.client_password)} 191 {int(case.reentry)}"
            )
            == "ok"
        )
        assert server.call("usable") == str(int(bool(case.methods)))
        server.call("state")
        client.call("state")
        for index, method in enumerate(case.methods):
            request = server.call("request").split()[1]
            assert bytes.fromhex(request)[3] == method
            if case.failures:
                for unknown in (3, 255):
                    assert (
                        client.call(f"recv-request {wire(bytes([unknown]))}") == "2 0"
                    )
                    client.call("state")
                for length in (0, 55, 57):
                    assert (
                        client.call(
                            f"recv-request {wire(bytes([method]) + bytes(length))}"
                        )
                        == f"2 {length}"
                    )
                    client.call("state")
            if case.low_order:
                changed = bytearray.fromhex(request)
                changed[4:36] = bytes(32)
                request = changed.hex()
            assert (
                client.call(f"recv-request {request}") == f"{0 if method == 1 else 1} 0"
            )
            client.call("state")
            if case.reentry:
                response = client.call("reentrant")
                assert response != "-"
            else:
                ok, response = client.call("response").split()
                if case.low_order:
                    assert ok == "0"
                    client.call("state")
                    return transcript
                assert ok == "1"
            if case.damage is not None:
                changed = bytearray.fromhex(response)
                changed[case.damage] ^= 1
                response = changed.hex()
            result = server.call(f"recv-response {response}")
            expected = 2 if index + 1 < len(case.methods) else 0 if case.success else 1
            assert result == f"{expected} 0"
            server.call("state")
        if case.success:
            assert server.call("peer") == public
            nonce = server.call("nonce").split()[1]
            assert client.call(f"recv-nonce {wire(b'')}") == "0 0"
            assert client.call(f"recv-nonce {nonce}") == "1 0"
            if case.failures:
                assert (
                    client.call(f"recv-nonce {wire(bytes.fromhex(nonce)[3:] + b'xyz')}")
                    == "1 3"
                )
            for side in (0, 1):
                sender, receiver = (client, server) if side == 0 else (server, client)
                assert sender.call(f"init-stream {side}") == "16"
                assert receiver.call(f"init-stream {side}") == "16"
                for length in (0, 1, 8, 15, 16, 17, 63, 64, 65, 127, 256):
                    plaintext = bytes((i * 17 + length) & 255 for i in range(length))
                    _, mac, ciphertext = sender.call(
                        f"encrypt {hex_bytes(plaintext)}"
                    ).split()
                    if case.failures:
                        tampered = bytearray.fromhex(mac)
                        tampered[0] ^= 128
                        failed = receiver.call(
                            f"decrypt {ciphertext} {tampered.hex()}"
                        ).split()
                        assert failed[0] == "0" and failed[2] == ciphertext
                    decrypted = receiver.call(f"decrypt {ciphertext} {mac}").split()
                    assert decrypted[0] == "1" and decrypted[2] == hex_bytes(plaintext)
                    if case.failures:
                        assert (
                            receiver.call(f"decrypt {ciphertext} {mac}").split()[0]
                            == "0"
                        )
        return transcript
    finally:
        server.close()
        client.close()


def main() -> None:
    out = Path(probe.output("auth-comparison"))
    binaries: dict[str, Path] = {}
    commands: dict[str, list[str | Path]] = {}
    sources = (
        "src/network/core/packet.cpp",
        "src/string.cpp",
        "src/core/string_builder.cpp",
        "src/core/string_consumer.cpp",
        "src/core/string_inplace.cpp",
        "src/core/utf8.cpp",
    )
    for label, source in probe.LABELS:
        binary = out / label
        commands[label] = [
            probe.compiler(),
            "-std=c++20",
            "-O2",
            "-DUNIX",
            "-DFMT_HEADER_ONLY",
            "-ffunction-sections",
            "-fdata-sections",
            "-I",
            source / "src",
            "-I",
            probe.BUILD / "generated-abi",
            probe.ROOT / "tools/migration/auth-comparison.cpp",
            *(source / name for name in sources),
            *(
                [source / "src/3rdparty/monocypher/monocypher.cpp"]
                if label == "reference"
                else []
            ),
            "-Wl,--gc-sections",
            *(probe.rust() if label == "candidate" else []),
        ]
        probe.compile([*commands[label], "-o", binary], f"{binary}-compile.log")
        binaries[label] = binary
    records = 0
    # Compile-only preflight (#923) runs no endpoint.
    if probe.RUN:
        for case in CASES:
            expected = scenario(
                binaries["reference"], binaries["reference"], probe.ENV, case
            )
            for server_label, client_label in (
                ("reference", "candidate"),
                ("candidate", "reference"),
                ("candidate", "candidate"),
            ):
                name = f"{case.name}-{server_label}-{client_label}"
                actual = scenario(
                    binaries[server_label], binaries[client_label], probe.ENV, case
                )
                if actual != expected:
                    (out / f"{name}.json").write_text(
                        json.dumps({"expected": expected, "actual": actual}, indent=2)
                        + "\n"
                    )
                    raise RuntimeError(
                        f"Authentication discrepancy: {name}; see retained transcript"
                    )
                records += len(actual)
    probe.passed(
        "Authentication comparison",
        public_cases=len(CASES),
        mixed_endpoint_records=records,
    )


if __name__ == "__main__":
    main()
