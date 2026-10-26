#!/usr/bin/env python3
"""Check generated ABI probes against their Rust declarations."""

import os
import re
import runpy
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
GENERATOR = runpy.run_path(str(ROOT / "tools/generate-abi.py"))
LAYOUT = re.compile(r'Layout\("([^"]+)", \{(.*)\}\);')
ITEM = re.compile(r"\w+\((?:[^()]|\([^()]*\))*\)")


def layouts(fragments):
    """Map each generated layout name to its fragment and ordered items."""
    result = {}
    for path, contents in fragments.items():
        if path.startswith("abi-layout-"):
            for line in contents.splitlines():
                name, values = LAYOUT.fullmatch(line).groups()
                result[name] = (path, ITEM.findall(values))
    return result


class GeneratedAbiTests(unittest.TestCase):
    def test_each_layout_and_visitor_lists_exactly_its_declared_fields(self):
        fragments = GENERATOR["generate"]()
        generated = layouts(fragments)
        records = list(GENERATOR["declarations"]())
        self.assertEqual(set(generated), {name for name, *_ in records})
        for name, group, _, fields, width, _ in records:
            with self.subTest(name=name):
                path, items = generated[name]
                self.assertEqual(path, f"abi-layout-{group}.inc")
                self.assertEqual(items[:2], [f"sizeof({name})", f"alignof({name})"])
                offsets = [
                    re.fullmatch(rf"offsetof\({re.escape(name)}, (\w+)\)", item)
                    for item in items[2:]
                ]
                cpp = [cpp for _, cpp in fields]
                self.assertEqual(
                    [match and match[1] for match in offsets[: len(cpp)]], cpp
                )
                self.assertEqual(
                    items[2 + len(cpp) :],
                    [f"sizeof(decltype({name}::{dict(fields)[width]}))"]
                    if width
                    else [],
                )
                visitor = fragments.get(f"abi-visit-{name}.hpp", "")
                visits = re.findall(r'visit\("(\w+)", table\.(\w+)\);', visitor)
                self.assertIn(visits, ([], [(field, field) for field in cpp]))
                self.assertNotIn("reinterpret_cast", visitor)

    def test_generation_is_deterministic_and_build_directory_only(self):
        with tempfile.TemporaryDirectory() as directory:
            subprocess.run(
                [
                    "python3",
                    "-X",
                    "utf8=0",
                    str(ROOT / "tools/generate-abi.py"),
                    directory,
                ],
                env={**os.environ, "LC_ALL": "C", "PYTHONCOERCECLOCALE": "0"},
                check=True,
            )
            self.assertEqual(
                GENERATOR["generate"](),
                {path.name: path.read_text() for path in Path(directory).iterdir()},
            )
        self.assertFalse(list((ROOT / "src").rglob("abi-layout-*.inc")))

    def test_synthetic_tree_aliases_widths_and_callback_retirement(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            rust = root / "rust/openttd-kernels/src"
            headers = root / "src/rust"
            rust.mkdir(parents=True)
            headers.mkdir(parents=True)
            (rust / "unrelated.rs").write_text('"Other" ["new"] => Local {};')
            source = rust / "nested" / "probe.rs"
            source.parent.mkdir()
            source.write_text(
                "layouts! {\n"
                '    "Probe" ["new"] => crate::nested::Local {\n'
                "        call => callback, // Native name differs.\n"
                "        r#type => kind,\n"
                "        size,\n"
                "    } width size: u16;\n"
                "}\n"
            )
            (headers / "probe.h").write_text(
                "struct Probe { void (*callback)() noexcept; int r; };"
            )
            self.assertEqual(
                list(GENERATOR["declarations"](root)),
                [
                    (
                        "Probe",
                        "new",
                        "crate::nested::Local",
                        [("call", "callback"), ("r#type", "kind"), ("size", "size")],
                        "size",
                        "u16",
                    )
                ],
            )
            source.write_text(
                'layouts! { "Probe" ["new"] => Local { call => callback }; }'
            )
            fragments = GENERATOR["generate"](root)
            self.assertEqual(
                layouts(fragments),
                {
                    "Probe": (
                        "abi-layout-new.inc",
                        [
                            "sizeof(Probe)",
                            "alignof(Probe)",
                            "offsetof(Probe, callback)",
                        ],
                    )
                },
            )
            self.assertIn(
                'visit("callback", table.callback)', fragments["abi-visit-Probe.hpp"]
            )
            output = root / "build"
            GENERATOR["write_fragments"](output, fragments)
            source.write_text('layouts! { "Replacement" ["new"] => Local {}; }')
            with self.assertRaisesRegex(ValueError, "callback table absent"):
                GENERATOR["generate"](root)
            (headers / "probe.h").rename(headers / "probe.retired")
            GENERATOR["write_fragments"](output, GENERATOR["generate"](root))
            self.assertFalse((output / "abi-visit-Probe.hpp").exists())
            (headers / "probe.retired").rename(headers / "probe.h")
            source.write_text('layouts! { "Probe" ["new"] => Local {}; }')
            with self.assertRaisesRegex(ValueError, "callbacks absent"):
                GENERATOR["generate"](root)


if __name__ == "__main__":
    unittest.main()
