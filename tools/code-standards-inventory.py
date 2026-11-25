#!/usr/bin/env python3
"""Code-standards inventory: the rows steering re-measures (#854).

At a revision (default HEAD) it counts, over rust/openttd-kernels/src:
- explanatory comment lines missing inside Rust bodies matched to the pinned
  original's bodies (per pair, original comment lines minus Rust ones, floored
  at zero);
- primitive-integer parameters and fields whose name names a concept with a
  named type;
- transparent wrapper types and named constants;
- literal sites that stand for a named value;
- Rust functions with more than seven parameters, as FFI exports, commands
  (taking the command's `flags`, as `u16` or `DoCommandFlags`) and helpers (#874), outside test code
  and `#[cfg(test)]` modules;
- over every Rust file under rust/, #892's assertion macros (`debug_assert*!`,
  `assert*!`, `unreachable!`) outside test code, comments and literals, as
  runtime sites (`assertion macro`) and sites in a `const` item's initialiser
  (`const assertion`), on a line of their own before the comment pairing.

With file paths it prints those files' residue instead of the totals, including
the finished-file residue classes (#901): calls to `*_raw` adapters and
`openttd_rust_*` exports outside export shims, Rust-internal `execute: bool`
parameters and `GameState<Vec<...>>` scratch pools. It also lists candidates
for the implementer and reviewer to judge, which enter no total and no gate:
service fields (`extern "C" fn` struct fields) whose only non-test call site in
the crate is in the given files (`one-call service`), functions that only
forward their parameters to another without the `_raw` suffix (`unsuffixed
adapter`), `1 << X.0` shifts outside a set type's `Self(...)` constructor
(`hand-built flag bit`), and functions and fields named `*flags*` of a
primitive integer type (`integer flag set`). Rows print as they are found,
before the slower comment pairing. `--gate BASE_REF` fails when
a file that migration/finished-files.txt lists gains one of those, or a command
over seven parameters, against the merge base; existing occurrences pass. The Rust
and the candidate C++ come from the revision's archive, the original from the
baseline commit that the revision's own migration/baseline.json pins. Counts are estimates for stocktakes,
not a gate: matching is by facade, export and name, as the 2026-11-06 overhaul
measured them. `--compare BASE_REF` prints, for the files given or else the
finished files that the head lists and the base lacks or changed, a table of
every row at the merge base and at the head, then each row new or grown at the
head; it gates nothing. It also lists, whether new or not, each file-level
`#![allow(...)]` naming `unsafe_code` in a file that the head lists as finished
and the base does not: a pass scopes `unsafe_code` to the items that need it.
`--worksets` prints #734's counts over docs/worksets.md and the finished list
at the head; it gates nothing either.
"""

import argparse
import importlib
import io
import json
import posixpath
import re
import subprocess
import sys
import tarfile
from collections import defaultdict
from collections.abc import Iterator
from dataclasses import dataclass, field
from pathlib import Path, PurePosixPath
from typing import Protocol

ROOT = Path(__file__).resolve().parents[1]
RUST = "rust/openttd-kernels/src"
COMMENTS = ("comment", "line_comment", "block_comment")

try:
    tree_sitter = importlib.import_module("tree_sitter")
    tree_sitter_cpp = importlib.import_module("tree_sitter_cpp")
    tree_sitter_rust = importlib.import_module("tree_sitter_rust")
except ImportError:
    import os

    os.execvp(
        "uv",
        [
            "uv",
            "run",
            "--no-project",
            "--with",
            "tree-sitter==0.25.2",
            "--with",
            "tree-sitter-rust==0.24.0",
            "--with",
            "tree-sitter-cpp==0.23.4",
            "python3",
            *sys.argv,
        ],
    )


class Node(Protocol):
    type: str
    start_byte: int
    end_byte: int
    children: list["Node"]
    named_children: list["Node"]
    prev_named_sibling: "Node | None"
    prev_sibling: "Node | None"
    parent: "Node | None"

    def child_by_field_name(self, name: str) -> "Node | None": ...

    @property
    def start_point(self) -> tuple[int, int]: ...

    @property
    def end_point(self) -> tuple[int, int]: ...


PARSERS = {
    "rust": tree_sitter.Parser(tree_sitter.Language(tree_sitter_rust.language())),
    "cpp": tree_sitter.Parser(tree_sitter.Language(tree_sitter_cpp.language())),
}


@dataclass
class Function:
    path: str
    name: str
    line: int
    body_line: int
    end: int
    comments: int
    doc: str
    calls: list[str] = field(default_factory=list)
    scope: str = ""

    @property
    def size(self) -> int:
        return self.end - self.line


def archive(revision: str, *paths: str) -> dict[str, bytes]:
    """Tracked files of paths at revision, read from `git archive`."""
    data = subprocess.check_output(["git", "archive", revision, "--", *paths], cwd=ROOT)
    files = {}
    with tarfile.open(fileobj=io.BytesIO(data)) as tar:
        for member in tar.getmembers():
            if member.isfile() and (handle := tar.extractfile(member)):
                files[member.name] = handle.read()
    return files


def walk(node: Node) -> Iterator[Node]:
    yield node
    for child in node.children:
        yield from walk(child)


def text(node: Node | None, data: bytes) -> str:
    return (
        data[node.start_byte : node.end_byte].decode(errors="replace") if node else ""
    )


def leading(node: Node, data: bytes) -> str:
    """Comments and attributes directly before an item."""
    found = []
    previous = node.prev_named_sibling
    while previous and previous.type in (*COMMENTS, "attribute_item"):
        found.append(text(previous, data))
        previous = previous.prev_named_sibling
    return "\n".join(reversed(found))


# `#[test]`, `#[cfg(test)]` and `#[cfg(all(test, ...))]`; `cfg(any(test, ...))`
# also builds production code.
TEST_ATTRIBUTE = re.compile(r"#\[\s*(?:test\s*\]|cfg\s*\(\s*(?:all\s*\(\s*)?test\b)")
TEST_FILE_ATTRIBUTE = re.compile(r"#!\[\s*cfg\s*\(\s*test\s*\)\s*\]")


def attributes(node: Node, data: bytes) -> list[str]:
    """The attribute items directly before an item, past the comments among them."""
    found = []
    previous = node.prev_named_sibling
    while previous and previous.type in (*COMMENTS, "attribute_item"):
        if previous.type == "attribute_item":
            found.append(text(previous, data))
        previous = previous.prev_named_sibling
    return found


def test_path(path: str) -> bool:
    return "/tests/" in path or path.endswith("/tests.rs")


def test_item(node: Node, data: bytes) -> bool:
    """A `mod tests` or `mod test`, or an item a test attribute marks."""
    name = node.child_by_field_name("name")
    if node.type == "mod_item" and text(name, data) in ("tests", "test"):
        return True
    return any(TEST_ATTRIBUTE.match(item) for item in attributes(node, data))


def in_test(node: Node, data: bytes, path: str) -> bool:
    if test_path(path):
        return True
    current: Node | None = node
    while current:
        if test_item(current, data):
            return True
        current = current.parent
    return False


def scope(node: Node, data: bytes) -> str:
    """A function's name inside its inline modules, functions and `impl` block,
    such as `signal::impl Update for Train::update`."""
    parts = [text(node.child_by_field_name("name"), data)]
    current = node.parent
    while current:
        name = current.child_by_field_name("name")
        if current.type == "impl_item":
            kind = " ".join(text(current.child_by_field_name("type"), data).split())
            trait = current.child_by_field_name("trait")
            parts.append(
                f"impl {text(trait, data)} for {kind}" if trait else f"impl {kind}"
            )
        elif current.type in ("mod_item", "function_item", "trait_item"):
            parts.append(text(name, data))
        current = current.parent
    return "::".join(reversed(parts))


