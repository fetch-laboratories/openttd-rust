#!/usr/bin/env python3
"""Seed each pattern the code-standards inventory counts and check its count."""

import importlib.util
import json
import subprocess
import tempfile
import unittest
from functools import partial
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
fn alloc(b: Box<u8>) { let p = Box::into_raw(b); drop(unsafe { std::boxed::Box::from_raw(p) }); }
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


SIGNAL = """
impl Signals {
    /// Temporary raw adapter of [`Self::update_signals_on_segment`].
    pub(crate) fn segment(self, tile: u32, side: u8, owner: u8) -> u8 {
        self.update_signals_on_segment(TileIndex(tile), DiagDirection(side), CompanyID(owner))
            .0
    }
    pub(crate) fn both(self, tile: u32, track: u8, owner: u8) {
        self.set_signals_on_both_dir(TileIndex(tile), Track(track), CompanyID(owner));
    }
    pub(crate) fn add_track(self, tile: u32, track: u8, owner: u8) {
        self.add_track_to_signal_buffer(TileIndex(tile), Track(track), CompanyID(owner));
    }
    pub(crate) fn add_side(self, tile: u32, side: u8, owner: u8) {
        self.add_side_to_signal_buffer(TileIndex(tile), DiagDirection(side), CompanyID(owner));
    }
    pub(crate) fn flush(self) {
        self.update_signals_in_buffer();
    }
    pub(crate) fn segment_raw(self, tile: u32) -> u8 {
        self.update(TileIndex(tile)).0
    }
    pub(crate) fn typed(self, tile: TileIndex) -> u8 {
        self.signals().segment(tile)
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_both(tile: u32) {
    both(TileIndex(tile))
}
"""
FLAGS = """
pub struct AirportFTAClassFlags(u8);
impl AirportFTAClassFlags {
    pub const fn from_flag(bit: AirportFTAClassFlag) -> Self {
        Self(1 << bit.0)
    }
}
pub struct Shell {
    pub flags: u8,
    pub fta: AirportFTAClassFlags,
}
fn fta_flags(fta: *const c_void) -> u8 {
    0
}
fn fta_flags_test(fta: *const c_void, flag: AirportFTAClassFlag) -> bool {
    fta_flags(fta) & (1 << flag.0) != 0
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_fta_flags(fta: *const c_void) -> u8 {
    0
}
"""
SERVICES = """
pub struct Services {
    pub once: extern "C" fn(u32) -> u8,
    pub twice: unsafe extern "C" fn(),
    pub tested: extern "C" fn(),
    pub shared: extern "C" fn(),
    pub size: u32,
}
pub struct Other {
    pub shared: extern "C" fn(),
}
fn work(s: &Services) {
    (s.once)(1);
    unsafe { (s.twice)() };
    unsafe { (s.twice)() };
    (s.shared)();
}
#[cfg(test)]
mod tests {
    fn probe(s: &Services) {
        (s.tested)();
        (s.once)(2);
    }
}
"""


