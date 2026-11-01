#!/usr/bin/env python3
"""Compare bounded actual Packet framing, bytes and callback states.

Limits: a bounded companion to the unchanged network tests and authentication
corpus. Allocation failure is environmental and excluded;
nonempty-short Recv_bytes is undefined upstream (#41) and excluded. No real
transport delivery, sockets or allocator-wide equivalence.
"""

import probe

SOURCES = (
    "src/network/core/packet.cpp",
    "src/string.cpp",
    "src/core/string_builder.cpp",
    "src/core/string_consumer.cpp",
    "src/core/string_inplace.cpp",
    "src/core/utf8.cpp",
)


def main():
    out = probe.output("packet-comparison")
    fixture = probe.ROOT / "tools/migration/packet-comparison.cpp"
    counts = {}
    for mode in ("O0", "O2"):

        def command(label, source, mode=mode):
            return [
                probe.compiler(),
                "-std=c++20",
                f"-{mode}",
                "-DUNIX",
                "-DFMT_HEADER_ONLY",
                "-ffunction-sections",
                "-fdata-sections",
                "-I",
                source / "src",
                "-I",
                probe.BUILD / "generated-abi",
                fixture,
                *(source / name for name in SOURCES),
                "-Wl,--gc-sections",
            ]

        output = probe.compare(out, mode, command, stderr="allowed")
        counts[f"{mode}_lines"] = len(output.splitlines())
    probe.passed("Packet comparison", **counts)


if __name__ == "__main__":
    main()