def comment_lines(body: Node, data: bytes) -> int:
    """Lines with comment content inside a body; delimiters alone do not count."""
    lines = set()
    for node in walk(body):
        if node.type not in COMMENTS:
            continue
        for offset, line in enumerate(text(node, data).splitlines()):
            content = re.sub(r"\*/\s*$", "", line)
            if re.sub(r"^\s*(?://[/!]?|/\*[*!]?|\*)\s?", "", content).strip():
                lines.add(node.start_point[0] + offset)
    return len(lines)


def signature(node: Node, data: bytes, language: str) -> str | None:
    """A defined function's name, or None for declarations and non-functions."""
    if language == "rust":
        return text(node.child_by_field_name("name"), data)
    declarator = node.child_by_field_name("declarator")
    if declarator is None:
        return None
    functions = [n for n in walk(declarator) if n.type == "function_declarator"]
    if not functions:
        return None
    return text(functions[0].child_by_field_name("declarator"), data)


def functions(
    path: str, data: bytes, language: str, tests: bool = True
) -> list[Function]:
    """Every function with a body; Rust test functions are left out unless asked."""
    found = []
    for node in walk(PARSERS[language].parse(data).root_node):
        body = node.child_by_field_name("body")
        if node.type not in ("function_item", "function_definition") or body is None:
            continue
        name = signature(node, data, language)
        if name is None or (not tests and in_test(node, data, path)):
            continue
        calls = [
            text(call.child_by_field_name("function"), data)
            for call in walk(body)
            if call.type == "call_expression"
        ]
        found.append(
            Function(
                path,
                name,
                node.start_point[0] + 1,
                body.start_point[0] + 1,
                node.end_point[0] + 1,
                comment_lines(body, data),
                leading(node, data),
                calls,
                scope(node, data) if language == "rust" else name,
            )
        )
    return found


def normalized(name: str) -> str:
    return re.sub("[^a-z0-9]", "", name.lower().removeprefix("cmd"))


# Rust module families whose original file has another stem.
ALIASES = {
    "road_stop": "roadstop",
    "orders": "order_cmd",
    "trees": "tree_cmd",
    "water": "water_cmd",
    "objects": "object_cmd",
    "disaster": "disaster_vehicle",
    "ground": "ground_vehicle",
    "ship_control": "ship_cmd",
    "effect": "effectvehicle",
    "town": "town_cmd",
    "train": "train_cmd",
    "aircraft": "aircraft_cmd",
}
# Branch-correspondence mappings the overhaul confirmed by hand (SIGNAL_* witnesses,
# shared removal exports): Rust (file suffix, name) to original (path, name).
MANUAL = {
    # The order half of `Vehicle::BeginLoading`; `vehicle/visits.rs` holds its tail.
    ("/orders.rs", "begin_loading_orders"): (
        "src/vehicle.cpp",
        "Vehicle::BeginLoading",
    ),
    ("/signal.rs", "explore"): ("src/signal.cpp", "ExploreSegment"),
    ("/station_cmd.rs", "remove_rail_command"): (
        "src/station_cmd.cpp",
        "CmdRemoveFromRailStation",
    ),
    ("/station_cmd.rs", "remove_road_command"): (
        "src/station_cmd.cpp",
        "CmdRemoveRoadStop",
    ),
}
GENERIC = ("new", "from", "default", "get", "set")


class Matcher:
    """Pairs non-test Rust functions with original functions, each original once."""

    def __init__(
        self, rust: list[Function], original: list[Function], facades: list[Function]
    ) -> None:
        self.rust, self.original = rust, original
        self.by_name: dict[str, list[int]] = defaultdict(list)
        self.by_path: dict[str, list[int]] = defaultdict(list)
        for index, function in enumerate(rust):
            self.by_name[function.name].append(index)
            self.by_path[function.path].append(index)
        self.original_by_name: dict[str, list[int]] = defaultdict(list)
        self.original_by_path_name: dict[tuple[str, str], list[int]] = defaultdict(list)
        self.original_normalized: dict[str, list[int]] = defaultdict(list)
        for index, function in enumerate(original):
            self.original_by_name[function.name].append(index)
            self.original_by_path_name[function.path, function.name].append(index)
            self.original_normalized[normalized(function.name)].append(index)
        self.mapped: dict[int, int] = {}
        self.through_facades(facades)
        self.through_names()
        self.by_hand()

    def put(self, rust: int, original: int) -> None:
        self.mapped.setdefault(rust, original)

    def facade_original(self, facade: Function) -> int | None:
        """A thin candidate facade's unique original function of the same name."""
        found = self.original_by_path_name.get((facade.path, facade.name), [])
        return (
            found[0]
            if len(found) == 1 and facade.end - facade.body_line <= 30
            else None
        )

    def through_facades(self, facades: list[Function]) -> None:
        """Facade, then its unique export, then the largest same-file callee."""
        origins: dict[str, set[int]] = defaultdict(set)
        for facade in facades:
            if (original := self.facade_original(facade)) is not None:
                for call in facade.calls:
                    if call.startswith("openttd_rust_"):
                        origins[call].add(original)
        for facade in facades:
            if (original := self.facade_original(facade)) is None:
                continue
            for call in facade.calls:
                if call.startswith("openttd_rust_") and len(origins[call]) == 1:
                    for export in self.by_name.get(call, []):
                        self.put(self.forwarded(export), original)

    def forwarded(self, export: int) -> int:
        """The export, or its largest same-file callee when that is larger."""
        function = self.rust[export]
        candidates = [
            other
            for call in function.calls
            for other in self.by_path[function.path]
            if self.rust[other].name == re.split(r"\.|::", call)[-1]
            and other != export
            and self.rust[other].name not in GENERIC
        ]
        largest = max(candidates, key=lambda i: self.rust[i].size, default=export)
        return largest if self.rust[largest].size > function.size else export

    def through_names(self) -> None:
        """Normalized name or documented original name in the matching module."""
        for index, function in enumerate(self.rust):
            if index in self.mapped or function.name.startswith("openttd_rust_"):
                continue
            family = (
                PurePosixPath(function.path)
                .relative_to(RUST)
                .parts[0]
                .removesuffix(".rs")
            )
            stems = (family, ALIASES.get(family))
            candidates = self.original_normalized.get(normalized(function.name), [])
            local = [
                i
                for i in candidates
                if PurePosixPath(self.original[i].path).stem in stems
            ]
            documented = {
                i
                for word in re.findall(r"\b[A-Z][A-Za-z0-9_:]+\b", function.doc)
                for i in self.original_by_name.get(word, [])
                if PurePosixPath(self.original[i].path).stem in stems
            }
            if len(local) == 1:
                self.put(index, local[0])
            elif len(documented) == 1:
                self.put(index, next(iter(documented)))
            elif len(candidates) == 1 and len(normalized(function.name)) >= 12:
                self.put(index, candidates[0])

    def by_hand(self) -> None:
        for index, function in enumerate(self.rust):
            for (suffix, name), key in MANUAL.items():
                found = self.original_by_path_name.get(key, [])
                if (
                    function.path.endswith(suffix)
                    and function.name == name
                    and len(found) == 1
                ):
                    self.put(index, found[0])

    def pairs(self) -> list[tuple[Function, Function]]:
        """One pair per original, through its largest mapped Rust body."""
        reverse: dict[int, list[int]] = defaultdict(list)
        for rust, original in self.mapped.items():
            reverse[original].append(rust)
        return [
            (
                self.rust[max(rust, key=lambda i: self.rust[i].size)],
                self.original[original],
            )
            for original, rust in reverse.items()
        ]


