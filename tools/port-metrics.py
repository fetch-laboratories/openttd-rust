#!/usr/bin/env python3
"""Print the roadmap progress metric for the current branch.

Reports code lines (Pygments 2.19.2, excluding comments, blank and preprocessor
lines), then physical-line totals for comparison with historical metrics.
--whole-fork adds original/retired/remaining/glue by area, compiled C++ distance,
whole-fork and simulation shares, and Rust exports/service callback slots.
--functions [AREA ...] adds the remaining lines per file and top-level function.
Glue includes the C++ record declarations tools/generate-abi.py generates.
This is a source/guard inventory, not full preprocessing or behavioral coverage.

Both guard forms are understood: `#ifdef WITH_RUST ... #else <original>
#endif` and `#ifndef WITH_RUST <original> #endif` (and the `defined()` forms).
"""

import argparse
import io
import json
import re
import runpy
import subprocess
import tarfile
import tempfile
from collections.abc import Iterator, Sequence
from functools import lru_cache
from pathlib import Path
from typing import NotRequired, TypedDict

from code_lines import (
    RUST_EXPORT,
    Comment,
    String,
    code_lines,
    tokens,
    without_comments,
)

ROOT = Path(__file__).resolve().parents[1]
BASELINE = json.loads((ROOT / "migration/baseline.json").read_text())["commit"]
# World-state owners and their commands/accessors; excludes GUI, platform, vendor
# and generic support. Match complete family names, not arbitrary substrings.
SIMULATION_FAMILIES = frozenset(
    """aircraft airport animated_tile articulated_vehicles autoreplace base_consist
    base_station cargoaction cargomonitor cargopacket cargotype clear company depot
    disaster_vehicle economy effectvehicle engine genworld ground_vehicle group
    heightmap house industry industrytype landscape map newgrf_airport
    newgrf_airporttiles newgrf_canal newgrf_cargo newgrf_engine newgrf_generic
    newgrf_house newgrf_industries newgrf_industrytiles newgrf_object newgrf_railtype
    newgrf_roadstop newgrf_roadtype newgrf_station newgrf_storage newgrf_town
    newgrf_townname object openttd order pbs rail road roadstop roadveh ship signal slope
    station subsidy terraform tgp tile tilearea timetable town townname train tree
    tunnel tunnelbridge vehicle void water waypoint bridge command cheat goal story
    league signs misc_cmd vehiclelist elrail autoslope""".split()
)
DIRECTIVE = re.compile(r"^\s*#\s*(if|ifdef|ifndef|elif|else|endif)\b(.*)")
POSITIVE = re.compile(r"^\s*(?:defined\s*\(?\s*WITH_RUST\s*\)?|WITH_RUST)\s*$")
NEGATIVE = re.compile(r"^\s*!\s*defined\s*\(?\s*WITH_RUST\s*\)?\s*$")


class AreaMetrics(TypedDict):
    original: int
    retired: int
    glue: int
    remaining: NotRequired[int]
    compiled: NotRequired[int]


class BoundaryMetrics(TypedDict):
    exports: int
    callback_slots: int


class FunctionRow(TypedDict):
    area: str
    path: str
    function: str
    line: int
    remaining: int
    facade: bool


class Metrics(TypedDict, total=False):
    rust: int
    tooling: int
    glue: int
    retired: int
    simulation: int
    net_glue: int
    glue_removed: int
    original_total: int
    whole_fork_retired: int
    whole_fork_percent: float
    simulation_retired: int
    simulation_total: int
    simulation_percent: float
    compiled_cpp: int
    compiled_percent: float
    areas: dict[str, AreaMetrics]
    totals: AreaMetrics
    boundary: BoundaryMetrics
    all_lines: "Metrics"
    functions: list[FunctionRow]


Hunk = tuple[int, int, int, int]
Guard = tuple[str | None, bool]


def git(*args: str) -> str:
    return subprocess.check_output(
        ["git", *args], cwd=ROOT, text=True, errors="replace"
    )