class CandidateTests(unittest.TestCase):
    """Candidate rows, outside residue and the gate."""

    def rows(self, source: str) -> set[tuple[str, str]]:
        rust = {RUST: source.lstrip()}
        return {
            (site.kind, site.detail)
            for site in INVENTORY.candidate_rows(rust, {RUST}, set())
        }

    def test_unsuffixed_adapters_are_listed_and_raw_ones_are_not(self) -> None:
        self.assertEqual(
            self.rows(SIGNAL),
            {
                ("unsuffixed adapter", name)
                for name in ("segment", "both", "add_track", "add_side", "flush")
            },
        )

    def test_a_same_name_alias_of_another_function_is_listed(self) -> None:
        aliases = (
            "fn foo(x: u32) -> u32 { other::foo(x) }\n"
            "fn bar(w: World) { w.bar() }\n"
            "fn rec(x: u32) -> u32 { rec(x) }\n"
            "impl A { fn get(self) { self.get() } fn put(self) { Self::put(self) } }\n"
        )
        self.assertEqual(
            self.rows(aliases),
            {("unsuffixed adapter", "foo"), ("unsuffixed adapter", "bar")},
        )

    def test_flag_bits_and_integer_flag_sets(self) -> None:
        self.assertEqual(
            self.rows(FLAGS),
            {
                ("integer flag set", "flags"),
                ("integer flag set", "fta_flags"),
                ("hand-built flag bit", "1 << flag.0"),
            },
        )

    def test_a_service_with_one_non_test_call_site_is_listed(self) -> None:
        rows = self.rows(SERVICES)
        self.assertEqual(
            rows,
            {
                ("one-call service", 'once (extern "C" fn(u32) -> u8)'),
                ("one-call service", 'shared (extern "C" fn(); declared in 2 tables)'),
            },
        )

    def test_candidates_enter_no_residue_total_or_gate(self) -> None:
        kinds = {"one-call service", "unsuffixed adapter", "hand-built flag bit"}
        kinds.add("integer flag set")
        self.assertFalse(kinds & set(INVENTORY.RESIDUE))
        for source in (SIGNAL, FLAGS, SERVICES):
            residue = {site.kind for site in INVENTORY.residue(RUST, source.lstrip())}
            self.assertFalse(kinds & residue)

    def test_rows_print_before_the_comment_pairing(self) -> None:
        printed: list[str] = []

        def pairs(_head: str, _tests: object = None) -> list[object]:
            raise TimeoutError("yielded")

        with (
            patch.object(INVENTORY, "archive", return_value={RUST: SIGNAL.encode()}),
            patch.object(INVENTORY, "comment_pairs", side_effect=pairs),
            patch("sys.argv", ["code-standards-inventory.py", RUST]),
            patch("builtins.print", side_effect=lambda *a, **_: printed.append(*a)),
            self.assertRaises(TimeoutError),
        ):
            INVENTORY.main()
        self.assertIn(f"{RUST}:8: unsuffixed adapter: both", printed)