def comment_pairs(
    head: str, tests: set[str] | None = None
) -> list[tuple[Function, Function]]:
    """Rust bodies paired with original ones; test files pair with nothing."""
    baseline = baseline_commit(head)
    candidate = archive(head, RUST, "src")
    rust = [
        function
        for path, data in sorted(candidate.items())
        if path.startswith(RUST) and path.endswith(".rs") and path not in (tests or ())
        for function in functions(path, data, "rust", tests=False)
    ]
    facades = [
        function
        for path, data in sorted(candidate.items())
        if cpp_source(path) and b"openttd_rust_" in data
        for function in functions(path, data, "cpp")
    ]
    original = [
        function
        for path, data in sorted(archive(baseline, "src").items())
        if cpp_source(path)
        for function in functions(path, data, "cpp")
    ]
    return Matcher(rust, original, facades).pairs()


def baseline_commit(revision: str) -> str:
    """The original commit pinned by the revision's own baseline manifest."""
    manifest = subprocess.check_output(
        ["git", "show", f"{revision}:migration/baseline.json"], cwd=ROOT
    )
    return json.loads(manifest)["commit"]


def cpp_source(path: str) -> bool:
    return (
        path.startswith("src/")
        and "/3rdparty/" not in path
        and path.endswith((".cpp", ".h", ".hpp"))
    )


# Parameter and field names whose concept has a named type in the original.
CONCEPTS = {
    "TileIndex": "tile tile_index start_tile end_tile dest_tile target_tile source_tile from_tile to_tile new_tile old_tile next_tile cur_tile current_tile depot_tile station_tile tunnel_tile bridge_tile tile_from tile_to end_tile_index base_tile ref_tile reference_tile",
    "CompanyID/Owner": "owner new_owner old_owner company company_id current_company local_company company_owner previous_owner owner_id",
    "StationID": "station station_id source_station dest_station target_station last_station next_station from_station to_station via_station load_station unload_station station1 station2",
    "TownID": "town town_id closest_town nearest_town target_town",
    "VehicleID": "vehicle vehicle_id front_id last_vehicle_id first_vehicle_id next_vehicle_id",
    "EngineID": "engine engine_id first_engine engine_type_id",
    "IndustryID": "industry industry_id source_industry dest_industry target_industry",
    "GroupID": "group group_id parent_group parent_group_id",
    "OrderListID/VehicleOrderID": "order_id order_list_id order_index vehicle_order_id from_order to_order",
    "Other pool IDs": "depot_id object_id sign_id subsidy_id road_stop_id goal_id story_page_id story_page_element_id league_table_id league_table_element_id link_graph_id link_graph_job_id cargo_packet_id cargo_payment_id",
    "CargoID/Types/Label": "cargo cargo_id cargo_type cargo_types cargo_mask cargo_label cargo_type_mask",
    "Direction family": "dir direction diagdir diag_dir diag_direction exitdir enterdir exit_dir enter_dir exit_direction enter_direction reverse_direction reverse_dir",
    "Track": "track track_index origin_track",
    "TrackBits": "tracks trackbits track_bits reserved_tracks blocked_tracks reachable_tracks available_tracks",
    "Trackdir": "trackdir td track_dir new_td old_td origin_td",
    "TrackdirBits": "trackdirs trackdirbits trackdir_bits td_bits",
    "RoadType/RailType": "road_type roadtype rail_type railtype old_road_type new_road_type old_rail_type new_rail_type",
    "RoadTypes/RailTypes": "road_types rail_types compatible_roadtypes compatible_railtypes powered_roadtypes powered_railtypes",
    "RoadBits": "road_bits roadbits rb",
    "RoadTramType": "rtt road_tram_type",
    "Axis": "axis",
    "Slope/Foundation": "slope tile_slope old_slope new_slope foundation",
    "TileType": "tile_type tiletype",
    "StationType/RoadStopType/Facilities": "station_type stationtype stop_type road_stop_type facilities facility",
    "VehicleType": "vehicle_type veh_type vtype",
    "Price/Expenses": "price_index price_category expenses_type expense_type expenses",
    "StringID": "string_id strid error_id error_message_id text_id",
    "Command/flag domains": "command command_id cmd command_flags cmd_flags do_command_flags",
    "TransportType": "transport_type transport",
    "SignalType/Variant/State": "signal_type sigtype signal_variant signal_state",
    "Order/Load/Unload enums": "order_type non_stop_flags load_flags unload_flags depot_action depot_type",
    "Other domain masks": "animation_trigger random_trigger callback_id grf_id sprite_id sprite pal palette water_class landscape_type climate tree_ground roadside rail_ground rail_ground_type rail_tile_type",
}
CONCEPT = {
    name: concept for concept, names in CONCEPTS.items() for name in names.split()
}
INTEGER = re.compile(
    r"\b([a-z_][a-z_0-9]*)\s*:\s*((?:u|i)(?:8|16|32|64|128)|usize|isize)(?![\w:])"
)
WRAPPER = re.compile(
    r"#\[repr\(transparent\)\][^;{]*?\bstruct\s+(\w+)\s*\(\s*(?:pub(?:\([^)]*\))?\s+)?([^)]*)\)"
)
CONSTANT = re.compile(r"\bconst\s+([A-Z][A-Z_0-9]*)\s*:\s*([^=;\n]+)")
LITERALS = {
    "StringID CMD_ERROR literal": r"\.error\(\s*(?:0[xX][fF]{4}|65535)\s*\)",
    "Price immediate argument": r"\b(?:company::price|economy_func::get_price)\(\s*(?:0[xX][0-9a-fA-F_]+|[0-9_]+)\s*[,)]",
    "RoadStopType comparison literal": r"\bstop_type\s*(?:==|!=|>=|<=|>|<)\s*\d+\b",
    "TrackBits 0x0c/0x30 comparison": r"\btracks\s*==\s*0x(?:0[cC]|30)\b",
}


def code_only(source: str) -> str:
    """Blank comments and strings, keeping offsets and line breaks."""
    pattern = r'(?s:/\*.*?\*/)|//[^\n]*|r(#+)?".*?"\1|"(?:\\.|[^"\\])*"'
    return re.sub(pattern, lambda m: re.sub(r"[^\n]", " ", m[0]), source, flags=re.S)


def homonym(name: str, kind: str) -> bool:
    """Reviewed names whose integer type shows another concept."""
    return (
        (name in ("tile", "vehicle", "station", "industry", "engine") and kind == "u8")
        or (name == "cargo" and kind != "u8")
        or name == "expenses"
    )


@dataclass
class Site:
    """One row; `key` names its occurrence for `--compare` where the detail
    does not (a function in its scope), and `amount` is what can grow."""

    path: str
    line: int
    kind: str
    detail: str
    key: str = ""
    amount: int = 0

    @property
    def identity(self) -> str:
        return self.key or self.detail


def type_sites(path: str, source: str) -> list[Site]:
    """Bare-integer concept sites, wrappers, constants and literal sites of one file."""
    code = code_only(source)

    def line(match: re.Match[str]) -> int:
        return code.count("\n", 0, match.start()) + 1

    sites = [
        Site(path, line(m), "bare integer", f"{m[1]}: {m[2]} ({CONCEPT[m[1]]})")
        for m in INTEGER.finditer(code)
        if m[1] in CONCEPT and not homonym(m[1], m[2])
    ]
    sites += [
        Site(path, line(m), "wrapper", f"{m[1]}({m[2].strip()})")
        for m in WRAPPER.finditer(code)
    ]
    sites += [Site(path, line(m), "constant", m[1]) for m in CONSTANT.finditer(code)]
    for concept, pattern in LITERALS.items():
        sites += [
            Site(path, line(m), "literal", f"{concept}: {m[0]}")
            for m in re.finditer(pattern, code)
        ]
    return sites


LONG_PARAMETERS = 7


PATH_ATTRIBUTE = re.compile(r'#\[\s*path\s*=\s*"([^"]+)"\s*\]')


