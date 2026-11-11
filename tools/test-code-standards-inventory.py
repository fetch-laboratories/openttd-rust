#!/usr/bin/env python3
"""Seed each pattern the code-standards inventory counts and check its count."""

import contextlib
import importlib.util
import io
import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location(
    "inventory", Path(__file__).with_name("code-standards-inventory.py")
)
assert SPEC and SPEC.loader
INVENTORY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(INVENTORY)
RUST = f"{INVENTORY.RUST}/tile_cmd.rs"

ORIGINAL = b"""
CommandCost CmdBuildThing(DoCommandFlags flags, TileIndex tile)
{
	/* Check the tile first. */
	if (!IsValidTile(tile)) return CMD_ERROR;
	/* Two lines of
	 * explanation. */
	return CommandCost();
}
"""
PORT = b"""
pub(crate) fn cmd_build_thing(tile: TileIndex) -> u8 {
    // Check the tile first.
    if !valid(tile) {
        return 1;
    }
    0
}
#[cfg(test)]
mod tests {
    fn cmd_build_thing() {}
}
"""


class CommentTests(unittest.TestCase):
    def test_comment_lines_and_test_functions(self) -> None:
        rust = INVENTORY.functions(RUST, PORT, "rust", tests=False)
        self.assertEqual([(f.name, f.comments) for f in rust], [("cmd_build_thing", 1)])
        original = INVENTORY.functions("src/tile_cmd.cpp", ORIGINAL, "cpp")
        self.assertEqual(
            [(f.name, f.comments) for f in original], [("CmdBuildThing", 3)]
        )

    def test_normalized_name_in_its_module_pairs_once(self) -> None:
        rust = INVENTORY.functions(RUST, PORT, "rust", tests=False)
        original = INVENTORY.functions("src/tile_cmd.cpp", ORIGINAL, "cpp")
        pairs = INVENTORY.Matcher(rust, original, []).pairs()
        self.assertEqual(
            [(r.name, o.name, o.comments - r.comments) for r, o in pairs],
            [("cmd_build_thing", "CmdBuildThing", 2)],
        )

    def test_facade_export_pairs_the_largest_callee(self) -> None:
        facade = b"CommandCost CmdBuildThing(DoCommandFlags f, TileIndex t)\n{\n\treturn openttd_rust_build_thing(t);\n}\n"
        port = (
            b'pub extern "C" fn openttd_rust_build_thing(t: u32) -> u8 {\n    build(t)\n}\n'
            b"fn build(t: u32) -> u8 {\n    let a = 1;\n    let b = 2;\n    a + b\n}\n"
        )
        rust = INVENTORY.functions(f"{INVENTORY.RUST}/other.rs", port, "rust")
        original = INVENTORY.functions("src/tile_cmd.cpp", ORIGINAL, "cpp")
        facades = INVENTORY.functions("src/tile_cmd.cpp", facade, "cpp")
        pairs = INVENTORY.Matcher(rust, original, facades).pairs()
        self.assertEqual(
            [(r.name, o.name) for r, o in pairs], [("build", "CmdBuildThing")]
        )


