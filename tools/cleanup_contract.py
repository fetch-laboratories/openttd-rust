"""Attributed review contracts and declared carries for cleanup checks.

A review record uses a fenced cleanup-acceptance JSON block. Shared GitHub
credentials are deliberate: independence is the Agent path, not the login.
"""

import json
import os
import re
from collections.abc import Callable, Iterable, Mapping
from typing import Any, NamedTuple

from vcs import gh, git

ATTRIBUTION = re.compile(r"Agent: (\S+) \| Model: (\S+) \| Reasoning effort: (\S+)")
BLOCK = re.compile(r"```cleanup-acceptance\s*\n(.*?)\n```", re.S)


def records(body: str) -> list[dict[str, Any]]:
    """Malformed contracts are errors, never silently ignored."""
    return [json.loads(match) for match in BLOCK.findall(body)]


def reviewed(
    proposal: dict[str, Any],
    comments: list[str],
    author: str,
    head: str,
    unchanged: Callable[[str], bool] | None = None,
) -> bool:
    """Require another attributed agent to accept the exact payload and head.

    With `unchanged`, an acceptance at an earlier full SHA also holds while the
    site's item text there equals the item text at the checked head.
    """
    if not author:
        return False
    for body in comments:
        attribution = ATTRIBUTION.search(body)
        if not attribution or attribution[1] == author:
            continue
        for record in records(body):
            accepted = record.get("head")
            if proposal in record.get("accept", []) and (
                accepted == head
                or unchanged is not None
                and isinstance(accepted, str)
                and re.fullmatch(r"[0-9a-f]{40}", accepted) is not None
                and unchanged(accepted)
            ):
                return True
    return False


class Ledger(NamedTuple):
    """One tool's keyed debt at base and head, and how carries compare it."""

    tool: str
    before: Mapping[str, Any]
    after: Mapping[str, Any]
    rule: Callable[[str], object]
    fits: Callable[[Any, Any], bool]


def carry_problem(carry: dict[str, Any], ledger: Ledger, valid: dict[str, str]) -> str:
    old, new = carry.get("from"), carry.get("to")
    before, after = ledger.before, ledger.after
    if not isinstance(old, str) or not isinstance(new, str):
        return "from and to must be keys"
    checks = (
        ("from does not exist at base", lambda: old not in before),
        ("from still exists at head", lambda: old in after),
        ("to existed at base", lambda: new in before),
        ("to does not exist at head", lambda: new not in after),
        ("lint or scope differs", lambda: ledger.rule(old) != ledger.rule(new)),
        ("head exceeds from", lambda: not ledger.fits(after[new], before[old])),
        ("key carried twice", lambda: new in valid or old in valid.values()),
    )
    return next((problem for problem, failed in checks if failed()), "")


def checked_carries(proposals: list[dict[str, Any]], ledger: Ledger) -> dict[str, str]:
    """Return valid `to -> from` rekeys; print why each other carry fails."""
    valid: dict[str, str] = {}
    for carry in proposals:
        if carry.get("kind") != "carry" or carry.get("tool") != ledger.tool:
            continue
        if problem := carry_problem(carry, ledger, valid):
            print(f"invalid carry ({problem}): {json.dumps(carry, sort_keys=True)}")
        else:
            valid[carry["to"]] = carry["from"]
    return valid


def carry_hints(ledger: Ledger, retired: Iterable[str], added: Iterable[str]) -> None:
    """Print ready records for one-to-one same-lint pairs, else candidates."""
    retired, added = sorted(retired), sorted(added)
    rule = ledger.rule
    for new in added:
        olds = [old for old in retired if rule(old) == rule(new)]
        news = [other for other in added if rule(other) == rule(new)]
        if len(olds) == len(news) == 1 and ledger.fits(
            ledger.after[new], ledger.before[olds[0]]
        ):
            carry = {"kind": "carry", "tool": ledger.tool, "from": olds[0], "to": new}
            print("ready carry: " + json.dumps(carry))
        elif olds:
            print(f"carry candidates for {new}: " + "; ".join(olds))


def associated_pull(pulls: list[dict[str, Any]], head: str) -> dict[str, Any] | None:
    return next(
        (
            pull
            for pull in pulls
            if pull["head"]["sha"] == head
            and (pull["state"] == "open" or pull.get("merged_at"))
        ),
        None,
    )


def context() -> tuple[str, list[dict[str, Any]], list[str], str]:
    """Read PR body and both ordinary comments and submitted review bodies."""
    head = git("rev-parse", "HEAD").strip()
    event = (
        json.loads(open(path).read())
        if (path := os.getenv("GITHUB_EVENT_PATH"))
        else {}
    )
    pull = event.get("pull_request", {})
    # Without an event payload, HEAD is the PR head itself, including a local
    # base-update merge. Only when HEAD has no associated PR is a two-parent HEAD
    # taken to be CI's synthetic merge checkout, whose second parent is the PR head.
    candidates = [head]
    if pull:
        head = pull["head"]["sha"]
    elif len(git("rev-list", "--parents", "-n", "1", "HEAD").split()) == 3:
        candidates.append(git("rev-parse", "HEAD^2").strip())
        head = candidates[-1]
    repository = os.getenv("GITHUB_REPOSITORY")
    if not repository:
        repository = json.loads(gh("repo", "view", "--json", "nameWithOwner"))[
            "nameWithOwner"
        ]
    number = pull.get("number")
    if not number:
        for sha in candidates:
            pulls = json.loads(gh("api", f"repos/{repository}/commits/{sha}/pulls"))
            candidate = associated_pull(pulls, sha)
            if candidate:
                number = candidate["number"]
                head = candidate["head"]["sha"]
                break
    if number is None:
        return "", [], [], head
    prefix = f"repos/{repository}"
    current = json.loads(gh("api", f"{prefix}/pulls/{number}"))
    if current["head"]["sha"] != head:
        raise ValueError("PR head changed since the captured checkout")
    body = current["body"] or ""
    author = ATTRIBUTION.search(body)
    comments = discussion(prefix, number) + source_reviews(prefix, body, number)
    return author[1] if author else "", records(body), comments, head


def source_reviews(prefix: str, body: str, number: int) -> list[str]:
    """Comments on the component PRs an integration PR names.

    Each component's own author still cannot accept its contract.
    """
    comments: list[str] = []
    for record in records(body):
        if record.get("kind") != "reviews":
            continue
        for source in record.get("pulls", []):
            if not isinstance(source, int) or source == number:
                continue
            owner = ATTRIBUTION.search(
                json.loads(gh("api", f"{prefix}/pulls/{source}"))["body"] or ""
            )
            if owner:
                comments.extend(
                    text
                    for text in discussion(prefix, source)
                    if (match := ATTRIBUTION.search(text)) and match[1] != owner[1]
                )
    return comments


def discussion(prefix: str, number: int) -> list[str]:
    """Ordinary comments and submitted review bodies of one PR."""
    comments: list[str] = []
    for endpoint in (f"issues/{number}/comments", f"pulls/{number}/reviews"):
        pages = json.loads(gh("api", "--paginate", "--slurp", f"{prefix}/{endpoint}"))
        comments.extend(row.get("body") or "" for page in pages for row in page)
    return comments