def module_files(path: str, node: Node, data: bytes) -> set[str]:
    """The files a `mod name;` item names: inside the inline modules around it,
    or where its `#[path]` attribute points."""
    file = PurePosixPath(path)
    inline = []
    current = node.parent
    while current:
        if current.type == "mod_item":
            inline.append(text(current.child_by_field_name("name"), data))
        current = current.parent
    stem = [] if file.name in ("lib.rs", "mod.rs", "main.rs") else [file.stem]
    directory = file.parent.joinpath(*stem, *reversed(inline))
    name = text(node.child_by_field_name("name"), data)
    for attribute in attributes(node, data):
        if found := PATH_ATTRIBUTE.match(attribute):
            # Outside inline modules a path is relative to the file's own folder.
            base = directory if inline else file.parent
            return {posixpath.normpath(PurePosixPath(base, found[1]))}
    return {str(directory / f"{name}.rs"), str(directory / name / "mod.rs")}


def declared_modules(path: str, source: str) -> tuple[bool, set[str], set[str]]:
    """Whether a file is test code by its path or inner attribute, and the module
    files its `mod name;` items declare, as (file, test modules, other modules)."""
    data = source.encode()
    root = PARSERS["rust"].parse(data).root_node
    whole = test_path(path) or any(
        child.type == "inner_attribute_item"
        and TEST_FILE_ATTRIBUTE.fullmatch(text(child, data))
        for child in root.children
    )
    tests: set[str] = set()
    others: set[str] = set()
    for node in walk(root):
        if node.type != "mod_item" or node.child_by_field_name("body") is not None:
            continue
        files = module_files(path, node, data)
        (tests if in_test(node, data, path) else others).update(files)
    return whole, tests, others


def test_modules(rust: dict[str, str]) -> set[str]:
    """Paths of the test files: those a test `mod name;` declares (`#[cfg(test)]`
    or `in_test`'s other rules, read from the syntax tree), those `in_test`'s path
    rule names, those opening with `#![cfg(test)]`, and the modules they declare."""
    found: set[str] = set()
    children: dict[str, set[str]] = {}
    for path, source in rust.items():
        whole, tests, others = declared_modules(path, source)
        found |= tests | ({path} if whole else set())
        children[path] = others
    pending = list(found)
    while pending:
        for child in children.get(pending.pop(), set()) - found:
            found.add(child)
            pending.append(child)
    return found


def long_functions(path: str, source: str) -> list[Site]:
    """Non-test Rust functions with more than LONG_PARAMETERS parameters, `self` included."""
    data = source.encode()
    found = []
    for node in walk(PARSERS["rust"].parse(data).root_node):
        parameters = node.child_by_field_name("parameters")
        if node.type != "function_item" or parameters is None:
            continue
        count = sum(
            p.type in ("parameter", "self_parameter", "variadic_parameter")
            for p in parameters.named_children
        )
        if count <= LONG_PARAMETERS or in_test(node, data, path):
            continue
        name = text(node.child_by_field_name("name"), data)
        head = text(node, data).split("fn ", 1)[0]
        if "no_mangle" in leading(node, data) or 'extern "C"' in head:
            kind = "ffi export"
        elif any(
            p.type == "parameter"
            and text(p.child_by_field_name("pattern"), data) == "flags"
            and re.fullmatch(
                r"u16|(?:[\w:]+::)?DoCommandFlags",
                text(p.child_by_field_name("type"), data),
            )
            for p in parameters.named_children
        ):
            kind = "command"
        else:
            kind = "helper"
        found.append(
            Site(
                path,
                node.start_point[0] + 1,
                kind,
                f"{name}: {count} parameters",
                scope(node, data),
                count,
            )
        )
    return found


# #892's assertion macros, by class; a macro name must be followed by `!` and a
# delimiter, so `assert != x`, `prefix_assert!` and `assert_matches!` are not sites.
ASSERTION = re.compile(
    r"(?<!\w)(debug_assert(?:_eq|_ne)?|assert(?:_eq|_ne)?|unreachable)\s*!\s*[(\[{]"
)
ASSERTION_CLASSES = ("debug_assert", "assert", "unreachable")
LITERAL_NODES = ("string_literal", "raw_string_literal", "char_literal")
CONST_ITEM = re.compile(r"\bconst\s+\$?\w+\s*:")


def assertion_code(data: bytes, root: Node) -> str:
    """The file's text with comments, literals and test items blanked, decoded
    byte for byte so that string offsets are the parser's byte offsets."""
    code = bytearray(data)

    def visit(node: Node) -> None:
        if node.type in (*COMMENTS, *LITERAL_NODES) or test_item(node, data):
            code[node.start_byte : node.end_byte] = re.sub(
                rb"[^\n]", b" ", code[node.start_byte : node.end_byte]
            )
            return
        for child in node.children:
            visit(child)

    visit(root)
    return code.decode("latin-1")


def contexts(root: Node, code: str) -> list[tuple[int, int, bool]]:
    """The spans that decide a site's class, as (start, end, compile time): each
    parsed `const` item's value and each function, and in a `macro_rules!` body,
    where the parser gives only tokens, the `const` initialisers of the token scan."""
    spans = []
    for node in walk(root):
        value = node.child_by_field_name("value")
        if node.type == "const_item" and value is not None:
            spans.append((value.start_byte, value.end_byte, True))
        elif node.type == "function_item":
            spans.append((node.start_byte, node.end_byte, False))
        elif node.type == "macro_definition":
            start = node.start_byte
            body = code[start : node.end_byte]
            spans += [(start + a, start + b, True) for a, b in const_spans(body)]
    return spans


def compile_time(spans: list[tuple[int, int, bool]], offset: int) -> bool:
    """Whether the innermost span around offset is a `const` initialiser."""
    around = [span for span in spans if span[0] <= offset < span[1]]
    return min(around, key=lambda s: s[1] - s[0], default=(0, 0, False))[2]


def assignment(code: str, index: int) -> bool:
    """A lone `=`, not part of `==`, `=>`, `<=`, `>=` or `!=`."""
    return code[index - 1] not in "=!<>" and code[index + 1 : index + 2] not in "=>"


def initialiser_start(code: str, start: int) -> int | None:
    """The `=` of a `const NAME: TYPE = ...;` whose type starts at start, or None
    for a declaration without one (a trait's `const X: u8;`)."""
    depth = 0
    for index in range(start, len(code)):
        char = code[index]
        if char == "=" and depth == 0 and assignment(code, index):
            return index
        if (char == ";" and depth == 0) or (char == "{" and depth == 0):
            return None
        depth += (char in "([{") - (char in ")]}")
        if depth < 0:
            return None
    return None


def initialiser(code: str, start: int) -> tuple[int, int] | None:
    """The span of the initialiser of a `const` item whose type starts at start,
    up to its `;` or the bracket that closes around it."""
    begin = initialiser_start(code, start)
    if begin is None:
        return None
    depth = 0
    for index in range(begin, len(code)):
        depth += (code[index] in "([{") - (code[index] in ")]}")
        if depth < 0 or (code[index] == ";" and depth == 0):
            return begin, index
    return begin, len(code)


def const_spans(code: str) -> list[tuple[int, int]]:
    """Initialisers of `const` items in a macro body's tokens; a generic
    `<const N: usize>` parameter and a `const fn` are not items with one."""
    spans = []
    for match in CONST_ITEM.finditer(code):
        if code[: match.start()].rstrip()[-1:] in ("<", ","):
            continue
        if (span := initialiser(code, match.end())) is not None:
            spans.append(span)
    return spans


