#!/usr/bin/env python3
"""Compare NewGRF-only variable-length curve and reversal inputs with pinned bodies.

The semantic rail scenarios use stock engines (length eight, no articulated parts).
This narrow copied-world probe calls production Rust entries, retaining the exact
original curve/reversal bodies as the oracle; it does not emulate a controller.
"""

import argparse
import re
from pathlib import Path

import probe

SIGNATURES = (
    "uint16_t Train::GetCurveSpeedLimit() const",
    "static void SwapTrainFlags(uint16_t *swap_flag1, uint16_t *swap_flag2)",
    "static void UpdateStatusAfterSwap(Train *v)",
    "void ReverseTrainSwapVeh(Train *v, int l, int r)",
)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build", type=Path, default=probe.BUILD)
    args = parser.parse_args()
    out = probe.output("train-edge-comparison")
    source = (probe.REFERENCE / "src/train_cmd.cpp").read_text()
    (out / "train-reference.inc").write_text(
        "\n".join(probe.body(source, signature) + "\n" for signature in SIGNATURES)
    )
    probe.compile(
        [
            probe.compiler(args.build),
            "-std=c++20",
            "-O2",
            *probe.target_flags(args.build),
            "-I",
            probe.ROOT / "src",
            "-I",
            out,
            probe.ROOT / "tools/migration/train-edge-comparison.cpp",
            *probe.rust(args.build, define=False),
            "-o",
            out / "train-edge",
        ],
        out / "compile.log",
    )
    output = probe.execute([out / "train-edge"], out / "run", stderr="merged")
    curve, reversal = re.findall(rb"\d+", output)[:2]
    probe.passed(
        "Train edge comparison",
        curve_cases=int(curve),
        reversal_cases=int(reversal),
    )


if __name__ == "__main__":
    main()
