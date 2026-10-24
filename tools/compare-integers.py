#!/usr/bin/env python3
"""Compare bounded integer parser/builder gaps against pinned unchanged C++ sources."""

import json
import re
import struct
import subprocess

import probe

import migration

ROOT = probe.ROOT
REFERENCE = probe.REFERENCE
TOOLS = ROOT / ".local/build-tools-rust"
OUT = ROOT / ".local/integer-comparison"


def cases():
    inputs = [
        b"",
        b"-",
        b"+1",
        b" 1",
        b"0x",
        b"-0x",
        b"0X-1",
        b"0x-1",
        b"0x-",
        b"-0x-1",
        b"0x+1",
        b"01",
        b"0129",
        b"0y",
        b"0x0x1",
        b"42\x00junk",
        b"x\x00\xff\n",
        b"0x\x00\xffab",
        b"999999999999999999999999999999999999999999x\x00\xff\nmore",
    ]
    for width in (8, 16, 32, 64):
        bounds = {
            0,
            1,
            (1 << (width - 1)) - 1,
            1 << (width - 1),
            (1 << (width - 1)) + 1,
            (1 << width) - 1,
            1 << width,
            (1 << width) + 1,
        }
        samples = inputs.copy()
        for value in sorted(bounds):
            decimal = str(value).encode()
            hexadecimal = format(value, "x").encode()
            octal = format(value, "o").encode()
            samples.extend(
                (
                    decimal,
                    b"-" + decimal,
                    decimal + b"xyz!tail",
                    b"0x" + hexadecimal,
                    b"-0x" + hexadecimal,
                    hexadecimal,
                    b"-" + hexadecimal,
                    octal,
                    b"-" + octal,
                )
            )
        for signed in (0, 1):
            for base in (0, 8, 10, 16):
                for clamp in (0, 1):
                    for src in dict.fromkeys(samples):
                        yield width, signed, base, clamp, src


def builder_cases():
    for width in (8, 16, 32, 64):
        mask = (1 << width) - 1
        for signed in (0, 1):
            low = -(1 << (width - 1)) if signed else 0
            high = (1 << (width - 1)) - 1 if signed else mask
            values = {low, low + 1, 0, 1, high - 1, high}
            if signed:
                values.add(-1)
            for base in range(2, 37):
                samples = values.copy()
                # Exactly 32 versus 33 bytes, including the minus sign. Only
                # representable inputs are included; reference to_chars is the oracle.
                for exponent in (30, 31, 32):
                    boundary = base**exponent
                    for value in (boundary - 1, boundary, boundary + 1):
                        if low <= value <= high:
                            samples.add(value)
                        if signed and low <= -value <= high:
                            samples.add(-value)
                for value in sorted(samples):
                    yield width, signed, base, value & mask


def compare_builders(env, binaries):
    corpus = list(builder_cases())
    records = [
        f"{width} {signed} {base} {bits:x}\n".encode()
        for width, signed, base, bits in corpus
    ]
    (OUT / "builder-corpus.txt").write_bytes(b"".join(records))
    streams = {}
    for name in ("reference", "candidate", "candidate-cpp"):
        command = [str(binaries[name, False]), "--builder"]
        result = subprocess.run(
            command,
            input=b"".join(records),
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=True,
        )
        (OUT / f"{name}-builder.txt").write_bytes(result.stdout)
        streams[name] = result.stdout.splitlines()
    expected = streams["reference"]
    assert len(expected) == len(corpus) + 13, "Builder probe inventory changed"
    for name in ("candidate", "candidate-cpp"):
        failures = [
            (
                index,
                records[index].decode().strip()
                if index < len(records)
                else "fixed sink/alias case",
                want.decode(),
                got.decode(),
            )
            for index, (want, got) in enumerate(
                zip(expected, streams[name], strict=False)
            )
            if want != got
        ]
        (OUT / f"{name}-builder-failures.json").write_text(
            json.dumps(failures, indent=2) + "\n"
        )
        if len(streams[name]) != len(expected) or failures:
            raise RuntimeError(
                f"Builder comparison failed: {name}; {OUT / f'{name}-builder-failures.json'}"
            )
    return len(corpus)


