"""A PR's reviewer call, on the review row, within the review limits (#1091).

`ci.py reviewer PR KIND` prints the `delegate_task` call that starts one
reviewer: the review row's model, provider and effort from docs/team.md's role
table, worktree isolation and the fixed message of docs/implementer-brief.md.
Before printing it, the command counts the PR's reviews, from the reports on
GitHub and the starts it printed in this checkout, and refuses a review beyond
what docs/workflow.md's Review gives a PR: a second whole-PR review of one kind
(ADR 0051), a second re-check (ADR 0025), or a third scoped review that is not a
base update's. The first two are fixed by their ADRs; the third prints the one
line the implementer sends root, and root's approval, passed back with --root,
lets that call through. A re-check waits for every first-stage report.

A start holds its place until its report lands: --again prints the same call
for a dispatch that failed, and --release frees a start whose reviewer ended
with no report, which counts as no review (docs/workflow.md, Review).

A report from another model or effort is void (ADR 0011). `ci.py report`
refuses one, by its attribution line and, under Claude Code, by the model the
reviewer's own process runs (`report_refusal`). A void report, or one with no
disposition, counts for nothing here.

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
from collections.abc import Generator
from contextlib import contextmanager
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import NotRequired, TypedDict

import review_report
import status_comment
import validation_execution
import vcs
from status_comment import Target

TEAM = vcs.ROOT / "docs" / "team.md"
LEDGER = vcs.ROOT / ".local" / "reviewers"
REVIEW_ROW = "Independent review"
KINDS = ("behaviour", "porting", "recheck", "base", "simulation", "records")
# A scoped review's subject and the review it gets (docs/workflow.md, Review).
SCOPED = {"base": "behaviour", "simulation": "behaviour", "records": "porting"}
SCOPED_LIMIT = 2
FIRST_STAGE = ("behaviour", "porting")
GAME_CODE = ("src/", "rust/")
# A first-stage name, numbered when a void review took the plain one.
FIRST = re.compile(r"(behaviour|porting)(?:-\d+)?")
PROVIDERS = {"gpt-": ("codex", "reasoningEffort"), "claude-": ("claudeAgent", "effort")}
CELL = re.compile(r"`([^`]+)`,\s*([a-z]+)")
HELP = """Print the delegate_task call that starts one reviewer of a PR, on the
review row of docs/team.md with the fixed message of docs/implementer-brief.md,
or refuse a review beyond the PR's limits and print the line to send root.
KIND is behaviour or porting (the first stage), recheck, or a scoped review:
base (a base update's conflicts), simulation (a CI-found simulation fix) or
records (the acceptance of a cleanup record or unsafe_code item a fix changed)."""


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


def review_pairs() -> set[tuple[str, str]]:
    return set(review_row().values())


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


def signed(body: str) -> list[tuple[str, str]]:
    """The model and effort of each attribution line outside fences and
    quotations."""
    lines = review_report.ATTRIBUTION_LINE.finditer(review_report.own_text(body))
    return [(line[3], line[4]) for line in lines]


DECIDED = re.compile(
    r"^\W*Disposition\W*:\W*(?:accepted|findings to remediate)\b", re.I | re.M
)


def decided(body: str) -> bool:
    """Whether a report records its disposition: on its closing line, or on a
    line of its own that later follow-up text pushed up."""
    own = review_report.own_text(body)
    known = review_report.disposition(body)[0] != review_report.UNKNOWN
    return known or DECIDED.search(own) is not None


def off_row(body: str) -> list[tuple[str, str]]:
    """The signed model and effort pairs that are not the review row's."""
    pairs = review_pairs()
    return [pair for pair in signed(body) if pair not in pairs]


def report_refusal(
    body: str,
    environ: dict[str, str] | os._Environ[str] = os.environ,
    proc: Path = Path("/proc"),
) -> str | None:
    """Why a report from this reviewer is void for its model or effort, or None."""
    allowed = review_models()
    row = " or ".join(
        f"{model} at {effort}" for model, effort in sorted(review_pairs())
    )
    running = running_model(environ, proc)
    if running is not None and running not in allowed:
        return (
            f"you run on {running}, and a review runs on the review row's {row} "
            "(docs/team.md, ADR 0011), so this report would be void. Tell the "
            "agent that started you it named the wrong model in its delegate_task "
            "call, and that `python3 tools/ci.py reviewer` prints the right one"
        )
    if wrong := off_row(body):
        model, effort = wrong[0]
        return (
            f"the report is signed Model: {model} at Reasoning effort: {effort}, "
            f"and a review runs on the review row's {row} (docs/team.md, ADR "
            "0011). If that is what you run on, tell the agent that started you to "
            "start a reviewer on the review row instead"
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


@dataclass(frozen=True)
class Refusal:
    reason: str
    fixed: bool  # an ADR's limit, which root's answer does not lift


def refusal(
    pr: int, kind: str, earlier: list[Review], required: tuple[str, ...] = FIRST_STAGE
) -> Refusal | None:
    """Why one more review of this kind is beyond the PR's limits, or None. A
    void review counts for nothing. `required` is the first stage the PR's
    shape needs: behaviour alone for a PR with no game code (ADR 0051)."""
    earlier = [review for review in earlier if not review.void]
    stages = [review.stage for review in earlier]
    if kind in ("behaviour", "porting") and kind in stages:
        return Refusal(
            f"a second whole-PR {kind} review: a PR is reviewed whole, once, by "
            "one reviewer of each kind (ADR 0051)",
            True,
        )
    if kind == "recheck" and "recheck" in stages:
        return Refusal(
            "a second re-check: a PR has one, and it is the last review (ADR "
            "0025). Fix what it reported or record it as residue, and queue "
            "(docs/workflow.md, Review)",
            True,
        )
    reported = {r.stage for r in earlier if r.stage in FIRST_STAGE and r.url}
    if kind == "recheck" and (missing := [s for s in required if s not in reported]):
        return Refusal(
            "a re-check before every first-stage report: it needs a "
            f"{' and a '.join(missing)} report on the review row",
            True,
        )
    scoped = [review.name for review in earlier if review.stage == "scoped"]
    if kind in SCOPED and kind != "base" and len(scoped) >= SCOPED_LIMIT:
        return Refusal(
            f"a scoped review after {len(scoped)} others that are not a base "
            f"update's ({', '.join(scoped)}): docs/workflow.md's Review gives a "
            "series of them no end, so root decides whether it goes on",
            False,
        )
    return None


# The kind of review a stage's next start asks for.
NEXT = {
    "behaviour": "behaviour",
    "porting": "porting",
    "recheck": "recheck",
    "base": "base",
}


def past_limits(
    pr: int, earlier: list[Review], required: tuple[str, ...] = FIRST_STAGE
) -> list[str]:
    """Each review that started where this command refuses, the reviews before
    it taken as they stood: what root had to answer first."""
    return [
        review.name
        for index, review in enumerate(earlier)
        if not review.void
        and refusal(pr, NEXT.get(review.stage, "records"), earlier[:index], required)
    ]


class DelegateTarget(TypedDict):
    providerInstanceId: str
    model: str
    options: dict[str, str]


class DelegateCall(TypedDict):
    """The `delegate_task` arguments that start one reviewer."""

    title: str
    role: str
    isolation: str
    target: DelegateTarget
    task: str


class LedgerEntry(TypedDict):
    """A start this command printed, kept until its report lands."""

    name: str
    kind: str
    at: str
    call: DelegateCall
    root: NotRequired[str]


class Head(TypedDict):
    sha: str


class Pull(TypedDict):
    """A PR as the API gives it, with the fields read here."""

    html_url: str
    head: Head


def ledger_path(pr: int) -> Path:
    return LEDGER / f"{pr}.json"


@contextmanager
def locked(pr: int) -> Generator[None]:
    """Hold the PR's ledger while a start is read, checked and written: two
    reviewers started together each see the other."""
    with validation_execution.file_lock(
        LEDGER / f"{pr}.lock", label=f"PR #{pr} reviewers"
    ):
        yield


def well_formed(entry: object) -> bool:
    """A start: its name, kind and time as text, and a saved call, if any, with
    the fields `delegate_task` takes."""
    if not isinstance(entry, dict) or not all(
        isinstance(entry.get(key), str) for key in ("name", "kind", "at")
    ):
        return False
    call = entry.get("call", {"task": "", "target": {}})
    return (
        isinstance(call, dict)
        and isinstance(call.get("task"), str)
        and isinstance(call.get("target"), dict)
    )


def started(pr: int) -> list[LedgerEntry]:
    path = ledger_path(pr)
    try:
        text = path.read_text()
    except FileNotFoundError:
        return []
    try:
        entries = json.loads(text)
        if not isinstance(entries, list) or not all(map(well_formed, entries)):
            raise ValueError("not a list of starts")
    except ValueError as error:
        raise ValueError(
            f"{path} is not a ledger of starts ({error}); read it, then repair or "
            "remove it"
        ) from error
    return entries


def save(pr: int, entries: list[LedgerEntry]) -> None:
    path = ledger_path(pr)
    temporary = path.with_suffix(".json.tmp")
    temporary.write_text(json.dumps(entries, indent=1) + "\n")
    os.replace(temporary, path)


def reviews(
    pr: int,
    found: list[review_report.ReviewReport],
    starts: list[LedgerEntry],
    void: frozenset[str] | set[str] = frozenset(),
    undecided: frozenset[str] | set[str] = frozenset(),
) -> list[Review]:
    """The PR's reviews: each report, and each start printed here that has none.
    A report whose URL is in `void` was signed off the review row; one in
    `undecided`, or with no agent, recorded no disposition and is no review."""
    seen: dict[str, Review] = {}
    for report in sorted(found, key=lambda item: item.reported_at):
        if report.agent == review_report.UNKNOWN or report.url in undecided:
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


def target(model: str, effort: str) -> DelegateTarget:
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
    if start.kind == "records":
        # An acceptance, not a third review of the fixes (#1073).
        return (
            f"Do a records review of PR {url} under `docs/review-brief.md`. {you} "
            f"Judge the cleanup records and `unsafe_code` items the commits {commits} "
            "change."
        )
    review = SCOPED.get(start.kind, start.kind)
    scope = f"the commits {commits}" if start.kind in SCOPED else "the whole PR"
    return f"Review PR {url} under `docs/review-brief.md` for {review}. {you} Review {scope}."


def hours_since(at: str, now: datetime) -> float:
    return (
        now - datetime.fromisoformat(at.replace("Z", "+00:00"))
    ).total_seconds() / 3600


def root_line(
    pr: int, head: str, asked: Start, reason: str, earlier: list[Review]
) -> str:
    now = datetime.now(timezone.utc)
    names = ", ".join(review.name.removeprefix(f"review-{pr}-") for review in earlier)
    since = (
        f"; {hours_since(earlier[0].at, now):.1f} h since the first" if earlier else ""
    )
    return (
        f"PR #{pr}, head {head[:10]}: a {asked.kind} review of "
        f"{' '.join(asked.commits)} would be {reason}. Reviews so far: "
        f"{len(earlier)} ({names}){since}. Approve it, or have what it would "
        "review recorded as residue and the PR queued?"
    )


def pull(repository: str, pr: int) -> Pull:
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


@dataclass(frozen=True)
class Facts:
    head: str
    earlier: list[Review]
    required: tuple[str, ...]  # the first stage the PR's shape needs


def pull_files(repository: str, pr: int) -> list[str]:
    pages = json.loads(
        status_comment.gh(
            "api",
            "--paginate",
            "--slurp",
            f"repos/{repository}/pulls/{pr}/files?per_page=100",
        )
    )
    return [item["filename"] for page in pages for item in page]


def first_stage(files: list[str]) -> tuple[str, ...]:
    """Behaviour and porting for a PR that changes game code, behaviour alone
    for one that changes only tools, CI or records (docs/workflow.md, Review)."""
    game = any(name.startswith(GAME_CODE) for name in files)
    return FIRST_STAGE if game else ("behaviour",)


def read_reviews(repository: str, pr: int) -> Facts:
    """The PR's head and its reviews, from GitHub and this checkout's ledger."""
    facts = pull(repository, pr)
    head = facts["head"]["sha"]
    comments = status_comment.comments(Target(repository, pr, "PR", str(pr)))
    submitted = pull_reviews(repository, pr)
    found = review_report.review_reports(comments, submitted, head)
    bodies = {
        item["html_url"]: item.get("body") or "" for item in [*comments, *submitted]
    }
    void = {url for url, body in bodies.items() if off_row(body)}
    # A report has one attribution and a disposition (`ci.py report`); any other
    # text under a header is no review.
    undecided = {
        url
        for url, body in bodies.items()
        if not decided(body) or len(review_report.attributions(body)) != 1
    }
    earlier = reviews(pr, found, started(pr), void, undecided)
    return Facts(head, earlier, first_stage(pull_files(repository, pr)))


def show(pr: int, facts: Facts) -> None:
    earlier = facts.earlier
    print(
        f"PR #{pr}, head {facts.head[:10]}: {len(earlier)} reviews; its first "
        f"stage is {' and '.join(facts.required)}"
    )
    for review in earlier:
        void = " VOID, off the review row" if review.void else ""
        where = review.url or "(started here, no report yet)"
        print(f"  {review.at} {review.stage}: {review.name}{void} {where}")
    past = past_limits(pr, earlier, facts.required)
    print(f"Past the limits: {len(past)}" + (f" ({', '.join(past)})" if past else ""))
    for kind in KINDS:
        found = refusal(pr, kind, earlier, facts.required)
        print(f"  {kind}: " + (f"refused, {found.reason}" if found else "allowed"))


def release(pr: int, name: str, earlier: list[Review]) -> None:
    reported = {review.name for review in earlier if review.url}
    entries = started(pr)
    kept = [entry for entry in entries if entry["name"] != name]
    if len(kept) == len(entries):
        raise ValueError(f"no start named {name} in {ledger_path(pr)}")
    if name in reported:
        raise ValueError(f"{name} has reported; a report is a review, not a start")
    save(pr, kept)
    print(f"Released {name}: it counts for nothing now.")


def again(pr: int, kind: str, earlier: list[Review], name: str | None) -> None:
    reported = {review.name for review in earlier if review.url}
    waiting = [
        entry
        for entry in started(pr)
        if entry["kind"] == kind and entry["name"] not in reported and "call" in entry
    ]
    if name is not None:
        waiting = [entry for entry in waiting if entry["name"] == name]
    if not waiting:
        raise ValueError(
            f"PR #{pr} has no {kind} start {name or ''} without a report to print again"
        )
    if len(waiting) > 1:
        names = ", ".join(entry["name"] for entry in waiting)
        raise ValueError(
            f"PR #{pr} has {len(waiting)} {kind} starts without a report ({names}); "
            "name the one whose dispatch failed: --again NAME"
        )
    print(json.dumps(waiting[0]["call"], indent=1))
    print(
        "The same start, again: pass it to delegate_task as written.", file=sys.stderr
    )


def refuse(
    pr: int, head: str, asked: Start, found: Refusal, earlier: list[Review]
) -> None:
    if found.fixed:
        print(f"Not started: PR #{pr} asks for {found.reason}.", file=sys.stderr)
        raise SystemExit(1)
    line = root_line(pr, head, asked, found.reason, earlier)
    print(
        f"Not started: PR #{pr} asks for {found.reason}.\nSend root this line, "
        "queued, and start no reviewer until it answers:\n\n"
        + line
        + "\n\nIf root approves it, run this command again with --root approve "
        "--root-answer '<root's answer>'; otherwise do as it says. Put the answer "
        "in the status comment.",
        file=sys.stderr,
    )
    raise SystemExit(1)


def begin(args: argparse.Namespace, repository: str, kind: str) -> None:
    pr = args.pr
    with locked(pr):
        facts = read_reviews(repository, pr)
        head, earlier = facts.head, facts.earlier
        if args.again:
            again(pr, kind, earlier, None if args.again is True else args.again)
            return
        name = name_for(pr, kind, earlier)
        findings = [
            review.url
            for review in earlier
            if review.stage in FIRST_STAGE and review.url and not review.void
        ]
        asked = Start(kind, name, args.commits or [], findings)
        if found := refusal(pr, kind, earlier, facts.required):
            if found.fixed or args.root != "approve":
                refuse(pr, head, asked, found, earlier)
        if kind == "recheck" and not findings:
            raise ValueError(
                f"PR #{pr} has no first-stage report for a re-check to check"
            )
        model, effort = own_row()
        call: DelegateCall = {
            "title": f"Review PR {pr} {name.removeprefix(f'review-{pr}-')}",
            "role": "review",
            "isolation": "worktree",
            "target": target(model, effort),
            "task": message(pull(repository, pr)["html_url"], asked, (model, effort)),
        }
        entry: LedgerEntry = {
            "name": name,
            "kind": kind,
            "at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "call": call,
        }
        if args.root == "approve":
            entry["root"] = args.root_answer
        save(pr, [*started(pr), entry])
    print(json.dumps(call, indent=1))
    print(
        "Pass this to delegate_task as written. If the dispatch fails, print it "
        f"again with `ci.py reviewer {pr} {kind} --again`.",
        file=sys.stderr,
    )


def run(args: argparse.Namespace, repository: str) -> None:
    pr = args.pr
    if args.check:
        show(pr, read_reviews(repository, pr))
        return
    if args.release:
        with locked(pr):
            release(pr, args.release, read_reviews(repository, pr).earlier)
        return
    if args.root and not args.root_answer:
        raise ValueError("--root approve takes root's words too: --root-answer TEXT")
    kind = args.kind
    if kind is None:
        raise ValueError("name the review's KIND, or pass --check or --release NAME")
    if (kind == "recheck" or kind in SCOPED) and not args.commits and not args.again:
        raise ValueError(
            f"a {kind} review names the commits it reviews: --commits SHA ..."
        )
    begin(args, repository, kind)


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
        "--root",
        choices=("approve",),
        help="root approved this scoped review past the limit",
    )
    parser.add_argument(
        "--root-answer", metavar="TEXT", help="root's answer, kept with the start"
    )
    parser.add_argument(
        "--again",
        nargs="?",
        const=True,
        metavar="NAME",
        help="print a start with no report again, for a failed dispatch",
    )
    parser.add_argument(
        "--release",
        metavar="NAME",
        help="free a start whose reviewer ended with no report",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="print the PR's reviews and limits; start nothing",
    )
    parser.add_argument("--repo")
