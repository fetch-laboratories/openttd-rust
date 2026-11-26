"""A PR's reviewer call, on the review row, within the review limits (#1091).

`ci.py reviewer PR KIND` prints the `delegate_task` call that starts one
reviewer: the review row's model, provider and effort from docs/team.md's role
table, worktree isolation and the fixed message of docs/implementer-brief.md.
Before printing it, the command counts the PR's reviews, from the reports on
GitHub and the starts it printed in this checkout, and refuses a review beyond
what docs/workflow.md's Review gives a PR: a second whole-PR review of one kind
(ADR 0051), a second re-check (ADR 0025), or a third scoped review that is not a
base update's. A refusal prints the one line the implementer sends root; root's
answer, passed back with --root, lets the call through.

A report from another model is void (ADR 0011). `ci.py report` refuses one, by
its attribution line and, under Claude Code, by the model the reviewer's own
process runs (`report_refusal`).

The limits sit here because the implementer is the one agent that sees its
reviews pile up, and the host sends root nothing while a child waits on a job
with `t3_background_wait` (docs/team.md, Background jobs).
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path

import review_report
import status_comment
import vcs
from status_comment import Target

TEAM = vcs.ROOT / "docs" / "team.md"
LEDGER = vcs.ROOT / ".local" / "reviewers"
REVIEW_ROW = "Independent review"
KINDS = ("behaviour", "porting", "recheck", "base", "simulation", "records")
# A scoped review's subject and the review it gets (docs/workflow.md, Review).
SCOPED = {"base": "behaviour", "simulation": "behaviour", "records": "porting"}
SCOPED_LIMIT = 2
# A first-stage name, numbered when a void review took the plain one.
FIRST = re.compile(r"(behaviour|porting)(?:-\d+)?")
PROVIDERS = {"gpt-": ("codex", "reasoningEffort"), "claude-": ("claudeAgent", "effort")}
CELL = re.compile(r"`([^`]+)`,\s*([a-z]+)")
HELP = """Print the delegate_task call that starts one reviewer of a PR, on the
review row of docs/team.md with the fixed message of docs/implementer-brief.md,
or refuse a review beyond the PR's limits and print the line to send root.
KIND is behaviour or porting (the first stage), recheck, or a scoped review:
base (a base update's conflicts), simulation (a CI-found simulation fix) or
records (a cleanup record or unsafe_code item a fix changed)."""


def review_row(team: Path = TEAM) -> dict[str, tuple[str, str]]:
    """The review row's model and effort under each root's host."""
    lines = team.read_text().splitlines()
    header = next(line for line in lines if line.startswith("| Role |"))
    hosts = [cell.strip() for cell in header.strip("|").split("|")][1:]
    row = next(line for line in lines if line.startswith(f"| {REVIEW_ROW} |"))
    cells = [cell.strip() for cell in row.strip("|").split("|")][1:]
    found = {}
    for host, cell in zip(hosts, cells, strict=True):
        match = CELL.search(cell)
        if match is None:
            raise ValueError(
                f"docs/team.md's {REVIEW_ROW} row has no model in {cell!r}"
            )
        found[host] = (match[1], match[2])
    return found


def review_models() -> set[str]:
    return {model for model, _ in review_row().values()}


def own_row(environ: dict[str, str] | os._Environ[str] = os.environ) -> tuple[str, str]:
    """The review row for this agent's host: an implementer runs on its root's."""
    row = review_row()
    host = "Claude Code" if environ.get("CLAUDECODE") else "Codex"
    for name, value in row.items():
        if host in name:
            return value
    return next(iter(row.values()))


def running_model(
    environ: dict[str, str] | os._Environ[str] = os.environ, proc: Path = Path("/proc")
) -> str | None:
    """The model this process's agent runs, where the host shows it: Claude Code
    starts its agent with `--model`. None where it cannot be read; a Codex
    reviewer's model is only what its report signs."""
    if not environ.get("CLAUDECODE"):
        return None
    pid = environ.get("CLAUDE_PID")
    try:
        argv = (proc / str(pid) / "cmdline").read_bytes().decode().split("\0")
    except (OSError, UnicodeDecodeError, TypeError):
        return "a Claude Code model"
    for index, arg in enumerate(argv):
        if arg.startswith("--model="):
            return arg.split("=", 1)[1]
        if arg == "--model" and index + 1 < len(argv):
            return argv[index + 1]
    return "a Claude Code model"


def signed_models(body: str) -> list[str]:
    """The model of each attribution line outside fences and quotations."""
    lines = review_report.ATTRIBUTION_LINE.finditer(review_report.own_text(body))
    return [line[3] for line in lines]


def report_refusal(
    body: str,
    environ: dict[str, str] | os._Environ[str] = os.environ,
    proc: Path = Path("/proc"),
) -> str | None:
    """Why a report from this reviewer is void for its model, or None."""
    allowed = review_models()
    row = " or ".join(sorted(allowed))
    running = running_model(environ, proc)
    if running is not None and running not in allowed:
        return (
            f"you run on {running}, and a review runs on the review row's {row} "
            "(docs/team.md, ADR 0011), so this report would be void. Tell the "
            "agent that started you it named the wrong model in its delegate_task "
            "call, and that `python3 tools/ci.py reviewer` prints the right one"
        )
    wrong = [model for model in signed_models(body) if model not in allowed]
    if wrong:
        return (
            f"the report is signed Model: {wrong[0]}, and a review runs on the "
            f"review row's {row} (docs/team.md, ADR 0011). If you run on "
            f"{wrong[0]}, tell the agent that started you to start a reviewer on "
            "the review row instead"
        )
    return None


@dataclass(frozen=True)
class Review:
    name: str  # the agent path's last part, such as review-1066-records-porting-2
    stage: str  # behaviour, porting, recheck, base or scoped
    at: str
    url: str | None = None
    disposition: str = review_report.UNKNOWN
    void: bool = False  # signed by a model off the review row (ADR 0011)


def stage(pr: int, agent: str) -> str:
    """A review's stage, from its agent name (`/root/review-<PR>-<rest>`)."""
    # Names before this command were written by hand: `pr-behaviour`,
    # `records-recheck` and `recheck-2` name the stages too.
    rest = agent.rsplit("/", 1)[-1].removeprefix(f"review-{pr}-").removeprefix("pr-")
    if "recheck" in rest:
        return "recheck"
    if first := FIRST.fullmatch(rest):
        return first[1]
    if rest.startswith("base"):
        return "base"
    return "scoped"


def refusal(pr: int, kind: str, earlier: list[Review]) -> str | None:
    """Why one more review of this kind is beyond the PR's limits, or None. A
    void review counts for nothing."""
    earlier = [review for review in earlier if not review.void]
    stages = [review.stage for review in earlier]
    if kind in ("behaviour", "porting") and kind in stages:
        return (
            f"a second whole-PR {kind} review: a PR is reviewed whole, once, by "
            "one reviewer of each kind (ADR 0051)"
        )
    if kind == "recheck" and "recheck" in stages:
        return "a second re-check: a PR has one, and it is the last review (ADR 0025)"
    scoped = [review.name for review in earlier if review.stage == "scoped"]
    if kind in SCOPED and kind != "base" and len(scoped) >= SCOPED_LIMIT:
        return (
            f"a scoped review after {len(scoped)} others that are not a base "
            f"update's ({', '.join(scoped)}): docs/workflow.md's Review gives a "
            "series of them no end, so root decides whether it goes on"
        )
    return None


# The kind of review a stage's next start asks for.
NEXT = {
    "behaviour": "behaviour",
    "porting": "porting",
    "recheck": "recheck",
    "base": "base",
}


def past_limits(pr: int, earlier: list[Review]) -> list[str]:
    """Each review that started where this command refuses, the reviews before
    it taken as they stood: what root had to answer first."""
    return [
        review.name
        for index, review in enumerate(earlier)
        if not review.void
        and refusal(pr, NEXT.get(review.stage, "records"), earlier[:index])
    ]


def ledger_path(pr: int) -> Path:
    return LEDGER / f"{pr}.json"


def started(pr: int) -> list[dict[str, str]]:
    try:
        return json.loads(ledger_path(pr).read_text())
    except (OSError, ValueError):
        return []


def reviews(
    pr: int,
    found: list[review_report.ReviewReport],
    starts: list[dict[str, str]],
    void: frozenset[str] | set[str] = frozenset(),
) -> list[Review]:
    """The PR's reviews: each report, and each start printed here that has none.
    A report whose URL is in `void` was signed off the review row."""
    seen: dict[str, Review] = {}
    for report in sorted(found, key=lambda item: item.reported_at):
        if report.agent == review_report.UNKNOWN:
            continue
        name = report.agent.rsplit("/", 1)[-1]
        seen.setdefault(
            name,
            Review(
                name,
                stage(pr, report.agent),
                report.reported_at,
                report.url,
                report.disposition,
                report.url in void,
            ),
        )
    for start in starts:
        seen.setdefault(
            start["name"], Review(start["name"], stage(pr, start["name"]), start["at"])
        )
    return sorted(seen.values(), key=lambda review: review.at)


def name_for(pr: int, kind: str, earlier: list[Review]) -> str:
    base = f"review-{pr}-" + (kind if kind not in SCOPED else f"{kind}-{SCOPED[kind]}")
    names = {review.name for review in earlier}
    if base not in names:
        return base
    number = 2
    while f"{base}-{number}" in names:
        number += 1
    return f"{base}-{number}"


def target(model: str, effort: str) -> dict[str, object]:
    for prefix, (provider, option) in PROVIDERS.items():
        if model.startswith(prefix):
            return {
                "providerInstanceId": provider,
                "model": model,
                "options": {option: effort},
            }
    raise ValueError(f"no provider known for the review row's model {model}")


@dataclass(frozen=True)
class Start:
    """One reviewer to start: what it reviews and what it is given."""

    kind: str
    name: str
    commits: list[str]
    findings: list[str]


def message(url: str, start: Start, row: tuple[str, str]) -> str:
    model, effort = row
    you = f"You are /root/{start.name}, on {model} at {effort} effort."
    commits = " ".join(start.commits)
    if start.kind == "recheck":
        return (
            f"Re-check PR {url} under `docs/review-brief.md`. {you} Check the fixing "
            f"commits {commits} against the findings in {' '.join(start.findings)}."
        )
    review = SCOPED.get(start.kind, start.kind)
    scope = f"the commits {commits}" if start.kind in SCOPED else "the whole PR"
    return f"Review PR {url} under `docs/review-brief.md` for {review}. {you} Review {scope}."


def hours_since(at: str, now: datetime) -> float:
    return (
        now - datetime.fromisoformat(at.replace("Z", "+00:00"))
    ).total_seconds() / 3600


def root_line(
    pr: int, head: str, reason: str, earlier: list[Review], now: datetime
) -> str:
    names = ", ".join(review.name.removeprefix(f"review-{pr}-") for review in earlier)
    since = (
        f"; {hours_since(earlier[0].at, now):.1f} h since the first" if earlier else ""
    )
    return (
        f"PR #{pr}, head {head[:10]}: asks for {reason}. Reviews so far: "
        f"{len(earlier)} ({names}){since}. Start it, or record what it would "
        "review as residue and queue?"
    )


def pull(repository: str, pr: int) -> dict:
    return json.loads(status_comment.gh("api", f"repos/{repository}/pulls/{pr}"))


def pull_reviews(repository: str, pr: int) -> list[review_report.Review]:
    pages = json.loads(
        status_comment.gh(
            "api",
            "--paginate",
            "--slurp",
            f"repos/{repository}/pulls/{pr}/reviews?per_page=100",
        )
    )
    return [review for page in pages for review in page]


def run(args: argparse.Namespace, repository: str) -> None:
    pr = args.pr
    facts = pull(repository, pr)
    head = facts["head"]["sha"]
    comments = status_comment.comments(Target(repository, pr, "PR", str(pr)))
    found = review_report.review_reports(comments, pull_reviews(repository, pr), head)
    allowed = review_models()
    void = {
        comment["html_url"]
        for comment in comments
        if any(
            model not in allowed for model in signed_models(comment.get("body") or "")
        )
    }
    starts = started(pr)
    earlier = reviews(pr, found, starts, void)
    now = datetime.now(timezone.utc)
    if args.check:
        print(f"PR #{pr}, head {head[:10]}: {len(earlier)} reviews")
        for review in earlier:
            void = " VOID, off the review row" if review.void else ""
            where = review.url or "(started here, no report yet)"
            print(f"  {review.at} {review.stage}: {review.name}{void} {where}")
        past = past_limits(pr, earlier)
        print(
            f"Past the limits: {len(past)}" + (f" ({', '.join(past)})" if past else "")
        )
        for kind in KINDS:
            reason = refusal(pr, kind, earlier)
            print(f"  {kind}: " + (f"refused, {reason}" if reason else "allowed"))
        return
    kind = args.kind
    if kind is None:
        raise ValueError("name the review's KIND, or pass --check")
    if (kind == "recheck" or kind in SCOPED) and not args.commits:
        raise ValueError(
            f"a {kind} review names the commits it reviews: --commits SHA ..."
        )
    reason = refusal(pr, kind, earlier)
    if reason and not args.root:
        print(
            f"Not started: PR #{pr} {reason}.\nSend root this line, queued, and start "
            "no reviewer until it answers:\n\n"
            + root_line(pr, head, reason, earlier, now)
            + "\n\nThen run this command again with --root '<root's answer>', and put "
            "the answer in the status comment.",
            file=sys.stderr,
        )
        raise SystemExit(1)
    findings = [
        review.url
        for review in earlier
        if review.stage in ("behaviour", "porting") and review.url and not review.void
    ]
    if kind == "recheck" and not findings:
        raise ValueError(f"PR #{pr} has no first-stage report for a re-check to check")
    model, effort = own_row()
    name = name_for(pr, kind, earlier)
    call = {
        "title": f"Review PR {pr} {name.removeprefix(f'review-{pr}-')}",
        "role": "review",
        "isolation": "worktree",
        "target": target(model, effort),
        "task": message(
            facts["html_url"],
            Start(kind, name, args.commits or [], findings),
            (model, effort),
        ),
    }
    entry = {"name": name, "kind": kind, "at": now.strftime("%Y-%m-%dT%H:%M:%SZ")}
    if args.root:
        entry["root"] = args.root
    LEDGER.mkdir(parents=True, exist_ok=True)
    ledger_path(pr).write_text(json.dumps([*starts, entry], indent=1) + "\n")
    print(json.dumps(call, indent=1))
    print("Pass this to delegate_task as written.", file=sys.stderr)


def register(commands: argparse._SubParsersAction) -> None:
    parser = commands.add_parser(
        "reviewer",
        help="print the delegate_task call for a PR's next reviewer, within its review limits",
        description=HELP,
    )
    parser.add_argument("pr", type=int, metavar="PR")
    parser.add_argument("kind", nargs="?", choices=KINDS, metavar="KIND")
    parser.add_argument(
        "--commits",
        nargs="+",
        metavar="SHA",
        help="the commits a re-check or scoped review reads",
    )
    parser.add_argument(
        "--root", metavar="ANSWER", help="root's answer to the line a refusal printed"
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="print the PR's reviews and limits; start nothing",
    )
    parser.add_argument("--repo")
