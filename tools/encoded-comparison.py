#!/usr/bin/env python3
"""Compare scoped encoded-string gaps using verbatim pristine functions and public APIs.

The sanitizer instruments the C++ fixture and adapters and checks Rust
allocator leaks, not Rust memory accesses.
"""

import json
import re
import resource
import subprocess

import probe

FUNCTIONS = {
    "src/strings.cpp": (
        "EncodedString GetEncodedStringWithArgs(StringID str, std::span<const StringParameter> params)",
        "EncodedString EncodedString::ReplaceParam(size_t param, StringParameter &&data) const",
    ),
    "src/saveload/saveload.cpp": (
        "void FixSCCEncoded(std::string &str, bool fix_code)",
        "void FixSCCEncodedNegative(std::string &str)",
    ),
}
MODES = {
    "debug": [],
    "debug-with-assert": ["-DWITH_ASSERT"],
    "ndebug": ["-DNDEBUG"],
    "ndebug-with-assert": ["-DNDEBUG", "-DWITH_ASSERT"],
}


def scopes(out, label, source):
    """Retain each complete function verbatim; no reference algorithm reimplementation."""
    header = '#include "stdafx.h"\n#include "strings_func.h"\n#include "core/string_builder.hpp"\n#include "core/string_consumer.hpp"\n#include "core/utf8.hpp"\n#include "table/control_codes.h"\n#ifdef WITH_RUST\n#include "rust/encoded_adapter.hpp"\n#endif\n'
    parts = []
    for filename, signatures in FUNCTIONS.items():
        original = (source / filename).read_text()
        for signature in signatures:
            parts.append(probe.body(original, signature + "\n{") + "\n")
    path = out / f"{label}-scoped.cpp"
    path.write_text(header + "\n".join(parts))
    return path


def hexbytes(value):
    return value.hex() or "-"


def token(value):
    if value is None:
        return "E"
    if isinstance(value, int):
        return "N" + format(value, "x")
    return "S" + hexbytes(value)


def corpus():
    encoded, internal, numeric, string = (
        chr(value).encode() for value in range(0xE000, 0xE004)
    )
    rs = b"\x1e"
    legacy = [
        b"",
        b"text",
        b"\xff",
        b"\xff" + encoded,
        b"\xf0\x9f\x98\x80" + encoded,
        encoded,
        encoded + b"0",
        encoded + b"0\xffignored",
        encoded + b"0:\xffignored",
        encoded + b'0:"x:y"',
        encoded + b'0:"unmatched',
        encoded + b'0:"' + encoded + b'12:q"',
        encoded + b'0:"x":',
        encoded + b"0::1",
        encoded + b"0:\x00:1",
        encoded + b'0:"x"2',
        encoded + b'0:"x""y"',
        encoded + b'0:abc::"":-xyz',
    ]
    for marker in (chr(0xE028).encode(), chr(0xE02A).encode()):
        legacy.extend(
            (
                marker + b"1",
                marker + b'1:"x:y":' + marker + b"2",
                encoded + b'0:"' + marker + b':x"',
            )
        )
    for fix in (0, 1):
        for value in legacy:
            yield f"legacy {fix} {hexbytes(value)}"
    for value in (b"", b"text", b"\xff", internal + b"0", encoded, encoded + b"000A"):
        yield f"negative {hexbytes(value)}"
    values = [
        b"0",
        b"0000",
        b"AaFF",
        b"00ABC",
        b"-0",
        b"-1",
        b"-7fffffffffffffff",
        b"-8000000000000000",
        b"-8000000000000001",
        b"ffffffffffffffff",
        b"10000000000000000",
        b"FFFFFFFFFFFFFFFF",
        b"",
        b"-",
        b"+1",
        b"xyz",
        b"-xyz",
        b"0x2",
        b"deadTAIL",
        b"-deadTAIL",
        b"\xff\x00bad",
    ]
    for value in values:
        for suffix in (b"", rs + string + b"tail", b"!suffix"):
            yield f"negative {hexbytes(encoded + b'0' + rs + numeric + value + suffix)}"
    yield f"negative {hexbytes(encoded + b'0' + rs + numeric + b'-xyz' + rs + numeric + b'10000000000000000')}"
    replacements = [None, 42, (1 << 64) - 1, b"", b"new", b"a\x00b", b"a" + rs + b"x"]
    records = [
        [],
        [b""],
        [b"", b""],
        [b"", b"", b""],
        [b"unknown", string + b""],
        [string + b"old", b"", numeric + b"2a"],
        [numeric + b"-1", string + b"tail"],
        [numeric + b"10000000000000000", b"unknown"],
        [b"\xffunknown", string + b"last"],
        [numeric + b"2fTAIL", string + b"last"],
        [numeric + b"-", numeric + b"1x", numeric + b"xyz"],
    ]
    for items in records:
        src = internal + b"000A" + (rs + rs.join(items) if items else b"")
        for index in (0, 1, 2, 99):
            for replacement in replacements:
                yield f"replace {index} {token(replacement)} {hexbytes(src)}"
    for src in (
        b"",
        encoded + b"0",
        b"plain",
        b"\xff",
        internal,
        internal + b"-1",
        internal + b"100000000",
        internal + b"0x2",
        internal + b"0x",
        internal + b"f" + rs + numeric + b"1",
    ):
        yield f"replace 0 N2a {hexbytes(src)}"
    for id_ in (0, 42, (1 << 32) - 1):
        for params in (
            [],
            [None],
            [None, None],
            [None, b"", None],
            [0, (1 << 64) - 1],
            [b"a\x00b", None, b"a" + rs + b"x"],
            [b"\xffbad", b""],
            [b"old", 42, b"tail"],
        ):
            arguments = " ".join(token(value) for value in params)
            yield f"serialize {id_} {arguments}"
            yield f"clear {id_} {arguments}"
            for index in (0, 1, 2, 99):
                for replacement in (None, b"", b"n\x00w", 15):
                    yield f"public-replace {id_} {index} {token(replacement)} {arguments}"
    for prefix in (encoded, internal, rs):
        yield f"serialize 0 S{prefix.hex()}"
        yield f"public-replace 0 0 S{prefix.hex()} N1"
        yield f"public-replace 0 99 S{prefix.hex()} N1"
    yield "serialize 0 D-1 D-2"
    yield "public-replace 0 0 D-2 D-1"
    for index in (0, 99, (1 << 64) - 1):
        yield f"default {index} N1"


