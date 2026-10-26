#!/usr/bin/env python3
"""List every open issue's place in the roadmap, unplaced and excluded first.

Usage: issues.py   (run from the repository root; needs an authenticated `gh`)

An open issue is placed when docs/roadmap.md names it outside an exclusion
line, when another open issue's checklist names it, or when it carries the
`deferred-improvement` label (AGENTS.md parks those until near-full
reproduction). It is excluded when the roadmap names it only on exclusion
lines; each such line must state the condition that brings it back. Every
roadmap line that excludes something is printed in full, so the reader can
check its return condition and spot a component it leaves out without an
issue; list items and table rows naming no issue are counted. STALE marks issues untouched for 7 days or more,
which may be finished work left open.
"""

import json
import re
import subprocess
import sys
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path

EXCLUSION = re.compile(
    r"unselected|not selected|outside (this|the work) order|stays? outside|"
    r"\bpaused\b|\bparked\b|\bdefer(red)? (to|until|behind|after)\b|backlog",
    re.I,
)
CHECKLIST = re.compile(r"^\s*[-*] \[[ xX]\]")
PARKED_LABEL = "deferred-improvement"
STALE_DAYS = 7


@dataclass
class Issue:
    number: int
    title: str
    labels: list[str]
    days_idle: int
    body: str


def open_issues() -> list[Issue]:
    raw = subprocess.check_output(
        [
            "gh",
            "issue",
            "list",
            "--state",
            "open",
            "--limit",
            "1000",
            "--json",
            "number,title,labels,updatedAt,body",
        ],
        text=True,
    )
    now = datetime.now(timezone.utc)
    issues = []
    for item in json.loads(raw):
        updated = datetime.fromisoformat(item["updatedAt"].replace("Z", "+00:00"))
        issues.append(
            Issue(
                number=item["number"],
                title=item["title"],
                labels=[label["name"] for label in item["labels"]],
                days_idle=(now - updated).days,
                body=item["body"] or "",
            )
        )
    return issues


def roadmap_lines() -> list[str]:
    """Roadmap logical lines: wrapped paragraphs and list items joined into one."""
    starts = re.compile(r"\s*([-*] |\d+\. |\||#+ )")
    joined: list[str] = []
    previous_blank = True
    for line in Path("docs/roadmap.md").read_text().splitlines():
        blank = not line.strip()
        if joined and not blank and not previous_blank and not starts.match(line):
            joined[-1] += " " + line.strip()
        else:
            joined.append(line)
        previous_blank = blank
    return joined


def mentions(text: str, number: int) -> bool:
    return re.search(rf"#{number}\b", text) is not None


def classify(issue: Issue, lines: list[str], issues: list[Issue]) -> str:
    named = [line for line in lines if mentions(line, issue.number)]
    if any(not EXCLUSION.search(line) for line in named):
        return "placed: roadmap"
    for other in issues:
        if other.number != issue.number and any(
            CHECKLIST.match(line) and mentions(line, issue.number)
            for line in other.body.splitlines()
        ):
            return f"placed: checklist of #{other.number}"
    if PARKED_LABEL in issue.labels:
        return "placed: deferred-improvement"
    if named:
        return "EXCLUDED: check its return condition"
    return "UNPLACED"


def main() -> int:
    issues = open_issues()
    lines = roadmap_lines()
    rows = sorted(
        ((classify(i, lines, issues), i) for i in issues),
        key=lambda row: (row[0].startswith("placed"), row[1].number),
    )
    for status, issue in rows:
        parked = status == f"placed: {PARKED_LABEL}"
        stale = " STALE" if issue.days_idle >= STALE_DAYS and not parked else ""
        print(f"#{issue.number:<5} {status:<40}{stale:6} {issue.title[:70]}")
    exclusions = [
        line.strip()
        for line in lines
        if EXCLUSION.search(line) and not re.match(r"#+ ", line)
    ]
    if exclusions:
        print("\nRoadmap exclusion lines; each needs a return condition, and each")
        print("component it leaves out without an issue number needs an issue:")
    for line in exclusions:
        print(f"  {line[:240]}")
    orphan_lines = [
        line
        for line in exclusions
        if re.match(r"([-|]|\d+\.)", line) and not re.search(r"#\d+", line)
    ]
    unplaced = sum(status == "UNPLACED" for status, _ in rows)
    excluded = sum(status.startswith("EXCLUDED") for status, _ in rows)
    print(
        f"{len(rows)} open: {unplaced} unplaced, {excluded} excluded, "
        f"{len(orphan_lines)} roadmap exclusions without an issue"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