def classify(text: str) -> list[str]:
    """Tag every line 'cpp' (only without WITH_RUST), 'rust' or 'common'."""
    stack: list[Guard] = []
    tags: list[str] = []
    # Split only on "\n", as git numbers lines, so tags line up with diff hunks.
    lines = text.split("\n")
    if lines[-1] == "":
        lines.pop()
    for line in lines:
        match = DIRECTIVE.match(line)
        kind, rest = (
            (
                match.group(1),
                re.sub(r"/\*.*?\*/", " ", match.group(2)).split("//")[0].strip(),
            )
            if match
            else (None, "")
        )
        if kind in ("if", "ifdef", "ifndef"):
            if (kind == "ifdef" and rest == "WITH_RUST") or (
                kind == "if" and POSITIVE.match(rest)
            ):
                frame = "rust"
            elif (kind == "ifndef" and rest == "WITH_RUST") or (
                kind == "if" and NEGATIVE.match(rest)
            ):
                frame = "cpp"
            else:
                frame = None
            stack.append((frame, False))
            tags.append("rust" if frame else state(stack[:-1]))
            continue
        if kind in ("elif", "else") and stack:
            frame, switched = stack[-1]
            tags.append("rust" if frame else state(stack[:-1]))
            if frame and not switched:
                # Every later branch of a WITH_RUST conditional is the other side.
                stack[-1] = ("cpp" if frame == "rust" else "rust", True)
            continue
        if kind == "endif" and stack:
            frame, _ = stack.pop()
            tags.append("rust" if frame else state(stack))
            continue
        tags.append(state(stack))
    return tags


def state(stack: list[Guard]) -> str:
    frames = [frame for frame, _ in stack]
    if "cpp" in frames:
        return "cpp"
    return "rust" if "rust" in frames else "common"