def compare_consumers(env, binaries):
    # Bounded audited gaps, not a second general consumer fuzz corpus.
    maximum = (
        1 << (8 * struct.calcsize("P"))
    ) - 1  # Native size_t; cross compilation is unsupported.
    corpus = []
    for operation in (0, 1, 2):
        for offset, requested, src in (
            (0, 0, b""),
            (0, maximum, b""),
            (1, 0, b"A\x00\xff"),
            (1, 8, b"A\x00\xff"),
            (1, maximum - 1, b"A\x00\xff"),
            (3, 0, b"A\x00\xff"),
            (3, maximum, b"A\x00\xff"),
            (1, 1, b"A\x00\xff"),
            (1, 2, b"A\x00\xff"),
        ):
            corpus.append((operation, offset, requested, src))
    for operation in range(3, 12):
        for offset, src in (
            (0, b""),
            (1, b"ABC"),
            (3, b"ABC"),
            (1, b"X\x01\x02\x03\x04\x05\x06\x07\x80"),
        ):
            corpus.append((operation, offset, 0, src))
    # ASCII/NUL, multi-byte, malformed and EOF transitions exercise the same
    # public methods against the pinned body, including one-byte failed reads.
    for operation in range(12, 16):
        for offset, src in (
            (0, b""),
            (0, b"\x00A"),
            (1, b"\x00A"),
            (1, b"A\xc2\x80B"),
            (3, b"A\xc2\x80B"),
            (4, b"A\xc2\x80B"),
            (1, b"A\xed\xa0\x80B"),
            (1, b"A\xff\x80B"),
            (2, b"A\xff\x80B"),
            (1, b"A\xf1\x80\x80"),
        ):
            corpus.append((operation, offset, 0, src))
    records = [
        f"{operation} {offset} {requested} {src.hex() or '-'}\n".encode()
        for operation, offset, requested, src in corpus
    ]
    (OUT / "consumer-corpus.txt").write_bytes(b"".join(records))
    streams = []
    for name in ("reference", "candidate", "candidate-cpp"):
        result = subprocess.run(
            [str(binaries[name, False]), "--consumer"],
            input=b"".join(records),
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=True,
        )
        (OUT / f"{name}-consumer.txt").write_bytes(result.stdout)
        assert len(result.stdout.splitlines()) == len(records)
        streams.append(result.stdout)
    assert streams[0] == streams[1] == streams[2], (
        "Consumer byte/cursor/diagnostic mismatch; retained streams in evidence"
    )
    fatal_cases = 0
    for operation, offset, requested, src in (
        (1, 1, 8, b"A\x00\xff"),
        (2, 1, 8, b"A\x00\xff"),
        (3, 3, 0, b"ABC"),
        (4, 2, 0, b"ABC"),
        (5, 1, 0, b"ABC"),
        (6, 1, 0, b"ABC"),
        (13, 0, 0, b""),
        (14, 0, 0, b""),
        (13, 3, 0, b"ABC"),
        (14, 3, 0, b"ABC"),
    ):
        record = f"{operation} {offset} {requested} {src.hex() or '-'}\n".encode()
        results = [
            subprocess.run(
                [str(binaries[name, True]), "--consumer"],
                input=record,
                env=env,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            for name in ("reference", "candidate")
        ]
        assert results[0].returncode == results[1].returncode == 2
        assert (
            results[0].stdout == results[1].stdout
            and results[0].stderr == results[1].stderr
        )
        fatal_cases += 1
    return len(corpus), fatal_cases


def compare_generators(env):
    outputs = {}
    settings_dir = REFERENCE / "src/table/settings"
    settings_inputs = [
        str(settings_dir / name)
        for name in re.findall(
            r"\$\{CMAKE_CURRENT_SOURCE_DIR\}/(\w+\.ini)",
            (settings_dir / "CMakeLists.txt").read_text(),
        )
    ]
    lang = REFERENCE / "src/lang"
    tools = {
        "reference": (migration.REFERENCE_BUILD / "src").resolve(),
        "candidate": TOOLS / "src",
    }
    for name, tool_dir in tools.items():
        out = OUT / (name + "-generated")
        out.mkdir(exist_ok=True)
        for filename in ("strings.h", "settings.h", "english.lng", "french.lng"):
            (out / filename).unlink(missing_ok=True)
        strgen = str(tool_dir / "strgen/strgen")
        settingsgen = str(tool_dir / "settingsgen/settingsgen")
        invocations = [
            [strgen, "-s", str(lang), "-d", str(out)],
            [strgen, "-s", str(lang), "-d", str(out), str(lang / "english.txt")],
            [strgen, "-s", str(lang), "-d", str(out), str(lang / "french.txt")],
            [
                settingsgen,
                "-o",
                str(out / "settings.h"),
                "-b",
                str(REFERENCE / "src/table/settings.h.preamble"),
                "-a",
                str(REFERENCE / "src/table/settings.h.postamble"),
                *settings_inputs,
            ],
        ]
        for index, command in enumerate(invocations):
            probe.compile(command, OUT / f"{name}-generator-{index}.log", env)
        outputs[name] = {
            filename: (out / filename).read_bytes()
            for filename in ("strings.h", "settings.h", "english.lng", "french.lng")
        }
    assert outputs["reference"] == outputs["candidate"], "Generator output mismatch"
    malformed = 0
    original = (lang / "english.txt").read_bytes()
    for index, pragma in enumerate(
        (
            b"##id 0x-",
            b"##id 999999999999999999999junk",
            b"##winlangid -0x80000001",
            b"##id 0x\x00\xff",
        )
    ):
        directory = OUT / "malformed" / str(index)
        directory.mkdir(parents=True, exist_ok=True)
        (directory / "english.txt").write_bytes(
            original.replace(b"##id 0x0000", pragma, 1)
        )
        results = []
        for _name, tool_dir in tools.items():
            command = [
                str(tool_dir / "strgen/strgen"),
                "-s",
                str(directory),
                "-d",
                str(directory),
            ]
            result = subprocess.run(
                command, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE
            )
            results.append((result.returncode, result.stdout, result.stderr))
        assert results[0] == results[1] and results[0][0] == 2, results
        malformed += 1
    return malformed


def main():
    probe.output("integer-comparison")
    env = probe.ENV
    binaries = {}
    for name, source in probe.LABELS:
        for fatal in (False,) if name == "candidate-cpp" else (False, True):
            binary = OUT / (name + ("-fatal" if fatal else ""))
            command = [
                probe.compiler(TOOLS),
                "-std=c++20",
                "-O0",
                "-ffunction-sections",
                "-fdata-sections",
                "-DFMT_HEADER_ONLY",
                "-I",
                source / "src",
                ROOT / "tools/integer_probe.cpp",
                source / "src/core/string_consumer.cpp",
                source / "src/core/string_builder.cpp",
                source / "src/core/utf8.cpp",
                "-Wl,--gc-sections",
                "-o",
                binary,
            ]
            if fatal:
                command.append("-DSTRGEN")
            if name == "candidate":
                command += probe.rust(TOOLS)
            probe.compile(command, OUT / (binary.name + "-compile.log"))
            binaries[name, fatal] = binary
    corpus = list(cases())
    records = [
        f"{width} {signed} {base} {clamp} {src.hex() or '-'}\n".encode()
        for width, signed, base, clamp, src in corpus
    ]
    (OUT / "corpus.txt").write_bytes(b"".join(records))
    results = {}
    for name in ("reference", "candidate"):
        result = subprocess.run(
            [str(binaries[name, False])],
            input=b"".join(records),
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=True,
        )
        (OUT / (name + ".txt")).write_bytes(result.stdout)
        results[name] = result.stdout.splitlines()
    assert len(results["reference"]) == len(corpus)
    assert len(results["candidate"]) == len(corpus)
    failures = [
        (index, records[index].decode().strip(), expected.decode(), actual.decode())
        for index, (expected, actual) in enumerate(
            zip(results["reference"], results["candidate"], strict=False)
        )
        if expected != actual
    ]
    (OUT / "failures.json").write_text(json.dumps(failures, indent=2) + "\n")
    if failures:
        raise RuntimeError(
            f"{len(failures)} integer discrepancies; see {OUT / 'failures.json'}"
        )
    fatal_cases = 0
    # These have errors in ReadIntegerBase, with prefix-relative and range spans.
    for case in (
        (8, 1, 0, 0, b"-0xffTAIL"),
        (8, 1, 0, 0, b"-0x100TAIL"),
        (32, 1, 0, 0, b"0x-"),
        (32, 0, 10, 0, b"-1"),
        (16, 1, 0, 0, b"0x\x00\xff\n"),
        (64, 0, 10, 0, b"9" * 80 + b"\x00\xff\n!"),
    ):
        width, signed, base, clamp, src = case
        record = f"{width} {signed} {base} {clamp} {src.hex()}\n".encode()
        outputs = [
            subprocess.run(
                [str(binaries[name, True])],
                input=record,
                env=env,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            for name in ("reference", "candidate")
        ]
        assert outputs[0].returncode == outputs[1].returncode == 2
        assert (
            outputs[0].stdout == outputs[1].stdout
            and outputs[0].stderr == outputs[1].stderr
        )
        fatal_cases += 1
    builders = compare_builders(env, binaries)
    consumer_bytes, consumer_fatal = compare_consumers(env, binaries)
    malformed = compare_generators(env)
    probe.passed(
        "Integer comparison",
        API_cases=len(corpus),
        fatal_diagnostic_cases=fatal_cases,
        builder_formats=builders,
        fixed_sink_alias_cases=13,
        consumer_byte_gaps=consumer_bytes,
        fatal_timing_gaps=consumer_fatal,
        malformed_strgen_pragmas=malformed,
    )


if __name__ == "__main__":
    main()
