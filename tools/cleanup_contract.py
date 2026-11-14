"""Tracked cleanup records, attributed review acceptances and declared carries.

A change's carries and exception proposals live in its branch's file under
`migration/cleanup-records/`, so the same tree gives the same result locally
and in CI. Only records the change adds against the merge base count: a merged
PR's file is inert at every later base, and `--write` deletes it. A reviewer's
acceptance stays an attributed PR comment in a fenced cleanup-acceptance JSON
block. Shared GitHub credentials are deliberate: independence is the Agent
path, not the login.
"""

import json
import os
import re
import subprocess
from collections.abc import Callable, Iterable, Mapping
from typing import Any, NamedTuple

import vcs
from vcs import ROOT, gh, git

ATTRIBUTION = re.compile(r"Agent: (\S+) \| Model: (\S+) \| Reasoning effort: (\S+)")
BLOCK = re.compile(r"```cleanup-acceptance\s*\n(.*?)\n```", re.S)
RECORDS = "migration/cleanup-records"
REASONS = ("reason", "retiring_owner", "retirement_condition")
CONTRACT = dict.fromkeys(
    ("tool", "site", "operation", "source_contract", *REASONS), "text"
)
# Each kind's fields: "text" is a nonempty string, "ints" a list of integers,
# "counts" a nonempty mapping to integers and "names" a mapping to strings.
FIELDS: dict[str, dict[str, str]] = {
    "carry": dict.fromkeys(("tool", "from", "to"), "text"),
    "lint": {**CONTRACT, "before": "ints", "after": "ints"},
    "suppression": {**CONTRACT, "before": "int", "after": "int"},
    "growth": {
        "counter": "text",
        "added": "counts",
        **dict.fromkeys(("existing_alternatives", *REASONS), "text"),
    },
    "refactor-proof": {"method": "text", "renames": "names"},
}


class ContractError(ValueError):
    """A malformed record or contract, reported in one line."""


def is_int(value: object) -> bool:
    return type(value) is int


SHAPES: dict[str, Callable[[Any], bool]] = {
    "text": lambda value: isinstance(value, str) and bool(value.strip()),
    "int": is_int,
    "ints": lambda value: isinstance(value, list) and all(map(is_int, value)),
    "counts": lambda value: (
        isinstance(value, dict) and bool(value) and all(map(is_int, value.values()))
    ),
    "names": lambda value: (
        isinstance(value, dict)
        and all(isinstance(item, str) for item in (*value, *value.values()))
    ),
}


def proposal(record: object, where: str) -> dict[str, Any]:
    """Check one record's kind and fields, naming the first bad one."""
    if not isinstance(record, dict):
        raise ContractError(f"{where}: a record must be a JSON object")
    kind = record.get("kind")
    fields = FIELDS.get(kind) if isinstance(kind, str) else None
    if fields is None:
        raise ContractError(f"{where}: field kind must be one of {', '.join(FIELDS)}")
    for name, shape in fields.items():
        if not SHAPES[shape](record.get(name)):
            raise ContractError(f"{where}: field {name} must be {shape}")
    return record


def parsed(text: str, where: str) -> object:
    try:
        return json.loads(text)
    except json.JSONDecodeError as error:
        raise ContractError(
            f"{where}: invalid JSON at line {error.lineno}: {error.msg}"
        ) from None


def file_records(text: str, where: str) -> list[dict[str, Any]]:
    data = parsed(text, where)
    if not isinstance(data, list):
        raise ContractError(f"{where}: the file must hold a JSON list of records")
    return [proposal(record, f"{where} record {n}") for n, record in enumerate(data, 1)]


def merge_base(base_ref: str) -> str:
    return vcs.merge_base(base_ref, run=git)


def at_base(base: str, path: str) -> str | None:
    """A record file's text at the merge base, or None where it is absent."""
    if not git("ls-tree", "--name-only", base, "--", path).strip():
        return None
    return git("show", f"{base}:{path}")


