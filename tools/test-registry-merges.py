#!/usr/bin/env python3
"""Merge two fictitious ports against the real registries (#280), in a temp repo."""

import re
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WITNESS = "rust/openttd-kernels/src/witness.rs"
ABI = "tools/migration/windows-abi.cpp"
RUST = "rust/openttd-kernels/src/services.rs"
HEADER = "src/rust/services_ffi.h"
NATIVE = "src/rust/services.cpp"
ADAPTER = "src/rust/company_adapter.cpp"
LAYOUT = "rust/openttd-kernels/src/abi.rs"
FILES = (WITNESS, ABI, RUST, HEADER, NATIVE, ADAPTER, LAYOUT)

with tempfile.TemporaryDirectory() as directory:
    root = Path(directory)

    def git(*args):
        return subprocess.check_output(["git", *args], cwd=root, text=True).strip()

    git("init", "-q")
    git("config", "user.name", "Registry merge regression")
    git("config", "user.email", "registry@example.invalid")
    for name in FILES:
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text((ROOT / name).read_text())
    git("add", ".")
    git("commit", "-qm", "Add: Registry fixture")
    base = git("rev-parse", "HEAD")
    original_names = set(re.findall(r'=> "([^"]+)"', (root / WITNESS).read_text()))
    assert original_names, "missing component witness blocks"
    expected = {name: [] for name in FILES}
    for component, block, field, include, probe, adapter in [
        ("harbor", "rail", "has_road_type", "history", "History", "presentation"),
        ("tram", "train", "tram_type", "train", "Trees", "vehicle"),
    ]:
        git("checkout", "-qb", component, base)
        additions = {
            WITNESS: [
                (
                    f"    {block} {{",
                    f"    {component} {{\n"
                    f'        {component.upper()}_PROBE = 0 => "{component}.probe",\n'
                    "    }\n",
                )
            ],
            ABI: [
                (
                    f'#include "rust/{include}_ffi.h"',
                    f'#include "rust/{component}_ffi.h"\n',
                ),
                (f"\t{probe}();", f"\t{component.title()}Probe();\n"),
                (
                    f"\t\toffsetof(OpenTTDSharedServices, {field})",
                    f"\t\toffsetof(OpenTTDSharedServices, {component}_probe),\n",
                ),
            ],
            HEADER: [
                (
                    f"\t{'bool' if field == 'has_road_type' else 'RoadType'} (*{field})",
                    f"\tbool (*{component}_probe)() noexcept;\n",
                )
            ],
            NATIVE: [
                (
                    f"\t\t.{field} =",
                    f"\t\t.{component}_probe = {component.title()}Probe,\n",
                )
            ],
            ADAPTER: [
                (
                    f'#include "{adapter}_services.h"',
                    f'#include "{component}_services.h"\n',
                )
            ],
            LAYOUT: [
                (f"                {field},", f"                {component}_probe,\n")
            ],
        }
        # Insert a Rust field before its existing documentation, without splitting it.
        source = (root / RUST).read_text()
        match = re.search(rf"(?:    ///[^\n]*\n)+    pub {field}:", source)
        assert match, field
        additions[RUST] = [
            (
                match[0],
                f'    /// Fictitious merge probe.\n    pub {component}_probe: extern "C" fn() -> bool,\n',
            )
        ]
        for name, edits in additions.items():
            path = root / name
            source = path.read_text()
            for anchor, addition in edits:
                assert source.count(anchor) == 1, (name, anchor)
                source = source.replace(anchor, addition + anchor)
                expected[name].append(addition)
            path.write_text(source)
        git("commit", "-qam", f"Add: Fictitious {component} registry entries")
    tree = git("merge-tree", "--write-tree", "harbor", "tram")
    for name in FILES:
        merged = git("show", f"{tree}:{name}")
        assert all(addition in merged for addition in expected[name]), name
        assert "<<<<<<<" not in merged, name
    names = set(re.findall(r'=> "([^"]+)"', git("show", f"{tree}:{WITNESS}")))
    assert names == original_names | {"harbor.probe", "tram.probe"}
    print(
        "git merge-tree --write-tree harbor tram: clean; both ports and all witness names retained"
    )
