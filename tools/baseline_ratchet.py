"""Shared shrinking-baseline CLI for source scanners."""

import argparse
from collections.abc import Callable, Iterable
from functools import partial
from pathlib import Path

from vcs import git, merge_base


def main(
    description: str | None,
    root: Path,
    baseline_path: str,
    folders: Iterable[str],
    suffixes: set[str],
    violations: Callable[[dict[str, str]], set[str]],
    *,
    new_label: str,
    summary: str,
) -> int:
    parser = argparse.ArgumentParser(description=description)
    parser.add_argument("--base-ref", default="origin/rust-migration")
    args = parser.parse_args()
    args.base_ref = merge_base(args.base_ref, run=partial(git, cwd=root), announce=True)
    files = {
        str(p.relative_to(root)): p.read_text()
        for folder in folders
        for p in (root / folder).rglob("*")
        if p.is_file() and p.suffix in suffixes
    }
    hits = violations(files)
    baseline = set((root / baseline_path).read_text().splitlines())
    tracked = git(
        "ls-tree", "--name-only", args.base_ref, baseline_path, cwd=root
    ).strip()
    previous = (
        set(git("show", f"{args.base_ref}:{baseline_path}", cwd=root).splitlines())
        if tracked
        else baseline
    )
    errors = [
        (new_label, hits - baseline),
        ("remove retired", baseline - hits),
        ("baseline cannot grow", baseline - previous),
    ]
    for label, names in errors:
        for name in sorted(names):
            print(f"{label}: {name}")
    count = sum(len(names) for _, names in errors)
    print(f"{summary.format(count=len(hits))}; {count} errors")
    return int(count > 0)


def debt_errors(
    current: dict[str, list[int]],
    baseline: dict[str, list[int]],
    allowed: dict[str, list[int]],
) -> list[str]:
    """Report growth only: measured or recorded debt above what base allows.

    Values are sorted magnitudes, one per distinct compiler site (deduplicated
    across Cargo targets). Removing a site cannot pay for another site's growth.
    A recorded row above the measured one is an unrecorded improvement, not an
    error.
    """
    errors = []
    for label, measured in (("debt growth", current), ("baseline growth", baseline)):
        for key in sorted(measured):
            values = sorted(measured[key], reverse=True)
            limit = sorted(allowed.get(key, []), reverse=True)
            if exceeds(values, limit):
                errors.append(f"{label}: {key}: {values} exceeds {limit}")
    return errors


def exceeds(values: list[int], allowed: list[int]) -> bool:
    """More sites, or any larger magnitude after sorting both descending."""
    values, allowed = sorted(values, reverse=True), sorted(allowed, reverse=True)
    return len(values) > len(allowed) or any(
        value > limit for value, limit in zip(values, allowed, strict=False)
    )