def tracked(base_ref: str) -> list[dict[str, Any]]:
    """The records this change adds, from the working tree.

    A file unchanged since the merge base is a merged PR's and inert. A new or
    changed file counts each record that file lacked at the merge base, so a
    later branch may repeat a merged record, such as an archive-proof claim.
    """
    base = merge_base(base_ref)
    found = []
    for path in sorted((ROOT / RECORDS).glob("*.json")):
        relative, text = str(path.relative_to(ROOT)), path.read_text()
        old = at_base(base, relative)
        if text == old:
            continue
        try:
            before = file_records(old, relative) if old is not None else []
        except ContractError:
            before = []
        found += [r for r in file_records(text, relative) if r not in before]
    return found


def write(base_ref: str, added: Iterable[dict[str, Any]]) -> None:
    """Add records to this branch's file and delete files unchanged since base."""
    base = merge_base(base_ref)
    branch = git("rev-parse", "--abbrev-ref", "HEAD").strip()
    if branch == "HEAD":
        raise ContractError("check out the change's branch to write cleanup records")
    own = ROOT / RECORDS / (branch.replace("/", "-") + ".json")
    for path in sorted((ROOT / RECORDS).glob("*.json")):
        relative = str(path.relative_to(ROOT))
        if path.read_text() == at_base(base, relative):
            path.unlink()
            print(f"deleted the merged records in {relative}")
    name = str(own.relative_to(ROOT))
    records = file_records(own.read_text(), name) if own.is_file() else []
    records += [record for record in added if record not in records]
    own.parent.mkdir(parents=True, exist_ok=True)
    own.write_text(json.dumps(records, indent=2, sort_keys=True) + "\n")
    print(f"recorded {len(records)} records in {name}")