def assertion_sites(path: str, source: str) -> list[Site]:
    """#892's sites of one non-test file: `assertion macro` at run time, or
    `const assertion` inside a `const` item's initialiser, the original's
    `static_assert`."""
    if test_path(path):
        return []
    data = source.encode()
    root = PARSERS["rust"].parse(data).root_node
    blanked = assertion_code(data, root)
    spans = contexts(root, blanked)
    # Names match in decoded text, where a UTF-8 identifier stays one word; the
    # class reads the span at the match's byte offset.
    code = blanked.encode("latin-1").decode()
    sites = []
    for match in ASSERTION.finditer(code):
        offset = len(code[: match.start()].encode())
        kind = "const assertion" if compile_time(spans, offset) else "assertion macro"
        line = code.count("\n", 0, match.start()) + 1
        sites.append(Site(path, line, kind, f"{match[1]}!"))
    return sites


def assertion_class(site: Site) -> str:
    name = site.detail.removesuffix("!")
    return next(c for c in ASSERTION_CLASSES if name.startswith(c))


def assertion_line(revision: str, sites: list[Site]) -> str:
    runtime = [s for s in sites if s.kind == "assertion macro"]
    compile_time = [s for s in sites if s.kind == "const assertion"]
    classes = ", ".join(
        f"{sum(assertion_class(s) == c for s in runtime)} {c}"
        for c in ASSERTION_CLASSES
    )
    return (
        f"Assertion macros at {revision} (#892): {len(runtime)} runtime sites in "
        f"{len({s.path for s in runtime})} files ({classes}), "
        f"{len(compile_time)} const assertions, {len(sites)} in "
        f"{len({s.path for s in sites})} files together"
    )


# Residue classes the finished-file gate checks (#901), keyed by identifier, never by line.
RESIDUE = (
    "raw adapter call",
    "export call",
    "execute flag",
    "scratch pool",
    "long command",
)
FINISHED = "migration/finished-files.txt"
# Standard-library calls named like raw adapters, which no pass can retire.
STD_RAW = re.compile(r"(?:std::boxed::)?Box::(?:into|from)_raw")


def export_shim(node: Node, data: bytes) -> bool:
    """Whether the innermost function around node is an FFI export."""
    current: Node | None = node
    while current and current.type != "function_item":
        current = current.parent
    if current is None:
        return False
    head = text(current, data).split("fn ", 1)[0]
    return "no_mangle" in leading(current, data) or 'extern "C"' in head


def callee(call: Node, data: bytes) -> str:
    function = call.child_by_field_name("function")
    if function is not None and function.type == "field_expression":
        function = function.child_by_field_name("field")
    elif function is not None and function.type == "generic_function":
        function = function.child_by_field_name("function")
    return re.split(r"::|\.", text(function, data))[-1]


def residue(path: str, source: str) -> list[Site]:
    """Finished-file residue of one non-test Rust file, outside test code.

    Calls to a `*_raw` adapter (not the standard library's `Box::into_raw` and
    `Box::from_raw`) or an `openttd_rust_*` export outside export shims,
    Rust-internal functions taking `execute: bool`, `GameState<Vec<...>>` scratch
    pools and commands over LONG_PARAMETERS parameters.
    """
    data = source.encode()
    found = [s for s in long_functions(path, source) if s.kind == "command"]
    found = [
        Site(s.path, s.line, "long command", s.detail.split(":")[0]) for s in found
    ]
    for node in walk(PARSERS["rust"].parse(data).root_node):
        line = node.start_point[0] + 1
        if node.type == "call_expression":
            name = callee(node, data)
            kind = (
                "raw adapter call"
                if name.endswith("_raw")
                and not STD_RAW.fullmatch(
                    text(node.child_by_field_name("function"), data)
                )
                else "export call"
                if name.startswith("openttd_rust_")
                else None
            )
            if kind and not in_test(node, data, path) and not export_shim(node, data):
                found.append(Site(path, line, kind, name))
        elif node.type == "function_item":
            parameters = node.child_by_field_name("parameters")
            if parameters is None or in_test(node, data, path):
                continue
            if export_shim(node, data):
                continue
            if any(
                p.type == "parameter"
                and text(p.child_by_field_name("pattern"), data) == "execute"
                and text(p.child_by_field_name("type"), data) == "bool"
                for p in parameters.named_children
            ):
                name = text(node.child_by_field_name("name"), data)
                found.append(Site(path, line, "execute flag", name))
        elif node.type == "generic_type" and re.match(
            r"GameState\s*<\s*Vec\s*<", text(node, data)
        ):
            if not in_test(node, data, path):
                found.append(Site(path, line, "scratch pool", pool_name(node, data)))
    return sorted(found, key=lambda s: (s.line, s.kind))


def pool_name(node: Node, data: bytes) -> str:
    """The static, field or binding a `GameState<Vec<...>>` type declares."""
    item = node.parent
    while item and item.type not in (
        "static_item",
        "const_item",
        "field_declaration",
        "let_declaration",
    ):
        item = item.parent
    if item is None:
        return text(node, data)
    named = item.child_by_field_name("name") or item.child_by_field_name("pattern")
    return text(named, data)


def finished_files(revision: str) -> list[str]:
    """The finished-file list at revision (empty before the list exists)."""
    listed = show(revision, FINISHED) or ""
    return [
        line.strip()
        for line in listed.splitlines()
        if line.strip() and not line.startswith("#")
    ]


def show(revision: str, path: str) -> str | None:
    """A file's text at revision, or None where it does not exist."""
    shown = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, capture_output=True
    )
    return shown.stdout.decode() if shown.returncode == 0 else None


def renamed_from(base: str, head: str, scope: str = RUST) -> dict[str, str]:
    """Head path to base path for files under scope that Git sees as renamed."""
    # NUL-delimited, so Git quotes no path: status, then one path or, for a
    # rename or copy, the old path and the new one.
    fields = subprocess.check_output(
        ["git", "diff", "--name-status", "-z", "-M", base, head, "--", scope],
        cwd=ROOT,
        text=True,
    ).split("\0")
    renames, index = {}, 0
    while index < len(fields) and fields[index]:
        status = fields[index]
        if status[0] in "RC":
            if status[0] == "R":
                renames[fields[index + 2]] = fields[index + 1]
            index += 3
        else:
            index += 2
    return renames


def growth(path: str, before: str, after: str) -> list[Site]:
    """Residue sites a file gained, per class; a renamed occurrence is not growth."""
    old, new = residue(path, before), residue(path, after)
    found = []
    for kind in RESIDUE:
        was = [s.detail for s in old if s.kind == kind]
        now = [s for s in new if s.kind == kind]
        if len(now) <= len(was):
            continue
        added = []
        for site in now:
            if site.detail in was:
                was.remove(site.detail)
            else:
                added.append(site)
        found += added or now
    return found


def gate(base_ref: str, head: str) -> int:
    """Fail when a finished file gains residue of a RESIDUE class (#901).

    A file's existing occurrences, read at the merge base, are its baseline; per
    class only growth fails, so renaming or moving an occurrence passes.
    """
    base = subprocess.check_output(
        ["git", "merge-base", base_ref, head], cwd=ROOT, text=True
    ).strip()
    files = finished_files(head)
    renames = renamed_from(base, head)
    missing = [path for path in files if show(head, path) is None]
    for path in missing:
        print(f"{FINISHED}: listed file is missing at {head}: {path}")
    failures = [
        site
        for path in files
        if (after := show(head, path)) is not None
        for site in growth(path, show(base, renames.get(path, path)) or "", after)
    ]
    for site in failures:
        print(
            f"{site.path}:{site.line}: new {site.kind} in a finished file: {site.detail}"
        )
    print(
        f"Finished-file residue gate: {len(files)} finished files, "
        f"{len(failures)} new residue sites"
    )
    return int(bool(failures or missing))


