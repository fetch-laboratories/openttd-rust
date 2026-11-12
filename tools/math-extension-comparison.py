#!/usr/bin/env python3
"""Compare the accepted wide-integer math templates with the pinned original.

Runs on every platform, including the macOS native library. The former bounded
math-comparison.cpp observations moved to Rust tests and the core boundary
comparison (#736 U; migration/evidence-replacements/736-u-vectors.json).
"""

import argparse
from pathlib import Path

import probe

import migration

LABELS = (
    ("reference", probe.REFERENCE),
    ("candidate", probe.ROOT),
)


def wide(out: Path, build: Path) -> int:
    configuration = migration.rust_configuration(build)
    fixture = probe.ROOT / "tools/migration/math-extension-comparison.cpp"
    width = f"-DOPENTTD_MATH_POINTER_BYTES={configuration['pointer_bytes']}"

    def command(source: Path) -> list[probe.Argument]:
        return [
            probe.compiler(build),
            "-std=c++20",
            "-O2",
            width,
            *probe.target_flags(build),
            "-I",
            source / "src",
            fixture,
        ]

    streams: dict[str, bytes] = {}
    for label, source in LABELS:
        binary = out / f"wide-{label}"
        argv = [*command(source), "-o", binary]
        argv += probe.rust(build) if label == "candidate" else []
        probe.compile(argv, f"{binary}.log")
        if probe.RUN:
            streams[label] = probe.execute([binary], binary, stderr="merged")
            probe.same(streams["reference"], streams[label], binary.name, out)
    return len(streams.get("reference", b"").splitlines())


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build", type=Path, default=probe.BUILD)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    out = probe.output("math-extension-comparison", args.output)
    probe.passed("Math extension comparison", wide_lines=wide(out, args.build))


if __name__ == "__main__":
    main()
