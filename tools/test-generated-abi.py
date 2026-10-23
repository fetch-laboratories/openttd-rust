#!/usr/bin/env python3
"""Check generated probes against the deleted lists, without keeping another inventory."""

import io
import os
import re
import runpy
import subprocess
import tarfile
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
# Last integrated tree before #284; Quick checks fetch full history.
BEFORE = "83bd72774e5cc62157f8415ee8c0e9d82fa4eff9"
CONVERSION = "df9a6c22da0462ef68a918debdc5e715a757291d"
GENERATOR = runpy.run_path(str(ROOT / "tools/generate-abi.py"))


def snapshot(revision):
    with tempfile.TemporaryDirectory() as directory:
        archive = subprocess.check_output(
            [
                "git",
                "archive",
                revision,
                "tools/generate-abi.py",
                "rust/openttd-kernels/src",
                "src/rust",
                "tools/migration",
            ],
            cwd=ROOT,
        )
        with tarfile.open(fileobj=io.BytesIO(archive)) as files:
            files.extractall(directory, filter="data")
        root = Path(directory)
        converter = runpy.run_path(str(root / "tools/generate-abi.py"))
        probes = (root / "tools/migration/windows-abi.cpp").read_text()
        probes += "\n".join(
            path.read_text()
            for path in (root / "tools/migration/windows-abi").glob("*.inc")
        )
        return converter["generate"](), probes


def original(path):
    return subprocess.check_output(
        ["git", "show", f"{BEFORE}:{path}"], cwd=ROOT, text=True
    )


def legacy_lists():
    result = {}
    paths = ["tools/migration/windows-abi.cpp"]
    paths += subprocess.check_output(
        ["git", "ls-tree", "-r", "--name-only", BEFORE, "src/tests"],
        cwd=ROOT,
        text=True,
    ).splitlines()
    for path in paths:
        if not path.endswith(".cpp"):
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
        # Freeze the conversion inputs, never today's evolving ABI. Current generation
        # is tested independently below, so later additions/retirements remain valid.
        generated, _ = snapshot(CONVERSION)
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

    def test_component_split_preserves_layouts_visitors_and_probe_bodies(self):
        split = "a42498e47f574c78cd1f0a96cfc61c901f397ef2"
        results = []
        for revision in (split + "^", split):
            fragments, probes = snapshot(revision)
            fragments = {
                name: sorted(text.splitlines())
                if name.startswith("abi-layout-")
                else text
                for name, text in fragments.items()
                if name.startswith(("abi-layout-", "abi-visit-"))
            }
            functions = dict(
                re.findall(
                    r"^(static [^\n]+\([^\n]*\))\n(\{.*?^\})", probes, re.M | re.S
                )
            )
            self.assertGreater(len(functions), 20)
            results.append((fragments, functions))
        self.assertEqual(results[0][0], results[1][0])
        for name, body in results[0][1].items():
            self.assertEqual(body, results[1][1][name], name)

    def test_generation_and_ground_current_layout_inventory(self):
        # These owners joined after the historical split. Require their current
        # module markers and complete field inventories, including data fields.
        inventory = {
            "tgp": {"OpenTTDTGPSettings"},
            "landscape_generation": {
                "OpenTTDTerrainTemplate",
                "OpenTTDFlatTileHeight",
                "OpenTTDGenerationLoop",
                "OpenTTDLandscapeGenerationSettings",
                "OpenTTDHeightmapImage",
                "OpenTTDHeightmapDecoder",
                "OpenTTDLandscapeGenerationServices",
            },
            "ground_vehicle": {
                "GroundVehicleCache",
                "OpenTTDGroundState",
                "OpenTTDGroundOwner",
                "OpenTTDGroundPowerInput",
                "OpenTTDGroundWeightInput",
                "OpenTTDGroundAccelerationInput",
                "OpenTTDGroundSpeedInput",
                "OpenTTDGroundPhysicsServices",
            },
        }
        records = {row[0]: row for row in GENERATOR["declarations"]()}
        current = {
            "OpenTTDGroundServices": "vehicle",
            "OpenTTDRoadSpeedRead": "road",
            "OpenTTDRoadLeaves": "road",
            "OpenTTDTrainServices": "train",
            "OpenTTDRoadCommandServices": "road_cmd",
            "OpenTTDVehiclePostTick": "vehicle/scheduler",
        }
        for module, names in inventory.items():
            source = (ROOT / f"rust/openttd-kernels/src/{module}.rs").read_text()
            self.assertEqual(source.count("layouts! {\n    fn abi_layout;"), 1)
            declared = set(re.findall(r'"([^"\n]+)" \["\w+"\] =>', source))
            self.assertEqual(declared, names)
            current.update(dict.fromkeys(names, module))
        for name, module in current.items():
            source = (ROOT / f"rust/openttd-kernels/src/{module}.rs").read_text()
            _, _, rust, fields, _, _ = records[name]
            structure = rust.rsplit("::", 1)[-1]
            body = re.search(
                rf"pub struct {structure}\s*\{{(.*?)^\}}", source, re.M | re.S
            )[1]
            self.assertEqual(
                {field for field, _ in fields},
                set(re.findall(r"^    pub (\w+):", body, re.M)),
                name,
            )

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

    def test_new_tables_aliases_and_callback_retirement(self):
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
                'layouts! { "Probe" ["new"] => Local { call => callback }; }'
            )
            (headers / "probe.h").write_text(
                "struct Probe { void (*callback)() noexcept; };"
            )
            fragments = GENERATOR["generate"](root)
            self.assertIn("offsetof(Probe, callback)", fragments["abi-layout-new.inc"])
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

    def test_aliases_width_metadata_and_private_layout_modules(self):
        declarations = list(GENERATOR["declarations"]())
        self.assertTrue(any(width for *_, width, _ in declarations))
        pairs = (pair for record in declarations for pair in record[3])
        self.assertTrue(any(rust != cpp for rust, cpp in pairs))
        modules = [
            path
            for path in (ROOT / "rust/openttd-kernels/src").rglob("*.rs")
            if "layouts! {" in path.read_text()
        ]
        self.assertGreater(len(modules), 10)
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