def child_limits():
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))


def normalized(result):
    stderr = result.stderr
    # libc assertion paths/functions differ between unchanged source and adapter.
    # Preserve the expression and signal; WITH_ASSERT's release handler is exact stdout.
    if result.returncode == -6:
        found = re.search(rb"Assertion [`'](.+)' failed", stderr)
        if found:
            stderr = b"assert-expression " + found[1] + b"\n"
    return result.returncode, result.stdout, stderr


def command(out, label, source, flags):
    return [
        probe.compiler(),
        "-std=c++20",
        "-O0",
        "-DFMT_HEADER_ONLY",
        "-ffunction-sections",
        "-fdata-sections",
        "-I",
        source / "src",
        probe.ROOT / "tools/migration/encoded-comparison.cpp",
        out / f"{label}-scoped.cpp",
        source / "src/core/string_builder.cpp",
        source / "src/core/string_consumer.cpp",
        source / "src/core/utf8.cpp",
        "-Wl,--gc-sections",
        *flags,
    ] + (probe.rust() if label == "candidate" else [])


def run(argv, data, env=None):
    return subprocess.run(
        list(map(str, argv)),
        input=data,
        env=env or probe.ENV,
        capture_output=True,
        preexec_fn=child_limits,
    )


def main():
    out = probe.output("encoded-comparison")
    records = list(dict.fromkeys(corpus()))
    (out / "corpus.txt").write_text("\n".join(records) + "\n")
    outputs, failures = {}, []
    for label, source in probe.LABELS:
        scopes(out, label, source)
        for mode, flags in MODES.items():
            binary = out / f"{label}-{mode}"
            probe.compile(
                [*command(out, label, source, flags), "-o", binary], f"{binary}.log"
            )
            results = []
            for index, record in enumerate(records):
                value = normalized(run([binary], (record + "\n").encode()))
                results.append(
                    {
                        "exit_code": value[0],
                        "stdout_hex": value[1].hex(),
                        "stderr_hex": value[2].hex(),
                    }
                )
                if (
                    label != "reference"
                    and results[-1] != outputs["reference", mode][index]
                ):
                    failures.append(
                        {
                            "candidate": label,
                            "mode": mode,
                            "input": record,
                            "expected": outputs["reference", mode][index],
                            "actual": results[-1],
                        }
                    )
            outputs[label, mode] = results
    if failures:
        (out / "failures.json").write_text(json.dumps(failures, indent=2) + "\n")
        raise RuntimeError(
            f"{len(failures)} encoded discrepancies; see {out / 'failures.json'}"
        )
    sanitizer = out / "candidate-sanitized"
    probe.compile(
        [
            *command(out, "candidate", probe.ROOT, MODES["ndebug"]),
            "-o",
            sanitizer,
            "-fsanitize=address,undefined",
            "-fno-omit-frame-pointer",
            "-g",
            "-no-pie",
        ],
        f"{sanitizer}.log",
    )
    sanitized_env = probe.ENV | {
        "ASAN_OPTIONS": "detect_leaks=1:abort_on_error=1",
        "UBSAN_OPTIONS": "halt_on_error=1:print_stacktrace=1",
    }
    expected = b"".join(
        bytes.fromhex(item["stdout_hex"]) for item in outputs["candidate", "ndebug"]
    )
    encoded, numeric = (chr(value).encode() for value in (0xE000, 0xE002))
    exception_bytes = encoded + b"0\x1e" + numeric + b"-xyz"
    exception_input = f"negative {hexbytes(exception_bytes)}\n".encode()
    # All assertions are disabled here; enabled assertion policies are compared above.
    for arguments, data in (
        ([], ("\n".join(records) + "\n").encode()),
        (["--throw"], exception_input),
        (["--copy-throw"], b""),
    ):
        result = run([sanitizer, *arguments], data, sanitized_env)
        name = arguments[0][2:] if arguments else "corpus"
        (out / f"sanitized-{name}.log").write_bytes(result.stdout + result.stderr)
        if result.returncode or result.stderr:
            raise RuntimeError(f"Sanitized {name} run failed; see retained log")
        if not arguments and result.stdout != expected:
            raise RuntimeError("Sanitizer corpus output changed")
        if arguments == ["--throw"] and b"exception " not in result.stdout:
            raise RuntimeError("Logger exception did not occur")
        if arguments == ["--copy-throw"] and not result.stdout.startswith(
            b"copy-exception "
        ):
            raise RuntimeError("Output-copy exception did not occur")
    probe.passed(
        "Encoded comparison",
        cases=len(records),
        assertion_modes=len(MODES),
    )


if __name__ == "__main__":
    main()
