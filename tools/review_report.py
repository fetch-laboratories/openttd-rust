"""Review reports on a PR: the reviewed commit, the reviewer and the disposition.

A report is a PR comment starting `Reviewed commit: <sha>`, attributed as
`Agent: <path> | Model: <model> | Reasoning effort: <effort>` and ending with its
disposition: accepted, or findings to remediate (docs/review-brief.md, Report).
What a report's text does not give is `unknown`, never guessed. Fenced blocks
and quoted lines, such as an earlier report quoted in a re-check, are not the
report's own text and give nothing.
"""

from __future__ import annotations

import re
from dataclasses import dataclass
from typing import NotRequired, TypedDict

from cleanup_contract import ATTRIBUTION

REVIEWED = re.compile(r"\bReviewed\s+(?:commit|head):\s*`?([0-9a-f]{7,40})\b", re.I)
FENCE = re.compile(r"^\s*```.*?^\s*```\s*$", re.M | re.S)
QUOTED = re.compile(r"^\s*>.*\n?", re.M)
DISPOSITIONS = ("accepted", "findings to remediate")
UNKNOWN = "unknown"


class User(TypedDict):
    login: str


class Comment(TypedDict):
    """An issue comment as the API lists it, with the fields these tools read."""

    id: int
    html_url: str
    updated_at: str
    body: str
    user: NotRequired[User | None]
    issue_url: NotRequired[str]


class Review(TypedDict):
    """A submitted PR review as the API lists it, with the fields read here."""

    html_url: str
    state: str
    body: str | None
    commit_id: NotRequired[str | None]
    submitted_at: NotRequired[str | None]


@dataclass
class ReviewReport:
    sha: str
    url: str
    reported_at: str
    source: str
    matches_head: bool
    agent: str = UNKNOWN
    disposition: str = UNKNOWN
    scope: str | None = None


def own_text(body: str) -> str:
    """The body without fenced blocks or quoted lines."""
    return QUOTED.sub("", FENCE.sub("", body))


def reviewed_sha(body: str) -> str | None:
    """The commit the report's header names: its first `Reviewed commit:` in its
    own text. A report rarely puts a line, such as its attribution, before it."""
    header = REVIEWED.search(own_text(body))
    return header[1] if header else None


def agent(body: str) -> str:
    """The report's attributed agent path."""
    attribution = ATTRIBUTION.search(own_text(body))
    return attribution[1] if attribution else UNKNOWN


def disposition(body: str) -> tuple[str, str | None]:
    """The disposition its closing line gives, and the scope after it, such as
    `whole PR` or `group 2`. A fenced block (a cleanup acceptance) or the
    attribution line after the disposition does not hide it."""
    lines = [
        line
        for line in own_text(body).splitlines()
        if line.strip() and not ATTRIBUTION.search(line)
    ]
    if not lines:
        return UNKNOWN, None
    closing = lines[-1].strip().strip("*_`-> ").strip()
    closing = re.sub(r"^\**disposition\**\s*:\**\s*", "", closing, flags=re.I)
    for name in DISPOSITIONS:
        if closing.lower().startswith(name):
            scope = closing[len(name) :].strip(" ,.;:*_`")
            return name, scope or None
    return UNKNOWN, None


def parsed(found: ReviewReport, body: str) -> ReviewReport:
    """The report with its agent and disposition read from its text."""
    found.agent = agent(body)
    found.disposition, found.scope = disposition(body)
    return found


def review_reports(
    comments: list[Comment], reviews: list[Review], head: str
) -> list[ReviewReport]:
    """Reports in PR comments, then submitted GitHub reviews, against `head`."""
    reports = [
        parsed(
            ReviewReport(
                sha,
                item["html_url"],
                item["updated_at"],
                "report text",
                head.startswith(sha),
            ),
            item.get("body") or "",
        )
        for item in comments
        if (sha := reviewed_sha(item.get("body") or ""))
    ]
    reports.extend(
        parsed(
            ReviewReport(
                sha,
                item["html_url"],
                at,
                "GitHub review: " + item["state"],
                sha == head,
            ),
            item.get("body") or "",
        )
        for item in reviews
        if (sha := item.get("commit_id")) and (at := item.get("submitted_at"))
    )
    return reports
