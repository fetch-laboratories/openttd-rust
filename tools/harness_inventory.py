#!/usr/bin/env python3
"""Reject silent harness loss: scenarios, comparisons, cases, masks and keys (#672).

migration/harness-inventory.json records default and soak-only scenario names,
each comparison driver's case lines, MASKS and KNOWN_FAILURES. The tree must
match it, so every removal shows in the diff. Against --base-ref, MASKS gains no
entry and a new KNOWN_FAILURES entry names an open issue. A scenario key that no
tool reads fails. run-comparisons.py checks reported case counts at run time.

Native CTest case names are read from the registered src/tests sources at the
tree and at --base-ref: a case that disappears needs a ctest: record naming it
exactly, unless a source: record retires its whole file, and a new ctest: record
names a case that exists at --base-ref and no longer exists in the tree (#856).
"""

import argparse
import ast
import copy
import dataclasses
import json
import os
import re
import urllib.request
from collections.abc import Callable, Iterable, Mapping
from pathlib import Path
from typing import NotRequired, TypedDict

import evidence_replacements as replacements
import vcs

ROOT = Path(__file__).resolve().parents[1]
INVENTORY = "migration/harness-inventory.json"
COUNT = re.compile(r"\b\d+\b")
# Catch's CTest name is the TEST_CASE literal; TEST_CASE_METHOD names its fixture first.
CASE = re.compile(r"\bTEST_CASE(?:_METHOD)?\s*\(")
# Cases deleted before this check existed, recorded afterwards (#856): their
# ctest: records name no case at any later base.
BELATED_CASES = {
    "Base vehicle scheduler - scalar member identity and lifetime",  # a9f471051f
    "Town growth - command boundaries and callback reentry",  # f360fca7f1
}
CASE_NAME = re.compile(
    r'\bTEST_CASE(?:_METHOD\s*\(\s*[\w:<>]+\s*,|\s*\()\s*"((?:[^"\\\n]|\\.)*)"'
)


class NativeInventory(TypedDict):
    native_sources: list[str]
    native_tests: list[str]
    native_targets: list[str]


class Inventory(TypedDict):
    default: list[str]
    soak: list[str]
    comparisons: dict[str, list[str]]
    masks: list[str]
    known_failures: dict[str, str]
    native_sources: NotRequired[list[str]]
    native_tests: NotRequired[list[str]]
    native_targets: NotRequired[list[str]]


def discover() -> list[Path]:
    tools = sorted(ROOT.glob("tools/*-comparison.py"))
    tools.append(ROOT / "tools/compare-integers.py")
    return [tool for tool in tools if tool.is_file()]


def counts(line: str) -> list[int]:
    return [int(number) for number in COUNT.findall(line)]


def case_errors(
    recorded: Mapping[str, list[str]], printed: Mapping[str, list[str]]
) -> list[str]:
    """Each run driver prints a nonzero inventory with no recorded line shrinking."""
    errors = []
    for driver, lines in sorted(printed.items()):
        if not lines:
            errors.append(f"{driver}: no case inventory")
        errors += [
            f"{driver}: zero count: {line}" for line in lines if 0 in counts(line)
        ]
        now = {COUNT.sub("#", line): counts(line) for line in lines}
        for line in recorded.get(driver, []):
            found = now.get(COUNT.sub("#", line))
            if found is None or any(
                a < b for a, b in zip(found, counts(line), strict=True)
            ):
                errors.append(f"{driver}: fewer cases than recorded: {line}")
    return errors


def scenario_keys(scenario: object) -> list[str]:
    if isinstance(scenario, Mapping):
        return [str(key) for key in scenario]
    if not dataclasses.is_dataclass(scenario):
        raise TypeError(f"untyped scenario record: {scenario!r}")
    return [field.name for field in dataclasses.fields(scenario)]


def unread_keys(scenarios: Iterable[object], sources: Iterable[Path]) -> list[str]:
    """Keys whose only spelling in the tools is a dictionary-literal key."""
    used = set()
    for path in sources:
        tree = ast.parse(path.read_text(), str(path))
        written = {
            id(key)
            for node in ast.walk(tree)
            if isinstance(node, ast.Dict)
            for key in node.keys
        }
        for node in ast.walk(tree):
            if isinstance(node, ast.Attribute):
                used.add(node.attr)
            elif isinstance(node, ast.Constant) and id(node) not in written:
                used.add(node.value)
    return sorted(
        {key for scenario in scenarios for key in scenario_keys(scenario)} - used
    )


