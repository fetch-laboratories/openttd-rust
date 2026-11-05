#!/usr/bin/env python3
"""Compare the scoped UTF-8 corpus against unchanged pinned-reference sources.

Limits: historical byte codec with all 16 flag combinations and ignored unknown
bits; in-place copy only in the original's defined disjoint/left-overlap domain;
bounded C++ append failure, not arbitrary allocator failure. Packet text loops
and other string algorithms remain unchanged.
"""

from pathlib import Path

import probe

SOURCES = (
    "src/core/utf8.cpp",
    "src/core/string_consumer.cpp",
    "src/core/string_builder.cpp",
    "src/core/string_inplace.cpp",
)


def main() -> None:
    out = probe.output("utf8-comparison")
    fixture = probe.ROOT / "tools/migration/utf8-comparison.cpp"
    counts: dict[str, int] = {}
    for mode, flags in (("asserts", []), ("ndebug", ["-DNDEBUG"])):

        def command(
            label: str, source: Path, flags: list[str] = flags
        ) -> list[probe.Argument]:
            return [
                probe.compiler(),
                "-std=c++20",
                "-O2",
                "-ffunction-sections",
                "-fdata-sections",
                "-DFMT_HEADER_ONLY",
                "-I",
                source / "src",
                fixture,
                *(source / name for name in SOURCES),
                "-Wl,--gc-sections",
                *flags,
            ]

        output = probe.compare(out, mode, command, stderr="merged")
        counts[f"{mode}_lines"] = len(output.splitlines())
    probe.passed("UTF-8 comparison", **counts)


if __name__ == "__main__":
    main()
