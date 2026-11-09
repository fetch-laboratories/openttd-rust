#!/usr/bin/env python3
"""Run the #736 C++ calling-boundary sections against the original and the candidate.

tools/migration/core-boundary-comparison.cpp keeps only fork C++ glue observations
that Rust tests cannot make (#736 U and P). Each section builds once per label and must
pass its pinned-original literals in both; the encoded assertion modes compare
exit, stdout and normalized stderr between labels, and the packet section requires
the candidate's stdout to equal the original's at -O0 and -O2 -DNDEBUG.
Linux only (GNU ld --wrap).
"""

import re
import subprocess
from collections.abc import Sequence
from pathlib import Path

import probe

FIXTURE = probe.ROOT / "tools/migration/core-boundary-comparison.cpp"
# Section: (optimization, defines, sources relative to the label's tree).
SECTIONS = {
    "INTEGER": (
        "-O0",
        ["-DSTRGEN"],
        ["core/string_consumer.cpp", "core/string_builder.cpp", "core/utf8.cpp"],
    ),
    "UTF8": (
        "-O2",
        [],
        [
            "core/utf8.cpp",
            "core/string_consumer.cpp",
            "core/string_builder.cpp",
            "core/string_inplace.cpp",
        ],
    ),
    "MATH": ("-O2", [], ["core/math_func.cpp"]),
    "BYTES": ("-O2", [], ["string.cpp", "core/string_consumer.cpp"]),
}
ENCODED_SOURCES = [
    "core/string_builder.cpp",
    "core/string_consumer.cpp",
    "core/utf8.cpp",
]
ENCODED_FUNCTIONS = {
    "src/strings.cpp": (
        "EncodedString GetEncodedStringWithArgs(StringID str, std::span<const StringParameter> params)",
        "EncodedString EncodedString::ReplaceParam(size_t param, StringParameter &&data) const",
    ),
    "src/saveload/saveload.cpp": (
        "void FixSCCEncoded(std::string &str, bool fix_code)",
        "void FixSCCEncodedNegative(std::string &str)",
    ),
}
ENCODED_MODES = {
    "debug": [],
    "debug-with-assert": ["-DWITH_ASSERT"],
    "ndebug": ["-DNDEBUG"],
    "ndebug-with-assert": ["-DNDEBUG", "-DWITH_ASSERT"],
}
PACKET_SOURCES = [
    "network/core/packet.cpp",
    "string.cpp",
    "core/string_builder.cpp",
    "core/string_consumer.cpp",
    "core/string_inplace.cpp",
    "core/utf8.cpp",
]
PACKET_MODES = {"O0": ["-O0"], "O2": ["-O2", "-DNDEBUG"]}
SANITIZE = ["-fsanitize=address,undefined", "-fno-omit-frame-pointer", "-g", "-no-pie"]


def build(
    out: Path, name: str, label: str, section: str, flags: Sequence[probe.Argument]
) -> Path:
    source = dict(probe.LABELS)[label]
    binary = out / f"{label}-{name}"
    command = [
        probe.compiler(),
        "-std=c++20",
        "-DFMT_HEADER_ONLY",
        "-ffunction-sections",
        "-fdata-sections",
        f"-DCORE_BOUNDARY_{section}",
        "-I",
        source / "src",
        FIXTURE,
        *flags,
        "-Wl,--gc-sections",
        "-o",
        binary,
    ]
    if label == "candidate":
        command += probe.rust()
        if section == "MATH":
            command += ["-Wl,--wrap=openttd_rust_int_sqrt"]
            command += ["-Wl,--wrap=openttd_rust_soft_clamp"]
    probe.compile(command, f"{binary}.log")
    return binary


