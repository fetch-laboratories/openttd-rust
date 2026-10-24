#!/usr/bin/env python3
"""Compare actual original/Rust X25519 endpoints, wire bytes and state transitions.

Limits: prescribed entropy verifies call order and bytes, not operating-system
RNG quality; nonempty-short enable nonce is undefined in pinned Packet and
excluded (#41); prefilled Poly1305/BLAKE2b final contexts supplement the
observed wipes, but compiler spills are not fully observable. Protocol
primitives are the real bundled Monocypher. The sanitizer instruments the C++
fixture, Packet, vendor code and facade and checks Rust allocation leaks, not
Rust memory accesses.
"""

import json
import subprocess

import probe

SECRET_SERVER = bytes(range(32)).hex()
SECRET_CLIENT = bytes(range(32, 64)).hex()


def hex_bytes(value):
    return value.hex() or "-"


def wire(payload):
    return ((len(payload) + 3).to_bytes(2, "little") + b"\x00" + payload).hex()


class Endpoint:
    def __init__(self, binary, role, transcript, env):
        self.role, self.transcript = role, transcript
        self.process = subprocess.Popen(
            [str(binary)],
            env=env,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )

    def call(self, command):
        self.process.stdin.write(command + "\n")
        self.process.stdin.flush()
        result = self.process.stdout.readline().rstrip("\n")
        if not result or result.startswith("error "):
            raise RuntimeError(f"{self.role} command {command}: {result}")
        self.transcript.append([self.role, command, result])
        return result

    def close(self):
        self.process.stdin.close()
        error = self.process.stderr.read()
        code = self.process.wait()
        if code or error:
            raise RuntimeError(f"{self.role} endpoint exited {code}: {error}")


