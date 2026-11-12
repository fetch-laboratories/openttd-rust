#!/usr/bin/env python3
"""Read-only PR readiness, milestone latency, stocktakes and base-update checks."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from dataclasses import asdict, dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, cast
from urllib.parse import urlencode

import base_update
import ci
from review_report import Comment, Review, ReviewReport, review_reports

ROOT = Path(__file__).resolve().parents[1]


@dataclass
class PullFacts:
    number: int
    url: str
    state: str
    head: str
    base: str
    base_ref: str
    base_tip: str
    mergeable: bool | None
    mergeable_state: str
    stable_snapshot: bool
    full: dict[str, Any] | None
    passing_full_url: str | None
    reports: list[ReviewReport]
    files: list[str]
    milestones: list[dict[str, Any]]


def pages(repository: str, path: str) -> list[dict[str, Any]]:
    """Read paginated REST arrays using the existing authenticated gh helper."""
    return [
        item
        for page in json.loads(
            ci.gh("api", "--paginate", "--slurp", f"repos/{repository}/{path}")
        )
        for item in page
    ]


def collect_pull(repository: str, number: int, *, latency: bool = False) -> PullFacts:
    pull = ci.api(repository, f"pulls/{number}")
    head = pull["head"]["sha"]
    # The PR's base sha is its last sync point, not the base branch's current tip.
    branch = f"branches/{pull['base']['ref']}"
    base_tip = ci.api(repository, branch)["commit"]["sha"]
    comments = pages(repository, f"issues/{number}/comments?per_page=100")
    reviews = pages(repository, f"pulls/{number}/reviews?per_page=100")
    statuses = pages(repository, f"commits/{head}/statuses?per_page=100")
    full = next((s for s in statuses if s["context"] == ci.CONTEXT), None)
    full_fact = (
        {
            key: full.get(key)
            for key in (
                "state",
                "target_url",
                "description",
                "created_at",
                "updated_at",
                "creator",
            )
        }
        if full
        else None
    )
    milestones = [
        {"event": key, "at": pull[key], "url": pull["html_url"], "cause": None}
        for key in ("created_at", "closed_at", "merged_at")
        if pull.get(key)
    ]
    if latency:
        milestones.extend(
            {
                "event": item["event"],
                "at": item.get("submitted_at") or item.get("created_at"),
                "url": item.get("html_url"),
                "cause": None,
            }
            for item in pages(repository, f"issues/{number}/timeline?per_page=100")
            if item.get("event")
            in {
                "ready_for_review",
                "convert_to_draft",
                "review_requested",
                "reviewed",
                "merged",
            }
        )
    passing_full_url = ci.passed_run(repository, head)
    files = [
        f["filename"] for f in pages(repository, f"pulls/{number}/files?per_page=100")
    ]
    current = ci.api(repository, f"pulls/{number}")
    current_tip = ci.api(repository, branch)["commit"]["sha"]
    return PullFacts(
        number,
        pull["html_url"],
        pull["state"],
        head,
        pull["base"]["sha"],
        pull["base"]["ref"],
        base_tip,
        pull.get("mergeable"),
        pull.get("mergeable_state", "unknown"),
        head == current["head"]["sha"]
        and pull["base"]["sha"] == current["base"]["sha"]
        and base_tip == current_tip,
        full_fact,
        passing_full_url,
        review_reports(
            cast(list[Comment], comments), cast(list[Review], reviews), head
        ),
        files,
        milestones,
    )


def overlaps(pulls: list[PullFacts]) -> list[dict[str, Any]]:
    return [
        {
            "prs": [left.number, right.number],
            "files": sorted(set(left.files) & set(right.files)),
        }
        for index, left in enumerate(pulls)
        for right in pulls[index + 1 :]
    ]


def timestamp(value: str) -> datetime:
    return datetime.fromisoformat(value.replace("Z", "+00:00"))


def elapsed(intervals: list[tuple[str, str]]) -> dict[str, float]:
    """Summed PR intervals and their elapsed union are different quantities."""
    ordered = sorted((timestamp(a), timestamp(b)) for a, b in intervals)
    merged: list[tuple[datetime, datetime]] = []
    for start, end in ordered:
        if end < start:
            raise ValueError("milestone interval ends before it starts")
        if merged and start <= merged[-1][1]:
            merged[-1] = (merged[-1][0], max(merged[-1][1], end))
        else:
            merged.append((start, end))
    return {
        "summed_per_pr_seconds": sum((b - a).total_seconds() for a, b in ordered),
        "elapsed_union_seconds": sum((b - a).total_seconds() for a, b in merged),
    }


def latency_facts(repository: str, pulls: list[PullFacts]) -> dict[str, Any]:
    earliest = min(
        m["at"] for p in pulls for m in p.milestones if m["event"] == "created_at"
    )
    query = urlencode({"per_page": 100, "created": ">=" + earliest})
    run_pages = json.loads(
        ci.gh(
            "api",
            "--paginate",
            "--slurp",
            f"repos/{repository}/actions/workflows/{ci.WORKFLOW}/runs?{query}",
        )
    )
    runs = [run for page in run_pages for run in page["workflow_runs"]]
    rows = []
    intervals = []
    for pull in pulls:
        milestones = list(pull.milestones)
        milestones.extend(
            {
                "event": "review_report",
                "at": report.reported_at,
                "url": report.url,
                "sha": report.sha,
                "cause": None,
            }
            for report in pull.reports
        )
        for run in runs:
            if not run.get("display_title", "").startswith(
                f"CI full PR {pull.number} "
            ):
                continue
            for field in ("created_at", "run_started_at", "updated_at"):
                if run.get(field):
                    milestones.append(
                        {
                            "event": "full_" + field,
                            "at": run[field],
                            "url": run["html_url"],
                            "conclusion": run.get("conclusion"),
                            "cause": None,
                            "current_head_status_url_matches": run["html_url"]
                            == pull.passing_full_url,
                        }
                    )
        created = next(m["at"] for m in milestones if m["event"] == "created_at")
        merged = next((m["at"] for m in milestones if m["event"] == "merged_at"), None)
        if merged:
            intervals.append((created, merged))
        rows.append(
            {
                "pr": pull.number,
                "created_to_merge_seconds": (
                    (timestamp(merged) - timestamp(created)).total_seconds()
                    if merged
                    else None
                ),
                "milestones": sorted(milestones, key=lambda m: m.get("at") or ""),
            }
        )
    return {
        "prs": rows,
        "completed_pr_intervals": elapsed(intervals),
        "limits": [
            "Absent milestones stay absent; no wait cause is inferred.",
            "Full updated_at is a workflow last-update timestamp, not a measured completion time.",
            "Run titles associate PR numbers; dispatch run head_sha is not the PR head.",
            "Report text uses its latest edit timestamp, not an inferred review start or approval.",
        ],
    }


def stocktake(start: str, end: str) -> dict[str, Any]:
    endpoints = [
        json.loads(
            subprocess.check_output(
                [
                    sys.executable,
                    str(ROOT / "tools/port-metrics.py"),
                    "--whole-fork",
                    "--json",
                    "--head",
                    rev,
                ],
                cwd=ROOT,
                text=True,
            )
        )
        for rev in (start, end)
    ]
    metadata = (
        "metric_version",
        "metric_scope",
        "metric_implementation",
        "baseline",
        "original_total",
    )
    if any(endpoints[0].get(key) != endpoints[1].get(key) for key in metadata):
        raise ValueError("stocktake endpoints use different metric versions or scopes")
    keys = (
        "original_total",
        "whole_fork_retired",
        "compiled_cpp",
        "fork_cpp",
        "candidate_cpp_upper_bound",
        "rust",
        "tooling",
    )
    return {
        **{key: endpoints[0][key] for key in metadata},
        "heads": [point["head"] for point in endpoints],
        "rows": [
            {
                "metric": key,
                "start": endpoints[0][key],
                "end": endpoints[1][key],
                "delta": endpoints[1][key] - endpoints[0][key],
            }
            for key in keys
        ],
        "endpoints": endpoints,
        "limits": [
            "Both endpoints are recounted by this metric; inventory corrections are not deletion credit."
        ],
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    for name in ("readiness", "latency"):
        command = commands.add_parser(name)
        command.add_argument("prs", nargs="+", type=int)
        command.add_argument("--repo")
        command.add_argument("--json", action="store_true")
    command = commands.add_parser("stocktake")
    command.add_argument("start")
    command.add_argument("end")
    command.add_argument("--json", action="store_true")
    base_update.add_parser(commands)
    args = parser.parse_args()
    if args.command == "base-update":
        sys.exit(base_update.run(args, Path.cwd()))
    result: dict[str, Any]
    if args.command == "stocktake":
        result = stocktake(args.start, args.end)
    else:
        repository = (
            args.repo
            or json.loads(ci.gh("repo", "view", "--json", "nameWithOwner"))[
                "nameWithOwner"
            ]
        )
        pulls = [
            collect_pull(repository, n, latency=args.command == "latency")
            for n in dict.fromkeys(args.prs)
        ]
        result = {
            "repository": repository,
            "pulls": [asdict(p) for p in pulls],
            "overlaps": overlaps(pulls),
        }
        if args.command == "latency":
            result["latency"] = latency_facts(repository, pulls)
    result.update(schema_version=1, observed_at=datetime.now(timezone.utc).isoformat())
    if args.json:
        print(json.dumps(result, indent=2))
    elif args.command == "stocktake":
        print(
            f"Metric v{result['metric_version']}: {result['heads'][0]} -> {result['heads'][1]}"
        )
        print("Metric | start | end | delta")
        for row in result["rows"]:
            print(
                f"{row['metric']} | {row['start']} | {row['end']} | {row['delta']:+d}"
            )
    else:
        print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