WORKSETS = "docs/worksets.md"
# `- **04 Cargo**: `cargo_flow.rs`, ...`: a workset and its files under RUST.
WORKSET_LINE = re.compile(r"- \*\*([^*]+)\*\*: (`[^`]+`(?:, `[^`]+`)*)")


@dataclass
class WorksetRow:
    """One workset's count: its files on disk, those finished and their lines,
    the lines left and each unfinished file with its lines."""

    name: str
    files: int = 0
    finished: int = 0
    finished_lines: int = 0
    left: int = 0
    unfinished: list[str] = field(default_factory=list)


def workset_manifest(text: str) -> tuple[dict[str, list[str]], list[str]]:
    """Each workset's entries in order, and a problem line per malformed line."""
    worksets: dict[str, list[str]] = {}
    problems = []
    for number, line in enumerate(text.splitlines(), 1):
        if not line.startswith("- "):
            continue
        if found := WORKSET_LINE.fullmatch(line.strip()):
            worksets.setdefault(found[1], []).extend(re.findall(r"`([^`]+)`", found[2]))
        else:
            problems.append(f"{WORKSETS}:{number}: not a workset line: {line}")
    return worksets, problems


@dataclass
class WorksetCount:
    """The manifest counted against a tree: a row per workset, the files the
    manifest names, and its entries without a file or listed twice."""

    rows: list[WorksetRow] = field(default_factory=list)
    seen: set[str] = field(default_factory=set)
    missing: list[str] = field(default_factory=list)
    twice: list[str] = field(default_factory=list)


def count_worksets(
    manifest: dict[str, list[str]], finished: set[str], lines: dict[str, int]
) -> WorksetCount:
    """Each entry counted once, in its first workset."""
    count = WorksetCount()
    for name, entries in manifest.items():
        row = WorksetRow(name)
        for entry in entries:
            path = f"{RUST}/{entry}"
            if path in count.seen:
                count.twice.append(f"{name}: {entry}")
                continue
            count.seen.add(path)
            if path not in lines:
                count.missing.append(f"{name}: {entry}")
                continue
            row.files += 1
            if path in finished:
                row.finished += 1
                row.finished_lines += lines[path]
            else:
                row.left += lines[path]
                row.unfinished.append(f"{entry} ({lines[path]})")
        count.rows.append(row)
    return count


def worksets(head: str) -> int:
    """#734's count: each workset's finished files and lines left, over the
    manifest and the finished list at head, and every Rust file there (#1063).
    A file's lines are its newline count. It gates nothing."""
    text = show(head, WORKSETS)
    if text is None:
        print(f"{WORKSETS} is not at {head}; reading the checkout's")
        text = (ROOT / WORKSETS).read_text()
    manifest, problems = workset_manifest(text)
    for problem in problems:
        print(problem)
    finished = set(finished_files(head))
    lines = {
        path: data.count(b"\n")
        for path, data in archive(head, RUST).items()
        if path.endswith(".rs")
    }
    count = count_worksets(manifest, finished, lines)
    rows = count.rows
    entries = sum(row.files for row in rows)
    done = sum(row.finished for row in rows)
    done_lines = sum(row.finished_lines for row in rows)
    left = sum(row.left for row in rows)
    print(
        f"Worksets at {head}: {entries} entries, {done} finished with {done_lines} "
        f"lines, {left} lines left in the other {entries - done}, "
        f"{done_lines + left} together"
    )
    print(
        "| Workset | Files | Finished | Finished lines | Lines left | Unfinished files |"
    )
    print("| --- | ---: | ---: | ---: | ---: | --- |")
    for row in rows:
        print(
            f"| {row.name} | {row.files} | {row.finished} | {row.finished_lines} | "
            f"{row.left} | {', '.join(row.unfinished)} |"
        )
    outside = sorted(finished - count.seen)
    unplaced = sorted(path for path in lines if path not in count.seen | finished)
    for title, paths in (
        ("Finished files in no workset", outside),
        ("Rust files in no workset and not finished", unplaced),
    ):
        total = sum(lines.get(path, 0) for path in paths)
        print(f"{title}: {len(paths)}, {total} lines")
        for path in paths:
            print(f"  {path} ({lines.get(path, 'no file')})")
    for title, found in (
        ("Manifest entries without a file", count.missing),
        ("Manifest entries listed twice", count.twice),
    ):
        print(f"{title}: {len(found)}")
        for entry in found:
            print(f"  {entry}")
    return 0


# Candidate rows: listed for the implementer and reviewer
# to judge, outside RESIDUE, the totals and --gate.
SERVICE = re.compile(r'\bextern\s*"C"\s*fn\b')
PRIMITIVE = re.compile(r"[iu](?:8|16|32|64|128|size)")
QUERIES = {
    "service call": (
        "(call_expression function: (parenthesized_expression"
        " (field_expression field: (field_identifier) @field)))"
    ),
    "service field": "(field_declaration name: (field_identifier) @name) @field",
}


def captures(query: str, node: Node) -> dict[str, list[Node]]:
    compiled = tree_sitter.Query(PARSERS["rust"].language, query)
    return tree_sitter.QueryCursor(compiled).captures(node)


@dataclass
class Services:
    """The crate's `extern "C" fn` fields and the non-test sites that call them."""

    signatures: dict[str, set[str]] = field(default_factory=lambda: defaultdict(set))
    tables: dict[str, int] = field(default_factory=lambda: defaultdict(int))
    calls: dict[str, list[tuple[str, int]]] = field(
        default_factory=lambda: defaultdict(list)
    )


def services(rust: dict[str, str], tests: set[str]) -> Services:
    found = Services()
    for path, source in sorted(rust.items()):
        data = source.encode()
        root = PARSERS["rust"].parse(data).root_node
        for node in captures(QUERIES["service field"], root).get("field", []):
            kind = text(node.child_by_field_name("type"), data)
            if SERVICE.search(kind):
                name = text(node.child_by_field_name("name"), data)
                found.signatures[name].add(" ".join(kind.split()))
                found.tables[name] += 1
        if path in tests:
            continue
        for node in captures(QUERIES["service call"], root).get("field", []):
            if not in_test(node, data, path):
                found.calls[text(node, data)].append((path, node.start_point[0] + 1))
    return found


def one_call_services(found: Services, paths: set[str]) -> list[Site]:
    """Service fields that the given files call from the crate's only call site."""
    rows = []
    for name, sites in found.calls.items():
        if name not in found.signatures or len(sites) != 1 or sites[0][0] not in paths:
            continue
        path, line = sites[0]
        signature = " | ".join(sorted(found.signatures[name]))
        tables = found.tables[name]
        also = f"; declared in {tables} tables" if tables > 1 else ""
        rows.append(Site(path, line, "one-call service", f"{name} ({signature}{also})"))
    return sorted(rows, key=lambda s: (s.detail.split(" ", 1)[1], s.path, s.line))


def parameter_names(node: Node, data: bytes) -> set[str]:
    parameters = node.child_by_field_name("parameters")
    names = {"self"}
    for parameter in parameters.named_children if parameters else []:
        pattern = parameter.child_by_field_name("pattern")
        if pattern is not None:
            names.add(text(pattern, data).removeprefix("mut "))
    return names


def forwarded_argument(node: Node, data: bytes, names: set[str]) -> bool:
    """A parameter, or a wrapper constructor of one such as `TileIndex(tile)`."""
    if node.type in ("identifier", "self"):
        return text(node, data) in names
    if node.type != "call_expression":
        return False
    function = node.child_by_field_name("function")
    arguments = node.child_by_field_name("arguments")
    inner = arguments.named_children if arguments else []
    return (
        function is not None
        and re.fullmatch(r"(?:\w+::)*[A-Z]\w*", text(function, data)) is not None
        and len(inner) == 1
        and forwarded_argument(inner[0], data, names)
    )


