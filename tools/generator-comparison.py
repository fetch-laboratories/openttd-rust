#!/usr/bin/env python3
"""Compare strgen/settingsgen outputs and malformed integer pragmas with the pinned original.

The generators parse integers through the candidate's Rust-routed StringConsumer.
The former integer_probe.cpp parser/builder/consumer observations now live in
Rust tests (#736 U; migration/evidence-replacements/736-u-vectors.json).
"""

import re
import subprocess
from collections.abc import Mapping

import probe

import migration

REFERENCE = probe.REFERENCE
TOOLS = probe.ROOT / ".local/build-tools-rust"
OUT = probe.ROOT / ".local/generator-comparison"


def compare_generators(env: Mapping[str, str]) -> tuple[int, int]:
    outputs: dict[str, dict[str, bytes]] = {}
    settings_dir = REFERENCE / "src/table/settings"
    settings_inputs = [
        str(settings_dir / name)
        for name in re.findall(
            r"\$\{CMAKE_CURRENT_SOURCE_DIR\}/(\w+\.ini)",
            (settings_dir / "CMakeLists.txt").read_text(),
        )
    ]
    lang = REFERENCE / "src/lang"
    tools = {
        "reference": (migration.REFERENCE_BUILD / "src").resolve(),
        "candidate": TOOLS / "src",
    }
    for name, tool_dir in tools.items():
        out = OUT / (name + "-generated")
        out.mkdir(exist_ok=True)
        for filename in ("strings.h", "settings.h", "english.lng", "french.lng"):
            (out / filename).unlink(missing_ok=True)
        strgen = str(tool_dir / "strgen/strgen")
        settingsgen = str(tool_dir / "settingsgen/settingsgen")
        invocations = [
            [strgen, "-s", str(lang), "-d", str(out)],
            [strgen, "-s", str(lang), "-d", str(out), str(lang / "english.txt")],
            [strgen, "-s", str(lang), "-d", str(out), str(lang / "french.txt")],
            [
                settingsgen,
                "-o",
                str(out / "settings.h"),
                "-b",
                str(REFERENCE / "src/table/settings.h.preamble"),
                "-a",
                str(REFERENCE / "src/table/settings.h.postamble"),
                *settings_inputs,
            ],
        ]
        # strgen writes tmp.xxx in its working directory and renames it into -d;
        # run it in out so the rename stays on one filesystem (.local may be linked).
        for index, command in enumerate(invocations):
            probe.compile(command, OUT / f"{name}-generator-{index}.log", env, cwd=out)
        outputs[name] = {
            filename: (out / filename).read_bytes()
            for filename in ("strings.h", "settings.h", "english.lng", "french.lng")
        }
    assert outputs["reference"] == outputs["candidate"], "Generator output mismatch"
    malformed = 0
    original = (lang / "english.txt").read_bytes()
    for index, pragma in enumerate(
        (
            b"##id 0x-",
            b"##id 999999999999999999999junk",
            b"##winlangid -0x80000001",
            b"##id 0x\x00\xff",
        )
    ):
        directory = OUT / "malformed" / str(index)
        directory.mkdir(parents=True, exist_ok=True)
        (directory / "english.txt").write_bytes(
            original.replace(b"##id 0x0000", pragma, 1)
        )
        results: list[tuple[int, bytes, bytes]] = []
        for _name, tool_dir in tools.items():
            command = [
                str(tool_dir / "strgen/strgen"),
                "-s",
                str(directory),
                "-d",
                str(directory),
            ]
            result = subprocess.run(
                command,
                cwd=directory,
                env=env,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            results.append((result.returncode, result.stdout, result.stderr))
        assert results[0] == results[1] and results[0][0] == 2, results
        malformed += 1
    return len(outputs["reference"]), malformed


def main() -> None:
    probe.output("generator-comparison")
    generated, malformed = compare_generators(probe.ENV)
    probe.passed(
        "Generator comparison",
        generated_outputs=generated,
        malformed_strgen_pragmas=malformed,
    )


if __name__ == "__main__":
    main()
