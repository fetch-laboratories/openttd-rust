"""Attributed review contracts and declared carries for cleanup checks.

A review record uses a fenced cleanup-acceptance JSON block. Shared GitHub
credentials are deliberate: independence is the Agent path, not the login.
"""

import json
import os
import re
from collections.abc import Callable, Iterable, Mapping
from typing import Any

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


def checked_carries(
    proposals: list[dict[str, Any]],
    tool: str,
    before: Mapping[str, Any],
    after: Mapping[str, Any],
    rule: Callable[[str], object],
    fits: Callable[[Any, Any], bool],
) -> dict[str, str]:
    """Return valid `to -> from` rekeys; print why each other carry fails."""
    valid: dict[str, str] = {}
    for carry in proposals:
        if carry.get("kind") != "carry" or carry.get("tool") != tool:
            continue
        old, new = carry.get("from"), carry.get("to")
        if not isinstance(old, str) or not isinstance(new, str):
            problem = "from and to must be keys"
        elif old not in before:
            problem = "from does not exist at base"
        elif old in after:
            problem = "from still exists at head"
        elif new in before:
            problem = "to existed at base"
        elif new not in after:
            problem = "to does not exist at head"
        elif rule(old) != rule(new):
            problem = "lint or scope differs"
        elif not fits(after[new], before[old]):
            problem = f"head {after[new]} exceeds {before[old]}"
        elif new in valid or old in valid.values():
            problem = "key carried twice"
        else:
            valid[new] = old
            continue
        print(f"invalid carry ({problem}): {json.dumps(carry, sort_keys=True)}")
    return valid


def carry_hints(
    tool: str,
    retired: Iterable[str],
    added: Iterable[str],
    before: Mapping[str, Any],
    after: Mapping[str, Any],
    rule: Callable[[str], object],
    fits: Callable[[Any, Any], bool],
) -> None:
    """Print ready records for one-to-one same-lint pairs, else candidates."""
    retired, added = sorted(retired), sorted(added)
    for new in added:
        olds = [old for old in retired if rule(old) == rule(new)]
        news = [other for other in added if rule(other) == rule(new)]
        if len(olds) == len(news) == 1 and fits(after[new], before[olds[0]]):
            carry = {"kind": "carry", "tool": tool, "from": olds[0], "to": new}
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
        candidate = associated_pull(pulls, head)
        number = candidate["number"] if candidate else None
        if candidate:
            head = candidate["head"]["sha"]
    if number is None:
        return "", [], [], head
    prefix = f"repos/{repository}"
    current = json.loads(gh("api", f"{prefix}/pulls/{number}"))
    if current["head"]["sha"] != head:
        raise ValueError("PR head changed since the captured checkout")
    body = current["body"] or ""
    author = ATTRIBUTION.search(body)
    comments = discussion(prefix, number)
    # Integration PRs name the component PRs whose content-bound acceptances
    # they reuse; each component's own author still cannot accept its contract.
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
    return author[1] if author else "", records(body), comments, head


def discussion(prefix: str, number: int) -> list[str]:
    """Ordinary comments and submitted review bodies of one PR."""
    comments: list[str] = []
    for endpoint in (f"issues/{number}/comments", f"pulls/{number}/reviews"):
        pages = json.loads(gh("api", "--paginate", "--slurp", f"{prefix}/{endpoint}"))
        comments.extend(row.get("body") or "" for page in pages for row in page)
    return comments