class AssertionTests(unittest.TestCase):
    """#892's sites: runtime assertion macros and const assertions outside tests."""

    def sites(self, source: str, path: str = RUST) -> list[tuple[int, str, str]]:
        return [
            (site.line, site.kind, site.detail)
            for site in INVENTORY.assertion_sites(path, source)
        ]

    def test_each_name_and_delimiter_counts_in_its_class(self) -> None:
        names = (
            "debug_assert",
            "debug_assert_eq",
            "debug_assert_ne",
            "assert",
            "assert_eq",
            "assert_ne",
            "unreachable",
        )
        calls = [
            f"    {name}!{open_}x{close};"
            for name in names
            for open_, close in ("()", "[]", "{}")
        ]
        source = "fn f() {\n" + "\n".join(calls) + "\n}\n"
        found = INVENTORY.assertion_sites(RUST, source)
        self.assertEqual(len(found), 21)
        self.assertEqual({s.kind for s in found}, {"assertion macro"})
        self.assertEqual(
            [INVENTORY.assertion_class(s) for s in found],
            ["debug_assert"] * 9 + ["assert"] * 9 + ["unreachable"] * 3,
        )
        line = INVENTORY.assertion_line("head", found)
        self.assertIn("21 runtime sites in 1 files (9 debug_assert, 9 assert", line)

    def test_comments_literals_and_test_items_are_not_sites(self) -> None:
        source = """
// assert!(a);
/* assert!(b); /* nested */ unreachable!() */
fn f() -> &'static str { "assert!(c)"; r#"assert!(d)"#; "x" }
#[cfg(test)]
mod checks { fn g() { assert!(e); } }
#[test]
fn probe() { assert_eq!(f, g); }
#[cfg(all(test, target_os = "linux"))]
mod linux { fn h() { unreachable!(); } }
"""
        self.assertEqual(self.sites(source), [])

    def test_test_files_are_not_scanned(self) -> None:
        crate = "rust/openttd-kernels/src"
        tree = {
            f"{crate}/lib.rs": "#[cfg(test)]\nmod stubs;\nmod live;\n",
            f"{crate}/stubs.rs": "fn f() { assert!(a); }\nmod nested;\n",
            f"{crate}/stubs/nested.rs": "fn f() { assert!(a); }\n",
            f"{crate}/live.rs": "#![cfg(test)]\nfn f() { assert!(a); }\n",
            f"{crate}/road/tests.rs": "fn f() { assert!(a); }\n",
        }
        tests = INVENTORY.test_modules(tree)
        self.assertEqual(
            {path for path in tree if path in tests},
            set(tree) - {f"{crate}/lib.rs"},
        )

    def test_production_sites_beside_test_lookalikes_are_counted(self) -> None:
        source = """
fn quote() -> char { let c = '"'; assert!(c != 'x'); c }
macro_rules! check { ($e:expr) => { debug_assert!($e) }; }
#[cfg(any(test, feature = "x"))]
fn shared() { unreachable!() }
// Not #[test]: this is production code.
fn after_comment() { assert_ne!(a, b); }
"""
        self.assertEqual(
            self.sites(source),
            [
                (2, "assertion macro", "assert!"),
                (3, "assertion macro", "debug_assert!"),
                (5, "assertion macro", "unreachable!"),
                (7, "assertion macro", "assert_ne!"),
            ],
        )

    def test_lookalike_macros_are_not_sites(self) -> None:
        source = (
            "fn f() { if assert != x {} prefix_assert!(a); assert_matches!(a, b); }\n"
        )
        self.assertEqual(self.sites(source), [])

    def test_a_test_declaration_in_a_comment_or_string_is_not_one(self) -> None:
        for declaration in (
            "// #[cfg(test)] mod production;\n",
            'const NOTE: &str = "#[cfg(test)] mod production;";\n',
        ):
            with self.subTest(declaration=declaration):
                tree = {"src/lib.rs": declaration, "src/production.rs": "fn f() {}"}
                self.assertNotIn("src/production.rs", INVENTORY.test_modules(tree))
                self.assertEqual(
                    self.sites("fn f() { assert!(a); }\n", "src/production.rs"),
                    [(1, "assertion macro", "assert!")],
                )

    def test_modules_resolve_inside_inline_modules_and_by_path(self) -> None:
        tree = {
            "src/lib.rs": (
                "mod checks;\n#[cfg(test)]\nmod outer { mod checks; }\n"
                '#[cfg(test)]\n#[path = "fixtures/table.rs"]\nmod table;\n'
            ),
            "src/checks.rs": "fn f() { assert!(a); }\n",
            "src/outer/checks.rs": "fn f() { assert!(a); }\n",
            "src/fixtures/table.rs": "fn f() { assert!(a); }\n",
        }
        tests = INVENTORY.test_modules(tree)
        self.assertNotIn("src/checks.rs", tests)
        self.assertIn("src/outer/checks.rs", tests)
        self.assertIn("src/fixtures/table.rs", tests)
        relative = {
            "src/lib.rs": '#[cfg(test)]\nmod outer { #[path = "../checks.rs"] mod checks; }\n',
            "src/checks.rs": "fn f() { assert!(a); }\n",
        }
        self.assertIn("src/checks.rs", INVENTORY.test_modules(relative))

    def test_a_unicode_identifier_is_one_word(self) -> None:
        source = 'fn f() { \u0100assert!(a); let s = "\u00e9"; assert!(b); }\n'
        self.assertEqual(self.sites(source), [(1, "assertion macro", "assert!")])
        later = "const \u00c9: () = { assert!(A) };\n"
        self.assertEqual(self.sites(later), [(1, "const assertion", "assert!")])

    def test_parsed_bounds_decide_a_site_in_ordinary_code(self) -> None:
        source = """
const X: S<{1}> = { assert!(true); S };
const _: () = { const fn checked(x: bool) { assert!(x); } };
"""
        self.assertEqual(
            self.sites(source),
            [(2, "const assertion", "assert!"), (3, "assertion macro", "assert!")],
        )

    def test_const_initialisers_hold_const_assertions(self) -> None:
        source = """
const _: () = assert!(A < B);
const LIMIT: [u8; 2] = { assert!(C); [0; 2] };
macro_rules! names { ($($n:expr),*) => { const _: () = { $(assert!($n < 4);)* }; }; }
const fn checked(a: u8) -> u8 { assert!(a < 4); a }
fn generic<const N: usize>() { assert!(N > 0); }
trait Limited { const MAX: u8; fn f() { debug_assert!(true); } }
"""
        self.assertEqual(
            self.sites(source),
            [
                (2, "const assertion", "assert!"),
                (3, "const assertion", "assert!"),
                (4, "const assertion", "assert!"),
                (5, "assertion macro", "assert!"),
                (6, "assertion macro", "assert!"),
                (7, "assertion macro", "debug_assert!"),
            ],
        )


