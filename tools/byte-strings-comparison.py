#!/usr/bin/env python3
"""Compare concrete byte-string gaps with the pristine native original APIs."""

import subprocess

import probe


def token(value):
    return value.hex() or "-"


def corpus():
    yield "null -"
    values = [
        b"",
        b"a",
        b"A",
        b"aa",
        b"aA",
        b"b",
        b"ababa",
        b"aba",
        b"abab",
        b"BAB",
        b"absent",
        b"abc",
        b"c",
        b"\x00",
        b"a\x00B",
        b"A\x00b",
        b"\x00b",
        b"x\x00a\x00Bz",
    ]
    for left in values:
        for right in values:
            yield f"pair {token(left)} {token(right)}"
    for byte in range(256):
        value = bytes([byte])
        for right in (b"\x00", b"A", b"\x7f", b"\xff", value):
            yield f"pair {token(value)} {token(right)}"
        for offset in (0, 1, 2, 3):
            yield f"lower {token(b'A' + value + b'Z')} {offset}"
        yield f"encode {token(value)}"
        for pair in (b"1" + value, value + b"1"):
            yield f"decode {token(pair)} 1"
    for value in (b"", b"lower", b"UPPER", b"Aa\x00Z", bytes(range(256))):
        yield f"encode {token(value)}"
        for offset in dict.fromkeys((0, len(value) // 2, len(value))):
            yield f"lower {token(value)} {offset}"
    for value in (
        b"",
        b"1",
        b"12",
        b"123",
        b"1234",
        b"12g4",
        b"123g",
        b"g234",
        b"1g34",
        b"123456789abcdef0",
        b"123456789ABCDEF0",
        b"123456g8",
        b"12\x0034",
        b"12\xff4",
    ):
        for length in range(6):
            yield f"decode {token(value)} {length}"
    all_hex = b"".join(f"{byte:02X}".encode() for byte in range(256))
    yield f"decode {token(all_hex)} 256"
    for value in (b"12345678", b"1234g678", b"1234567g", b"g2345678", b"1234567"):
        backing = b"\xa5\xa5" + value + b"\xa5" * 12
        for offset in range(11):
            for length in (len(value) // 2, len(value) // 2 + 1):
                yield f"overlap {token(backing)} 2 {len(value)} {offset} {length}"
    values = [
        b"",
        b" ",
        b" \t\v\f\r ",
        b"\n",
        b" \n ",
        b"  a  b  ",
        b"\x00",
        b"\x00a\x00",
        b"\xff\x00a\xff",
        b"\xff\xff",
        b"aaaa",
        b"xabxx",
    ]
    sets = [
        b"",
        b" \t\v\f\r",
        b"\x00",
        b"\xff",
        b"\xff\x00",
        b"a",
        b"x",
        bytes(range(256)),
    ]
    for value in values:
        yield f"inplace {token(value)}"
        for set_ in sets:
            yield f"trim {token(value)} {token(set_)}"


def main():
    out = probe.output("byte-strings-comparison")
    records = list(dict.fromkeys(corpus()))
    data = ("\n".join(records) + "\n").encode()
    (out / "corpus.txt").write_bytes(data)
    locales = subprocess.check_output(
        ["locale", "-a"], env=probe.ENV, text=True
    ).splitlines()
    if "C" not in locales:
        raise RuntimeError("Native C locale is unavailable")
    fixture = probe.ROOT / "tools/migration/byte-strings-comparison.cpp"
    outputs = {}
    for signedness, flags in (
        ("native", []),
        ("unsigned-char", ["-funsigned-char"]),
    ):
        for label, source in probe.LABELS:
            binary = out / f"{label}-{signedness}"
            command = [
                probe.compiler(),
                "-std=c++20",
                "-O2",
                "-DFMT_HEADER_ONLY",
                "-ffunction-sections",
                "-fdata-sections",
                "-I",
                source / "src",
                fixture,
                source / "src/string.cpp",
                source / "src/core/string_consumer.cpp",
                "-Wl,--gc-sections",
                "-o",
                binary,
                *flags,
            ]
            if label == "candidate":
                command += probe.rust()
            probe.compile(command, f"{binary}.log")
            for index, locale in enumerate(locales):
                name = f"{binary.name}-locale-{index}"
                output = probe.execute(
                    [binary, locale], out / name, input=data, stderr="allowed"
                )
                outputs[label, signedness, locale] = output
                probe.same(outputs["reference", signedness, locale], output, name, out)
    sanitizer = out / "candidate-sanitized"
    command = [
        probe.compiler(),
        "-std=c++20",
        "-O1",
        "-DFMT_HEADER_ONLY",
        "-ffunction-sections",
        "-fdata-sections",
        "-I",
        probe.ROOT / "src",
        fixture,
        *(
            probe.ROOT / "src" / name
            for name in (
                "string.cpp",
                "core/string_consumer.cpp",
                "core/string_builder.cpp",
                "core/string_inplace.cpp",
                "core/utf8.cpp",
            )
        ),
        "-Wl,--gc-sections",
        *probe.rust(),
        "-fsanitize=address,undefined",
        "-fno-omit-frame-pointer",
        "-no-pie",
        "-o",
        sanitizer,
    ]
    probe.compile(command, f"{sanitizer}.log")
    env = probe.ENV | {
        "ASAN_OPTIONS": "detect_leaks=1:halt_on_error=1",
        "UBSAN_OPTIONS": "halt_on_error=1",
    }
    actual = probe.execute(
        [sanitizer, "C"], out / "sanitized-C", input=data, env=env, stderr="allowed"
    )
    probe.same(outputs["reference", "native", "C"], actual, "sanitized-C", out)
    probe.passed(
        "Byte-string comparison",
        cases=len(records),
        locales=len(locales),
        char_signedness_modes=2,
    )


if __name__ == "__main__":
    main()
