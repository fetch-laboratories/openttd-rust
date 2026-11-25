"""A reviewer's report on a PR or a plan issue, for ci.py report (#1063).

A report opens with its header, `Reviewed commit:` or `Reviewed head:` on a PR
and `Reviewed plan: #<issue> at <commit>` on an issue, carries one attribution
line and closes with its disposition (docs/review-brief.md). The command reads a
body as the report readers in review_report.py do, so what it posts is what the
lint gates and `ci.py state` read. The agent's report is the comment by this
login that opens with the target's header and whose attribution names the
agent; a re-check's reviewer has a name of its own, so its report is another's.
Unlike a status comment, a report carries no marker: its header opens it.
"""

from __future__ import annotations

import argparse
import re
import sys

import review_report
import status_comment
from cleanup_contract import ATTRIBUTION
from review_report import Comment
from status_comment import Target

HELP = """Post a review report on a PR (or a plan review on an --issue) from a
file, or replace the one this agent posted. The body is posted as written: it
must open with its header, carry exactly one attribution line naming --agent
outside fenced blocks and quotations, and close with its disposition. Before
replacing, the command reads the comment again and refuses if it changed."""


def opens_report(target: Target, body: str) -> bool:
    """Whether the body opens with the report header for the target's kind."""
    if target.kind == "issue":
        return review_report.reviewed_plan(body) is not None
    return review_report.reviewed_sha(body) is not None


def refusal(target: Target, agent: str, body: str) -> str | None:
    """Why the body is not a report the readers can read, or None."""
    if target.kind == "issue":
        plan = review_report.reviewed_plan(body)
        if plan is None or plan[0] != target.number:
            return (
                f"a plan review opens with `Reviewed plan: #{target.number} at "
                "<commit>` on its first line"
            )
        names = review_report.PLAN_DISPOSITIONS
    else:
        if review_report.reviewed_sha(body) is None:
            return (
                "a review report opens with `Reviewed commit: <sha>` on its first line"
            )
        names = review_report.DISPOSITIONS
    found = [line[1] for line in ATTRIBUTION.finditer(review_report.own_text(body))]
    if found != [agent]:
        return (
            f"the report needs exactly one attribution line for {agent} outside "
            f"fenced blocks and quotations, and has {found or 'none'}"
        )
    if review_report.disposition(body, names)[0] == review_report.UNKNOWN:
        return f"the report's closing line gives no disposition: {' or '.join(names)}"
    return None


def reports(target: Target, found: list[Comment], agent: str, author: str) -> list:
    """The agent's reports among the comments: this login's, opening with the
    target's header, attributed to the agent."""
    return [
        comment
        for comment in found
        if (comment.get("user") or {}).get("login") == author
        and opens_report(target, body := comment.get("body") or "")
        and review_report.agent(body) == agent
    ]


def command(target: Target, agent: str, rest: str) -> str:
    return f"python3 tools/ci.py report {target.again} --agent {agent} {rest}"


def one(target: Target, found: list[Comment], agent: str) -> Comment | None:
    if len(found) > 1:
        raise ValueError(
            f"{len(found)} reports by {agent} on {target.kind} #{target.number}: "
            f"{status_comment.listed(found)}. Delete the extra ones on GitHub, then "
            "run this again"
        )
    return found[0] if found else None


def write(target: Target, agent: str, body: str) -> None:
    if reason := refusal(target, agent, body):
        raise ValueError(f"not posted: {reason}")
    status_comment.check_kind(target)
    author = status_comment.login()

    def find(found: list[Comment]) -> Comment | None:
        return one(target, reports(target, found, agent, author), agent)

    mine = find(status_comment.comments(target))
    if mine is None:
        result = status_comment.create(target, agent, author, body, find)
    else:
        result = status_comment.update(target, mine, body)
    print(result["html_url"])
    print("Show it again: " + command(target, agent, "--show"))


def show(target: Target, agent: str) -> None:
    status_comment.check_kind(target)
    author = status_comment.login()
    mine = one(
        target, reports(target, status_comment.comments(target), agent, author), agent
    )
    if mine is None:
        print(f"No report by {agent} on {target.kind} #{target.number}.")
        return
    print(mine["body"])
    print(f"Report: {mine['html_url']}", file=sys.stderr)
    print("Replace it: " + command(target, agent, "--body-file FILE"), file=sys.stderr)


def run(args: argparse.Namespace, repository: str) -> None:
    if (args.pr is None) == (args.issue is None):
        raise ValueError(
            "name one PR or issue: python3 tools/ci.py report PR --agent PATH ..., "
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
        if args.body_file:
            raise ValueError("--show only reads; drop --body-file")
        show(target, args.agent)
        return
    body = status_comment.read_body(args.body_file)
    if body is None:
        raise ValueError(
            "pass the report with --body-file FILE (or - for stdin), or read it "
            "with --show"
        )
    write(target, args.agent, body)


def register(commands: argparse._SubParsersAction) -> None:
    parser = commands.add_parser(
        "report",
        help="post or replace your review report on a PR or plan issue",
        description=HELP,
    )
    parser.add_argument("pr", type=int, nargs="?", metavar="PR")
    parser.add_argument("--issue", type=int, metavar="N", help="a plan issue")
    parser.add_argument(
        "--agent", required=True, metavar="PATH", help="your agent path"
    )
    parser.add_argument("--body-file", metavar="FILE", help="the report; - reads stdin")
    parser.add_argument(
        "--show", action="store_true", help="print your report; read-only"
    )
    parser.add_argument("--repo")