class BaselineTests(unittest.TestCase):
    def test_the_revision_manifest_pins_the_original_not_the_working_tree(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = root / "migration/baseline.json"
            manifest.parent.mkdir()
            manifest.write_text(json.dumps({"commit": "a" * 40}))
            for command in (
                ["init", "-q"],
                ["add", "migration/baseline.json"],
                ["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-qm", "base"],
            ):
                subprocess.run(["git", *command], cwd=root, check=True)
            manifest.write_text(json.dumps({"commit": "b" * 40}))  # Dirty, uncommitted.
            with patch.object(INVENTORY, "ROOT", root):
                self.assertEqual(INVENTORY.baseline_commit("HEAD"), "a" * 40)


class SiteTests(unittest.TestCase):
    SOURCE = """
/// tile: u32 in a comment is not a site.
pub fn place(tile: u32, owner: u8, cargo: u32, vehicle: u8, count: u32) {}
struct Record { station: u16, town: u16, name: &'static str }
#[repr(transparent)]
pub struct TileIndex(pub u32);
pub const MAX_TILES: u32 = 4;
const TEXT: &str = "town: u16";
fn f() {
    c.error(0xFFFF);
    company::price(3);
    if stop_type == 1 && tracks == 0x30 {}
}
"""

    def test_each_kind_counts_its_seeded_sites(self) -> None:
        sites = INVENTORY.type_sites(RUST, self.SOURCE)
        found: dict[str, list[str]] = {}
        for site in sites:
            found.setdefault(site.kind, []).append(site.detail)
        self.assertEqual(
            found["bare integer"],
            [
                "tile: u32 (TileIndex)",
                "owner: u8 (CompanyID/Owner)",
                "station: u16 (StationID)",
                "town: u16 (TownID)",
            ],
        )
        self.assertEqual(found["wrapper"], ["TileIndex(u32)"])
        self.assertEqual(found["constant"], ["MAX_TILES", "TEXT"])
        self.assertEqual(len(found["literal"]), 4)
        self.assertEqual([s.line for s in sites if s.kind == "wrapper"], [5])

    def test_reviewed_homonyms_are_not_sites(self) -> None:
        for name, kind in (("cargo", "u32"), ("vehicle", "u8"), ("expenses", "i64")):
            self.assertTrue(INVENTORY.homonym(name, kind))
        self.assertFalse(INVENTORY.homonym("cargo", "u8"))


class LongFunctionTests(unittest.TestCase):
    SOURCE = """
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_a(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8, g: u8, h: u8) {}
pub unsafe extern "C" fn openttd_rust_b(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8, g: u8, h: u8) {}
impl X {
    fn build(&mut self, tile: u32, flags: u16, a: u8, b: u8, c: u8, d: u8, e: u8) {}
    fn walk(&self, a: u8, b: u8, c: u8, d: u8, e: u8, f: u8, flags: u32) {}
    fn seven(&self, a: u8, b: u8, c: u8, d: u8, e: u8, f: u8) {}
}
#[cfg(test)]
mod tests {
    fn stub(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8, g: u8, h: u8) {}
}
"""

    def test_each_class_counts_its_seeded_functions(self) -> None:
        found = [
            (site.kind, site.detail)
            for site in INVENTORY.long_functions(RUST, self.SOURCE)
        ]
        self.assertEqual(
            found,
            [
                ("ffi export", "openttd_rust_a: 8 parameters"),
                ("ffi export", "openttd_rust_b: 8 parameters"),
                ("command", "build: 8 parameters"),
                ("helper", "walk: 8 parameters"),
            ],
        )

    def test_cfg_test_module_files_are_test_code(self) -> None:
        rust = {
            "src/lib.rs": "#[cfg(test)]\nmod test_tables;\nmod live;",
            "src/a/mod.rs": "#[cfg(test)]\npub(crate) mod stubs;",
        }
        self.assertEqual(
            INVENTORY.test_modules(rust),
            {
                "src/test_tables.rs",
                "src/test_tables/mod.rs",
                "src/a/stubs.rs",
                "src/a/stubs/mod.rs",
            },
        )


class ResidueTests(unittest.TestCase):
    SOURCE = """
static IDLE: GameState<Vec<u32>> = GameState::new(Vec::new());
pub(crate) fn refit(execute: bool, label: u32) -> u8 {
    let cargo = openttd_rust_cargo_by_label(label);
    crate::slope_func::is_steep_slope_raw(cargo)
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_refit(execute: bool) -> u8 {
    openttd_rust_cargo_by_label(is_steep_slope_raw(0))
}
#[cfg(test)]
mod tests {
    fn stub(execute: bool) { openttd_rust_refit(execute); }
}
"""

    def test_each_class_outside_export_shims_and_tests(self) -> None:
        found = [
            (s.line, s.kind, s.detail)
            for s in INVENTORY.residue_sites(RUST, self.SOURCE)
        ]
        self.assertEqual(
            sorted(found),
            [
                (2, "scratch pool", "IDLE"),
                (3, "execute flag", "refit"),
                (4, "export call", "openttd_rust_cargo_by_label"),
                (5, "raw call", "is_steep_slope_raw"),
            ],
        )

    def gate(self, edit: str) -> tuple[int, str]:
        """Commit a finished and an unfinished file, apply edit to both, run the gate."""
        clean = "pub(crate) fn refit(label: u32) -> u8 {\n    get(label)\n}\n"
        clean += "fn keep() -> u8 {\n    is_steep_slope_raw(0)\n}\n"
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "migration").mkdir()
            (root / "migration/finished-files.txt").write_text("# Finished\ndone.rs\n")
            for name in ("done.rs", "open.rs"):
                (root / name).write_text(clean)

            def git(*command: str) -> str:
                return subprocess.run(
                    ["git", "-c", "user.name=t", "-c", "user.email=t@t", *command],
                    cwd=root,
                    check=True,
                    capture_output=True,
                    text=True,
                ).stdout.strip()

            git("init", "-q")
            git("add", ".")
            git("commit", "-qm", "base")
            base = git("rev-parse", "HEAD")
            for name in ("done.rs", "open.rs"):
                (root / name).write_text(edit)
            git("commit", "-qam", "head")
            output = io.StringIO()
            with patch.object(INVENTORY, "ROOT", root):
                with contextlib.redirect_stdout(output):
                    status = INVENTORY.gate(base)
            return status, output.getvalue()

    def test_seeded_violation_fails_only_in_a_finished_file(self) -> None:
        seeded = "pub(crate) fn refit(label: u32) -> u8 {\n    get_raw(label)\n}\n"
        seeded += "fn keep() -> u8 {\n    is_steep_slope_raw(0)\n}\n"
        status, output = self.gate(seeded)
        self.assertEqual(status, 1)
        self.assertIn("done.rs:2: new raw call in a finished file: get_raw", output)
        self.assertNotIn("open.rs", output)
        self.assertNotIn("is_steep_slope_raw", output)

    def test_renamed_or_moved_occurrence_passes(self) -> None:
        renamed = "fn kept() -> u8 {\n    is_steep_slope_raw(0)\n}\n"
        renamed += "pub(crate) fn refit_cargo(label: u32) -> u8 {\n    get(label)\n}\n"
        self.assertEqual(self.gate(renamed)[0], 0)
        callee = "fn keep() -> u8 {\n    is_flat_slope_raw(0)\n}\n"
        self.assertEqual(self.gate(callee)[0], 0)


if __name__ == "__main__":
    unittest.main()
