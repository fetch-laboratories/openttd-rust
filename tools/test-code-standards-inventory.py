#!/usr/bin/env python3
"""Seed each pattern the code-standards inventory counts and check its count."""

import importlib.util
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
    fn typed(&self, flags: DoCommandFlags, a: u8, b: u8, c: u8, d: u8, e: u8, f: u8) {}
    fn qualified(flags: crate::command_type::DoCommandFlags, a: u8, b: u8, c: u8, d: u8, e: u8, f: u8, g: u8) {}
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
                ("command", "typed: 8 parameters"),
                ("command", "qualified: 8 parameters"),
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


FINISHED_BASE = """
pub(crate) fn cmd_build_thing(flags: DoCommandFlags, tile: TileIndex) -> CommandCost {
    slope_raw(tile);
    CommandCost::default()
}
"""
SEEDS = {
    "raw adapter call": "    is_steep_slope_raw(tile);\n",
    "export call": "    unsafe { openttd_rust_cargo_by_label(label) };\n",
}


class ResidueGateTests(unittest.TestCase):
    """#901: growth of a residue class in a finished file fails; elsewhere it passes."""

    def test_each_class_is_found_outside_tests_and_export_shims(self) -> None:
        source = """
static POOL: GameState<Vec<u32>> = GameState::new(Vec::new());
fn set_flags(tile: TileIndex, execute: bool) { helper_raw(tile); openttd_rust_x(); s.get_raw(); }
fn cmd_long(flags: u16, a: u8, b: u8, c: u8, d: u8, e: u8, f: u8, g: u8) {}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_shim(tile: u32) { typed_raw(tile); openttd_rust_y(); }
#[cfg(test)]
mod tests {
    fn probe(execute: bool) { openttd_rust_z(); helper_raw(1); }
}
"""
        found = {(s.kind, s.detail) for s in INVENTORY.residue(RUST, source.lstrip())}
        self.assertEqual(
            found,
            {
                ("scratch pool", "POOL"),
                ("execute flag", "set_flags"),
                ("raw adapter call", "helper_raw"),
                ("raw adapter call", "get_raw"),
                ("export call", "openttd_rust_x"),
                ("long command", "cmd_long"),
            },
        )

    def gate(self, finished: bool, base: str, head: str) -> int:
        def show(revision: str, path: str) -> str | None:
            if path == INVENTORY.FINISHED:
                return RUST + "\n" if finished else "# Nothing finished.\n"
            if path != RUST:
                return None
            return base if revision == "base" else head

        with (
            patch.object(INVENTORY, "show", side_effect=show),
            patch.object(INVENTORY, "renamed_from", return_value={}),
            patch.object(INVENTORY.subprocess, "check_output", return_value="base\n"),
            patch("builtins.print"),
        ):
            return INVENTORY.gate("origin/rust-migration", "head")

    def test_a_listed_file_missing_at_head_fails(self) -> None:
        def show(revision: str, path: str) -> str | None:
            return RUST + "\n" if path == INVENTORY.FINISHED else None

        with (
            patch.object(INVENTORY, "show", side_effect=show),
            patch.object(INVENTORY, "renamed_from", return_value={}),
            patch.object(INVENTORY.subprocess, "check_output", return_value="base\n"),
            patch("builtins.print"),
        ):
            self.assertEqual(INVENTORY.gate("origin/rust-migration", "head"), 1)

    def test_seeded_violation_fails_only_in_a_finished_file(self) -> None:
        for kind, line in SEEDS.items():
            seeded = FINISHED_BASE.replace(
                "    slope_raw(tile);\n", "    slope_raw(tile);\n" + line
            )
            with self.subTest(kind=kind):
                self.assertEqual(self.gate(True, FINISHED_BASE, seeded), 1)
                self.assertEqual(self.gate(False, FINISHED_BASE, seeded), 0)
        flagged = FINISHED_BASE.replace(
            "tile: TileIndex)", "tile: TileIndex, execute: bool)"
        )
        self.assertEqual(self.gate(True, FINISHED_BASE, flagged), 1)
        typed = "pub(crate) fn cmd_build(flags: DoCommandFlags) {}\n"
        long = typed.replace(
            "flags: DoCommandFlags",
            "flags: DoCommandFlags, a: u8, b: u8, c: u8, d: u8, e: u8, f: u8, g: u8",
        )
        self.assertEqual(self.gate(True, typed, long), 1)
        self.assertEqual(self.gate(False, typed, long), 0)

    def test_existing_and_renamed_occurrences_pass(self) -> None:
        self.assertEqual(self.gate(True, FINISHED_BASE, FINISHED_BASE), 0)
        moved = (
            FINISHED_BASE.replace("cmd_build_thing", "cmd_build_other") + "\n// moved\n"
        )
        self.assertEqual(self.gate(True, FINISHED_BASE, moved), 0)
        renamed = """
static SCRATCH: GameState<Vec<u8>> = GameState::new(Vec::new());
"""
        pool = renamed.replace("SCRATCH", "POOL")
        self.assertEqual(self.gate(True, pool, renamed), 0)
        self.assertEqual(self.gate(True, pool, pool + renamed), 1)


if __name__ == "__main__":
    unittest.main()
