"""An agent's status comment on a PR or issue, for ci.py comment.

A status comment is one whose first line is the hidden marker MARKER naming the
agent's exact path. Only such comments are read as status comments, edited or
created; an attribution line names an author, not a status comment, and a
reviewer's report carries one too. A legacy comment becomes a status comment
only through --comment-id, after checks that it is not a report.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from dataclasses import dataclass
from typing import NotRequired, TypedDict

from cleanup_contract import ATTRIBUTION
from review_report import REVIEWED, Comment

MARKER = "<!-- status comment: {agent} -->"
MARKED = re.compile(r"\A<!-- status comment: (\S+) -->[ \t]*(?:\r?\n|\Z)")
ANY_MARKER = re.compile(r"<!-- status comment:")
LOOSE_ATTRIBUTION = re.compile(r"^\W*Agent:\s")
ACCEPTANCE = "```cleanup-acceptance"
API = "https://api.github.com/repos/"
# A create that has not answered in this time is looked up, not repeated.
CREATE_TIMEOUT = 60
HELP = """Create or replace this agent's one status comment on a PR (or --issue).
It is found by a hidden first-line marker naming the exact agent path, never by
its attribution or by being the newest. The body keeps or gains one attribution
line for the agent. Before replacing it, the command reads the comment again
and refuses if it changed since the first read; GitHub has no compare-and-swap,
so this narrows the window for a lost edit but cannot close it."""


@dataclass(frozen=True)
class Target:
    repository: str
    number: int
    kind: str  # "PR" or "issue"
    again: str  # the arguments that name it again, such as `935` or `--issue 7`


def gh(*args: str, payload: str | None = None, timeout: float | None = None) -> str:
    return subprocess.run(
        ["gh", *args],
        input=payload,
        capture_output=True,
        text=True,
        check=True,
        timeout=timeout,
    ).stdout


class Issue(TypedDict):
    """An issue as the API gives it; a PR's carries `pull_request`."""

    number: int
    pull_request: NotRequired[dict[str, object]]


def issue(target: Target) -> Issue:
    return json.loads(gh("api", f"repos/{target.repository}/issues/{target.number}"))


def by_id(target: Target, comment_id: int) -> Comment:
    return json.loads(
        gh("api", f"repos/{target.repository}/issues/comments/{comment_id}")
    )


def login() -> str:
    return gh("api", "user", "--jq", ".login").strip()


def comments(target: Target) -> list[Comment]:
    """Every comment on the PR or issue, through every page."""
    pages = json.loads(
        gh(
            "api",
            "--paginate",
            "--slurp",
            f"repos/{target.repository}/issues/{target.number}/comments?per_page=100",
        )
    )
    return [comment for page in pages for comment in page]


def marked_for(body: str) -> str | None:
    """The agent path a body's first-line marker names, if it has one."""
    match = MARKED.match(body)
    return match[1] if match else None


def unmarked(body: str) -> str:
    return MARKED.sub("", body, count=1)


def ours(found: list[Comment], agent: str, author: str) -> list[Comment]:
    """The agent's status comments: its exact marker, written by this account."""
    return [
        comment
        for comment in found
        if marked_for(comment.get("body") or "") == agent
        and (comment.get("user") or {}).get("login") == author
    ]


def is_report(body: str) -> bool:
    return bool(REVIEWED.search(body)) or ACCEPTANCE in body


def command(target: Target, agent: str, rest: str) -> str:
    return f"python3 tools/ci.py comment {target.again} --agent {agent} {rest}"


def listed(found: list[Comment]) -> str:
    return "; ".join(f"{comment['id']} {comment['html_url']}" for comment in found)


def one(found: list[Comment], target: Target, agent: str) -> Comment | None:
    if len(found) > 1:
        raise ValueError(
            f"{len(found)} status comments for {agent} on {target.kind} "
            f"#{target.number}: {listed(found)}. Delete the extra ones on GitHub, "
            "then run this again"
        )
    return found[0] if found else None


def attributed(body: str, agent: str, model: str | None, effort: str | None) -> str:
    """The body with exactly one attribution line, for `agent`: kept when the
    body has it, appended from --model and --effort when it has none."""
    found = list(ATTRIBUTION.finditer(body))
    malformed = [
        line
        for line in body.splitlines()
        if LOOSE_ATTRIBUTION.match(line) and not ATTRIBUTION.search(line)
    ]
    if len(found) > 1 or malformed:
        raise ValueError(
            "the body needs exactly one attribution line, `Agent: PATH | Model: "
            "MODEL | Reasoning effort: EFFORT`; remove the others or fix the line"
        )
    if found:
        line = found[0]
        if line[1] != agent:
            raise ValueError(
                f"the body's attribution names {line[1]}, not {agent}; a status "
                "comment carries its own agent's line"
            )
        if model not in (None, line[2]) or effort not in (None, line[3]):
            raise ValueError(
                f"--model or --effort conflicts with the body's line ({line[0]}); "
                "drop the options or fix the line"
            )
        return body
    if not model or not effort:
        raise ValueError(
            "the body has no attribution line: pass --model and --effort, exactly "
            "as your host reports them, and it is appended"
        )
    return f"{body}\n\nAgent: {agent} | Model: {model} | Reasoning effort: {effort}"


