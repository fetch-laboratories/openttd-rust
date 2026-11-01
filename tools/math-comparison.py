#!/usr/bin/env python3
"""Compare the bounded and wide-template math corpora with pinned-reference sources.

The bounded corpus counts Rust routing with GNU link wrapping, so it runs on
Linux; Darwin runs the accepted wide templates against the native library.
"""

import argparse
import subprocess
from pathlib import Path

import probe

import migration

LABELS = (
    ("reference", probe.REFERENCE),
    ("candidate", probe.ROOT),
)
# The pinned strict template never accepted these extended-type combinations.
REJECTED = {
    "wide-signed-source": "ClampTo<unsigned __int128>(int64_t(-1))",
    "wide-signed-destination": "ClampTo<__int128>(uint64_t(7))",
    "wide-source": "ClampTo<uint64_t>(static_cast<unsigned __int128>(7))",
}


def bounded(out, counts):
    fixture = probe.ROOT / "tools/migration/math-comparison.cpp"
    signature = "uint GetPartialPixelZ(int x, int y, Slope corners)"
    for label, source in LABELS:
        # The simulation harness keeps coordinates inside a tile. Extract the
        # exact production body to exercise the defined wider domain.
        (out / f"landscape-{label}.cpp").write_text(
            '#include "stdafx.h"\n#include "landscape.h"\n'
            '#include "slope_func.h"\n'
            '#ifdef WITH_RUST\n#include "rust/ffi.h"\n#endif\n'
            + probe.body((source / "src/landscape.cpp").read_text(), signature)
            + "\n"
        )
    for mode, flags in (("asserts", []), ("ndebug", ["-DNDEBUG"])):
        streams = {}
        for label, source in LABELS:
            binary = out / f"{label}-{mode}"
            command = [
                probe.compiler(),
                "-std=c++20",
                "-O2",
                "-ffunction-sections",
                "-fdata-sections",
                "-DFMT_HEADER_ONLY",
                "-fsanitize=undefined",
                "-fno-sanitize-recover=undefined",
                "-I",
                source / "src",
                fixture,
                source / "src/core/math_func.cpp",
                out / f"landscape-{label}.cpp",
                "-Wl,--gc-sections",
                "-o",
                binary,
                *flags,
            ]
            if label == "candidate":
                command += ["-DMATH_ROUTE_PROBE", *probe.rust()]
                command += ["-Wl,--wrap=openttd_rust_int_sqrt"]
                command += ["-Wl,--wrap=openttd_rust_soft_clamp"]
            probe.compile(command, f"{binary}.log")
            lines = probe.execute([binary], binary, stderr="merged").splitlines(True)
            routing = [line for line in lines if line.startswith(b"Rust routing:")]
            if label == "candidate" and not routing:
                raise RuntimeError("Candidate math build did not report Rust routing")
            streams[label] = b"".join(line for line in lines if line not in routing)
            probe.same(streams["reference"], streams[label], binary.name, out)
        counts[f"{mode}_lines"] = len(streams["reference"].splitlines())
    for name, expression in REJECTED.items():
        source_file = out / f"{name}.cpp"
        source_file.write_text(
            '#include "stdafx.h"\n#include "core/math_func.hpp"\nauto result = '
            + expression
            + ";\n"
        )
        for label, source in LABELS:
            command = [probe.compiler(), "-std=c++20", "-fsyntax-only"]
            command += ["-I", str(source / "src"), str(source_file)]
            if label == "candidate":
                command.append("-DWITH_RUST")
            if not subprocess.run(
                command, env=probe.ENV, capture_output=True
            ).returncode:
                raise RuntimeError(f"{label} accepted rejected {name}")
    counts["preserved_rejections"] = len(REJECTED)


def wide(out, build, counts):
    configuration = migration.rust_configuration(build)
    fixture = probe.ROOT / "tools/migration/math-extension-comparison.cpp"
    width = f"-DOPENTTD_MATH_POINTER_BYTES={configuration['pointer_bytes']}"

    def command(label, source):
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

    streams = {}
    for label, source in LABELS:
        binary = out / f"wide-{label}"
        argv = [*command(label, source), "-o", binary]
        argv += probe.rust(build) if label == "candidate" else []
        probe.compile(argv, f"{binary}.log")
        streams[label] = probe.execute([binary], binary, stderr="merged")
        probe.same(streams["reference"], streams[label], binary.name, out)
    counts["wide_lines"] = len(streams["reference"].splitlines())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build", type=Path, default=probe.BUILD)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    out = probe.output("math-comparison", args.output)
    counts = {}
    if migration.rust_configuration(args.build)["platform"] == "Linux":
        bounded(out, counts)
    wide(out, args.build, counts)
    probe.passed("Math comparison", **counts)


if __name__ == "__main__":
    main()