class TestCodeTests(unittest.TestCase):
    def test_test_files_pair_with_no_original_body(self) -> None:
        files = {RUST: PORT, "src/tile_cmd.cpp": ORIGINAL}
        with (
            patch.object(INVENTORY, "archive", return_value=files),
            patch.object(INVENTORY, "baseline_commit", return_value="original"),
        ):
            self.assertEqual(len(INVENTORY.comment_pairs("head")), 1)
            self.assertEqual(INVENTORY.comment_pairs("head", {RUST}), [])

    def test_compare_finds_renames_across_the_rust_tree(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            run = partial(subprocess.run, cwd=root, check=True, capture_output=True)
            run(["git", "init", "--quiet"])
            source = root / "rust/demo/src/lib.rs"
            source.parent.mkdir(parents=True)
            source.write_text("fn f() { assert!(a); }\n")
            commit = ["git", "-c", "user.name=t", "-c", "user.email=t@t", "commit"]
            run(["git", "add", "-A"])
            run([*commit, "--quiet", "-m", "Add: lib"])
            run(["git", "mv", "rust/demo/src/lib.rs", "rust/demo/src/t\u00e9st.rs"])
            (root / "rust/demo/src/new.rs").write_text("fn g() {}\n")
            run(["git", "add", "-A"])
            run([*commit, "--quiet", "-m", "Move: test"])
            with patch.object(INVENTORY, "ROOT", root):
                self.assertEqual(INVENTORY.renamed_from("HEAD~1", "HEAD"), {})
                renames = INVENTORY.renamed_from("HEAD~1", "HEAD", "rust")
            self.assertEqual(
                renames, {"rust/demo/src/t\u00e9st.rs": "rust/demo/src/lib.rs"}
            )

    def test_a_comment_naming_a_test_attribute_is_not_one(self) -> None:
        source = b"// Unlike #[test] helpers:\nfn live(a: u8) {}\n#[cfg(all(test, unix))]\nfn probe() {}\n"
        names = [f.name for f in INVENTORY.functions(RUST, source, "rust", tests=False)]
        self.assertEqual(names, ["live"])


def function(name: str, scope: str, line: int, comments: int) -> object:
    return INVENTORY.Function(RUST, name, line, line, line + 3, comments, "", [], scope)


class CompareTests(unittest.TestCase):
    """--compare: each row at the merge base and the head, and what is new."""

    BASE = """
fn cmd_build(tile: u32) { old_raw(tile); }
impl A { fn update(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8, g: u8, h: u8) {} }
impl B { fn update(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8, g: u8, h: u8) {} }
"""

    def compare(
        self,
        trees: dict[str, dict[str, str]],
        files: list[str] | None = None,
        renames: dict[str, str] | None = None,
        pairs: dict[str, list[tuple[object, object]]] | None = None,
    ) -> list[str]:
        """Run --compare between revisions "base" and "head" and return its lines."""
        printed: list[str] = []

        def show(revision: str, path: str) -> str | None:
            if path == INVENTORY.FINISHED:
                return "".join(f"{p}\n" for p in trees.get(f"{revision}-list", {}))
            return trees[revision].get(path)

        def archive(revision: str, *_paths: str) -> dict[str, bytes]:
            return {p: t.encode() for p, t in trees[revision].items()}

        with (
            patch.object(INVENTORY, "show", side_effect=show),
            patch.object(INVENTORY, "archive", side_effect=archive),
            patch.object(INVENTORY, "renamed_from", return_value=renames or {}),
            patch.object(INVENTORY.subprocess, "check_output", return_value="base\n"),
            patch.object(
                INVENTORY,
                "comment_pairs",
                side_effect=lambda revision, _tests: (pairs or {}).get(revision, []),
            ),
            patch("builtins.print", side_effect=lambda *a, **_: printed.append(*a)),
        ):
            self.assertEqual(INVENTORY.compare("origin", "head", files or []), 0)
        return "\n".join(printed).splitlines()

    def changed(self, lines: list[str]) -> list[str]:
        start = next(i for i, line in enumerate(lines) if "new or grown" in line)
        return lines[start + 1 :]

    def test_table_counts_both_sides_and_lists_new_rows(self) -> None:
        head = self.BASE.replace("old_raw(tile);", "old_raw(tile); assert!(t);")
        head += "const LIMIT: u8 = 4;\n"
        lines = self.compare({"base": {RUST: self.BASE}, "head": {RUST: head}}, [RUST])
        self.assertIn("| bare integer | 1 | 1 |", lines)
        self.assertIn("| raw adapter call | 1 | 1 |", lines)
        self.assertIn("| long helper | 2 | 2 |", lines)
        self.assertIn("| constant | 0 | 1 |", lines)
        self.assertIn("| assertion macro | 0 | 1 |", lines)
        self.assertNotIn("| wrapper | 0 | 0 |", lines)
        self.assertEqual(
            self.changed(lines),
            [f"{RUST}:2: assertion macro: assert!", f"{RUST}:5: constant: LIMIT"],
        )

    def test_a_replaced_occurrence_is_new_and_a_moved_one_is_not(self) -> None:
        replaced = self.BASE.replace("old_raw", "new_raw")
        lines = self.compare(
            {"base": {RUST: self.BASE}, "head": {RUST: replaced}}, [RUST]
        )
        self.assertIn("| raw adapter call | 1 | 1 |", lines)
        self.assertEqual(self.changed(lines), [f"{RUST}:2: raw adapter call: new_raw"])
        moved = "\n\n// Moved down.\n" + self.BASE
        lines = self.compare({"base": {RUST: self.BASE}, "head": {RUST: moved}}, [RUST])
        self.assertEqual(self.changed(lines), [])

    def test_only_the_method_that_grew_is_listed(self) -> None:
        grown = self.BASE.replace(
            "impl A { fn update(a: u8,", "impl A { fn update(z: u8, a: u8,"
        )
        lines = self.compare({"base": {RUST: self.BASE}, "head": {RUST: grown}}, [RUST])
        self.assertEqual(
            self.changed(lines),
            [f"{RUST}:3: long helper: update: 9 parameters (grown from 8)"],
        )
        missing = {
            "base": [(function("f", "impl A::f", 2, 0), function("F", "F", 1, 1))],
            "head": [
                (function("f", "impl B::f", 2, 0), function("F", "F", 1, 1)),
                (function("f", "impl A::f", 9, 0), function("F", "F", 1, 3)),
            ],
        }
        lines = self.compare(
            {"base": {RUST: self.BASE}, "head": {RUST: self.BASE}},
            [RUST],
            pairs=missing,
        )
        self.assertIn("| missing comment lines | 1 | 4 |", lines)
        self.assertEqual(
            self.changed(lines),
            [
                f"{RUST}:2: missing comment lines: 1 (f; original {RUST}:1 F)",
                f"{RUST}:9: missing comment lines: 3 (f; original {RUST}:1 F) (grown from 1)",
            ],
        )

    def test_finished_files_are_selected_by_change_and_paired_through_renames(
        self,
    ) -> None:
        crate = INVENTORY.RUST
        same, new, moved = f"{crate}/same.rs", f"{crate}/new.rs", f"{crate}/moved.rs"
        old = f"{crate}/old.rs"
        trees = {
            "base": {same: self.BASE, new: self.BASE, old: self.BASE},
            "head": {
                same: self.BASE,
                new: self.BASE,
                moved: self.BASE + "fn x_raw() {}\n",
            },
            "base-list": {same: ""},
            "head-list": {same: "", new: "", moved: ""},
        }
        lines = self.compare(trees, renames={moved: old})
        self.assertIn(f"- {moved} (at the base {old})", lines)
        self.assertIn(f"- {new}", lines)
        self.assertNotIn(f"- {same}", lines)
        self.assertEqual(self.changed(lines), [])
        lines = self.compare(trees, [same], renames={moved: old})
        self.assertEqual(
            [line for line in lines if line.startswith("- ")], [f"- {same}"]
        )

    def test_no_selected_file_prints_the_notice(self) -> None:
        trees = {"base": {RUST: self.BASE}, "head": {RUST: self.BASE}}
        trees["base-list"] = trees["head-list"] = {RUST: ""}
        lines = self.compare(trees)
        self.assertEqual(lines, ["No finished file changed between base and head."])


if __name__ == "__main__":
    unittest.main()