def native_inventory(ref: str | None = None) -> NativeInventory:
    """Census tracked evidence and its existing CMake registration, without builds."""
    paths = (
        git(
            "ls-tree", "-r", "--name-only", ref, "--", "tools", "src/tests"
        ).splitlines()
        if ref
        else git("ls-files", "--", "tools", "src/tests").splitlines()
    )
    sources = [
        path
        for path in paths
        if Path(path).suffix in replacements.NATIVE_SUFFIXES
        and (ref is not None or (ROOT / path).is_file())
    ]
    test_cmake = (
        git("show", f"{ref}:src/tests/CMakeLists.txt")
        if ref
        else (ROOT / "src/tests/CMakeLists.txt").read_text()
    )
    tests = [
        f"src/tests/{name}"
        for block in re.findall(r"add_test_files\((.*?)\)", test_cmake, re.S)
        for name in block.split()
        if Path(name).suffix in replacements.NATIVE_SUFFIXES
    ]
    # Existing literal CMake executable/library targets, including native Rust
    # test support. Component Python probes are covered by source/driver IDs.
    cmake_paths = [
        "CMakeLists.txt",
        *(
            git("ls-tree", "-r", "--name-only", ref, "--", "cmake").splitlines()
            if ref
            else git("ls-files", "--", "cmake").splitlines()
        ),
    ]
    targets = []
    for path in cmake_paths:
        if not path.endswith(("CMakeLists.txt", ".cmake")):
            continue
        text = git("show", f"{ref}:{path}") if ref else (ROOT / path).read_text()
        targets += re.findall(
            r"^\s*add_(?:executable|library)\(\s*([\w:.-]+)(?:\s|\))", text, re.M
        )
    if ref is None:
        retired = {
            item.removeprefix("source:")
            for record in replacements.load()
            for item in record["retires"]
            if item.startswith("source:")
        }
        tests = [path for path in tests if path not in retired]
    return {
        "native_sources": sorted(sources),
        "native_tests": sorted(tests),
        "native_targets": sorted(targets),
    }


def case_names(text: str, path: str) -> tuple[list[str], list[str]]:
    """Each TEST_CASE's CTest name, or an error where the literal is not plain."""
    names, found = [], []
    for name in CASE_NAME.findall(text):
        if re.search(r'\\[^"\\]', name):
            found.append(f"{path}: unsupported escape in TEST_CASE name {name!r}")
        names.append(re.sub(r"\\(.)", r"\1", name))
    if len(names) != len(CASE.findall(text)):
        found.append(f"{path}: TEST_CASE without a plain string-literal name")
    return names, found


def native_cases(ref: str | None = None) -> tuple[dict[str, str], list[str]]:
    """CTest case names of the registered native test sources, with their files."""
    cases: dict[str, str] = {}
    found = []
    present = set(git("ls-tree", "-r", "--name-only", ref, "--", "src/tests").split()) if ref else None
    for path in native_inventory(ref)["native_tests"]:
        if not path.endswith(".cpp") or (present is not None and path not in present):
            continue
        text = git("show", f"{ref}:{path}") if ref else (ROOT / path).read_text()
        names, errors = case_names(text, path)
        found += errors
        cases.update(dict.fromkeys(names, path))
    return cases, found


def retirements(ref: str | None = None) -> set[str]:
    """Every retired identity in the replacement manifests at ref or in the tree."""
    if ref is None:
        return {item for record in replacements.load() for item in record["retires"]}
    paths = git(
        "ls-tree", "--name-only", ref, "--", "migration/evidence-replacements/"
    ).split()
    return {
        item
        for path in paths
        if path.endswith(".json")
        for record in json.loads(git("show", f"{ref}:{path}"))
        for item in record["retires"]
    }