def adapter_callee(only: Node, data: bytes, names: set[str]) -> str | None:
    """The callee a one-expression body forwards its parameters to, as written
    (`other::foo`, `self.foo`), or None."""
    node: Node | None = only
    if (
        only.type == "field_expression"
        and text(only.child_by_field_name("field"), data) == "0"
    ):
        node = only.child_by_field_name("value")
    if node is None or node.type != "call_expression":
        return None
    function = node.child_by_field_name("function")
    arguments = node.child_by_field_name("arguments")
    if function is None or arguments is None:
        return None
    if function.type == "field_expression":
        receiver = function.child_by_field_name("value")
        if receiver is None or text(receiver, data) not in names:
            return None
    elif function.type not in ("identifier", "scoped_identifier"):
        return None
    if all(forwarded_argument(a, data, names) for a in arguments.named_children):
        return text(function, data)
    return None


def unsuffixed_adapter(node: Node, data: bytes) -> str | None:
    """A function that only forwards to another, under a name without `_raw`."""
    name = text(node.child_by_field_name("name"), data)
    body = node.child_by_field_name("body")
    head = text(node, data).split("fn ", 1)[0]
    if name.endswith("_raw") or body is None or 'extern "C"' in head:
        return None
    statements = [c for c in body.named_children if c.type not in COMMENTS]
    if len(statements) != 1:
        return None
    (only,) = statements
    if only.type == "expression_statement" and only.named_children:
        only = only.named_children[0]
    callee = adapter_callee(only, data, parameter_names(node, data))
    # Only the function itself is not another: `other::foo` and `w.foo` are.
    recursion = (name, f"self.{name}", f"Self::{name}")
    return name if callee is not None and callee not in recursion else None


def function_rows(path: str, node: Node, data: bytes) -> list[Site]:
    """An unsuffixed adapter or an integer flag set that a function item is."""
    line = node.start_point[0] + 1
    rows = []
    if name := unsuffixed_adapter(node, data):
        rows.append(Site(path, line, "unsuffixed adapter", name))
    head = text(node, data).split("fn ", 1)[0]
    name = text(node.child_by_field_name("name"), data)
    returned = text(node.child_by_field_name("return_type"), data)
    if "flags" in name and PRIMITIVE.fullmatch(returned) and 'extern "C"' not in head:
        rows.append(Site(path, line, "integer flag set", name))
    return rows


def hand_built_bit(node: Node, data: bytes) -> bool:
    """`1 << X.0` outside a set type's own `Self(1 << ...)` constructor."""
    if not re.fullmatch(
        r"1\w*\s*<<\s*\(?[\w.:]+\.0\)?(?:\s+as\s+\w+)?", text(node, data)
    ):
        return False
    call = node.parent.parent if node.parent else None
    return not (
        call is not None
        and call.type == "call_expression"
        and text(call.child_by_field_name("function"), data) == "Self"
    )


def flag_rows(path: str, source: str) -> list[Site]:
    """Unsuffixed adapters, hand-built flag bits and integer flag sets of one file."""
    data = source.encode()
    rows = []
    for node in walk(PARSERS["rust"].parse(data).root_node):
        line = node.start_point[0] + 1
        if node.type == "function_item":
            if not in_test(node, data, path):
                rows += function_rows(path, node, data)
        elif node.type == "field_declaration":
            name = text(node.child_by_field_name("name"), data)
            kind = text(node.child_by_field_name("type"), data)
            if "flags" in name and PRIMITIVE.fullmatch(kind):
                if not in_test(node, data, path):
                    rows.append(Site(path, line, "integer flag set", name))
        elif node.type == "binary_expression" and hand_built_bit(node, data):
            if not in_test(node, data, path):
                rows.append(Site(path, line, "hand-built flag bit", text(node, data)))
    return rows


def candidate_rows(
    rust: dict[str, str], paths: set[str], tests: set[str]
) -> Iterator[Site]:
    """Each candidate row of the given files, as it is found."""
    for path in sorted(paths):
        yield from flag_rows(path, rust[path])
    yield from one_call_services(services(rust, tests), paths)


def show_row(site: Site, prefix: str = "") -> None:
    print(f"{site.path}:{site.line}: {prefix}{site.kind}: {site.detail}", flush=True)


def listed_rows(
    rust: dict[str, str], paths: set[str], tests: set[str], shown: bool
) -> list[Site]:
    """The residue classes of paths, except long commands, then the candidates."""
    rows = []
    for path in sorted(paths):
        for site in residue(path, rust[path]):
            if site.kind != "long command":
                rows.append(site)
                if shown:
                    show_row(site)
    for site in candidate_rows(rust, paths, tests):
        rows.append(site)
        if shown:
            show_row(site)
    return rows


def file_rows(
    rust: dict[str, str], paths: list[str], tests: set[str], shown: bool
) -> tuple[list[Site], list[Site]]:
    """The type sites and long functions of paths, printing every row when shown.

    Each row prints as soon as it is found, cheapest first and before the slow
    comment pairing: a run a harness yields to the background has shown its rows.
    """
    sites, long = [], []
    for path in paths:
        found = sorted(type_sites(path, rust[path]), key=lambda s: s.line)
        sites += found
        for site in found if shown else []:
            if site.kind in ("bare integer", "literal"):
                show_row(site)
    for path in paths:
        if path in tests:
            continue
        found = long_functions(path, rust[path])
        long += found
        for site in found if shown else []:
            show_row(site, "long ")
    return sites, long


@dataclass
class Measure:
    """One revision's rows for the measured files; `listed` (residue and
    candidates) is filled only when files were named."""

    revision: str
    pairs: int
    sites: list[Site]
    long: list[Site]
    listed: list[Site]
    assertions: list[Site]
    missing: list[Site]

    def rows(self) -> list[Site]:
        """Every row under the name --compare counts it by."""
        long = [
            Site(s.path, s.line, f"long {s.kind}", s.detail, s.key, s.amount)
            for s in self.long
        ]
        return [*self.missing, *self.sites, *long, *self.listed, *self.assertions]


def rust_sources(revision: str) -> dict[str, str]:
    """Every tracked Rust file under rust/ at revision."""
    return {
        path: data.decode()
        for path, data in archive(revision, "rust").items()
        if path.endswith(".rs")
    }


def missing_rows(
    revision: str, selected: set[str] | None, tests: set[str]
) -> tuple[int, list[Site]]:
    """The matched bodies' count and the bodies that lack original comment lines."""
    pairs = [
        (r, o)
        for r, o in comment_pairs(revision, tests)
        if selected is None or r.path in selected
    ]
    rows = [
        Site(
            r.path,
            r.line,
            "missing comment lines",
            f"{o.comments - r.comments} ({r.name}; original {o.path}:{o.line} {o.name})",
            r.scope,
            o.comments - r.comments,
        )
        for r, o in pairs
        if o.comments > r.comments
    ]
    return len(pairs), sorted(rows, key=lambda s: (s.path, s.line))


def measure(
    revision: str, tree: dict[str, str], selected: set[str] | None, shown: bool
) -> Measure:
    """The inventory of selected files (None for the whole tree) at revision.

    Shown, it prints each named file's rows as they are found and the
    assertion line before the slow comment pairing.
    """
    rust = {
        path: source for path, source in tree.items() if path.startswith(f"{RUST}/")
    }
    measured = [path for path in sorted(rust) if selected is None or path in selected]
    tests = test_modules(tree)
    rows = shown and selected is not None
    sites, long = file_rows(rust, measured, tests, rows)
    listed = (
        listed_rows(rust, set(measured) - tests, tests, rows)
        if selected is not None
        else []
    )
    assertions = [
        site
        for path in sorted(tree)
        if (selected is None or path in selected) and path not in tests
        for site in assertion_sites(path, tree[path])
    ]
    if shown:
        for site in assertions if rows else []:
            show_row(site)
        print(assertion_line(revision, assertions), flush=True)
    pairs, missing = missing_rows(revision, selected, tests)
    if rows:
        for site in missing:
            show_row(site)
    return Measure(revision, pairs, sites, long, listed, assertions, missing)