def acceptances(body: str, where: str) -> list[dict[str, Any]]:
    """A comment's acceptance blocks: objects with `head` and `accept`."""
    found = []
    for n, text in enumerate(BLOCK.findall(body), 1):
        block = parsed(text, f"{where} block {n}")
        if not isinstance(block, dict) or "accept" not in block:
            continue
        if not isinstance(block["accept"], list) or not isinstance(
            block.get("head"), str
        ):
            raise ContractError(
                f"{where} block {n}: field accept must be a list and head a SHA"
            )
        found.append(block)
    return found


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
    for n, body in enumerate(comments, 1):
        attribution = ATTRIBUTION.search(body)
        if not attribution or attribution[1] == author:
            continue
        for record in acceptances(body, f"comment {n} by {attribution[1]}"):
            accepted = record["head"]
            if proposal in record["accept"] and (
                accepted == head
                or unchanged is not None
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
    old, new = carry["from"], carry["to"]
    before, after = ledger.before, ledger.after
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
        if carry["kind"] != "carry" or carry["tool"] != ledger.tool:
            continue
        if problem := carry_problem(carry, ledger, valid):
            print(f"invalid carry ({problem}): {json.dumps(carry, sort_keys=True)}")
        else:
            valid[carry["to"]] = carry["from"]
    return valid


def carry_hints(
    ledger: Ledger, retired: Iterable[str], added: Iterable[str]
) -> list[dict[str, str]]:
    """Print and return ready records for one-to-one same-lint pairs.

    Print candidates for the other added keys.
    """
    retired, added = sorted(retired), sorted(added)
    ready = []
    rule = ledger.rule
    for new in added:
        olds = [old for old in retired if rule(old) == rule(new)]
        news = [other for other in added if rule(other) == rule(new)]
        if len(olds) == len(news) == 1 and ledger.fits(
            ledger.after[new], ledger.before[olds[0]]
        ):
            carry = {"kind": "carry", "tool": ledger.tool, "from": olds[0], "to": new}
            print("ready carry: " + json.dumps(carry))
            ready.append(carry)
        elif olds:
            print(f"carry candidates for {new}: " + "; ".join(olds))
    return ready


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


def commit_pulls(repository: str, sha: str) -> list[dict[str, Any]]:
    """PRs GitHub associates with a commit; none for a commit it does not know,
    such as an unpushed local merge."""
    try:
        return json.loads(
            gh("api", f"repos/{repository}/commits/{sha}/pulls", capture_stderr=True)
        )
    except subprocess.CalledProcessError:
        return []


def context(base_ref: str) -> tuple[str, list[dict[str, Any]], list[str], str]:
    """Tracked proposals, with the PR's author, discussion and head.

    Carries need no review, so a change with only carries reads no GitHub
    state. An unreadable discussion leaves every acceptance unchecked.
    """
    proposals = tracked(base_ref)
    head = git("rev-parse", "HEAD").strip()
    if all(record["kind"] == "carry" for record in proposals):
        return "", proposals, [], head
    try:
        author, comments, head = pull_discussion(head)
    except (subprocess.CalledProcessError, OSError) as error:
        print(f"review comments unreadable ({error}); acceptances unchecked")
        return "", proposals, [], head
    return author, proposals, comments, head


def pull_discussion(head: str) -> tuple[str, list[str], str]:
    """The PR author's attribution, its comments and review bodies, and its head."""
    event = (
        json.loads(open(path).read())
        if (path := os.getenv("GITHUB_EVENT_PATH"))
        else {}
    )
    pull = event.get("pull_request", {})
    # Without an event payload, HEAD is normally the PR head itself, including a
    # pushed local base-update merge. A two-parent HEAD without its own PR is
    # either CI's synthetic merge checkout, whose second parent is the PR head
    # (open, or merged by this very merge commit), or an unpushed local
    # base-update merge, whose first parent is the open PR's head. A merged PR
    # reached through a base parent never matches.
    candidates: list[tuple[str, Callable[[dict[str, Any]], bool]]] = [
        (head, lambda _: True)
    ]
    if pull:
        head = pull["head"]["sha"]
    elif len(git("rev-list", "--parents", "-n", "1", "HEAD").split()) == 3:
        merge = head
        second = git("rev-parse", "HEAD^2").strip()
        candidates += [
            (
                second,
                lambda found: (
                    found["state"] == "open" or found.get("merge_commit_sha") == merge
                ),
            ),
            (
                git("rev-parse", "HEAD^1").strip(),
                lambda found: found["state"] == "open",
            ),
        ]
        head = second
    repository = os.getenv("GITHUB_REPOSITORY")
    if not repository:
        repository = json.loads(gh("repo", "view", "--json", "nameWithOwner"))[
            "nameWithOwner"
        ]
    number = pull.get("number")
    if not number:
        for sha, accepts in candidates:
            pulls = [found for found in commit_pulls(repository, sha) if accepts(found)]
            candidate = associated_pull(pulls, sha)
            if candidate:
                number = candidate["number"]
                head = candidate["head"]["sha"]
                break
    if number is None:
        return "", [], head
    prefix = f"repos/{repository}"
    current = json.loads(gh("api", f"{prefix}/pulls/{number}"))
    if current["head"]["sha"] != head:
        raise ContractError("PR head changed since the captured checkout")
    author = ATTRIBUTION.search(current["body"] or "")
    return author[1] if author else "", discussion(prefix, number), head


def discussion(prefix: str, number: int) -> list[str]:
    """Ordinary comments and submitted review bodies of one PR."""
    comments: list[str] = []
    for endpoint in (f"issues/{number}/comments", f"pulls/{number}/reviews"):
        pages = json.loads(gh("api", "--paginate", "--slurp", f"{prefix}/{endpoint}"))
        comments.extend(row.get("body") or "" for page in pages for row in page)
    return comments


def run(main: Callable[[], int]) -> None:
    """Exit with `main`'s status; a contract error is one line, not a traceback."""
    try:
        status = main()
    except ContractError as error:
        print(f"cleanup records: {error}")
        status = 1
    raise SystemExit(status)
