#!/usr/bin/env python3
"""Reject independent C++/boundary growth; require reviewed exceptions.

An acceptance binds to the PR's own growth: the counter and its added keys and
counts, not the counter's base and head totals, so another PR's merge does not
void it (#904). It names the reviewed head, and also holds at a later head while
the source of every added key is byte-identical, so a base update or an
unrelated push keeps it. Identity counters (exports, callback slots) stay
exact-head.

A proposal lives in the change's tracked cleanup records; the acceptance is a
reviewer's attributed PR comment. Growth whose proposal awaits that acceptance
is pending review: it passes on pull-request events and fails in the merge
group. Growth with no proposal always fails.
"""

import argparse

# Load pinned AST dependencies before code_lines can re-exec for Pygments.
import importlib
import importlib.util
import json
import subprocess
from functools import partial
from pathlib import Path
from typing import Any

from cleanup_contract import context, reviewed, run
from vcs import merge_base

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


BOUND = ("counter", "added")
FILE_KEYED = {"tools_cpp", "tests_cpp", "outside_cpp"}


def known(revision: str) -> bool:
    """A shallow checkout may lack the accepted commit; then only the head counts."""
    check = ["git", "cat-file", "-e", f"{revision}^{{commit}}"]
    return subprocess.run(check, cwd=METRICS.ROOT, capture_output=True).returncode == 0


def same_source(growth: dict[str, Any], current: str, accepted_at: str) -> bool:
    """Whether every added key's source is identical at the accepted commit."""
    counter, keys = growth["counter"], growth["added"]
    if not known(accepted_at):
        return False
    if counter == "generated_records":
        before = METRICS.generated_records(accepted_at)
        after = METRICS.generated_records(current)
        return all(key in before and before[key] == after.get(key) for key in keys)
    if counter in FILE_KEYED:
        return all(
            (text := METRICS.show(current, key)) != ""
            and METRICS.show(accepted_at, key) == text
            for key in keys
        )
    return False


def proposed(growth: dict[str, Any], proposal: dict[str, Any]) -> bool:
    # "old" and "new" are the counter's totals, which other merges move.
    return proposal["kind"] == "growth" and all(
        proposal.get(key) == growth[key] for key in BOUND
    )


def accepted(
    growth: dict[str, Any],
    contract: tuple[str, list[dict[str, Any]], list[str], str],
    current: str = "HEAD",
) -> bool:
    """`contract` is `cleanup_contract.context()`: author, proposals, comments, head."""
    author, proposals, comments, head = contract
    for proposal in proposals:
        if not proposed(growth, proposal):
            continue
        unchanged = partial(same_source, growth, current)
        if reviewed(proposal, comments, author, head, unchanged):
            return True
    return False


def unaccepted(
    growth: list[dict[str, Any]],
    contract: tuple[str, list[dict[str, Any]], list[str], str],
    current: str,
    event: str,
) -> bool:
    """Report each delta; whether any fails on this event."""
    failed = False
    for delta in growth:
        counter = delta["counter"]
        if accepted(delta, contract, current):
            accepted_at = contract[3]
            print(
                json.dumps(
                    {"accepted_at": accepted_at, "growth": delta}, sort_keys=True
                )
            )
        elif not any(proposed(delta, proposal) for proposal in contract[1]):
            print(
                f"unreviewed growth: {counter} has no proposal in the cleanup records"
            )
            failed = True
        elif event == "merge_group":
            print(f"unreviewed growth: {counter} lacks its reviewer's acceptance")
            failed = True
        else:
            print(f"pending review: {counter} awaits its reviewer's acceptance")
    return failed


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-ref", default="origin/rust-migration")
    parser.add_argument("--head", default="HEAD")
    parser.add_argument(
        "--event",
        default="pull_request",
        help="the CI event; growth pending review fails only in merge_group",
    )
    args = parser.parse_args()
    args.base_ref = merge_base(args.base_ref, args.head, run=METRICS.git, announce=True)
    METRICS.keep_code_lines()
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
    return int(unaccepted(growth, context(args.base_ref), args.head, args.event))


if __name__ == "__main__":
    run(main)