def prepared(body: str, agent: str, model: str | None, effort: str | None) -> str:
    """The comment's full text: the marker, then the attributed body."""
    if marked_for(body) == agent:
        body = unmarked(body)
    if ANY_MARKER.search(body):
        raise ValueError(
            "the body carries a status-comment marker for another agent or in "
            "another place; remove it, the command adds the marker"
        )
    body = body.strip("\n")
    if not body.strip():
        raise ValueError("the body is empty; write the status to a file and pass it")
    body = attributed(body, agent, model, effort)
    return MARKER.format(agent=agent) + "\n" + body


def check_kind(target: Target) -> None:
    found = issue(target)
    if target.kind == "issue" and "pull_request" in found:
        raise ValueError(f"#{target.number} is a PR; name it without --issue")
    if target.kind == "PR" and "pull_request" not in found:
        raise ValueError(f"#{target.number} is an issue; name it with --issue")


def adoptable(target: Target, comment: Comment, agent: str, author: str) -> None:
    """Refuse a legacy comment that is not this agent's own status on this PR."""
    where = f"{API}{target.repository}/issues/{target.number}".lower()
    body = comment.get("body") or ""
    agents = [line[1] for line in ATTRIBUTION.finditer(body)]
    refusals = [
        (
            (comment.get("issue_url") or "").lower() != where,
            f"is on {comment.get('issue_url')}, not {target.kind} #{target.number}",
        ),
        (
            (comment.get("user") or {}).get("login") != author,
            f"was written by {(comment.get('user') or {}).get('login')}, not {author}",
        ),
        (
            marked_for(body) not in (None, agent),
            f"is the status comment of {marked_for(body)}",
        ),
        (agents != [agent], f"has attribution for {agents or 'nobody'}, not {agent}"),
        (is_report(body), "is a review report or cleanup acceptance"),
    ]
    reasons = [reason for refused, reason in refusals if refused]
    if reasons:
        raise ValueError(
            f"comment {comment['id']} cannot become {agent}'s status comment: it "
            f"{'; it '.join(reasons)}. See the candidates with: "
            + command(target, agent, "--show")
        )


def update(target: Target, comment: Comment, body: str) -> Comment:
    """Replace the comment's body, refusing one that changed since it was read."""
    fresh = by_id(target, comment["id"])
    if (fresh.get("body"), fresh.get("updated_at")) != (
        comment.get("body"),
        comment.get("updated_at"),
    ):
        raise ValueError(
            f"comment {comment['id']} changed since it was read "
            f"(updated {fresh.get('updated_at')}); read it with --show, merge your "
            "change into it, and run this again"
        )
    return json.loads(
        gh(
            "api",
            "--method",
            "PATCH",
            f"repos/{target.repository}/issues/comments/{comment['id']}",
            "--input",
            "-",
            payload=json.dumps({"body": body}),
        )
    )


def create(target: Target, agent: str, author: str, body: str) -> Comment:
    """Post the comment once. A post that fails or does not answer may still have
    landed, so the marker is looked up instead of posting again."""
    try:
        return json.loads(
            gh(
                "api",
                "--method",
                "POST",
                f"repos/{target.repository}/issues/{target.number}/comments",
                "--input",
                "-",
                payload=json.dumps({"body": body}),
                timeout=CREATE_TIMEOUT,
            )
        )
    except (subprocess.TimeoutExpired, subprocess.CalledProcessError) as error:
        landed = one(ours(comments(target), agent, author), target, agent)
        if landed is None:
            raise ValueError(
                f"creating the status comment failed ({error}) and none is on "
                f"{target.kind} #{target.number}; run the same command again"
            ) from error
        print(f"The create did not answer ({error}), but the comment is there")
        return landed