def scoped(out: Path, label: str, source: Path) -> Path:
    """The encoded section's original or candidate production bodies, verbatim."""
    header = '#include "stdafx.h"\n#include "strings_func.h"\n#include "core/string_builder.hpp"\n#include "core/string_consumer.hpp"\n#include "core/utf8.hpp"\n#include "table/control_codes.h"\n#ifdef WITH_RUST\n#include "rust/encoded_adapter.hpp"\n#endif\n'
    parts = [
        probe.body((source / filename).read_text(), signature + "\n{") + "\n"
        for filename, signatures in ENCODED_FUNCTIONS.items()
        for signature in signatures
    ]
    path = out / f"{label}-encoded-scoped.cpp"
    path.write_text(header + "\n".join(parts))
    return path


def outcome(binary: Path, argument: str) -> tuple[int, bytes, bytes]:
    result = subprocess.run(
        [str(binary), argument], env=probe.ENV, capture_output=True, check=False
    )
    stderr = result.stderr
    # libc assertion paths differ between trees; keep the expression and signal.
    found = re.search(rb"Assertion [`'](.+)' failed", stderr)
    if result.returncode == -6 and found:
        stderr = b"assert-expression " + found[1] + b"\n"
    return result.returncode, result.stdout, stderr


def encoded(out: Path) -> int:
    builds = 0
    for mode, flags in ENCODED_MODES.items():
        outcomes = {}
        for label, source in probe.LABELS:
            sources = [scoped(out, label, source)]
            sources += [source / "src" / name for name in ENCODED_SOURCES]
            binary = build(
                out,
                f"encoded-{mode}",
                label,
                "ENCODED",
                ["-O0", *sources, *flags],
            )
            builds += 1
            probe.execute([binary, "--logs"], f"{binary}-logs")
            outcomes[label] = [
                outcome(binary, argument)
                for argument in ("--assert-numeric", "--assert-string")
            ]
        if outcomes["reference"] != outcomes["candidate"]:
            raise RuntimeError(f"Encoded assertion outcomes differ in {mode}")
    sources = [scoped(out, "candidate", probe.ROOT)]
    sources += [probe.ROOT / "src" / name for name in ENCODED_SOURCES]
    sanitized = build(
        out,
        "encoded-sanitized",
        "candidate",
        "ENCODED",
        ["-O0", *sources, "-DNDEBUG", *SANITIZE],
    )
    env = probe.ENV | {
        "ASAN_OPTIONS": "detect_leaks=1:abort_on_error=1",
        "UBSAN_OPTIONS": "halt_on_error=1:print_stacktrace=1",
    }
    for argument in ("--logs", "--copy-throw"):
        probe.execute([sanitized, argument], f"{sanitized}{argument}", env=env)
    return builds + 1


def packet(out: Path) -> int:
    """Native Packet glue: the candidate's stream must equal the original's in each mode."""
    for mode, flags in PACKET_MODES.items():
        outputs = {}
        for label, source in probe.LABELS:
            sources = [source / "src" / name for name in PACKET_SOURCES]
            binary = build(
                out,
                f"packet-{mode}",
                label,
                "PACKET",
                [*flags, "-DUNIX", "-Werror=mismatched-tags", *sources],
            )
            outputs[label] = probe.execute([binary], binary)
        if not outputs["reference"].endswith(b"packet boundary passed\n"):
            raise RuntimeError(f"Packet boundary stream incomplete in {mode}")
        probe.same(outputs["reference"], outputs["candidate"], f"packet-{mode}", out)
    return len(PACKET_MODES) * len(probe.LABELS)


def main() -> None:
    out = probe.output("core-boundary-comparison")
    builds = 0
    for section, (optimization, defines, sources) in SECTIONS.items():
        for label, source in probe.LABELS:
            flags = [
                optimization,
                *defines,
                *(source / "src" / name for name in sources),
            ]
            binary = build(out, section.lower(), label, section, flags)
            probe.execute([binary], binary, stderr="merged")
            builds += 1
    builds += encoded(out)
    builds += packet(out)
    probe.passed("Core boundary comparison", sections=len(SECTIONS) + 2, builds=builds)


if __name__ == "__main__":
    main()
