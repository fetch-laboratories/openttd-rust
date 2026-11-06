#!/usr/bin/env python3
"""Reject independent C++/boundary growth; require exact-head reviewed exceptions."""

import argparse

# Load pinned AST dependencies before code_lines can re-exec for Pygments.
import importlib
import importlib.util
import json
from pathlib import Path
from typing import Any

from cleanup_contract import context, reviewed

importlib.import_module("rust_syntax")

SPEC = importlib.util.spec_from_file_location(
    "port_metrics", Path(__file__).with_name("port-metrics.py")
)
assert SPEC and SPEC.loader
METRICS: Any = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(METRICS)


def increases(
    before: dict[str, dict[str, int]], after: dict[str, dict[str, int]]
) -> list[dict[str, Any]]:
    result = []
    for counter in sorted(before.keys() | after.keys()):
        old, new = before.get(counter, {}), after.get(counter, {})
        a, b = sum(old.values()), sum(new.values())
        if b > a:
            result.append(
                {
                    "counter": counter,
                    "old": a,
                    "new": b,
                    "added": {
                        key: value - old.get(key, 0)
                        for key, value in sorted(new.items())
                        if value > old.get(key, 0)
                    },
                }
            )
    return result


def accepted(
    growth: dict[str, Any],
    proposals: list[dict[str, Any]],
    comments: list[str],
    author: str,
    head: str,
) -> bool:
    for proposal in proposals:
        if proposal.get("kind") != "growth" or any(
            proposal.get(key) != value for key, value in growth.items()
        ):
            continue
        if not all(
            isinstance(proposal.get(key), str) and proposal[key].strip()
            for key in (
                "reason",
                "existing_alternatives",
                "retiring_owner",
                "retirement_condition",
            )
        ):
            continue
        if reviewed(proposal, comments, author, head):
            return True
    return False


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-ref", default="origin/rust-migration")
    parser.add_argument("--head", default="HEAD")
    args = parser.parse_args()
    before = METRICS.growth_inventory(args.base_ref)
    after = METRICS.growth_inventory(args.head)
    growth = increases(before, after)
    print(
        json.dumps(
            {
                "metric_version": METRICS.METRIC_VERSION,
                "metric_scope": METRICS.METRIC_SCOPE,
                "base": before,
                "head": after,
                "historical_boundary": {
                    "base": METRICS.boundary(args.base_ref),
                    "head": METRICS.boundary(args.head),
                },
                "growth": growth,
            },
            sort_keys=True,
            indent=2,
        )
    )
    if not growth:
        return 0
    author, proposals, comments, head = context()
    errors = []
    for delta in growth:
        if accepted(delta, proposals, comments, author, head):
            print(json.dumps({"accepted_at": head, "growth": delta}, sort_keys=True))
        else:
            errors.append(delta["counter"])
    for counter in errors:
        print(f"unreviewed growth: {counter}")
    return int(bool(errors))


if __name__ == "__main__":
    raise SystemExit(main())