def chosen(
    target: Target, agent: str, author: str, comment_id: int | None
) -> Comment | None:
    """The comment to replace: the agent's marked one, or an adopted legacy one."""
    found = comments(target)
    mine = one(ours(found, agent, author), target, agent)
    if comment_id is None or (mine and mine["id"] == comment_id):
        return mine
    if mine:
        raise ValueError(
            f"{agent} already has a status comment on {target.kind} "
            f"#{target.number}: {mine['html_url']}; edit that one with: "
            + command(target, agent, "--body-file FILE")
        )
    comment = by_id(target, comment_id)
    adoptable(target, comment, agent, author)
    return comment


def write(args: argparse.Namespace, target: Target, body: str | None) -> None:
    check_kind(target)
    author = login()
    comment = chosen(target, args.agent, author, args.comment_id)
    if body is None:
        body = (comment or {}).get("body") or ""
    text = prepared(body, args.agent, args.model, args.effort)
    if comment is None:
        result = create(target, args.agent, author, text)
    else:
        result = update(target, comment, text)
    print(result["html_url"])
    print("Show it again: " + command(target, args.agent, "--show"))


def show(target: Target, agent: str) -> None:
    """Print the status comment's body (stdout) and where it is (stderr), or say
    there is none and list the agent's unmarked comments. Creates nothing."""
    check_kind(target)
    author = login()
    found = comments(target)
    mine = one(ours(found, agent, author), target, agent)
    if mine:
        print(unmarked(mine["body"]))
        print(f"Status comment: {mine['html_url']}", file=sys.stderr)
        print(
            "Replace it: " + command(target, agent, "--body-file FILE"), file=sys.stderr
        )
        return
    print(f"No status comment for {agent} on {target.kind} #{target.number}.")
    legacy = [
        comment
        for comment in found
        if marked_for(comment.get("body") or "") is None
        and (comment.get("user") or {}).get("login") == author
        and [line[1] for line in ATTRIBUTION.finditer(comment.get("body") or "")]
        == [agent]
    ]
    for comment in legacy:
        kind = (
            "review report, not adoptable" if is_report(comment["body"]) else "comment"
        )
        print(f"  {comment['id']} ({kind}): {comment['html_url']}")
    adoptable_ids = [str(c["id"]) for c in legacy if not is_report(c["body"])]
    if adoptable_ids:
        chosen_id = adoptable_ids[0] if len(adoptable_ids) == 1 else "ID"
        print(
            "Adopt it as the status comment: "
            + command(target, agent, f"--comment-id {chosen_id}")
        )
    print(
        "Create one: "
        + command(target, agent, "--body-file FILE --model MODEL --effort EFFORT")
    )


def read_body(name: str | None) -> str | None:
    if name is None:
        return None
    if name == "-":
        return sys.stdin.read()
    with open(name, encoding="utf-8") as file:
        return file.read()


def run(args: argparse.Namespace, repository: str) -> None:
    if (args.pr is None) == (args.issue is None):
        raise ValueError(
            "name one PR or issue: python3 tools/ci.py comment PR --agent PATH ..., "
            "or --issue N"
        )
    if not re.fullmatch(r"/[^\s>]*", args.agent):
        raise ValueError(
            f"--agent takes your agent path, such as /root/x, not {args.agent!r}"
        )
    number = args.issue if args.pr is None else args.pr
    again = f"--issue {number}" if args.pr is None else str(number)
    if args.repo:
        again += f" --repo {repository}"
    target = Target(repository, number, "issue" if args.pr is None else "PR", again)
    if args.show:
        if args.body_file or args.comment_id or args.model or args.effort:
            raise ValueError("--show only reads; drop the other options")
        show(target, args.agent)
    elif args.body_file is None and args.comment_id is None:
        raise ValueError(
            "pass the status with --body-file FILE (or - for stdin), or read it with "
            "--show"
        )
    else:
        write(args, target, read_body(args.body_file))


def register(commands: argparse._SubParsersAction) -> None:
    parser = commands.add_parser(
        "comment",
        help="create or replace your one status comment on a PR or issue",
        description=HELP,
    )
    parser.add_argument("pr", type=int, nargs="?", metavar="PR")
    parser.add_argument("--issue", type=int, metavar="N", help="an issue, not a PR")
    parser.add_argument(
        "--agent", required=True, metavar="PATH", help="your agent path"
    )
    parser.add_argument(
        "--body-file", metavar="FILE", help="the new body; - reads stdin"
    )
    parser.add_argument("--model", help="for a body without an attribution line")
    parser.add_argument("--effort", help="for a body without an attribution line")
    parser.add_argument(
        "--show",
        action="store_true",
        help="print the status comment, or list your unmarked comments; read-only",
    )
    parser.add_argument(
        "--comment-id",
        type=int,
        metavar="ID",
        help="adopt your legacy status comment ID by adding the marker",
    )
    parser.add_argument("--repo")
