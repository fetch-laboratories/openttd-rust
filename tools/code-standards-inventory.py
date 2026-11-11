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
  (taking the command's `flags: u16`) and helpers (#874), outside test code
  and `#[cfg(test)]` modules;
- finished-file residue (#901): calls to `*_raw` adapters and `openttd_rust_*`
  exports outside export shims and test code, Rust-internal functions taking
  `execute: bool`, and `GameState<Vec<...>>` scratch pools.

With file paths it prints those files' residue instead of the totals. The Rust
and the candidate C++ come from the revision's archive, the original from the
baseline commit that the revision's own migration/baseline.json pins. Counts are estimates for stocktakes,
not a gate: matching is by facade, export and name, as the 2026-11-06 overhaul
measured them.

`--gate --base-ref REF` is the Quick gate (#901): it fails when a file listed in
migration/finished-files.txt has more sites of a residue class, or of commands
over seven parameters, than at the merge base, and names each new site.
"""

import argparse
import importlib
import io
import json
import re
import subprocess
import sys
import tarfile
from collections import Counter, defaultdict
from collections.abc import Iterator
from dataclasses import dataclass, field
from pathlib import Path, PurePosixPath
from typing import Protocol

ROOT = Path(__file__).resolve().parents[1]
RUST = "rust/openttd-kernels/src"
FINISHED = "migration/finished-files.txt"
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


def in_test(node: Node, data: bytes, path: str) -> bool:
    if "/tests/" in path or path.endswith("/tests.rs"):
        return True
    current: Node | None = node
    while current:
        name = current.child_by_field_name("name")
        if current.type == "mod_item" and text(name, data) in ("tests", "test"):
            return True
        if re.search(r"#\[(?:cfg\(test\)|test\])", leading(current, data)):
            return True
        current = current.parent
    return False


def comment_lines(body: Node, data: bytes) -> int:
    """Lines with comment content inside a body; delimiters alone do not count."""
    lines = set()
    for node in walk(body):
        if node.type not in COMMENTS:
            continue
        for offset, line in enumerate(text(node, data).splitlines()):
            content = re.sub(r"^\s*(?://[/!]?|/\*[*!]?|\*)\s?", "", line)
            if re.sub(r"\*/\s*$", "", content).strip():
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


def comment_pairs(head: str) -> list[tuple[Function, Function]]:
    baseline = baseline_commit(head)
    candidate = archive(head, RUST, "src")
    rust = [
        function
        for path, data in sorted(candidate.items())
        if path.startswith(RUST) and path.endswith(".rs")
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
    path: str
    line: int
    kind: str
    detail: str


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


def test_modules(rust: dict[str, str]) -> set[str]:
    """Paths of the module files that a `#[cfg(test)] mod name;` declares."""
    found = set()
    for path, source in rust.items():
        parent = PurePosixPath(path)
        if parent.name not in ("lib.rs", "mod.rs"):
            parent = parent.with_suffix("")
        else:
            parent = parent.parent
        for name in re.findall(r"#\[cfg\(test\)\]\s*(?:pub\S*\s+)?mod (\w+);", source):
            found |= {str(parent / f"{name}.rs"), str(parent / name / "mod.rs")}
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
        if exported(node, data):
            kind = "ffi export"
        elif any(
            p.type == "parameter"
            and text(p.child_by_field_name("pattern"), data) == "flags"
            and text(p.child_by_field_name("type"), data) == "u16"
            for p in parameters.named_children
        ):
            kind = "command"
        else:
            kind = "helper"
        found.append(
            Site(path, node.start_point[0] + 1, kind, f"{name}: {count} parameters")
        )
    return found


def exported(node: Node, data: bytes) -> bool:
    """An FFI export: `#[no_mangle]` or an `extern "C"` function."""
    head = text(node, data).split("fn ", 1)[0]
    return "no_mangle" in leading(node, data) or 'extern "C"' in head


def residue_sites(path: str, source: str) -> list[Site]:
    """#901's finished-file residue classes in one file, outside test code."""
    data = source.encode()
    found = []
    for node in walk(PARSERS["rust"].parse(data).root_node):
        line = node.start_point[0] + 1
        if node.type == "call_expression":
            called = text(node.child_by_field_name("function"), data)
            name = re.split(r"::|\.", re.sub(r"::<.*", "", called, flags=re.S))[-1]
            if not (name.endswith("_raw") or name.startswith("openttd_rust_")):
                continue
            owner = node.parent
            while owner and owner.type != "function_item":
                owner = owner.parent
            if (owner and exported(owner, data)) or in_test(node, data, path):
                continue
            kind = "export call" if name.startswith("openttd_rust_") else "raw call"
            found.append(Site(path, line, kind, name))
        elif node.type == "function_item":
            parameters = node.child_by_field_name("parameters")
            if parameters is None or exported(node, data) or in_test(node, data, path):
                continue
            if any(
                p.type == "parameter"
                and text(p.child_by_field_name("pattern"), data) == "execute"
                and text(p.child_by_field_name("type"), data) == "bool"
                for p in parameters.named_children
            ):
                name = text(node.child_by_field_name("name"), data)
                found.append(Site(path, line, "execute flag", name))
    code = code_only(source)
    for match in re.finditer(r"\bstatic\s+(\w+)\s*:\s*GameState\s*<\s*Vec\s*<", code):
        line = code.count("\n", 0, match.start()) + 1
        found.append(Site(path, line, "scratch pool", match[1]))
    return found


def gate_sites(path: str, source: str) -> list[Site]:
    """Residue and commands over seven parameters, keyed by identifier."""
    long = [
        Site(s.path, s.line, "long command", s.detail.split(":")[0])
        for s in long_functions(path, source)
        if s.kind == "command"
    ]
    return residue_sites(path, source) + long


def gate(base_ref: str) -> int:
    """Fail when a finished file gains a residue site over the merge base (#901).

    Sites are counted per file and class, so moving or renaming an existing
    occurrence passes; only growth fails, and it names the head sites whose
    identifier outnumbers the base's (all of the class's sites when none does).
    """
    base = subprocess.check_output(
        ["git", "merge-base", base_ref, "HEAD"], cwd=ROOT, text=True
    ).strip()
    listed = [
        line.strip()
        for line in (ROOT / FINISHED).read_text().splitlines()
        if line.strip() and not line.startswith("#")
    ]
    errors = 0
    for path in listed:
        if not (ROOT / path).is_file():
            print(f"{FINISHED}: listed file is missing: {path}")
            errors += 1
            continue
        before = subprocess.run(
            ["git", "show", f"{base}:{path}"], cwd=ROOT, capture_output=True, text=True
        )
        old = gate_sites(path, before.stdout) if before.returncode == 0 else []
        new = gate_sites(path, (ROOT / path).read_text())
        for kind in sorted({s.kind for s in new}):
            head = [s for s in new if s.kind == kind]
            allowed = Counter(s.detail for s in old if s.kind == kind)
            if len(head) <= sum(allowed.values()):
                continue
            seen: Counter[str] = Counter()
            added = []
            for site in head:
                seen[site.detail] += 1
                if seen[site.detail] > allowed[site.detail]:
                    added.append(site)
            for site in added or head:
                print(
                    f"{site.path}:{site.line}: new {site.kind} in a finished file: {site.detail}"
                )
                errors += 1
    print(f"Finished-file residue: {len(listed)} files; {errors} errors")
    return int(errors > 0)


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
        "--gate", action="store_true", help=f"fail on residue growth in {FINISHED}"
    )
    parser.add_argument("--base-ref", default="origin/rust-migration")
    args = parser.parse_args()
    if args.gate:
        return gate(args.base_ref)
    rust = {
        path: data.decode()
        for path, data in archive(args.head, RUST).items()
        if path.endswith(".rs")
    }
    selected = {str(PurePosixPath(path)) for path in args.files}
    unknown = selected - set(rust)
    if unknown:
        sys.exit(f"not a Rust source at {args.head}: {', '.join(sorted(unknown))}")
    pairs = [
        (r, o)
        for r, o in comment_pairs(args.head)
        if not selected or r.path in selected
    ]
    sites = [
        site
        for path, source in sorted(rust.items())
        if not selected or path in selected
        for site in type_sites(path, source)
    ]
    tests = test_modules(rust)
    long = [
        site
        for path, source in sorted(rust.items())
        if (not selected or path in selected) and path not in tests
        for site in long_functions(path, source)
    ]
    residue = [
        site
        for path, source in sorted(rust.items())
        if (not selected or path in selected) and path not in tests
        for site in residue_sites(path, source)
    ]
    missing = [
        (r, o, o.comments - r.comments) for r, o in pairs if o.comments > r.comments
    ]
    if selected:
        for r, o, deficit in sorted(missing, key=lambda m: (m[0].path, m[0].line)):
            print(
                f"{r.path}:{r.line}: missing comment lines: {deficit} ({r.name}; original {o.path}:{o.line} {o.name})"
            )
        for site in sorted(sites, key=lambda s: (s.path, s.line)):
            if site.kind in ("bare integer", "literal"):
                print(f"{site.path}:{site.line}: {site.kind}: {site.detail}")
        for site in long:
            print(f"{site.path}:{site.line}: long {site.kind}: {site.detail}")
        for site in residue:
            print(f"{site.path}:{site.line}: {site.kind}: {site.detail}")
    counts = {
        kind: sum(s.kind == kind for s in sites)
        for kind in ("bare integer", "wrapper", "constant", "literal")
    }
    print(
        f"Code-standards inventory at {args.head}: {len(pairs)} matched bodies, "
        f"{sum(m[2] for m in missing)} missing comment lines, "
        f"{counts['bare integer']} bare-integer sites, {counts['wrapper']} wrappers, "
        f"{counts['constant']} named constants, {counts['literal']} literal sites, "
        f"{len(long)} functions over {LONG_PARAMETERS} parameters "
        f"({sum(s.kind == 'ffi export' for s in long)} FFI exports, "
        f"{sum(s.kind == 'helper' for s in long)} helpers, "
        f"{sum(s.kind == 'command' for s in long)} commands), "
        + ", ".join(
            f"{sum(s.kind == kind for s in residue)} {kind}s"
            for kind in ("raw call", "export call", "execute flag", "scratch pool")
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