@lru_cache(maxsize=512)
def show(rev: str, path: str) -> str:
    result = subprocess.run(
        ["git", "show", f"{rev}:{path}"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        errors="replace",
    )
    return result.stdout if result.returncode == 0 else ""


def hunks(base: str, head: str, old: str, new: str) -> Iterator[Hunk]:
    """Yield (old start, old count, new start, new count) of a zero-context diff."""
    for line in git(
        "diff", "-U0", "-M", base, head, "--", *dict.fromkeys((old, new))
    ).splitlines():
        hunk = re.match(r"^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@", line)
        if hunk:
            a, b, c, d = (
                int(value) if value is not None else 1 for value in hunk.groups()
            )
            yield a, b, c, d


def added_lines(changes: Sequence[Hunk]) -> list[int]:
    """Return 1-based new-side line numbers added by the hunks."""
    return [n for _, _, start, count in changes for n in range(start, start + count)]


def surviving(changes: Sequence[Hunk], length: int) -> dict[int, int]:
    """Map each unchanged old-side line number to its new-side line number."""
    removed: set[int] = set()
    shift: list[tuple[int, int]] = []
    mapping: dict[int, int] = {}
    for old_start, old_count, _new_start, new_count in changes:
        removed.update(range(old_start, old_start + old_count))
        shift.append(
            (
                old_start + old_count - 1 if old_count else old_start,
                new_count - old_count,
            )
        )
    for n in range(1, length + 1):
        if n not in removed:
            mapping[n] = n + sum(delta for end, delta in shift if end < n)
    return mapping


def changes(base: str, head: str) -> Iterator[tuple[str, str, str]]:
    """Yield (status, old path, new path) with renames paired, NUL-safe."""
    fields = git("diff", "--name-status", "-z", "-M", base, head).split("\0")
    i = 0
    while i < len(fields) - 1:
        status = fields[i]
        if status[0] in "RC":
            yield status[0], fields[i + 1], fields[i + 2]
            i += 3
        else:
            yield status[0], fields[i + 1], fields[i + 1]
            i += 2


def simulation_source(path: str) -> bool:
    """The single source inventory for the whole-fork simulation denominator."""
    name = Path(path).stem
    return "gui" not in name.split("_") and (
        path.startswith(("src/linkgraph/", "src/pathfinder/"))
        or path
        in {
            "src/timer/timer_game_calendar.cpp",
            "src/timer/timer_game_calendar.h",
            "src/timer/timer_game_economy.cpp",
            "src/timer/timer_game_economy.h",
            "src/timer/timer_game_common.cpp",
            "src/timer/timer_game_common.h",
        }
        or (
            path.startswith("src/")
            and path.count("/") == 1
            and any(
                name == family or name.startswith(family + "_")
                for family in SIMULATION_FAMILIES
            )
        )
    )


SOURCES = (".cpp", ".h", ".hpp", ".c", ".cc", ".mm")
AREAS = (
    "Simulation logic",
    "Simulation storage",
    "NewGRF",
    "Save/load",
    "Script API",
    "Network",
    "Support",
    "Graphics and viewport",
    "GUI",
    "Platform",
    "Data tables",
    "Vendored",
    "Upstream tests",
    "Boundary layer",
)


def area(path: str) -> str:
    """Disjoint source inventory; directory-specific areas precede owner families."""
    name = Path(path).stem
    for prefix, label in (
        ("src/rust/", "Boundary layer"),
        ("src/3rdparty/", "Vendored"),
        ("src/tests/", "Upstream tests"),
        ("src/table/", "Data tables"),
        ("src/saveload/", "Save/load"),
        ("src/script/", "Script API"),
        ("src/network/", "Network"),
    ):
        if path.startswith(prefix):
            return label
    if (
        "gui" in name.split("_")
        or path.startswith("src/widgets/")
        or name.startswith(("widget", "window", "querystring"))
    ):
        return "GUI"
    if path.startswith(
        tuple("src/" + p + "/" for p in ("os", "video", "music", "sound", "driver"))
    ):
        return "Platform"
    if path.startswith(("src/ai/", "src/game/")):
        return "Script API"
    if name in {"driver", "crashlog"}:
        return "Platform"
    if simulation_source(path):
        return (
            "Simulation storage"
            if path.endswith((".h", ".hpp")) or name == "map" or name.endswith("_map")
            else "Simulation logic"
        )
    if name.startswith("newgrf"):
        return "NewGRF"
    if name.startswith(
        (
            "gfx",
            "viewport",
            "sprite",
            "palette",
            "blitter",
            "zoom",
            "fontcache",
            "screenshot",
        )
    ) or path.startswith(("src/blitter/", "src/fontcache/")):
        return "Graphics and viewport"
    return "Support"


def boundary(head: str) -> BoundaryMetrics:
    exports = slots = 0
    for path in git("ls-tree", "-r", "--name-only", head, "rust/").splitlines():
        if not path.endswith(".rs"):
            continue
        text = without_comments(show(head, path), path, strings=True)
        exports += len(RUST_EXPORT.findall(text))
        # Function-pointer fields in boundary records, including private fields,
        # nested services and Option callbacks; function parameters don't count.
        for body in re.findall(r"\bstruct\s+\w+\s*\{([^{}]*)\}", text, re.S):
            slots += len(
                re.findall(
                    r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?\w+\s*:\s*"
                    r'(?:Option\s*<\s*)?(?:unsafe\s+)?extern\s+"C"\s+fn\s*\(',
                    body,
                )
            )
    return dict(exports=exports, callback_slots=slots)


def source_metrics(
    base: str,
    head: str = "HEAD",
    glue_lines: dict[str, set[int]] | None = None,
    all_lines: bool = False,
    areas: dict[str, AreaMetrics] | None = None,
    retired_lines: dict[str, set[int]] | None = None,
) -> Metrics:
    """Raw diff counts; optionally collect compiled added and retired lines."""
    result: Metrics = {
        "rust": 0,
        "tooling": 0,
        "glue": 0,
        "retired": 0,
        "simulation": 0,
    }
    sources = (".cpp", ".h", ".hpp", ".c", ".cc", ".mm")
    for status, old, new in changes(base, head):
        if not new.startswith(("rust/", "tools/")) and not (
            (old.startswith("src/") or new.startswith("src/")) and new.endswith(sources)
        ):
            continue
        before, after = (
            ("" if status == "A" else show(base, old)),
            ("" if status == "D" else show(head, new)),
        )
        before_code = (
            set(range(1, len(classify(before)) + 1))
            if all_lines
            else code_lines(before, old)
        )
        after_code = (
            set(range(1, len(classify(after)) + 1))
            if all_lines
            else code_lines(after, new)
        )
        diff = [] if status == "D" else list(hunks(base, head, old, new))
        # Lines of `new` that do not come from `old` (all of them for an added file).
        numbers = added_lines(diff)
        if new.startswith(("rust/", "tools/")) and status != "D":
            result["rust" if new.startswith("rust/") else "tooling"] += len(
                set(numbers) & after_code
            )
        if not (old.startswith("src/") or new.startswith("src/")) or not new.endswith(
            sources
        ):
            continue
        head_tags, base_tags = classify(after), classify(before)
        kept = {} if status == "D" else surviving(diff, len(base_tags))
        # Original lines compiled with WITH_RUST that are now removed or C++-only.
        gone = {
            n
            for n, tag in enumerate(base_tags, 1)
            if n in before_code
            and tag == "common"
            and (n not in kept or head_tags[kept[n] - 1] == "cpp")
        }
        retired = len(gone)
        if retired_lines is not None:
            retired_lines.setdefault(old, set()).update(gone)
        result["retired"] += retired
        if simulation_source(old):
            result["simulation"] += retired
        if areas is not None:
            areas[area(old)]["retired"] += retired
        if status == "D":
            continue
        added_glue = {
            n for n in numbers if n in after_code and head_tags[n - 1] != "cpp"
        }
        result["glue"] += len(added_glue)
        if areas is not None:
            areas[area(new)]["glue"] += len(added_glue)
        if glue_lines is not None:
            glue_lines[new] = added_glue
    return result


# Generated declarations are compiled glue in the boundary layer's area.
GENERATED_AREA = "Boundary layer"


@lru_cache(maxsize=8)
def generated_glue(rev: str, all_lines: bool = False) -> int:
    """Lines of the C++ record declarations tools/generate-abi.py emits at `rev`."""
    paths = ["rust/openttd-kernels/src", "tools/generate-abi.py"]
    listed = git("ls-tree", "--name-only", rev, "--", *paths).split()
    if len(listed) != len(paths):
        return 0
    archive = subprocess.check_output(["git", "archive", rev, "--", *paths], cwd=ROOT)
    with tempfile.TemporaryDirectory() as directory:
        with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
            tar.extractall(directory, filter="data")
        generator = runpy.run_path(str(Path(directory) / "tools/generate-abi.py"))
        if "shared_records" not in generator:
            return 0
        fragments = generator["generate"](Path(directory))
    total = 0
    for name, text in fragments.items():
        if name.startswith("abi-record-"):
            count = len(classify(text)) if all_lines else len(code_lines(text, name))
            total += count
    return total


def metrics(
    base: str,
    head: str = "HEAD",
    whole_fork: bool = False,
    all_lines: bool = False,
    retired_lines: dict[str, set[int]] | None = None,
) -> Metrics:
    result = source_metrics(base, head, all_lines=all_lines)
    before_glue: dict[str, set[int]] = {}
    after_glue: dict[str, set[int]] = {}
    before = source_metrics(BASELINE, base, before_glue, all_lines=all_lines)
    areas: dict[str, AreaMetrics] = {
        name: dict(original=0, retired=0, glue=0) for name in AREAS
    }
    after = source_metrics(BASELINE, head, after_glue, all_lines, areas, retired_lines)
    generated_before = generated_glue(base, all_lines)
    generated_after = generated_glue(head, all_lines)
    after["glue"] += generated_after
    areas[GENERATED_AREA]["glue"] += generated_after
    result["retired"] = after["retired"] - before["retired"]
    result["net_glue"] = after["glue"] - before["glue"] - generated_before
    result["glue"] = max(generated_after - generated_before, 0)
    result["glue_removed"] = max(generated_before - generated_after, 0)
    # Compare fork line identities, excluding restored original lines and
    # including unchanged fork lines whose WITH_RUST compilation state changes.
    for status, old, new in changes(base, head):
        old_lines, new_lines = before_glue.get(old, set()), after_glue.get(new, set())
        if not old_lines and not new_lines:
            continue
        kept = (
            {}
            if status in ("A", "D")
            else surviving(
                list(hunks(base, head, old, new)), len(classify(show(base, old)))
            )
        )
        retained = {kept[n] for n in old_lines if n in kept and kept[n] in new_lines}
        result["glue"] += len(new_lines - retained)
        result["glue_removed"] += len(old_lines) - len(retained)
    result.pop("simulation")
    if whole_fork:
        paths = git("ls-tree", "-r", "--name-only", BASELINE, "src/").splitlines()
        total = simulation_total = 0
        for path in paths:
            if not path.endswith(SOURCES):
                continue
            text = show(BASELINE, path)
            count = len(classify(text)) if all_lines else len(code_lines(text, path))
            areas[area(path)]["original"] += count
            total += count
            if simulation_source(path):
                simulation_total += count
        for row in areas.values():
            row["remaining"] = row["original"] - row["retired"]
            row["compiled"] = row["remaining"] + row["glue"]
        compiled = sum(row["compiled"] for row in areas.values())
        result.update(
            original_total=total,
            whole_fork_retired=after["retired"],
            whole_fork_percent=100 * after["retired"] / total if total else 0,
            simulation_retired=after["simulation"],
            simulation_total=simulation_total,
            simulation_percent=100 * after["simulation"] / simulation_total
            if simulation_total
            else 0,
            compiled_cpp=compiled,
            compiled_percent=100 * compiled / total if total else 0,
            areas=areas,
            totals=dict(
                original=total,
                retired=after["retired"],
                remaining=total - after["retired"],
                glue=after["glue"],
                compiled=compiled,
            ),
            boundary=boundary(head),
        )
    return result


def spans(text: str, path: str) -> list[tuple[str, int, int, int]]:
    """Heuristic top-level functions: (name, first, opening brace, last line)."""
    masked = "".join(
        re.sub(r"[^\n]", " ", value) if kind in Comment or kind in String else value
        for kind, value in tokens(text, path)
    )
    found: list[tuple[str, int, int, int]] = []
    header, start, opened, inner, init, kind = "", 0, 0, 0, 0, ""
    for n, line in enumerate(masked.split("\n"), 1):
        if (
            not inner
            and not init
            and line.strip() in ("public:", "protected:", "private:")
        ):
            header = ""
            continue
        if not inner and not init and not line.strip():
            header = ""  # Blank, comment or directive lines end a header.
        for ch in line:
            if inner:
                inner += {"{": 1, "}": -1}.get(ch, 0)
                if not inner and kind:
                    found.append((kind, start, opened, n))
                continue
            # Braces inside arguments and constructor member initialisers.
            if init or (
                ch == "{"
                and (
                    header.count("(") > header.count(")")
                    or re.search(r"\)\s*:(?!:)", header)
                    and re.search(r"[\w>]\s*$", header)
                )
            ):
                init += {"{": 1, "}": -1}.get(ch, 0)
                header += ch
                continue
            if ch == "{":
                pre = header.split("(")[0]
                if (
                    "(" not in header
                    and re.search(
                        r"\b(?:namespace|class|struct|union|extern)\b", header
                    )
                    and not re.search(r"\benum\b|=", header)
                ):
                    pass  # Namespaces, classes and linkage blocks are transparent.
                elif "(" not in header or re.search(
                    r"=|\b(?:class|struct|union|enum)\b",
                    re.sub(r"\boperator\b.*", "", pre, flags=re.S),
                ):
                    inner, kind = 1, ""
                else:
                    name = re.search(
                        r"((?:[\w~]+\s*::\s*)*(?:operator\b.*|~?\w+))\s*$", pre, re.S
                    )
                    kind = re.sub(r"\s+", "", name.group(1)) if name else "?"
                    inner, opened = 1, n
                header = ""
            elif ch in ";}":
                header = ""
            else:
                if not header.strip() and not ch.isspace():
                    start = n
                header += ch
        header += "\n"
    return found


def function_rows(
    names: Sequence[str], retired: dict[str, set[int]]
) -> list[FunctionRow]:
    """Attribute each area's remaining original code lines to top-level functions."""
    rows: list[FunctionRow] = []
    for path in git("ls-tree", "-r", "--name-only", BASELINE, "src/").splitlines():
        if not path.endswith(SOURCES) or area(path) not in names:
            continue
        text = show(BASELINE, path)
        lines = without_comments(text, path, strings=True).split("\n")
        code = code_lines(text, path)
        left = code - retired.get(path, set())
        found: list[tuple[str, int, set[int], bool]] = []
        for name, first, opened, last in spans(text, path):
            body = set(range(first, last + 1))
            signature = set(range(first, opened))
            header = "\n".join(lines[first - 1 : opened])
            initializer = re.search(r"\)\s*:(?!:)\s*", header)
            if initializer:
                signature = set(
                    range(first, first + header[: initializer.end()].count("\n"))
                )
            if re.fullmatch(r"[^{}]*\{", lines[opened - 1].strip()) and (
                not initializer or lines[opened - 1].strip() == "{"
            ):
                signature.add(opened)
            if re.fullmatch(r"};?", lines[last - 1].strip()):
                signature.add(last)
            facade = bool(body & code - left) and body & left <= signature
            found.append((name, first, body & left, facade))
            left -= body
        found.append(("(file scope)", min(left, default=0), left, False))
        rows += [
            FunctionRow(
                area=area(path),
                path=path,
                function=name,
                line=line,
                remaining=len(lines),
                facade=facade,
            )
            for name, line, lines, facade in found
            if lines
        ]
    return rows


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument(
        "--target",
        default="origin/rust-migration",
        help="branch the PR targets (default: origin/rust-migration)",
    )
    parser.add_argument(
        "--head", default="HEAD", help="revision to measure (default: HEAD)"
    )
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--whole-fork", action="store_true")
    parser.add_argument(
        "--functions",
        nargs="*",
        choices=AREAS,
        metavar="AREA",
        help="list remaining lines per function for these areas (default: all); "
        "implies --whole-fork",
    )
    args = parser.parse_args()
    args.whole_fork |= args.functions is not None
    base = (
        BASELINE
        if args.whole_fork
        else git("merge-base", args.target, args.head).strip()
    )
    retired: dict[str, set[int]] = {}
    result = metrics(base, args.head, args.whole_fork, retired_lines=retired)
    if args.functions is not None:
        result["functions"] = function_rows(args.functions or AREAS, retired)
    result["all_lines"] = metrics(base, args.head, args.whole_fork, all_lines=True)
    if args.json:
        print(json.dumps(dict(result, base=base)))
    else:
        print(
            f"Code lines (Pygments 2.19.2): Rust added: {result['rust']} | tooling added: {result['tooling']} | "
            f"C++ glue added: {result['glue']} | original C++ retired: {result['retired']} | "
            f"fork glue removed: {result['glue_removed']} | net glue: {result['net_glue']}"
        )
        legacy = result["all_lines"]
        print(
            "All lines: "
            + " | ".join(
                f"{key}: {legacy[key]}"
                for key in (
                    "rust",
                    "tooling",
                    "glue",
                    "retired",
                    "glue_removed",
                    "net_glue",
                )
            )
        )
        if args.whole_fork:
            for label, data in (("Code", result), ("All lines", legacy)):
                print(
                    f"{label}: whole-fork retired {data['whole_fork_retired']} / {data['original_total']} ({data['whole_fork_percent']:.2f}%); "
                    f"simulation retired {data['simulation_retired']} / {data['simulation_total']} ({data['simulation_percent']:.2f}%)"
                )
            print(
                "Area | original | retired | remaining | glue | compiled C++ (code lines)"
            )
            for name, row in (*result["areas"].items(), ("TOTAL", result["totals"])):
                print(
                    name
                    + " | "
                    + " | ".join(
                        str(row[k])
                        for k in (
                            "original",
                            "retired",
                            "remaining",
                            "glue",
                            "compiled",
                        )
                    )
                )
            print(
                f"Compiled C++: {result['compiled_cpp']} / {result['original_total']} ({result['compiled_percent']:.2f}%)"
            )
            print(
                f"Boundary: {result['boundary']['exports']} exports | {result['boundary']['callback_slots']} callback slots"
            )
        if args.functions is not None:
            rows = result["functions"]
            print("Area | file:line | function | remaining | facade (code lines)")
            for row in sorted(rows, key=lambda row: -row["remaining"]):
                print(
                    f"{row['area']} | {row['path']}:{row['line']} | {row['function']} | "
                    f"{row['remaining']} | {'facade' if row['facade'] else ''}"
                )
            for name in args.functions or AREAS:
                mine = [row for row in rows if row["area"] == name]
                facades = [row["remaining"] for row in mine if row["facade"]]
                print(
                    f"{name}: {len(mine)} rows sum {sum(row['remaining'] for row in mine)}"
                    f" (remaining {result['areas'][name]['remaining']}); "
                    f"{len(facades)} facades, {sum(facades)} lines"
                )


if __name__ == "__main__":
    main()