def totals_line(found: Measure) -> str:
    counts = {
        kind: sum(s.kind == kind for s in found.sites)
        for kind in ("bare integer", "wrapper", "constant", "literal")
    }
    long = found.long
    return (
        f"Code-standards inventory at {found.revision}: {found.pairs} matched bodies, "
        f"{sum(s.amount for s in found.missing)} missing comment lines, "
        f"{counts['bare integer']} bare-integer sites, {counts['wrapper']} wrappers, "
        f"{counts['constant']} named constants, {counts['literal']} literal sites, "
        f"{len(long)} functions over {LONG_PARAMETERS} parameters "
        f"({sum(s.kind == 'ffi export' for s in long)} FFI exports, "
        f"{sum(s.kind == 'helper' for s in long)} helpers, "
        f"{sum(s.kind == 'command' for s in long)} commands)"
    )


# --compare's rows, in table order; missing comment lines count their lines.
COMPARED = (
    "missing comment lines",
    "bare integer",
    "wrapper",
    "constant",
    "literal",
    "long ffi export",
    "long helper",
    "long command",
    "raw adapter call",
    "export call",
    "execute flag",
    "scratch pool",
    "one-call service",
    "unsuffixed adapter",
    "hand-built flag bit",
    "integer flag set",
    "assertion macro",
    "const assertion",
)


def allowed_lints(attribute: Node, data: bytes) -> set[str]:
    """The lints an `allow(...)` attribute names: its argument list's own
    identifiers, not those inside comments, strings or a `reason`, and not
    the second half of a tool path such as `clippy::x`."""
    kinds = [child for child in attribute.named_children if child.type not in COMMENTS]
    if len(kinds) != 2 or text(kinds[0], data) != "allow":
        return set()
    names = set()
    for child in kinds[1].children:
        before = child.prev_sibling
        if child.type == "identifier" and (before is None or before.type != "::"):
            names.add(text(child, data))
    return names


def module_unsafe_allows(path: str, source: str) -> list[Site]:
    """Each `#![allow(...)]` at the top level of a file that names `unsafe_code`.

    The parser reads the attributes, so comments and the inner attributes of
    an inline `mod` do not count, and spacing such as `# ! [allow(...)]` does.
    """
    data = source.encode()
    rows = []
    for node in PARSERS["rust"].parse(data).root_node.named_children:
        if node.type != "inner_attribute_item":
            continue
        attributes = [c for c in node.named_children if c.type == "attribute"]
        if attributes and "unsafe_code" in allowed_lints(attributes[0], data):
            rows.append(
                Site(path, node.start_point[0] + 1, "module unsafe_code allow", "")
            )
    return rows


def count(rows: list[Site], kind: str) -> int:
    chosen = [row for row in rows if row.kind == kind]
    return (
        sum(row.amount for row in chosen)
        if kind == "missing comment lines"
        else len(chosen)
    )


def changed_rows(
    before: list[Site], after: list[Site]
) -> list[tuple[Site, Site | None]]:
    """Head rows of one file with no base row of the same kind and identity, and
    those whose amount grew, paired with the base row they grew from.

    Lines never pair rows, so an occurrence that only moved is not new, and one
    replaced by another of a new name is new however the counts compare.
    """
    unpaired: dict[tuple[str, str], list[Site]] = defaultdict(list)
    for row in before:
        unpaired[row.kind, row.identity].append(row)
    found: list[tuple[Site, Site | None]] = []
    for row in after:
        same = unpaired[row.kind, row.identity]
        if not same:
            found.append((row, None))
            continue
        base = same.pop(0)
        if row.amount > base.amount:
            found.append((row, base))
    return found


def compared_files(base: str, head: str, renames: dict[str, str]) -> list[str]:
    """Finished files at head that the base list lacks or whose text changed."""
    listed = set(finished_files(base))
    return [
        path
        for path in finished_files(head)
        if path not in listed or show(base, renames.get(path, path)) != show(head, path)
    ]


def compare(base_ref: str, head: str, files: list[str]) -> int:
    """Print the inventory of a pass's files at the merge base and at head, and
    the rows new or grown at head. It reports and gates nothing."""
    base = subprocess.check_output(
        ["git", "merge-base", base_ref, head], cwd=ROOT, text=True
    ).strip()
    renames = renamed_from(base, head, "rust")
    selected = files or compared_files(base, head, renames)
    after_tree = rust_sources(head)
    if unknown := set(selected) - set(after_tree):
        sys.exit(f"not a Rust source at {head}: {', '.join(sorted(unknown))}")
    if not selected:
        print(f"No finished file changed between {base} and {head}.")
        return 0
    paths = {path: renames.get(path, path) for path in selected}
    before_tree = rust_sources(base)
    before = measure(
        base, before_tree, set(paths.values()) & set(before_tree), False
    ).rows()
    after = measure(head, after_tree, set(selected), False).rows()
    print(f"Inventory at the merge base {base} and at {head} of:")
    for path in sorted(paths):
        renamed = f" (at the base {paths[path]})" if paths[path] != path else ""
        print(f"- {path}{renamed}")
    print("\n| Row | Base | Head |\n| --- | ---: | ---: |")
    for kind in COMPARED:
        old, new = count(before, kind), count(after, kind)
        if old or new:
            print(f"| {kind} | {old} | {new} |")
    finishing = set(finished_files(head)) - set(finished_files(base))
    allows = [
        row
        for path in sorted(set(selected) & finishing)
        for row in module_unsafe_allows(path, after_tree[path])
    ]
    if allows:
        print(
            f"\n{len(allows)} module-scope `unsafe_code` allows in files this branch finishes:"
        )
        for row in allows:
            print(f"{row.path}:{row.line}: {row.kind}")
    changed = [
        pair
        for path in sorted(paths)
        for pair in sorted(
            changed_rows(
                [row for row in before if row.path == paths[path]],
                [row for row in after if row.path == path],
            ),
            key=lambda pair: pair[0].line,
        )
    ]
    print(f"\n{len(changed)} rows new or grown at the head:")
    for row, grown in changed:
        was = f" (grown from {grown.amount})" if grown else ""
        print(f"{row.path}:{row.line}: {row.kind}: {row.detail}{was}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument(
        "--head", default="HEAD", help="revision to measure (default HEAD)"
    )
    parser.add_argument(
        "files", nargs="*", help="print these Rust files' residue instead of totals"
    )
    parser.add_argument(
        "--gate",
        metavar="BASE_REF",
        help=f"fail on new residue in the files {FINISHED} lists, against the merge base with BASE_REF",
    )
    parser.add_argument(
        "--compare",
        metavar="BASE_REF",
        help=f"compare the files given, or else the changed files {FINISHED} lists, at the merge base with BASE_REF and at the head",
    )
    parser.add_argument(
        "--worksets",
        action="store_true",
        help=f"print #734's counts from {WORKSETS}: each workset's finished files and lines left, and the files in no workset",
    )
    args = parser.parse_args()
    files = [str(PurePosixPath(path)) for path in args.files]
    if args.worksets:
        return worksets(args.head)
    if args.gate:
        return gate(args.gate, args.head)
    if args.compare:
        return compare(args.compare, args.head, files)
    tree = rust_sources(args.head)
    if unknown := set(files) - set(tree):
        sys.exit(f"not a Rust source at {args.head}: {', '.join(sorted(unknown))}")
    found = measure(args.head, tree, set(files) or None, True)
    print(totals_line(found))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
