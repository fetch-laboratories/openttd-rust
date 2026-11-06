#!/usr/bin/env python3
"""Reject silent harness loss: scenarios, comparisons, cases, masks and keys (#672).

migration/harness-inventory.json records default and soak-only scenario names,
each comparison driver's case lines, MASKS and KNOWN_FAILURES. The tree must
match it, so every removal shows in the diff. Against --base-ref, MASKS gains no
entry and a new KNOWN_FAILURES entry names an open issue. A scenario key that no
tool reads fails. run-comparisons.py checks reported case counts at run time.
"""

import argparse
import ast
import dataclasses
import json
import os
import re
import urllib.request
from collections.abc import Callable, Iterable, Mapping
from pathlib import Path
from typing import TypedDict

import vcs

ROOT = Path(__file__).resolve().parents[1]
INVENTORY = "migration/harness-inventory.json"
COUNT = re.compile(r"\b\d+\b")


class Inventory(TypedDict):
    default: list[str]
    soak: list[str]
    comparisons: dict[str, list[str]]
    masks: list[str]
    known_failures: dict[str, str]


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
        "--write", action="store_true", help="record the tree, keeping case lines"
    )
    args = parser.parse_args()
    tree, unread = tree_inventory()
    recorded = load((ROOT / INVENTORY).read_text())
    if args.write:
        for driver in tree["comparisons"]:
            tree["comparisons"][driver] = recorded["comparisons"].get(driver, [])
        write(tree)
        return 0
    tracked = git("ls-tree", "--name-only", args.base_ref, INVENTORY).strip()
    base = load(git("show", f"{args.base_ref}:{INVENTORY}")) if tracked else recorded
    found = errors(tree, recorded, base)
    found += [f"scenario key read by no tool: {key}" for key in unread]
    for line in removals(recorded, base):
        print(f"recorded removal: {line}")
    for line in found:
        print(f"error: {line}")
    print(
        f"Harness inventory: {len(tree['default'])} default, {len(tree['soak'])} soak-only scenarios, {len(tree['comparisons'])} comparisons; {len(found)} errors"
    )
    return int(bool(found))


if __name__ == "__main__":
    raise SystemExit(main())
