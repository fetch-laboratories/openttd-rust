#!/usr/bin/env python3
"""Check generated probes against the deleted lists, without keeping another inventory."""

import re
import runpy
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
# Last integrated tree before #284; Quick checks fetch full history.
BEFORE = "83bd72774e5cc62157f8415ee8c0e9d82fa4eff9"
GENERATOR = runpy.run_path(str(ROOT / "tools/generate-abi.py"))


def original(path):
    return subprocess.check_output(
        ["git", "show", f"{BEFORE}:{path}"], cwd=ROOT, text=True
    )


def legacy_lists():
    result = {}
    paths = ["tools/migration/windows-abi.cpp"]
    paths += [
        str(path.relative_to(ROOT)) for path in (ROOT / "src/tests").glob("*.cpp")
    ]
    for path in paths:
        # A newly added test has no inventory in the pre-conversion tree.
        if path == "src/tests/service_tables.cpp":
            continue
        source = original(path)
        aliases = dict(re.findall(r"using\s+(\w+)\s*=\s*([^;]+);", source))
        for match in re.finditer(
            r'(?:Layout\(\d+,\s*"[^"]+",|AbiLayoutMismatch\(\d+,)\s*\{(.*?)\}',
            source,
            re.S,
        ):
            values = re.sub(r"\s+", "", re.sub(r"//[^\n]*", "", match[1])).rstrip(",")
            name = re.search(r"sizeof\(([^)]+)\)", values)[1]
            name = aliases.get(name, name)
            for alias, actual in aliases.items():
                values = re.sub(r"\b" + alias + r"\b", actual, values)
            result.setdefault(name, [])
            if values not in result[name]:
                result[name].append(values)
    # The two train slices formerly repeated size/alignment; the new list is complete.
    for name, lists in result.items():
        result[name] = lists[0] + "".join(
            "," + value.split(",", 2)[2] for value in lists[1:]
        )
    return result


class GeneratedAbiTests(unittest.TestCase):
    def test_every_deleted_probe_and_native_only_list_is_preserved(self):
        generated = GENERATOR["generate"]()
        actual = {}
        for path, contents in generated.items():
            if not path.endswith(".inc"):
                continue
            for name, values in re.findall(
                r'Layout\("([^"]+)", \{([^\n]+)\}\);', contents
            ):
                self.assertNotIn(name, actual)
                actual[name] = re.sub(r"\s+", "", values)
        self.assertEqual(legacy_lists(), actual)
        for name, group, *_ in GENERATOR["declarations"]():
            if group in {
                "industry",
                "company_economy",
                "engine",
                "fleet",
                "station_service",
            }:
                self.assertIn(f'Layout("{name}"', generated[f"abi-layout-{group}.inc"])

    def test_generation_is_deterministic_and_build_directory_only(self):
        with tempfile.TemporaryDirectory() as directory:
            subprocess.run(
                ["python3", str(ROOT / "tools/generate-abi.py"), directory], check=True
            )
            self.assertEqual(
                GENERATOR["generate"](),
                {path.name: path.read_text() for path in Path(directory).iterdir()},
            )
        self.assertFalse(list((ROOT / "src").rglob("abi-layout-*.inc")))

    def test_aliases_width_metadata_and_private_layout_modules(self):
        declarations = list(GENERATOR["declarations"]())
        self.assertTrue(any(width for *_, width, _ in declarations))
        self.assertTrue(
            any(
                rust != cpp for *_, fields, _, _ in declarations for rust, cpp in fields
            )
        )
        for module in GENERATOR["SOURCES"]:
            source = (ROOT / f"rust/openttd-kernels/src/{module}.rs").read_text()
            self.assertRegex(source, GENERATOR["DECLARATION"])
        visitors = "".join(
            value
            for key, value in GENERATOR["generate"]().items()
            if key.startswith("abi-visit-")
        )
        for name, _, _, fields, _, _ in declarations:
            if f"const {name} &table" in visitors:
                for _, cpp in fields:
                    self.assertIn(f'visit("{cpp}", table.{cpp});', visitors)
        self.assertNotIn("reinterpret_cast", visitors)


if __name__ == "__main__":
    unittest.main()
