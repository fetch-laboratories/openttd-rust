#!/usr/bin/env python3
"""Check the pool-limit Split failure and large queues absent from play saves."""

import probe
from simulation.cargo_storage import scaling_probe


def main():
    out = probe.output("cargo-storage-comparison")
    original = (probe.REFERENCE / "src/cargopacket.cpp").read_text()
    body = probe.body(original, "CargoPacket *CargoPacket::Split(uint new_size)")
    fixture = probe.ROOT / "tools/simulation/cargo-split-reference.cpp"
    source = out / "split.cpp"
    source.write_text(fixture.read_text().replace("@UNCHANGED_SPLIT@", body))

    def command(label, root):
        return [
            probe.compiler(),
            "-std=c++20",
            "-O2",
            "-include",
            "cstdlib",
            "-include",
            "initializer_list",
            "-I",
            probe.ROOT / "src",
            source,
        ]

    labels = probe.LABELS[:2]
    output = probe.compare(out, "split", command, labels=labels, stderr="allowed")
    # Pool-limit Split failure: early return and no mutation or allocation.
    probe.passed("Cargo pool-limit Split comparison", cases=len(output.splitlines()))


if __name__ == "__main__":
    main()
    scaling_probe()
