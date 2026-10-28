#!/usr/bin/env python3
"""Print the roadmap progress metric for the current branch.

Counts, from the merge base with the target branch to HEAD (comments and
blank lines included):

  rust     lines added under rust/
  tooling  lines added under tools/
  glue     fork-added src/ lines newly compiled when WITH_RUST is defined
  retired  change in retired lines of the pinned original, excluding fork glue
  net_glue change in compiled fork-added src/ lines against the pinned original
  glue_removed  fork-added lines removed or made portable-only

--whole-fork measures totals against the pinned original and the line share of
simulation sources retired. Comments, blank lines and data tables count; this is
a source-line measure, not a claim of complete behavior or subsystem coverage.

Both guard forms are understood: `#ifdef WITH_RUST ... #else <original>
#endif` and `#ifndef WITH_RUST <original> #endif` (and the `defined()` forms).
"""

import argparse
import json
import re
import subprocess
from pathlib import Path

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
    tunnel tunnelbridge vehicle void water waypoint bridge""".split()
)
DIRECTIVE = re.compile(r"^\s*#\s*(if|ifdef|ifndef|elif|else|endif)\b(.*)")
POSITIVE = re.compile(r"^\s*(?:defined\s*\(?\s*WITH_RUST\s*\)?|WITH_RUST)\s*$")
NEGATIVE = re.compile(r"^\s*!\s*defined\s*\(?\s*WITH_RUST\s*\)?\s*$")


def git(*args):
    return subprocess.check_output(
        ["git", *args], cwd=ROOT, text=True, errors="replace"
    )


def classify(text):
    """Tag every line 'cpp' (only without WITH_RUST), 'rust' or 'common'."""
    stack, tags = [], []
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
            stack.append([frame, False])
            tags.append("rust" if frame else state(stack[:-1]))
            continue
        if kind in ("elif", "else") and stack:
            frame, switched = stack[-1]
            tags.append("rust" if frame else state(stack[:-1]))
            if frame and not switched:
                # Every later branch of a WITH_RUST conditional is the other side.
                stack[-1] = ["cpp" if frame == "rust" else "rust", True]
            continue
        if kind == "endif" and stack:
            frame, _ = stack.pop()
            tags.append("rust" if frame else state(stack))
            continue
        tags.append(state(stack))
    return tags


def state(stack):
    frames = [frame for frame, _ in stack]
    if "cpp" in frames:
        return "cpp"
    return "rust" if "rust" in frames else "common"


def show(rev, path):
    result = subprocess.run(
        ["git", "show", f"{rev}:{path}"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        errors="replace",
    )
    return result.stdout if result.returncode == 0 else ""


def hunks(base, head, old, new):
    """Yield (old start, old count, new start, new count) of a zero-context diff."""
    for line in git(
        "diff", "-U0", "-M", base, head, "--", *dict.fromkeys((old, new))
    ).splitlines():
        hunk = re.match(r"^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@", line)
        if hunk:
            yield tuple(
                int(value) if value is not None else 1 for value in hunk.groups()
            )


def added_lines(changes):
    """Return 1-based new-side line numbers added by the hunks."""
    return [n for _, _, start, count in changes for n in range(start, start + count)]


def surviving(changes, length):
    """Map each unchanged old-side line number to its new-side line number."""
    removed, shift, mapping = set(), [], {}
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


def changes(base, head):
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


def simulation_source(path):
    """The single source inventory for the whole-fork simulation denominator."""
    name = Path(path).stem
    return "gui" not in name.split("_") and (
        path.startswith(("src/linkgraph/", "src/pathfinder/"))
        or path in {
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


def source_metrics(base, head="HEAD", glue_lines=None):
    """Raw diff counts; optionally collect compiled added-line identities."""
    result = {"rust": 0, "tooling": 0, "glue": 0, "retired": 0, "simulation": 0}
    sources = (".cpp", ".h", ".hpp", ".c", ".cc", ".mm")
    for status, old, new in changes(base, head):
        before, after = (
            ("" if status == "A" else show(base, old)),
            ("" if status == "D" else show(head, new)),
        )
        diff = [] if status == "D" else list(hunks(base, head, old, new))
        # Lines of `new` that do not come from `old` (all of them for an added file).
        numbers = added_lines(diff)
        if new.startswith(("rust/", "tools/")) and status != "D":
            result["rust" if new.startswith("rust/") else "tooling"] += len(numbers)
        if not (old.startswith("src/") or new.startswith("src/")) or not new.endswith(
            sources
        ):
            continue
        head_tags, base_tags = classify(after), classify(before)
        kept = {} if status == "D" else surviving(diff, len(base_tags))
        # Original lines compiled with WITH_RUST that are now removed or C++-only.
        retired = sum(
            1
            for n, tag in enumerate(base_tags, 1)
            if tag == "common" and (n not in kept or head_tags[kept[n] - 1] == "cpp")
        )
        result["retired"] += retired
        if simulation_source(old):
            result["simulation"] += retired
        if status == "D":
            continue
        added_glue = {
            n for n in numbers if n <= len(head_tags) and head_tags[n - 1] != "cpp"
        }
        result["glue"] += len(added_glue)
        if glue_lines is not None:
            glue_lines[new] = added_glue
    return result


def metrics(base, head="HEAD", whole_fork=False):
    result = source_metrics(base, head)
    before_glue, after_glue = {}, {}
    before = source_metrics(BASELINE, base, before_glue)
    after = source_metrics(BASELINE, head, after_glue)
    result["retired"] = after["retired"] - before["retired"]
    result["net_glue"] = after["glue"] - before["glue"]
    result["glue"], result["glue_removed"] = 0, 0
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
        total = sum(
            len(classify(show(BASELINE, path)))
            for path in paths
            if simulation_source(path) and path.endswith((".cpp", ".h", ".hpp"))
        )
        result.update(
            simulation_retired=after["simulation"],
            simulation_total=total,
            simulation_percent=100 * after["simulation"] / total,
        )
    return result


def main():
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
    args = parser.parse_args()
    base = (
        BASELINE
        if args.whole_fork
        else git("merge-base", args.target, args.head).strip()
    )
    result = metrics(base, args.head, args.whole_fork)
    if args.json:
        print(json.dumps(dict(result, base=base)))
    else:
        print(
            f"Rust added: {result['rust']} | tooling added: {result['tooling']} | "
            f"C++ glue added: {result['glue']} | original C++ retired: {result['retired']} | "
            f"fork glue removed: {result['glue_removed']} | net glue: {result['net_glue']}"
        )
        if args.whole_fork:
            print(
                f"Simulation original C++ retired: {result['simulation_retired']} / "
                f"{result['simulation_total']} ({result['simulation_percent']:.2f}%)"
            )


if __name__ == "__main__":
    main()
