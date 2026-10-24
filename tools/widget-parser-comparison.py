#!/usr/bin/env python3
"""Compare scoped widget-parser gaps against unchanged original control bodies.

Limits: native Linux. Only widget.cpp and the oracle/fixture assertion flags
vary; other production objects keep their native flags. Fatal null paths
compare termination category and callback entry, not assertion text or
location. Background shapes use existing default vertical children; no
rendering or complete GUI equivalence.
"""

import re
import shlex
import subprocess

import probe

ROOT = probe.ROOT
REFERENCE = probe.REFERENCE
BUILD = probe.BUILD
extract = probe.body
SIGNATURES = (
    "static bool IsAttributeWidgetPartType(",
    "bool IsContainerWidgetType(",
    "static std::unique_ptr<NWidgetBase> MakeNWidget(const NWidgetPart &nwid)",
    "static std::span<const NWidgetPart>::iterator MakeNWidget(",
    "static std::span<const NWidgetPart>::iterator MakeWidgetTree(",
    "std::unique_ptr<NWidgetBase> MakeNWidgets(",
    "std::unique_ptr<NWidgetBase> MakeWindowNWidgetTree(",
)


def main():
    out = probe.output("widget-parser-comparison")
    original_source = (REFERENCE / "src/widget.cpp").read_text()
    current_source = (ROOT / "src/widget.cpp").read_text()
    bodies = {
        signature: extract(original_source, signature) for signature in SIGNATURES
    }
    for signature in (
        "void ApplyNWidgetPartAttribute(",
        SIGNATURES[2],
        "class NWidgetLayer : public NWidgetContainer {",
    ):
        original = extract(original_source, signature)
        if original != extract(current_source, signature):
            raise RuntimeError(f"Retained typed primitive changed: {signature}")
    renamed = "\n\n".join(bodies.values())
    for name in (
        "IsAttributeWidgetPartType",
        "IsContainerWidgetType",
        "MakeNWidget",
        "MakeWidgetTree",
        "MakeNWidgets",
        "MakeWindowNWidgetTree",
    ):
        # Replace longest matching identifiers rather than prefixes of MakeNWidgets.
        renamed = re.sub(r"\b" + name + r"\b", "Original" + name, renamed)
    oracle = out / "original.cpp"
    oracle.write_text(
        '#include "stdafx.h"\n#include "window_gui.h"\n#include "table/strings.h"\n'
        + "void ApplyNWidgetPartAttribute(const NWidgetPart &, NWidgetBase *);\n"
        + extract(original_source, "class NWidgetLayer : public NWidgetContainer {")
        + ";\n"
        + renamed
        + "\n"
    )
    env = probe.ENV
    if probe.migration.rust_configuration(BUILD)["platform"] != "Linux":
        raise RuntimeError(
            "This production-object semantic fixture currently requires native Linux"
        )
    compiler = probe.compiler()
    flags = [
        compiler,
        "-std=c++20",
        "-DWITH_RUST",
        "-DUNIX",
        "-DPOINTER_IS_64BIT",
        "-DFMT_HEADER_ONLY",
        "-I",
        str(ROOT / "src"),
        "-I",
        str(BUILD / "generated"),
        "-I",
        str(ROOT / "src/script/api"),
        "-I",
        str(BUILD / "generated/script/api"),
        "-I",
        str(ROOT / "src/3rdparty/squirrel/include"),
    ]
    # Reuse the actual game object/library inputs, replacing only widget.cpp and
    # test entry points. This preserves registered descriptors and real primitives.
    raw = (
        subprocess.check_output(
            ["ninja", "-C", str(BUILD), "-t", "commands", "openttd_test"], env=env
        )
        .decode()
        .splitlines()[-1]
    )
    link = shlex.split(raw.removeprefix(": && "))
    if "&&" in link:
        link = link[: link.index("&&")]
    if link[0] != compiler:
        raise RuntimeError("Unexpected native test link compiler")
    inputs, cursor = [], 1
    while cursor < len(link):
        token = link[cursor]
        cursor += 1
        if token == "-o":
            cursor += 1
            continue
        if token.startswith("-Wl,--dependency-file="):
            continue
        if token.endswith(".o"):
            if "CMakeFiles/openttd_test.dir/" in token and not token.endswith(
                "/mock_spritecache.cpp.o"
            ):
                continue
            if token.endswith("/src/widget.cpp.o"):
                continue
            token = str(BUILD / token)
        elif token.endswith((".a", ".so")) and not token.startswith("/"):
            token = str(BUILD / token)
        inputs.append(token)
    failures = []

    def command(argv, label):
        probe.compile(argv, out / f"{label}.log")

    modes = {
        "custom-assert": ["-O2", "-DNDEBUG", "-DWITH_ASSERT"],
        "standard-assert": ["-O0"],
        "release": ["-O2", "-DNDEBUG"],
    }
    for mode, policy in modes.items():
        objects = []
        for label, source in (
            ("widget", ROOT / "src/widget.cpp"),
            ("oracle", oracle),
            ("fixture", ROOT / "tools/migration/widget-parser-comparison.cpp"),
        ):
            obj = out / f"{mode}-{label}.o"
            command(
                [*flags, *policy, "-c", str(source), "-o", str(obj)],
                f"{mode}-{label}-compile",
            )
            objects.append(str(obj))
        binary = out / mode
        command([compiler, *objects, *inputs, "-o", str(binary)], f"{mode}-link")
        probe.execute([binary], out / mode)
        null_records = []
        for version in ("original", "candidate"):
            result = subprocess.run(
                [str(binary), "--null", version],
                env=env,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            null_records.append(
                {"exit": result.returncode, "stdout": result.stdout.decode()}
            )
        if mode == "release":
            if null_records[0] != null_records[1] or null_records[0]["exit"] != 0:
                failures.append({"mode": mode, "null_records": null_records})
        elif (
            any(
                record["exit"] == 0 or "null-callback" not in record["stdout"]
                for record in null_records
            )
            or null_records[0]["exit"] != null_records[1]["exit"]
        ):
            failures.append({"mode": mode, "null_records": null_records})
    if failures:
        raise RuntimeError(f"Null-generator disposition changed: {failures}")
    registered = int((out / "custom-assert.out").read_text().splitlines()[0].split()[1])
    probe.passed(
        "Widget parser comparison",
        registered_descriptors=registered,
        modes=len(modes),
    )


if __name__ == "__main__":
    main()
