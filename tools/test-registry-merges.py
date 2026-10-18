#!/usr/bin/env python3
"""Merge two fictitious ports off HEAD against the real registries (#280)."""

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

with tempfile.TemporaryDirectory() as directory:
    root = Path(directory)

    def git(*args):
        return subprocess.check_output(["git", *args], cwd=root, text=True).strip()

    # Detached housekeeping could still write while TemporaryDirectory removes
    # this short-lived clone. Configure it before fetch/checkout can start it.
    subprocess.run(
        [
            "git",
            "clone",
            "-q",
            "--shared",
            "--no-checkout",
            "-c",
            "maintenance.auto=false",
            "-c",
            "gc.auto=0",
            str(ROOT),
            str(root),
        ],
        check=True,
    )
    base = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True
    ).strip()
    git("checkout", "-q", "--detach", base)
    git("config", "user.name", "Registry merge regression")
    git("config", "user.email", "registry@example.invalid")
    original_names = set(re.findall(r'=> "([^"]+)"', (root / WITNESS).read_text()))
    assert original_names, "missing component witness blocks"
    expected = {}
    for component, issue in [("house", 256), ("landscape", 212)]:
        git("checkout", "-qb", component, base)
        marker = f"// Registry: {component} (#{issue}).\n"
        probe = f"{component}_probe"
        field = f"{component}_registry_probe"
        assert f"pub {field}:" not in (root / RUST).read_text(), field
        header = f"{component}_registry_probe"
        assert f'#include "rust/{header}.h"' not in (root / ABI).read_text(), header
        additions = {
            WITNESS: [
                f'    {probe} {{\n        {probe.upper()} = 0 => "{probe}.hit",\n    }}\n'
            ],
            ABI: [
                f'#include "rust/{header}.h"\n',
                f"\t\toffsetof(OpenTTDSharedServices, {field}),\n",
                f"\t{component.title()}Probe();\n",
            ],
            RUST: [
                f'    /// Fictitious merge probe.\n    pub {field}: extern "C" fn() -> bool,\n',
                f"        {field}: fixture_{field},\n",
            ],
            HEADER: [
                f"\tbool (*{field})() noexcept;\n",
                f"\t\t.{field} = Fixture{component.title()}Probe,\n",
            ],
            NATIVE: [f"\t\t.{field} = {component.title()}Probe,\n"],
            ADAPTER: [f'#include "{field}.h"\n'],
            LAYOUT: [f"                {field},\n"],
        }
        for name, entries in additions.items():
            path = root / name
            parts = path.read_text().split(marker)
            assert len(parts) == len(entries) + 1, (name, marker)
            path.write_text(
                parts[0]
                + "".join(
                    marker + entry + tail
                    for entry, tail in zip(entries, parts[1:], strict=True)
                )
            )
            expected.setdefault(name, []).extend(entries)
        git("commit", "-qam", f"Add: Fictitious {component} registry entries")
    tree = git("merge-tree", "--write-tree", "house", "landscape")
    for name, entries in expected.items():
        merged = git("show", f"{tree}:{name}")
        assert all(entry in merged for entry in entries), name
        assert "<<<<<<<" not in merged, name
    names = set(re.findall(r'=> "([^"]+)"', git("show", f"{tree}:{WITNESS}")))
    assert names == original_names | {"house_probe.hit", "landscape_probe.hit"}
    print(
        "git merge-tree --write-tree house landscape: clean; both ports and all witness names retained"
    )