def ctest_case_errors(
    base: Mapping[str, str],
    tree: Mapping[str, str],
    retired: set[str],
    base_retired: set[str],
) -> list[str]:
    """A lost case needs its exact ctest: record; a new record names a lost case."""
    found = [
        f"CTest case removed without a ctest: record: {name}"
        for name, path in sorted(base.items())
        if name not in tree
        and f"ctest:{name}" not in retired
        and f"source:{path}" not in retired
    ]
    for item in sorted(retired - base_retired):
        name = item.removeprefix("ctest:")
        if not item.startswith("ctest:"):
            continue
        if name not in base and name not in BELATED_CASES:
            found.append(f"ctest: record names no CTest case at the base: {name}")
        elif name in tree:
            found.append(f"ctest: record retires a CTest case that still exists: {name}")
    return found


def replacement_errors(tree: Inventory, base: Inventory) -> list[str]:
    records = replacements.load()
    retired = [
        f"{prefix}:{name}"
        for key, prefix in (
            ("native_sources", "source"),
            ("native_tests", "source"),
            ("native_targets", "target"),
            ("comparisons", "comparison"),
            ("default", "scenario"),
            ("soak", "scenario"),
        )
        for name in sorted(set(base.get(key, [])) - set(tree.get(key, [])))
    ]
    return replacements.errors(records, tree["default"]) + replacements.missing(
        retired, records
    )


def registered_inventory(recorded: Inventory, tree: Inventory) -> Inventory:
    """Component manifests account for losses without rewriting shared history.

    Keep the archived case/scenario/source inventories. Only a named replacement
    contract can account for a removed entry or register a new scenario.
    """
    records = replacements.load()
    retired = {item for record in records for item in record["retires"]}
    expected = copy.deepcopy(recorded)
    for key, prefix in (
        ("native_sources", "source"),
        ("native_tests", "source"),
        ("native_targets", "target"),
        ("default", "scenario"),
        ("soak", "scenario"),
    ):
        expected[key] = [
            name
            for name in recorded.get(key, [])
            if f"{prefix}:{name}" not in retired or name in tree.get(key, [])
        ]
    expected["comparisons"] = {
        name: cases
        for name, cases in recorded["comparisons"].items()
        if f"comparison:{name}" not in retired or name in tree["comparisons"]
    }
    expected["default"] = sorted(
        set(expected["default"])
        | {
            check.removeprefix("scenario:")
            for check in replacements.destinations(records)
            if check.startswith("scenario:")
        }
    )
    return expected


def tree_inventory() -> tuple[Inventory, list[str]]:
    """The tree's inventory without case lines, and its unread scenario keys."""
    from simulation import core

    default = core.scenario_list(False)
    soak = core.scenario_list(True)
    names = sorted(scenario["name"] for scenario in default)
    sources = [
        path
        for path in [*ROOT.glob("tools/*.py"), *ROOT.glob("tools/simulation/*.py")]
        if not path.name.startswith("test-") and path.name != Path(__file__).name
    ]
    inventory = Inventory(
        default=names,
        soak=sorted({scenario["name"] for scenario in soak} - set(names)),
        comparisons={tool.name: [] for tool in discover()},
        masks=sorted(":".join(key) for key in core.MASKS),
        known_failures={":".join(k): v for k, v in core.KNOWN_FAILURES.items()},
    )
    native = native_inventory()
    inventory["native_sources"] = native["native_sources"]
    inventory["native_tests"] = native["native_tests"]
    inventory["native_targets"] = native["native_targets"]
    return inventory, unread_keys(soak, sources)


def issue_state(number: int) -> str:
    repository = os.environ.get("GITHUB_REPOSITORY", "fetch-laboratories/openttd-rust")
    token = os.environ.get("GITHUB_TOKEN")
    request = urllib.request.Request(
        f"https://api.github.com/repos/{repository}/issues/{number}",
        headers={"Authorization": f"Bearer {token}"} if token else {},
    )
    with urllib.request.urlopen(request, timeout=30) as response:
        issue = json.load(response)
    return "pull request" if "pull_request" in issue else issue["state"]


