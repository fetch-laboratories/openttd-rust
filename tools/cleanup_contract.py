"""Exact-head, attributed review contracts for cleanup checks.

A review record uses a fenced cleanup-acceptance JSON block. Shared GitHub
credentials are deliberate: independence is the Agent path, not the login.
"""

import json
import os
import re
from typing import Any

from vcs import gh, git

ATTRIBUTION = re.compile(r"Agent: (\S+) \| Model: (\S+) \| Reasoning effort: (\S+)")
BLOCK = re.compile(r"```cleanup-acceptance\s*\n(.*?)\n```", re.S)


def records(body: str) -> list[dict[str, Any]]:
    """Malformed contracts are errors, never silently ignored."""
    return [json.loads(match) for match in BLOCK.findall(body)]


def reviewed(
    proposal: dict[str, Any], comments: list[str], author: str, head: str
) -> bool:
    """Require another attributed agent to accept the exact payload and head."""
    if not author:
        return False
    for body in comments:
        attribution = ATTRIBUTION.search(body)
        if not attribution or attribution[1] == author:
            continue
        for record in records(body):
            if record.get("head") == head and proposal in record.get("accept", []):
                return True
    return False


def context() -> tuple[str, list[dict[str, Any]], list[str], str]:
    """Read PR body and both ordinary comments and submitted review bodies."""
    head = git("rev-parse", "HEAD").strip()
    event = (
        json.loads(open(path).read())
        if (path := os.getenv("GITHUB_EVENT_PATH"))
        else {}
    )
    pull = event.get("pull_request", {})
    if pull:
        head = pull["head"]["sha"]
    elif len(git("rev-list", "--parents", "-n", "1", "HEAD").split()) == 3:
        head = git("rev-parse", "HEAD^2").strip()
    repository = os.getenv("GITHUB_REPOSITORY")
    if not repository:
        repository = json.loads(gh("repo", "view", "--json", "nameWithOwner"))[
            "nameWithOwner"
        ]
    number = pull.get("number")
    if not number:
        pulls = json.loads(gh("api", f"repos/{repository}/commits/{head}/pulls"))
        candidate = next(
            (p for p in pulls if p["state"] == "open" or p.get("merged_at")), None
        )
        number = candidate["number"] if candidate else None
        if candidate:
            head = candidate["head"]["sha"]
    if number is None:
        return "", [], [], head
    prefix = f"repos/{repository}"
    body = json.loads(gh("api", f"{prefix}/pulls/{number}"))["body"] or ""
    author = ATTRIBUTION.search(body)
    comments: list[str] = []
    for endpoint in (f"issues/{number}/comments", f"pulls/{number}/reviews"):
        pages = json.loads(gh("api", "--paginate", "--slurp", f"{prefix}/{endpoint}"))
        comments.extend(row.get("body") or "" for page in pages for row in page)
    return author[1] if author else "", records(body), comments, head
