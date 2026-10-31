#!/usr/bin/env python3
"""Compare demonstrated generic-history gaps through unchanged public interfaces.

The sanitizer instruments the C++ fixture and adapters and checks Rust
allocator leaks, not Rust memory accesses. NOT_REACHED/AssertFailedError stubs
check dispatch and C++ unwinding, not game fatal text or source locations.
"""

from pathlib import Path

import probe

REDUCERS = {
    "src/industry_cmd.cpp": (
        "Industry::ProducedHistory SumHistory(std::span<const Industry::ProducedHistory> history)",
        "Industry::AcceptedHistory SumHistory(std::span<const Industry::AcceptedHistory> history)",
    ),
    "src/town_cmd.cpp": (
        "Town::SuppliedHistory SumHistory(std::span<const Town::SuppliedHistory> history)",
    ),
}


def portable_specialization(text: str) -> str:
    """Retain the original body behind the town owner's WITH_RUST facade."""
    if "\n#ifdef WITH_RUST\n" not in text:
        return text
    header, branches = text.split("\n#ifdef WITH_RUST\n", 1)
    _, portable = branches.split("\n#else\n", 1)
    body, tail = portable.split("\n#endif\n", 1)
    return header + "\n" + body.lstrip("\n") + "\n" + tail


def reducers(out: Path) -> dict[str, Path]:
    """Extract each source's reducers; require unchanged portable reference bodies."""
    pieces = {"reference": [], "candidate": []}
    for filename, signatures in REDUCERS.items():
        source = (probe.REFERENCE / filename).read_text()
        candidate = (probe.ROOT / filename).read_text()
        for signature in signatures:
            # Read the actual specialization, including any owner facade.
            marker = "template <>\n" + signature + "\n{"
            original = probe.body(source, marker) + "\n"
            current = probe.body(candidate, marker) + "\n"
            if (
                filename != "src/industry_cmd.cpp"
                and portable_specialization(current) != original
            ):
                raise RuntimeError(f"Production reducer changed: {signature}")
            pieces["reference"].append(original)
            pieces["candidate"].append(current)
    paths = {}
    for label, bodies in pieces.items():
        paths[label] = out / f"{label}-production-reducers.cpp"
        paths[label].write_text(
            '#include "stdafx.h"\n#include "misc/history_func.hpp"\n#include "industry.h"\n#include "town.h"\n'
            + "\n".join(bodies)
        )
    return paths


def main():
    out = probe.output("history-comparison")
    production = reducers(out)
    fixture = probe.ROOT / "tools/migration/history-comparison.cpp"
    outputs = {}
    for optimization in ("O0", "O2"):

        def command(label, source, optimization=optimization):
            return [
                probe.compiler(),
                "-std=c++20",
                f"-{optimization}",
                "-DFMT_HEADER_ONLY",
                "-ffunction-sections",
                "-fdata-sections",
                "-I",
                source / "src",
                fixture,
                production["candidate" if label == "candidate" else "reference"],
                source / "src/misc/history.cpp",
                "-Wl,--gc-sections",
            ]

        outputs[optimization] = probe.compare(out, optimization, command)
    original = outputs["O0"].splitlines()
    # These concrete gaps must actually occur in the pristine output.
    if b"grouping 0 1" not in original:
        raise RuntimeError(
            "Nested production rounding fixture no longer demonstrates grouping"
        )
    if not any(
        line.startswith(b"query ")
        and b" fatal " not in line
        and line.split()[5] != line.split()[6]
        for line in original
    ):
        raise RuntimeError("First-child validity versus OR validity gap not exercised")
    sanitized = out / "candidate-sanitized"
    probe.compile(
        [
            *command("candidate", probe.ROOT, "O0"),
            "-o",
            sanitized,
            *probe.rust(),
            "-fsanitize=address,undefined",
            "-fno-omit-frame-pointer",
            "-g",
            "-no-pie",
        ],
        f"{sanitized}.log",
    )
    env = probe.ENV | {
        "ASAN_OPTIONS": "detect_leaks=1:abort_on_error=1",
        "UBSAN_OPTIONS": "halt_on_error=1:print_stacktrace=1",
    }
    probe.same(
        outputs["O0"],
        probe.execute([sanitized], sanitized, env=env),
        "candidate-sanitized",
        out,
    )
    probe.passed("History comparison", records_at_O0_and_O2=len(original))


if __name__ == "__main__":
    main()