def scenario(server_binary, client_binary, env, extra_server, extra_client, mode):
    transcript = []
    server = Endpoint(server_binary, "server", transcript, env)
    client = Endpoint(client_binary, "client", transcript, env)
    try:
        server.call(f"new {SECRET_SERVER} 73")
        client.call(f"new {SECRET_CLIENT} 191")
        request = server.call("request").split()[1]
        assert client.call(f"recv-request {request}") == "1 0"
        response = client.call(f"response {hex_bytes(extra_client)}").split()[1]
        result = server.call(f"recv-response {response} {hex_bytes(extra_server)}")
        success = extra_server == extra_client
        assert result == ("0 0" if success else "1 0")
        server.call("state")
        client.call("state")
        if mode == "failures":
            # Covered after a valid exchange so prior peer/key preservation is defined.
            for payload in (b"", bytes(55), bytes(57)):
                server.call(f"recv-response {wire(payload)} {hex_bytes(extra_server)}")
                server.call("state")
                client.call(f"recv-request {wire(payload)}")
                client.call("state")
            for point in (3, 35, 51, len(bytes.fromhex(response)) - 1):
                damaged = bytearray.fromhex(response)
                damaged[point] ^= 1
                server.call(f"recv-response {damaged.hex()} {hex_bytes(extra_server)}")
                server.call("state")
                server.call(f"recv-response {response} {hex_bytes(extra_server)}")
            for low_order in (bytes(32), b"\x01" + bytes(31)):
                bad_request = bytearray.fromhex(request)
                bad_request[3:35] = low_order
                client.call(f"recv-request {bad_request.hex()}")
                assert (
                    client.call(f"response {hex_bytes(extra_client)}").split()[0] == "0"
                )
                client.call("state")
                bad_response = bytearray.fromhex(response)
                bad_response[3:35] = low_order
                server.call(
                    f"recv-response {bad_response.hex()} {hex_bytes(extra_server)}"
                )
                server.call("state")
            client.call(f"recv-request {request}")
            response = client.call(f"response {hex_bytes(extra_client)}").split()[1]
            assert (
                server.call(f"recv-response {response} {hex_bytes(extra_server)}")
                == "0 0"
            )
            # Nonzero short nonce input is undefined in pinned Packet::Recv_bytes (#41).
            before = client.call("nonce")
            assert client.call(f"recv-nonce {wire(b'')}") == "0 0"
            assert client.call("nonce") == before
        nonce = server.call("nonce").split()[1]
        assert client.call(f"recv-nonce {nonce}") == "1 0"
        if mode == "failures":
            nonce_payload = bytes.fromhex(nonce)[3:]
            assert client.call(f"recv-nonce {wire(nonce_payload + b'xyz')}") == "1 3"
        if success:
            for side in (0, 1):
                sender, receiver = (client, server) if side == 0 else (server, client)
                sender.call(f"init-stream {side}")
                receiver.call(f"init-stream {side}")
                for length in (0, 1, 8, 15, 16, 17, 63, 64, 65, 127, 256):
                    plaintext = bytes((i * 17 + length) & 255 for i in range(length))
                    encrypted = sender.call(f"encrypt {hex_bytes(plaintext)}").split()
                    mac, ciphertext = encrypted[1:3]
                    if mode == "failures":
                        tampered = bytearray.fromhex(mac)
                        tampered[0] ^= 128
                        failed = receiver.call(
                            f"decrypt {ciphertext} {tampered.hex()}"
                        ).split()
                        assert failed[0] == "0" and failed[2] == ciphertext
                    if mode == "failures" and length:
                        damaged = bytearray.fromhex(ciphertext)
                        damaged[-1] ^= 1
                        assert (
                            receiver.call(f"decrypt {damaged.hex()} {mac}").split()[0]
                            == "0"
                        )
                    decrypted = receiver.call(f"decrypt {ciphertext} {mac}").split()
                    assert decrypted[0] == "1" and decrypted[2] == hex_bytes(plaintext)
                    assert decrypted[3:] == encrypted[3:] and decrypted[4] == "0"
                    if mode == "failures":
                        assert (
                            receiver.call(f"decrypt {ciphertext} {mac}").split()[0]
                            == "0"
                        )
                sender.call("copy-stream")
                first = sender.call("encrypt 010203")
                sender.call("swap-stream")
                assert sender.call("encrypt 010203") == first
                sender.call("assign-stream")
                sender.call("swap-stream")
        for endpoint in (server, client):
            endpoint.call("copy")
            assert endpoint.call("move-copy") == "1"
            endpoint.call("assign")
        # Separate public derived-key facade: exact halves, rejected peer preservation, copies.
        server_pub = bytes.fromhex(request)[3:35].hex()
        client.call(f"derive {SECRET_CLIENT} {server_pub} 0 {hex_bytes(extra_client)}")
        client.call("copy-keys")
        client.call(f"derive {SECRET_CLIENT} {'00' * 32} 0 {hex_bytes(extra_client)}")
        client.call("copy-keys")
        alias = client.call(f"alias-keys {SECRET_CLIENT} {server_pub}").split()
        assert alias[0:2] == ["1", "1"] and alias[3] == "1"
        assert alias[2] != alias[4]
        for operation in ("packet", "log", "output"):
            assert server.call(f"throw-operations {operation}") == "1 1 1"
            assert client.call(f"throw-operations {operation}") == "1 1 1"
        for after in (0, 1):
            assert server.call(f"throw-construction {SECRET_SERVER} {after}") == "1 1"
            assert client.call(f"throw-construction {SECRET_CLIENT} {after}") == "1 1"
        assert server.call("drop").split()[0] == "1"
        assert client.call("drop").split()[0] == "1"
    finally:
        server.close()
        client.close()
    return transcript