def errors(
    tree: Inventory,
    recorded: Inventory,
    base: Inventory,
    state: Callable[[int], str] = issue_state,
) -> list[str]:
    found = []
    for key in ("default", "soak", "masks"):
        for name in sorted(set(recorded[key]) ^ set(tree[key])):
            where = "tree" if name in tree[key] else "inventory"
            found.append(f"{key} {name} only in the {where}; update {INVENTORY}")
    if recorded["known_failures"] != tree["known_failures"]:
        found.append(f"KNOWN_FAILURES differs from {INVENTORY}")
    if set(recorded["comparisons"]) != set(tree["comparisons"]):
        found.append(f"comparison drivers differ from {INVENTORY}")
    for key in ("native_sources", "native_tests", "native_targets"):
        if tree.get(key) != recorded.get(key):
            found.append(f"{key} differs from {INVENTORY}")
    found += case_errors({}, recorded["comparisons"])
    found += [
        f"mask added or widened: {m}" for m in set(tree["masks"]) - set(base["masks"])
    ]
    for key, value in recorded["known_failures"].items():
        if base["known_failures"].get(key) != value:
            match = re.fullmatch(r"#(\d+)", value)
            if not match or state(int(match[1])) != "open":
                found.append(f"known failure {key} must name an open issue: {value}")
    return found


def removals(recorded: Inventory, base: Inventory) -> list[str]:
    lines = [
        f"{key}: {name}"
        for key in ("default", "soak")
        for name in sorted(set(base[key]) - set(recorded[key]))
    ]
    for driver, cases in sorted(base["comparisons"].items()):
        now = recorded["comparisons"].get(driver)
        lines += [f"comparison {driver}"] if now is None else []
        lines += [f"{driver}: {case}" for case in cases if now and case not in now]
    return lines


def load(text: str) -> Inventory:
    return json.loads(text)


def write(inventory: Inventory) -> None:
    (ROOT / INVENTORY).write_text(json.dumps(inventory, indent=1) + "\n")


def git(*args: str) -> str:
    return vcs.git(*args, cwd=ROOT)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-ref", default="origin/rust-migration")
    parser.add_argument(
        "--write",
        action="store_true",
        help="record additions, retaining archived evidence and case lines",
    )
    args = parser.parse_args()
    tree, unread = tree_inventory()
    recorded = load((ROOT / INVENTORY).read_text())
    if args.write:
        for key in (
            "default",
            "soak",
            "native_sources",
            "native_tests",
            "native_targets",
        ):
            tree[key] = sorted(set(tree.get(key, [])) | set(recorded.get(key, [])))
        tree["comparisons"] = {
            driver: recorded["comparisons"].get(driver, [])
            for driver in sorted(
                set(tree["comparisons"]) | set(recorded["comparisons"])
            )
        }
        write(tree)
        return 0
    tracked = git("ls-tree", "--name-only", args.base_ref, INVENTORY).strip()
    base = load(git("show", f"{args.base_ref}:{INVENTORY}")) if tracked else recorded
    if "native_sources" not in base:
        native = native_inventory(args.base_ref)
        base["native_sources"] = native["native_sources"]
        base["native_tests"] = native["native_tests"]
        base["native_targets"] = native["native_targets"]
    expected = registered_inventory(recorded, tree)
    found = errors(tree, expected, base)
    for key in (
        "default",
        "soak",
        "native_sources",
        "native_tests",
        "native_targets",
        "comparisons",
    ):
        if set(base.get(key, [])) - set(recorded.get(key, [])):
            found.append(
                f"archived {key} removed; retain inventory and register replacements"
            )
    found += replacement_errors(tree, base)
    base_cases, base_case_errors = native_cases(args.base_ref)
    tree_cases, tree_case_errors = native_cases()
    found += tree_case_errors + ctest_case_errors(
        base_cases, tree_cases, retirements(), retirements(args.base_ref)
    )
    if base_case_errors:
        print(f"note: {len(base_case_errors)} unparsed TEST_CASE names at the base")
    found += case_errors(base["comparisons"], expected["comparisons"])
    found += [f"scenario key read by no tool: {key}" for key in unread]
    for line in removals(recorded, base):
        print(f"recorded removal: {line}")
    for line in found:
        print(f"error: {line}")
    print(
        f"Harness inventory: {len(tree['default'])} default, {len(tree['soak'])} soak-only scenarios, {len(tree['comparisons'])} comparisons, {len(tree_cases)} native CTest cases; {len(found)} errors"
    )
    return int(bool(found))


if __name__ == "__main__":
    raise SystemExit(main())