def main():
    out = probe.output("auth-comparison")
    env = probe.ENV
    fixture = probe.ROOT / "tools/migration/auth-comparison.cpp"
    commands, binaries = {}, {}
    sources = (
        "src/network/core/packet.cpp",
        "src/string.cpp",
        "src/core/string_builder.cpp",
        "src/core/string_inplace.cpp",
        "src/core/utf8.cpp",
    )
    for label, source in probe.LABELS:
        binary = out / label
        # Include unchanged actual vendor source and expose only its private
        # coarse helper. No copied field/ladder oracle or altered vendor body.
        shim = out / f"{label}-ladder.cpp"
        shim.write_text(
            f'#include "{source / "src/3rdparty/monocypher/monocypher.cpp"}"\n'
            'extern "C" void FixtureLadder(uint8_t *out, const uint8_t *scalar, const uint8_t *point, int32_t bits) { scalarmult(out, scalar, point, bits); }\n'
        )
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
            fixture,
            *(source / name for name in sources[:1]),
            shim,
            *(source / name for name in sources[1:]),
            "-Wl,--gc-sections",
            *(probe.rust() if label == "candidate" else []),
        ]
        probe.compile([*commands[label], "-o", binary], f"{binary}-compile.log")
        binaries[label] = binary
    scenarios = [
        (b"", b"", "normal"),
        (b"password", b"password", "normal"),
        (b"a\x00b\xff", b"a\x00b\xff", "failures"),
        (b"password", b"wrong", "normal"),
        (b"password", b"", "normal"),
    ]
    comparisons = 0
    for index, (server_extra, client_extra, mode) in enumerate(scenarios):
        expected = scenario(
            binaries["reference"],
            binaries["reference"],
            env,
            server_extra,
            client_extra,
            mode,
        )
        for server_label, client_label in (
            ("reference", "candidate"),
            ("candidate", "reference"),
            ("candidate", "candidate"),
            ("candidate-cpp", "candidate-cpp"),
        ):
            name = f"scenario-{index}-{server_label}-{client_label}"
            actual = scenario(
                binaries[server_label],
                binaries[client_label],
                env,
                server_extra,
                client_extra,
                mode,
            )
            if actual != expected:
                (out / f"{name}.json").write_text(
                    json.dumps({"expected": expected, "actual": actual}, indent=2)
                    + "\n"
                )
                raise RuntimeError(
                    f"Mixed authentication discrepancy: {name}; see retained transcript"
                )
            comparisons += len(actual)
    # Only uncovered primitive variants/alias/buffer/padding/state cases. Expected
    # bytes always come from the actual pinned vendor binary, never from Rust.
    primitives = {}
    for label, _ in probe.LABELS:
        primitives[label] = probe.execute(
            [binaries[label], "--primitives"], out / f"{label}-primitives"
        )
        probe.same(
            primitives["reference"], primitives[label], f"{label}-primitives", out
        )
    # Instrument the complete C++ fixture, Packet, vendor and facade boundary;
    # stable Rust allocations are leak-observable, Rust accesses are not ASan-instrumented.
    sanitizer = out / "candidate-sanitized"
    probe.compile(
        [
            *commands["candidate"],
            "-o",
            sanitizer,
            "-fsanitize=address,undefined",
            "-fno-omit-frame-pointer",
            "-g",
            "-no-pie",
        ],
        f"{sanitizer}-compile.log",
    )
    sanitized_env = env | {
        "ASAN_OPTIONS": "detect_leaks=1:abort_on_error=1",
        "UBSAN_OPTIONS": "halt_on_error=1:print_stacktrace=1",
    }
    expected = scenario(
        binaries["reference"],
        binaries["reference"],
        env,
        b"a\x00b\xff",
        b"a\x00b\xff",
        "failures",
    )
    for server_binary, client_binary in (
        (binaries["reference"], sanitizer),
        (sanitizer, binaries["reference"]),
    ):
        actual = scenario(
            server_binary,
            client_binary,
            sanitized_env,
            b"a\x00b\xff",
            b"a\x00b\xff",
            "failures",
        )
        if actual != expected:
            raise RuntimeError("Sanitizer output changed")
    sanitized = probe.execute(
        [sanitizer, "--primitives"], f"{sanitizer}-primitives", env=sanitized_env
    )
    probe.same(
        primitives["reference"], sanitized, "candidate-sanitized-primitives", out
    )
    probe.passed(
        "Authentication comparison",
        mixed_endpoint_records=comparisons,
        bounded_primitive_records=len(primitives["reference"].splitlines()),
    )


if __name__ == "__main__":
    main()
