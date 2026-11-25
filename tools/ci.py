#!/usr/bin/env python3
"""Request scoped GitHub CI for an immutable PR head, using ordinary gh CLI."""

from __future__ import annotations

import argparse
import json
import re
import signal
import subprocess
import sys
import time
import uuid
from collections.abc import Callable, Collection, Mapping
from dataclasses import asdict, dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, NotRequired, TypedDict, cast
from urllib.parse import quote, urlsplit

import ci_log
import cleanup_contract
import preflight_stamp
import review_report
import status_comment
import vcs

PROFILES = ("rust", "native", "platform", "full")
WORKFLOW = "ci-request.yml"
PROTECTED_REF = "rust-migration"
CONTEXT = "Full validation"
# ci-invalidate.yml's merge-queue status on a PR head: no Full ran on that head.
QUEUED = "Full runs in the merge queue"
# A wait runs as a background job; a native run takes up to an hour.
WAIT_TIMEOUT = 5400
# A queue-wait runs as a background job; the queue's Full takes about an hour.
QUEUE_TIMEOUT = 5400
# GitHub computes mergeability for a few seconds after a push, and for minutes
# after a base push on a busy day; a request runs as a background job and waits.
MERGEABLE_WAIT = 300
MERGEABLE_INTERVAL = 3
# Quick's required check on a PR head, from its pull_request run.
QUICK = "Quick validation"
# The workflow whose reporting job is the Quick validation check (#1063).
QUICK_WORKFLOW = "ci-quick.yml"
# The one branch prefix of deliberate negative controls; `failures` leaves them out.
CONTROL = "control/"
FAILED = ("failure", "timed_out", "startup_failure")
# A merge-group run is cancelled only by a job timeout or by hand: its concurrency
# group is its own run, so no later run cancels it. Either removes the PR.
REMOVED = (*FAILED, "cancelled")
REQUESTS = Path(".local/ci-requests")
# Job conclusions that fail a run; a cancelled job or lost runner has no failed step.
JOB_FAILED = (
    "failure",
    "timed_out",
    "cancelled",
    "action_required",
    "startup_failure",
    "stale",
)
RUN_URL = re.compile(
    r"https://github\.com/([^/\s]+/[^/\s]+)/actions/runs/(\d+)"
    r"(?:/attempts/(\d+))?(?:/job/(\d+))?/?(?:[?#]\S*)?"
)


# A read that GitHub answers with a gateway error is tried this many times.
READ_ATTEMPTS = 3
RETRY_PAUSE = 2
# gh reports a gateway error as `HTTP 502: Bad Gateway (URL)`. GitHub also
# answers a healthy login's read with `Bad credentials (HTTP 401)` now and then:
# a wait stopped twice on it in one job (#1063).
TRANSIENT = re.compile(
    r"\bHTTP 50[234]\b|\b50[234] (?:Bad Gateway|Service Unavailable|Gateway Time-?out)"
    r"|Bad credentials \(HTTP 401\)"
)
# gh api flags that make a request other than a GET.
WRITES = ("-X", "--method", "-f", "--raw-field", "-F", "--field", "--input")


def read_only(args: tuple[str, ...]) -> bool:
    """Whether a gh call only reads: a REST GET, a variable list, a repo view or
    the fixed QUEUE_STATE query (a POST that changes nothing). No mutation and no
    pr, workflow or run subcommand is a read here."""
    if args[:2] in (("variable", "list"), ("repo", "view")):
        return True
    if args[:1] != ("api",):
        return False
    if args[1:2] == ("graphql",):
        return f"query={QUEUE_STATE}" in args
    return not any(arg.split("=", 1)[0] in WRITES for arg in args[1:])


def transient(error: subprocess.CalledProcessError) -> bool:
    """A gateway error or a false 401, which GitHub returns now and then."""
    return bool(TRANSIENT.search(f"{error.stderr or ''} {error.output or ''}"))


def gh(*args: str) -> str:
    """Run gh; a read that fails with HTTP 502, 503, 504 or a `Bad credentials`
    401 is tried again."""
    attempts = READ_ATTEMPTS if read_only(args) else 1
    for attempt in range(1, attempts + 1):
        try:
            return vcs.gh(*args, capture_stderr=True).strip()
        except subprocess.CalledProcessError as error:
            if attempt == attempts or not transient(error):
                raise
            print(
                f"gh {' '.join(args[:2])}: transient GitHub error, attempt "
                f"{attempt} of {attempts}; trying again",
                file=sys.stderr,
                flush=True,
            )
            time.sleep(RETRY_PAUSE * attempt)
    raise AssertionError("unreachable")


def api(repository: str, path: str) -> dict:
    return json.loads(gh("api", f"repos/{repository}/{path}"))


def detached_gh(*args: str) -> str:
    """gh in its own session, so a signal to this command's process group,
    such as Ctrl-C or signal-witness.py's SIGTERM, cannot end it mid-request:
    its outcome is always known. Not retried: callers use it for writes."""
    return subprocess.run(
        ["gh", *args],
        check=True,
        capture_output=True,
        text=True,
        start_new_session=True,
    ).stdout.strip()


def repository_of(repo: str | None) -> str:
    return (
        repo
        or json.loads(gh("repo", "view", "--json", "nameWithOwner"))["nameWithOwner"]
    )


def current_head(pull: dict, head: str | None = None) -> str:
    if pull["state"] != "open" or pull["base"]["ref"] != PROTECTED_REF:
        raise ValueError("PR must be open and target rust-migration")
    current = pull["head"]["sha"]
    if head is not None and current != head:
        raise HeadMoved(
            f"PR head changed: requested {head}, current {current}; that request "
            "no longer covers the PR, so request CI for the new head"
        )
    return current


class HeadMoved(ValueError):
    """The PR's head is no longer the one a request or wait was made for."""


class MergeabilityPending(ValueError):
    """GitHub has not yet computed the PR's mergeability."""


def validate_pull(pull: dict, head: str | None = None) -> str:
    # Mergeability selects the tested merge, so only requests require it. A base
    # push can leave it pending; the requested run and its result remain valid.
    current = current_head(pull, head)
    if pull.get("mergeable") is False:
        raise ValueError(
            f"The PR conflicts with {PROTECTED_REF}: merge origin/{PROTECTED_REF} "
            "into its branch (docs/workflow.md, Integration, step 1), push, and "
            "request again"
        )
    if pull.get("mergeable") is not True or not pull.get("merge_commit_sha"):
        raise MergeabilityPending(
            "GitHub is still computing the PR's mergeability; request again in a minute"
        )
    return current


def await_mergeable(repository: str, pr: int, pull: dict, head: str) -> dict:
    """Re-read a PR whose mergeability is pending, for up to MERGEABLE_WAIT
    seconds, at the same head; a conflict, closed PR or new head stops at once."""
    deadline = time.monotonic() + MERGEABLE_WAIT
    waiting = False
    while True:
        try:
            validate_pull(pull, head)
            return pull
        except MergeabilityPending:
            if time.monotonic() >= deadline:
                raise
        if not waiting:
            print(
                f"Waiting up to {MERGEABLE_WAIT} seconds for GitHub to compute PR "
                f"#{pr}'s mergeability",
                flush=True,
            )
            waiting = True
        time.sleep(MERGEABLE_INTERVAL)
        pull = api(repository, f"pulls/{pr}")


def expected_head(pull: dict, expected: str | None) -> str:
    """The PR's head, refused when it is not the head the caller expects."""
    head = current_head(pull)
    if expected and not head.startswith(expected):
        raise ValueError(
            f"PR head on GitHub is {head}, not the expected {expected}; push that "
            "commit, or pass the head you mean to test"
        )
    return head


def pushed_head(repository: str, pr: int, pull: dict, expected: str) -> dict:
    """The PR once GitHub shows the expected head, re-read for up to
    MERGEABLE_WAIT seconds: just after a push it can still answer with the
    previous head."""
    deadline = time.monotonic() + MERGEABLE_WAIT
    announced = False
    while not current_head(pull).startswith(expected) and time.monotonic() < deadline:
        if not announced:
            print(
                f"PR #{pr}'s head on GitHub is {current_head(pull)}, not {expected}; "
                f"waiting up to {MERGEABLE_WAIT} seconds for the push to show",
                flush=True,
            )
            announced = True
        time.sleep(MERGEABLE_INTERVAL)
        pull = api(repository, f"pulls/{pr}")
    return pull


def passed_run(repository: str, head: str) -> str | None:
    """Return the run URL of a successful Full validation already run for the head."""
    # Combined statuses omit creators; only the gate's GitHub Actions source qualifies.
    pages = json.loads(
        gh(
            "api",
            "--paginate",
            "--slurp",
            f"repos/{repository}/commits/{head}/statuses?per_page=100",
        )
    )
    full = next(
        (item for page in pages for item in page if item["context"] == CONTEXT), None
    )
    if full and full["state"] == "success" and full.get("description") != QUEUED:
        creator = full.get("creator") or {}
        if (
            creator.get("login") == "github-actions[bot]"
            and creator.get("type") == "Bot"
        ):
            return full["target_url"]
    return None


def variable(repository: str, name: str) -> str | None:
    variables = json.loads(
        gh("variable", "list", "--repo", repository, "--json", "name,value")
    )
    return next((item["value"] for item in variables if item["name"] == name), None)


def changes_definitions(repository: str, pr: int) -> bool:
    """Whether a PR may change .github/workflows/; ci-request.yml's plan refuses
    such an ordinary Full in merge-queue mode, under the same rule."""
    pages = json.loads(
        gh(
            "api",
            "--paginate",
            "--slurp",
            f"repos/{repository}/pulls/{pr}/files?per_page=100",
        )
    )
    files = [item for page in pages for item in page]

    def definition(path: object) -> bool:
        return not isinstance(path, str) or path.startswith(".github/workflows/")

    return not 0 < len(files) < 3000 or any(
        definition(item.get("filename"))
        or item.get("previous_filename") is not None
        and definition(item["previous_filename"])
        for item in files
    )


def find_run(record: dict, runs: list[dict]) -> dict | None:
    if record.get("run_id"):
        return next((run for run in runs if run["id"] == record["run_id"]), None)
    if record["bootstrap"]:
        prefix = f"CI full PR {record['pr']} bootstrap-"
        matches = [
            run
            for run in runs
            if run.get("display_title", "").startswith(prefix)
            and run.get("event") == "pull_request"
            and run.get("created_at", "") >= record["created_at"]
        ]
    else:
        title = f"CI {record['profile']} PR {record['pr']} {record['request']}"
        matches = [
            run
            for run in runs
            if run.get("display_title") == title
            and run.get("event") == "workflow_dispatch"
            and run.get("head_branch") == PROTECTED_REF
        ]
    if len(matches) > 1:
        raise ValueError("Request matched multiple runs; specify its recorded run_id")
    return matches[0] if matches else None


def write_request(path: Path, record: Mapping[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(record, indent=2) + "\n")


def require_stamp(pr: int, head: str, unstamped: str | None) -> None:
    """A request or ready mark needs a preflight pass on the head's tree (#920)."""
    if unstamped:
        log = preflight_stamp.log_override("unstamped", f"PR #{pr}", head, unstamped)
        print(f"PR #{pr} at {head} has no preflight stamp check; logged in {log}")
        return
    try:
        entry = preflight_stamp.stamp_for(head)
    except subprocess.CalledProcessError as error:
        raise ValueError(f"PR head {head} is not in this clone; fetch it") from error
    if entry is None:
        raise ValueError(
            f"PR #{pr} head {head} has no preflight stamp; run "
            "`python3 tools/preflight.py` on it"
        )


def require_attribution(pr: int, body: str | None) -> None:
    """The PR body carries the attribution line the growth gate reads its author
    from; the merge group's Full fails without it, after every other check."""
    if not cleanup_contract.ATTRIBUTION.search(body or ""):
        raise ValueError(
            f"PR #{pr}'s body has no attribution line; add one in the form "
            "`Agent: <path> | Model: <exact model> | Reasoning effort: <effort>`"
        )


def ready(args: argparse.Namespace) -> str:
    repository = repository_of(args.repo)
    pull = api(repository, f"pulls/{args.pr}")
    head = current_head(pull)
    require_stamp(args.pr, head, None)
    require_attribution(args.pr, pull.get("body"))
    quick = latest_quick(repository, head)
    if not pull.get("draft") and quick and quick["conclusion"] == "skipped":
        # Quick skipped a draft; a push landing with the ready mark left only
        # that skip at this head. Marking ready again starts it.
        gh("pr", "ready", str(args.pr), "--repo", repository, "--undo")
        gh("pr", "ready", str(args.pr), "--repo", repository)
        print(
            f"PR #{args.pr} at {head} had only a skipped {QUICK}; returned it to "
            "draft and marked it ready again to start one"
        )
        return head
    gh("pr", "ready", str(args.pr), "--repo", repository)
    print(f"PR #{args.pr} at {head} marked ready")
    return head


def ready_command(args: argparse.Namespace) -> None:
    head = ready(args)
    if args.wait:
        quick_wait(repository_of(args.repo), args.pr, args.timeout, args.interval, head)


def full_already_passed(repository: str, pr: int, head: str) -> bool:
    """Whether an ordinary Full request is unneeded; refuse one queue mode bars."""
    # A new request would reset a passing status, including one carried across a
    # docs-only base update, to pending and repeat the run.
    if run := passed_run(repository, head):
        print(f"PR #{pr} at {head} already passed Full validation: {run}")
        return True
    try:
        queue = variable(repository, "CI_MERGE_QUEUE") == "true"
    except subprocess.CalledProcessError:
        # Reading variables needs its own token permission, which an ordinary
        # request never needed; the plan applies the same rule server-side.
        queue = False
    if queue and changes_definitions(repository, pr):
        raise ValueError(
            "In merge-queue mode a PR changing .github/workflows/ needs root's "
            "reviewed bootstrap Full"
        )
    return False


def label_request(repository: str, pr: int, pull: dict, head: str) -> None:
    if variable(repository, "CI_ON_DEMAND") == "true":
        raise ValueError(
            "Label requests need CI_ON_DEMAND unset; root must review workflow changes first"
        )
    # Remove/add is intentional: only the explicit label event requests a run.
    labels = {label["name"] for label in pull["labels"]}
    if "ci:full" in labels:
        gh("pr", "edit", str(pr), "--repo", repository, "--remove-label", "ci:full")
    validate_pull(api(repository, f"pulls/{pr}"), head)
    gh("pr", "edit", str(pr), "--repo", repository, "--add-label", "ci:full")


# Git's SCP-like remote syntax, `user@host:path`, where the path is not absolute.
SCP_REMOTE = re.compile(r"^(?:[^@/:]+@)?([^/:]+):(?!/)(.+)$")


def github_repository(url: str) -> str | None:
    """The `owner/name` of a remote URL on github.com itself, or None."""
    if "://" in url:
        parts = urlsplit(url)
        if parts.query or parts.fragment:
            return None
        try:
            host, path = parts.hostname, parts.path
        except ValueError:
            return None
    elif scp := SCP_REMOTE.match(url):
        host, path = scp[1].lower(), scp[2]
    else:
        return None
    name = path.strip("/").removesuffix(".git")
    owner_name = name.split("/")
    if host != "github.com" or len(owner_name) != 2 or not all(owner_name):
        return None
    return name


class HeadRepository(TypedDict):
    full_name: str


class PullHead(TypedDict):
    """The fields of a PR's `head` that name its commit, branch and repository."""

    sha: str
    ref: NotRequired[str]
    repo: NotRequired[HeadRepository | None]


def checkout_git(*arguments: str) -> str:
    """A Git query in this tool's checkout; empty when Git answers nothing."""
    found = subprocess.run(
        ["git", "-C", str(vcs.ROOT), *arguments], capture_output=True, text=True
    )
    return found.stdout.strip() if found.returncode == 0 else ""


def checkout_head(pr: int, head: PullHead) -> str | None:
    """This checkout's HEAD when the checkout is the PR's own, else None.

    It is the PR's when its `origin` is the PR's head repository on github.com
    and its branch, or that branch's upstream on origin, is the PR's head
    branch. There a tracking ref that is not at HEAD, ahead or behind, is
    refused at once rather than waited for.
    """
    branch = head.get("ref")
    repository = (head.get("repo") or {"full_name": ""})["full_name"]
    local = checkout_git("symbolic-ref", "--quiet", "--short", "HEAD")
    if not branch or not repository or not local:
        return None
    tracking_ref = f"refs/remotes/origin/{branch}"
    # Fully qualified, so only an upstream on origin can name the PR's branch.
    upstream = checkout_git(
        "rev-parse", "--symbolic-full-name", f"{local}@{{upstream}}"
    )
    remote = github_repository(checkout_git("remote", "get-url", "origin")) or ""
    if (branch != local and upstream != tracking_ref) or (
        remote.lower() != repository.lower()
    ):
        return None
    commit = checkout_git("rev-parse", "HEAD")
    tracking = checkout_git("rev-parse", "--verify", "--quiet", tracking_ref)
    if tracking != commit:
        raise ValueError(
            f"This checkout is PR #{pr}'s branch {branch} at {commit}, but "
            f"origin/{branch} is at {tracking or 'no commit'}: push, update the "
            "checkout, or pass --head"
        )
    print(
        f"Requesting for this checkout's HEAD {commit}: it is PR #{pr}'s branch "
        f"{branch}, and origin/{branch} is at it",
        flush=True,
    )
    return commit


def requestable(repository: str, args: argparse.Namespace) -> tuple[dict, str] | None:
    """The PR and the head to request for, or None when Full already passed it.

    Without --head, the PR's own checkout names the head it pushed, so a read
    that still shows the previous head waits for the push to show.
    """
    pull = api(repository, f"pulls/{args.pr}")
    expected = args.head or checkout_head(args.pr, cast(PullHead, pull["head"]))
    if expected:
        pull = pushed_head(repository, args.pr, pull, expected)
    head = expected_head(pull, expected)
    require_stamp(args.pr, head, args.unstamped)
    if (
        args.profile == "full"
        and not args.bootstrap
        and full_already_passed(repository, args.pr, head)
    ):
        return None
    if args.bootstrap and args.profile != "full":
        raise ValueError("Bootstrap label requests only support --profile full")
    try:
        return await_mergeable(repository, args.pr, pull, head), head
    except HeadMoved as moved:
        if expected and not args.head:
            # The checkout's head pins the request as --head would: no retry.
            raise ValueError(
                f"{moved}; this checkout's HEAD {expected} was the head to request"
            ) from moved
        raise


def request(args: argparse.Namespace) -> Path | None:
    repository = repository_of(args.repo)
    try:
        found = requestable(repository, args)
    except HeadMoved:
        # Just after a push GitHub can still answer with the previous head, then
        # move to the new one while its mergeability is computed. A caller who
        # named no head asked for the current one, so read the PR once more.
        if args.head:
            raise
        print(
            f"PR #{args.pr}'s head moved while GitHub computed its mergeability "
            "(a push just before this request); requesting for the new head",
            flush=True,
        )
        found = requestable(repository, args)
    if found is None:
        return None
    pull, head = found
    # The identifier ends with the head, so enqueue finds a head's requests (#922).
    request_id = f"{uuid.uuid4().hex}-{head}"
    record = {
        "schema_version": 1,
        "repository": repository,
        "pr": args.pr,
        "head": head,
        "base": pull["base"]["sha"],
        "profile": args.profile,
        "request": request_id,
        "bootstrap": args.bootstrap,
        "created_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    }
    path = args.request_file or REQUESTS / f"{request_id}.json"
    write_request(path, record)
    # Nothing after this point retries: a second dispatch would repeat a paid run.
    if args.bootstrap:
        label_request(repository, args.pr, pull, head)
    else:
        gh(
            "workflow",
            "run",
            WORKFLOW,
            "--repo",
            repository,
            "--ref",
            PROTECTED_REF,
            "-f",
            f"pr={args.pr}",
            "-f",
            f"head={head}",
            "-f",
            f"profile={args.profile}",
            "-f",
            f"request={request_id}",
        )
    print(
        f"Requested {args.profile} CI for PR #{args.pr} at {head}\nRequest file: {path}"
    )
    if not args.wait:
        print(f"Wait for it as a background job: python3 tools/ci.py wait {path}")
    return path


def read_request(path: Path) -> dict:
    record = json.loads(path.read_text())
    if record.get("schema_version") != 1:
        raise ValueError("Unsupported CI request record schema")
    return record


def recorded_run(path: Path, record: dict) -> dict | None:
    """The request's run, found once by its identity and then kept in its file."""
    repository = record["repository"]
    if record.get("run_id"):
        return api(repository, f"actions/runs/{record['run_id']}")
    runs = api(repository, f"actions/workflows/{WORKFLOW}/runs?per_page=100")[
        "workflow_runs"
    ]
    run = find_run(record, runs)
    if run:
        record["run_id"] = run["id"]
        record["run_url"] = run["html_url"]
        write_request(path, record)
    return run


def status(path: Path) -> tuple[dict, dict | None]:
    record = read_request(path)
    repository = record["repository"]
    current_head(api(repository, f"pulls/{record['pr']}"), record["head"])
    run = recorded_run(path, record)
    if run and run["status"] != "completed":
        run["failed_jobs"] = failed_jobs(repository, run["id"])
    return record, run


def run_jobs(
    repository: str, run_id: int, attempt: int | None = None
) -> list[ci_log.Job]:
    """Every job of the run's attempt, the latest by default, through every page."""
    jobs = f"attempts/{attempt}/jobs?" if attempt else "jobs?filter=latest&"
    pages = json.loads(
        gh(
            "api",
            "--paginate",
            "--slurp",
            f"repos/{repository}/actions/runs/{run_id}/{jobs}per_page=100",
        )
    )
    return [job for page in pages for job in page["jobs"]]


def failed_jobs(repository: str, run_id: int) -> list[ci_log.Job]:
    return [
        job for job in run_jobs(repository, run_id) if job["conclusion"] in JOB_FAILED
    ]


class RunFailed(ValueError):
    """A requested run failed; `run_id` names it for the log summary."""

    def __init__(
        self,
        message: str,
        repository: str,
        run_id: int | None,
        attempt: int | None = None,
    ) -> None:
        super().__init__(message)
        self.repository = repository
        self.run_id = run_id
        self.attempt = attempt


def verify_children(record: dict, run: dict) -> None:
    if run.get("failed_jobs"):
        failed = ", ".join(
            f"{job['name']} ({job['html_url']})" for job in run["failed_jobs"]
        )
        raise RunFailed(
            f"Failed child jobs: {failed}. Other jobs continue; successful jobs remain available for retry.",
            record["repository"],
            run.get("id"),
        )


def verify_completion(record: dict, run: dict) -> None:
    if run["conclusion"] != "success":
        raise RunFailed(
            f"Requested CI concluded {run['conclusion']}: {run['html_url']}",
            record["repository"],
            run.get("id"),
        )
    if record["profile"] != "full":
        return
    repository = record["repository"]
    current_head(api(repository, f"pulls/{record['pr']}"), record["head"])
    combined = api(repository, f"commits/{record['head']}/status")
    matching = [
        item
        for item in combined["statuses"]
        if item["context"] == CONTEXT and item["target_url"] == run["html_url"]
    ]
    if len(matching) != 1 or matching[0]["state"] != "success":
        raise ValueError(
            "This run did not publish a successful Full validation commit status"
        )


class WaitTimeout(Exception):
    """The run is still going when the wait ends; resuming is not a failure."""


def report_failed(jobs: list[ci_log.Job], shown: set[int]) -> None:
    """Print each failed job once, as it appears."""
    for job in jobs:
        if job["id"] not in shown:
            shown.add(job["id"])
            print(f"failed job: {job['name']} ({job['html_url']})", flush=True)


def rerun_hint(
    repository: str, run_id: int, failed: list[ci_log.Job], attempt: int | None = None
) -> str:
    """The failed jobs and the rerun command; a rerun follows the run's latest
    attempt, so a pinned earlier attempt says so."""
    names = ", ".join(job["name"] for job in failed) or "none listed"
    hint = (
        f"failed jobs: {names}; rerun them with: "
        f"gh run rerun {run_id} --failed --repo {repository}"
    )
    if attempt is not None:
        try:
            latest = api(repository, f"actions/runs/{run_id}").get(
                "run_attempt", attempt
            )
        except subprocess.CalledProcessError:
            return hint  # Best effort: the failure itself is already known.
        if latest != attempt:
            hint += (
                f" (it reruns attempt {latest}'s failed jobs, not attempt {attempt}'s)"
            )
    return hint


def wait(path: Path, timeout: int, interval: int, complete: bool = False) -> None:
    """Wait on a request's run. With `complete`, a failed job is reported and
    the wait goes on until the run completes, so it can be rerun."""
    deadline = time.monotonic() + timeout
    previous = None
    shown: set[int] = set()
    while True:
        record, run = status(path)
        state = "awaiting workflow run" if run is None else run["status"]
        if state != previous:
            print(state + (f": {run['html_url']}" if run else ""), flush=True)
            previous = state
        if run and complete:
            report_failed(run.get("failed_jobs") or [], shown)
        elif run:
            verify_children(record, run)
        if run and run["status"] == "completed":
            if complete and run["conclusion"] != "success":
                failed = failed_jobs(record["repository"], run["id"])
                raise RunFailed(
                    f"Requested CI concluded {run['conclusion']}: {run['html_url']}; "
                    + rerun_hint(record["repository"], run["id"], failed),
                    record["repository"],
                    run["id"],
                )
            verify_completion(record, run)
            print(f"{record['profile']} CI passed for {record['head']}")
            return
        if time.monotonic() >= deadline:
            flag = " --complete" if complete else ""
            raise WaitTimeout(
                f"CI still running after {timeout} seconds; "
                f"resume with: python3 tools/ci.py wait {path}{flag}"
            )
        time.sleep(min(interval, max(0, deadline - time.monotonic())))


@dataclass(frozen=True)
class RunTarget:
    """A run and the attempt a wait follows."""

    repository: str
    run_id: int
    attempt: int

    @property
    def url(self) -> str:
        return (
            f"https://github.com/{self.repository}/actions/runs/{self.run_id}"
            f"/attempts/{self.attempt}"
        )


def run_target(text: str, repo: str | None) -> RunTarget:
    """A run ID or URL; a URL with /attempts/N pins that attempt, and otherwise
    the run's latest attempt, read now, is pinned."""
    if match := RUN_URL.fullmatch(text):
        repository, run_id, attempt = match[1], int(match[2]), match[3]
    elif text.isdigit():
        repository, run_id, attempt = repository_of(repo), int(text), None
    else:
        raise ValueError(f"--run takes a run ID or URL, not {text!r}")
    if attempt is None:
        attempt = api(repository, f"actions/runs/{run_id}").get("run_attempt", 1)
    return RunTarget(repository, run_id, int(attempt))


def wait_run(target: RunTarget, timeout: int, interval: int, complete: bool) -> None:
    """Wait on one attempt of any run, with no request record: a cadence run, a
    rerun or a merge-group run."""
    deadline = time.monotonic() + timeout
    previous = None
    shown: set[int] = set()
    path = f"actions/runs/{target.run_id}/attempts/{target.attempt}"
    while True:
        run = api(target.repository, path)
        jobs = run_jobs(target.repository, target.run_id, target.attempt)
        failed = [job for job in jobs if job["conclusion"] in JOB_FAILED]
        if run["status"] != previous:
            print(f"{run['status']}: {target.url}", flush=True)
            previous = run["status"]
        if failed and not complete:
            raise RunFailed(
                f"Failed jobs in {target.url}: "
                + ", ".join(f"{job['name']} ({job['html_url']})" for job in failed)
                + ". Other jobs continue; wait with --complete to see the run end.",
                target.repository,
                target.run_id,
                target.attempt,
            )
        report_failed(failed, shown)
        if run["status"] == "completed":
            if run["conclusion"] == "success":
                print(f"Run {target.run_id} attempt {target.attempt} passed")
                return
            raise RunFailed(
                f"Run {target.url} concluded {run['conclusion']}; "
                + rerun_hint(target.repository, target.run_id, failed, target.attempt),
                target.repository,
                target.run_id,
                target.attempt,
            )
        if time.monotonic() >= deadline:
            flag = " --complete" if complete else ""
            raise WaitTimeout(
                f"Run still going after {timeout} seconds; resume with: "
                f"python3 tools/ci.py wait --run {target.url}{flag}"
            )
        time.sleep(min(interval, max(0, deadline - time.monotonic())))


def quick_wait(
    repository: str, pr: int, timeout: int, interval: int, head: str | None = None
) -> None:
    """Wait for Quick validation at `head` (the PR's head when None); a push
    ends the wait. A skipped run is no verdict: on a draft the wait says to run
    `ready`; on a ready PR it waits up to MERGEABLE_WAIT seconds for the run the
    ready mark starts, then says the same."""
    head = current_head(api(repository, f"pulls/{pr}"), head)
    deadline = time.monotonic() + timeout
    grace = time.monotonic() + min(timeout, MERGEABLE_WAIT)
    previous = None
    while True:
        pull = api(repository, f"pulls/{pr}")
        current_head(pull, head)
        quick = latest_quick(repository, head)
        skipped = quick is not None and quick["conclusion"] == "skipped"
        now = time.monotonic()
        if skipped and now < deadline and (pull.get("draft") or now >= grace):
            raise ValueError(
                f"{QUICK} at {head} only skipped, as it does for a draft; start it "
                f"with: python3 tools/ci.py ready {pr} --wait"
            )
        state = "awaiting Quick" if quick is None or skipped else quick["status"]
        if state != previous:
            where = "" if quick is None or skipped else f": {quick['html_url']}"
            print(f"{QUICK} at {head}: {state}{where}", flush=True)
            previous = state
        if quick and not skipped and quick["status"] == "completed":
            if quick["conclusion"] == "success":
                print(f"{QUICK} passed at {head}")
                return
            found = RUN_URL.search(quick.get("details_url") or "")
            raise RunFailed(
                f"{QUICK} at {head} concluded {quick['conclusion']}: "
                f"{quick['html_url']}",
                repository,
                int(found[2]) if found else None,
            )
        if time.monotonic() >= deadline:
            raise WaitTimeout(
                f"{QUICK} still running after {timeout} seconds; resume with: "
                f"python3 tools/ci.py wait {pr} --quick"
            )
        time.sleep(min(interval, max(0, deadline - time.monotonic())))


class RequestRecord(TypedDict):
    """A request file (schema 1); `run_id` and `run_url` once its run is found."""

    schema_version: int
    repository: str
    pr: int
    head: str
    base: str | None
    profile: str
    request: str
    bootstrap: bool
    created_at: str
    run_id: NotRequired[int]
    run_url: NotRequired[str]


@dataclass
class Candidate:
    """A request `wait PR` may choose: its record, its file and where it was found."""

    record: RequestRecord
    path: Path
    source: str  # "local record" or "GitHub"


@dataclass
class DispatchedRun:
    """A requested run, with the profile and request identifier its name gives."""

    run: ci_log.Run
    profile: str
    request: str


def local_requests(repository: str, pr: int, head: str) -> list[Candidate]:
    """This checkout's request records for the PR head, bootstrap labels aside."""
    found = []
    for path in sorted(REQUESTS.glob("*.json")):
        try:
            record = json.loads(path.read_text())
        except (OSError, ValueError):
            continue
        if (
            record.get("schema_version") == 1
            and record.get("repository") == repository
            and record.get("pr") == pr
            and record.get("head") == head
            and not record.get("bootstrap")
        ):
            found.append(Candidate(record, path, "local record"))
    return found


# GitHub's run listing returns at most this many runs of one filtered search.
SEARCH_LIMIT = 1000


def utc_stamp(moment: datetime) -> str:
    return moment.astimezone(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


class RunsPage(TypedDict):
    """One page of the workflow runs listing."""

    total_count: NotRequired[int]
    workflow_runs: list[ci_log.Run]


def workflow_runs(
    repository: str, event: str, opened: str, until: datetime | None = None
) -> list[ci_log.Run]:
    """Every ci-request.yml run of `event` created from `opened` to `until` (now),
    paginated. A search GitHub cuts at its limit is split in two, so a PR open
    across more runs than that is read back to its opening."""
    start = datetime.fromisoformat(opened.replace("Z", "+00:00"))
    end = until or datetime.now(timezone.utc)
    span = quote(f"{utc_stamp(start)}..{utc_stamp(end)}")
    pages: list[RunsPage] = json.loads(
        gh(
            "api",
            "--paginate",
            "--slurp",
            f"repos/{repository}/actions/workflows/{WORKFLOW}/runs"
            f"?event={event}&created={span}&per_page=100",
        )
    )
    runs = [run for page in pages for run in page["workflow_runs"]]
    total = max((page.get("total_count", 0) for page in pages), default=0)
    if total <= len(runs) or (end - start).total_seconds() < 2:
        return runs
    middle = start + (end - start) / 2
    found = {
        run["id"]: run
        for run in workflow_runs(repository, event, utc_stamp(middle), end)
        + workflow_runs(repository, event, opened, middle)
    }
    return list(found.values())


def pr_dispatched_runs(repository: str, pr: int, opened: str) -> list[DispatchedRun]:
    """The PR's requested runs on GitHub, found by their identifier, which ends with
    the commit they test (#922): every dispatched run since the PR opened at
    `opened`."""
    found = []
    for run in workflow_runs(repository, "workflow_dispatch", opened):
        title = (run.get("display_title") or "").split()
        if (
            len(title) == 5
            and title[0] == "CI"
            and title[1] in PROFILES
            and title[2:4] == ["PR", str(pr)]
            and run.get("head_branch") == PROTECTED_REF
        ):
            found.append(DispatchedRun(run, title[1], title[4]))
    return found


def dispatched_runs(
    repository: str, pr: int, opened: str, head: str
) -> list[DispatchedRun]:
    """The PR's requested runs at `head`."""
    return [
        dispatched
        for dispatched in pr_dispatched_runs(repository, pr, opened)
        if dispatched.request.endswith(f"-{head}")
    ]


def remote_requests(
    repository: str, pr: int, opened: str, head: str
) -> list[Candidate]:
    """Request records rebuilt from the PR head's requested runs on GitHub."""
    found = []
    for dispatched in dispatched_runs(repository, pr, opened, head):
        run = dispatched.run
        record: RequestRecord = {
            "schema_version": 1,
            "repository": repository,
            "pr": pr,
            "head": head,
            "base": None,
            "profile": dispatched.profile,
            "request": dispatched.request,
            "bootstrap": False,
            "created_at": run["created_at"],
            "run_id": run["id"],
            "run_url": run["html_url"],
        }
        path = REQUESTS / f"{dispatched.request}.json"
        found.append(Candidate(record, path, "GitHub"))
    return found


def newest_request(
    candidates: list[Candidate], pr: int, head: str, profile: str | None
) -> Candidate:
    """The newest candidate; the newest of each other profile is named."""
    newest: dict[str, Candidate] = {}
    for candidate in sorted(
        candidates, key=lambda c: (c.record["created_at"], c.record["request"])
    ):
        newest[candidate.record["profile"]] = candidate
    if not newest:
        named = f"{profile} " if profile else ""
        raise ValueError(
            f"No {named}request for PR #{pr} at its head {head}; request one with: "
            f"python3 tools/ci.py request {pr} --profile {profile or 'native'} --wait"
        )
    chosen = max(
        newest.values(), key=lambda c: (c.record["created_at"], c.record["request"])
    )
    for other in sorted(set(newest) - {chosen.record["profile"]}):
        print(
            f"PR #{pr} also has a {other} request at {head}; wait on it with: "
            f"python3 tools/ci.py wait {pr} --profile {other}",
            flush=True,
        )
    return chosen


def resolve_request(repository: str, pr: int, profile: str | None) -> Path:
    """Fix, once, the newest request for the PR's current head among this
    checkout's records and the requested runs GitHub lists, one per request: a
    request made elsewhere counts, and so does one GitHub does not list yet. A
    later request is not followed."""
    pull = api(repository, f"pulls/{pr}")
    head = current_head(pull)
    local = local_requests(repository, pr, head)
    known = {candidate.record["request"] for candidate in local}
    remote = [
        candidate
        for candidate in remote_requests(repository, pr, pull["created_at"], head)
        if candidate.record["request"] not in known
    ]
    chosen = newest_request(
        [c for c in local + remote if profile in (None, c.record["profile"])],
        pr,
        head,
        profile,
    )
    record = chosen.record
    if chosen.source == "GitHub":
        write_request(chosen.path, record)
    print(
        f"Waiting on the newest {record['profile']} request for PR #{pr} at {head} "
        f"(from {chosen.source}): {record.get('run_url') or 'run not listed yet'}",
        flush=True,
    )
    return chosen.path


QUEUE_STATE = """
query($owner: String!, $name: String!, $number: Int!) {
  repository(owner: $owner, name: $name) {
    pullRequest(number: $number) {
      state
      mergeCommit { oid }
      mergeQueueEntry { state position }
      autoMergeRequest { enabledAt }
    }
  }
}
"""


class Commit(TypedDict):
    oid: str


class QueueEntry(TypedDict):
    state: str
    position: int


class QueueState(TypedDict):
    """The fields of QUEUE_STATE's pull request that queue_wait reads."""

    state: str
    mergeCommit: Commit | None
    mergeQueueEntry: QueueEntry | None
    autoMergeRequest: dict[str, str] | None


class QueueRun(TypedDict):
    """The fields of a merge-group workflow run that removal reports."""

    id: int
    head_branch: str
    head_sha: str
    created_at: str
    status: str
    conclusion: str | None
    html_url: str
    run_attempt: NotRequired[int]


class QueueRemoved(Exception):
    """The PR left the merge queue without merging; `run_id` names its last
    merge-group run, if any, for the log summary."""

    def __init__(self, message: str, repository: str, run_id: int | None) -> None:
        super().__init__(message)
        self.repository = repository
        self.run_id = run_id


def entry_text(entry: QueueEntry) -> str:
    """A queue entry as `queued at position 2` or `queued awaiting_checks at ...`."""
    state = entry["state"].lower()
    words = "queued" if state == "queued" else f"queued {state}"
    return f"{words} at position {entry['position']}"


def queue_position(pull: QueueState) -> str | None:
    """The PR's place in the queue, or None once it reads as out of the queue."""
    if pull["state"] != "OPEN":
        return None
    if entry := pull["mergeQueueEntry"]:
        return entry_text(entry)
    return "awaiting entry to the merge queue" if pull["autoMergeRequest"] else None


def queue_state(repository: str, pr: int) -> QueueState:
    owner, name = repository.split("/", 1)
    answer = json.loads(
        gh(
            "api",
            "graphql",
            "-f",
            f"query={QUEUE_STATE}",
            "-F",
            f"owner={owner}",
            "-F",
            f"name={name}",
            "-F",
            f"number={pr}",
        )
    )
    return answer["data"]["repository"]["pullRequest"]


def queue_runs(repository: str, pr: int) -> list[QueueRun]:
    """The PR's merge-group runs of the CI request workflow, newest first."""
    runs: list[QueueRun] = api(
        repository, f"actions/workflows/{WORKFLOW}/runs?event=merge_group&per_page=100"
    )["workflow_runs"]
    prefix = f"gh-readonly-queue/{PROTECTED_REF}/pr-{pr}-"
    ours = [run for run in runs if (run["head_branch"] or "").startswith(prefix)]
    return sorted(ours, key=lambda run: run["created_at"], reverse=True)


def queue_run(repository: str, pr: int) -> QueueRun | None:
    """The PR's latest merge-group run of the CI request workflow."""
    return next(iter(queue_runs(repository, pr)), None)


def removal(repository: str, pr: int, state: str) -> QueueRemoved:
    run = queue_run(repository, pr)
    if run is None:
        return QueueRemoved(
            f"PR #{pr} left the merge queue ({state}); no merge-group run found",
            repository,
            None,
        )
    lines = [
        f"PR #{pr} left the merge queue ({state}); merge-group run "
        f"{run['status']}/{run['conclusion']}: {run['html_url']}"
    ]
    lines += [
        f"failed job: {job['name']} ({job['html_url']})"
        for job in failed_jobs(repository, run["id"])
    ]
    return QueueRemoved("\n".join(lines), repository, run["id"])


def queue_wait(repository: str, pr: int, timeout: int, interval: int) -> str:
    """Block until the PR merges (return its merge commit) or leaves the queue."""
    deadline = time.monotonic() + timeout
    previous = None
    while True:
        pull = queue_state(repository, pr)
        commit = pull["mergeCommit"]
        if pull["state"] == "MERGED" and commit:
            return commit["oid"]
        # A merging PR reads for a moment as open and out of the queue before it
        # reads as merged, so a removal counts once a second read confirms it.
        state = queue_position(pull)
        if state is None and previous == "leaving the merge queue":
            raise removal(repository, pr, pull["state"].lower())
        state = state or "leaving the merge queue"
        if state != previous:
            print(state, flush=True)
            previous = state
        if time.monotonic() >= deadline:
            raise WaitTimeout(
                f"PR #{pr} still queued after {timeout} seconds; "
                f"resume with: python3 tools/ci.py queue-wait {pr} --repo {repository}"
            )
        time.sleep(min(interval, max(0, deadline - time.monotonic())))


class CheckRun(TypedDict):
    """A check run as the check-runs API lists it, with the fields read here."""

    status: str
    conclusion: str | None
    started_at: str | None
    html_url: str
    details_url: NotRequired[str]


def latest_quick(repository: str, head: str) -> CheckRun | None:
    """The head's latest Quick validation check run, from its pull_request run.

    A skipped run carries no verdict: one left from the PR's draft can be stamped
    after the run that validated the same head, so it counts only when no other
    run exists. With no check or only a skip, an unfinished Quick run at the head
    answers as a check in progress.
    """
    checks = api(
        repository, f"commits/{head}/check-runs?check_name={quote(QUICK)}&filter=latest"
    )
    runs: list[CheckRun] = sorted(
        checks["check_runs"],
        key=lambda run: (run["conclusion"] != "skipped", run["started_at"] or ""),
    )
    if runs and runs[-1]["conclusion"] != "skipped":
        return runs[-1]
    return running_quick(repository, head) or (runs[-1] if runs else None)


def running_quick(repository: str, head: str) -> CheckRun | None:
    """The head's unfinished pull_request run of Quick's workflow, as a check in
    progress. The Quick validation check reports Quick's other jobs and appears
    only once they end, so for most of a run the head has no such check, or only
    the skip its draft push left (#1063)."""
    runs = api(
        repository,
        f"actions/workflows/{QUICK_WORKFLOW}/runs?event=pull_request"
        f"&head_sha={head}&per_page=100",
    )["workflow_runs"]
    running = [run for run in runs if run["status"] != "completed"]
    if not running:
        return None
    run = max(running, key=lambda run: run.get("run_started_at") or "")
    return CheckRun(
        status=run["status"],
        conclusion=None,
        started_at=run.get("run_started_at"),
        html_url=run["html_url"],
        details_url=run["html_url"],
    )


def bootstrap_runs(
    repository: str, pr: int, opened: str, commits: Collection[str]
) -> list[ci_log.Run]:
    """The PR's ci:full bootstrap label runs of ci-request.yml at `commits`, since
    the PR opened at `opened`, paginated.

    Every label starts a run; only a ci:full bootstrap runs its plan. A run whose
    plan has not been decided yet may be a bootstrap, so an unfinished run counts
    until its plan job is skipped.
    """
    return [
        run
        for run in workflow_runs(repository, "pull_request", opened)
        if runs_for(run, pr) and run.get("head_sha") in commits
        if is_bootstrap(repository, run)
    ]


def runs_for(run: ci_log.Run, pr: int) -> bool:
    """Whether a ci-request.yml run's name, `CI PROFILE PR N ...`, names PR `pr`."""
    return (run.get("display_title") or "").split()[2:4] == ["PR", str(pr)]


def is_bootstrap(repository: str, run: ci_log.Run) -> bool:
    """Whether a label run of ci-request.yml is, or may yet be, a bootstrap: its
    plan ran, or it has not finished and its plan is not yet skipped."""
    plans = [
        job
        for job in api(repository, f"actions/runs/{run['id']}/jobs")["jobs"]
        if job["name"] == "plan"
    ]
    return any(job["conclusion"] != "skipped" for job in plans) or (
        not plans and run["status"] != "completed"
    )


def pr_commits(repository: str, head: str) -> list[str]:
    """The PR's own commits, oldest first and ending at `head`: those of
    `compare/BASE...HEAD`. A comparison that omits commits counts the head only."""
    compare = api(repository, f"compare/{PROTECTED_REF}...{head}")
    commits = [row["sha"] for row in compare.get("commits", [])]
    if not commits or compare.get("total_commits", 0) > len(commits):
        return [head]
    return commits if commits[-1] == head else [*commits, head]


@dataclass
class TestedRun:
    """A requested or bootstrap run of the PR, with the commit it tested."""

    profile: str  # a requested profile, or "full bootstrap"
    commit: str
    run: ci_log.Run


@dataclass
class PrRuns:
    """One read of the PR's commits and of its requested runs on them, which
    `state` shows and `enqueue`'s checks judge."""

    commits: list[str]
    runs: list[TestedRun]
    unread: list[str] = field(default_factory=list)


def recorded_runs(
    repository: str, pr: int, commits: list[str], found: dict[int, TestedRun]
) -> list[str]:
    """Add to `found` the runs this checkout's request records name on `commits`
    that the listings did not; return those GitHub could not return."""
    # Requests made before #922 carry no commit in their identifier; their records
    # do. A recorded label run is judged as a listed one, by what GitHub says it
    # tested.
    unread = []
    for record_path in REQUESTS.glob("*.json"):
        record = cast(RequestRecord, json.loads(record_path.read_text()))
        run_id = record.get("run_id")
        if record.get("pr") != pr or record.get("head") not in commits or not run_id:
            continue
        if run_id in found:
            continue
        try:
            run = cast(ci_log.Run, api(repository, f"actions/runs/{run_id}"))
        except subprocess.CalledProcessError:
            unread.append(f"run {run_id} of {record_path.name}")
            continue
        if not runs_for(run, pr):
            continue
        if run.get("event") == "pull_request":
            commit = run.get("head_sha", "")
            if commit in commits and is_bootstrap(repository, run):
                found[run_id] = TestedRun("full bootstrap", commit, run)
        else:
            profile = record.get("profile") or "requested"
            found[run_id] = TestedRun(profile, record["head"], run)
    return unread


def pr_requested_runs(
    repository: str, pr: int, opened: str, commits: list[str]
) -> PrRuns:
    """The PR's requested and bootstrap runs on its `commits`, oldest first by
    `(created_at, id)`: the one order `enqueue` and `state` read. A recorded
    run GitHub cannot return is named in `unread`."""
    found: dict[int, TestedRun] = {}
    for dispatched in pr_dispatched_runs(repository, pr, opened):
        commit = dispatched.request.rpartition("-")[2]
        if commit in commits:
            found[dispatched.run["id"]] = TestedRun(
                dispatched.profile, commit, dispatched.run
            )
    for run in bootstrap_runs(repository, pr, opened, commits):
        found[run["id"]] = TestedRun("full bootstrap", run["head_sha"], run)
    unread = recorded_runs(repository, pr, commits, found)
    runs = sorted(
        found.values(), key=lambda tested: (tested.run["created_at"], tested.run["id"])
    )
    return PrRuns(commits, runs, unread)


def ejected_head(repository: str, run: QueueRun) -> str:
    """The PR head a merge-group run tested: its group commit's second parent."""
    return api(repository, f"commits/{run['head_sha']}")["parents"][1]["sha"]


# GitHub lists at most this many files of a comparison.
COMPARE_FILES = 300


def uninert_base_changes(repository: str, head: str) -> list[str]:
    """Paths the base changed since head forked that are not inert Markdown.

    Root's docs-only commits move the base several times an hour; a head behind
    only by them needs no base merge, preflight and native run to requeue. Both
    paths of a rename count, and a listing at GitHub's limit counts as not inert.
    """
    files = api(repository, f"compare/{head}...{PROTECTED_REF}").get("files", [])
    if len(files) >= COMPARE_FILES:
        return [f"{len(files)} or more files"]
    paths = [
        path
        for row in files
        for path in (row["filename"], row.get("previous_filename"))
        if path
    ]
    return [path for path in dict.fromkeys(paths) if not preflight_stamp.inert(path)]


def requeue_gate(repository: str, pr: int, head: str, retry: str | None) -> None:
    """After the queue removes a PR for a failure, it queues again only with a new
    head that contains base and has a preflight stamp, or once unchanged after a
    runner or network failure the agent names with --retry."""
    runs = queue_runs(repository, pr)
    if not runs or runs[0]["conclusion"] not in REMOVED:
        return
    url = runs[0]["html_url"]
    if ejected_head(repository, runs[0]) != head:
        behind = api(repository, f"compare/{PROTECTED_REF}...{head}")["behind_by"]
        if behind and (others := uninert_base_changes(repository, head)):
            raise ValueError(
                f"The queue removed PR #{pr} ({url}); merge {PROTECTED_REF} into it "
                f"first: its head is {behind} commits behind, changing "
                + ", ".join(others[:3])
            )
        require_stamp(pr, head, None)
        return
    if not retry:
        raise ValueError(
            f"The queue removed PR #{pr} at this head ({url}). Fix the cause, merge "
            f"{PROTECTED_REF} and pass preflight; for a runner or network failure, "
            "queue it once more with --retry REASON"
        )
    failed = [
        run
        for run in runs
        if run["conclusion"] in REMOVED and ejected_head(repository, run) == head
    ]
    if len(failed) > 1:
        raise ValueError(
            f"PR #{pr} failed the queue {len(failed)} times at {head}; "
            "a second failure goes to root"
        )
    log = preflight_stamp.log_override("requeue", f"PR #{pr}", head, retry)
    print(f"Queueing PR #{pr} again unchanged after {url}; logged in {log}")


def read_pr_runs(repository: str, pr: int, opened: str, head: str) -> PrRuns:
    return pr_requested_runs(repository, pr, opened, pr_commits(repository, head))


def gate_run(repository: str, pr: int, head: str, read: PrRuns) -> TestedRun:
    """The run `enqueue` reads: the PR's newest requested or bootstrap run on any of
    its own commits, which must have succeeded. A workflow change also needs a
    green bootstrap Full at the head itself (root-workflow.md, CI administration),
    and its newest run there."""
    if read.unread:
        raise ValueError(
            f"PR #{pr}'s request records name runs GitHub did not return: "
            + ", ".join(read.unread)
        )
    runs = read.runs
    if not runs:
        raise ValueError(
            f"PR #{pr} has no requested run on its commits up to {head}; "
            f"run ci.py request {pr} --profile native"
        )
    newest = runs[-1]
    run = newest.run
    if run["conclusion"] != "success":
        raise ValueError(
            f"The newest requested run of PR #{pr}, {newest.profile} at "
            f"{newest.commit}, is not green: "
            f"{run['status']}/{run['conclusion']}: {run['html_url']}"
        )
    if changes_definitions(repository, pr):
        if newest.commit != head:
            raise ValueError(
                f"PR #{pr} changes .github/workflows/, so its passing run must be at "
                f"its head {head}; the newest, {newest.profile} {run['html_url']}, "
                f"ran at {newest.commit}"
            )
        bootstraps = [
            tested.run
            for tested in runs
            if tested.profile == "full bootstrap" and tested.commit == head
        ]
        if not bootstraps or bootstraps[-1]["conclusion"] != "success":
            state = bootstraps and bootstraps[-1]["html_url"]
            raise ValueError(
                f"PR #{pr} changes .github/workflows/ and needs root's green "
                f"bootstrap Full at its head {head}: {state or 'none'}"
            )
    return newest


def enqueue_checks(
    repository: str, pr: int, pull: dict, read: PrRuns | None = None
) -> str:
    """enqueue's checks before its requeue gate, read-only: return the head, or
    raise naming the first that fails."""
    head = current_head(pull)
    if pull.get("draft"):
        raise ValueError(f"PR #{pr} is a draft; mark it ready with ci.py ready")
    require_attribution(pr, pull.get("body"))
    quick = latest_quick(repository, head)
    if quick and quick["conclusion"] == "skipped":
        raise ValueError(
            f"{QUICK} at {head} only skipped, as it does for a draft; run "
            f"python3 tools/ci.py ready {pr} to start it"
        )
    if not quick or quick["conclusion"] != "success":
        state = (
            quick and f"{quick['status']}/{quick['conclusion']}: {quick['html_url']}"
        )
        raise ValueError(
            f"{QUICK} at {head} is not green: {state}"
            if state
            else f"{QUICK} at {head} is not green: no run. If the push was just "
            f"now, its run has not appeared yet: wait with python3 tools/ci.py wait "
            f"{pr} --quick. If no Quick started, as after a [skip ci] push, push a "
            "commit without [skip ci]"
        )
    if read is None:
        read = read_pr_runs(repository, pr, pull["created_at"], head)
    gate_run(repository, pr, head, read)
    missing_acceptances(repository, pr, pull, head)
    return head


def added_records(repository: str, head: str) -> list[dict[str, Any]]:
    """The cleanup records the PR adds at its head.

    They are read from the commit, not the working tree, against the merge base
    GitHub reports, so neither this checkout nor a stale local base matters.
    """
    found = api(repository, f"compare/{PROTECTED_REF}...{head}")
    base = found["merge_base_commit"]["sha"]
    for commit in (head, base):
        if not cleanup_contract.has_commit(commit):
            raise ValueError(f"Commit {commit} is not in this clone; fetch it")
    return cleanup_contract.records_between(base, head)


# lint-policy.py --same-item's status for a changed item.
ITEM_DIFFERS = 3


def unsafe_item_unchanged(
    site: str, head: str, unreadable: set[str]
) -> Callable[[str], bool]:
    """Whether an unsafe_code item's text at an accepted commit equals head's.
    An accepted commit this clone lacks counts as changed and is noted in
    `unreadable`, so the search goes on through later acceptances.

    lint-policy.py runs as its own process: its parser dependencies re-execute
    the importing process under uv.
    """

    def unchanged(accepted: str) -> bool:
        if not cleanup_contract.has_commit(accepted):
            unreadable.add(accepted)
            return False
        script = Path(__file__).with_name("lint-policy.py")
        command = [sys.executable, str(script), "--same-item", site, accepted, head]
        done = subprocess.run(command, capture_output=True, text=True)
        if done.returncode not in (0, ITEM_DIFFERS):
            raise ValueError(
                f"Comparing {site} at {accepted} and {head} failed (exit "
                f"{done.returncode}): {(done.stderr or done.stdout).strip()[-2000:]}"
            )
        return done.returncode == 0

    return unchanged


def missing_acceptances(repository: str, pr: int, pull: dict, head: str) -> None:
    """Refuse, naming each record the PR adds that no reviewer has accepted.

    With Quick green at the head every growth, suppression and exception has a
    record, so every record accepted is what the merge group's gates will read.
    """
    records = [
        record
        for record in added_records(repository, head)
        if record["kind"] in cleanup_contract.NEEDS_REVIEW
    ]
    if not records:
        return
    attribution = cleanup_contract.ATTRIBUTION.search(pull.get("body") or "")
    author = attribution[1] if attribution else ""
    comments = cleanup_contract.discussion(f"repos/{repository}", pr, run=gh)
    missing = []
    for record in records:
        name = cleanup_contract.record_name(record)
        if not cleanup_contract.reviewed(record, comments, author, head):
            missing.append(name)
            continue
        if not cleanup_contract.unsafe_site(record):
            continue
        unreadable: set[str] = set()
        unchanged = unsafe_item_unchanged(record["site"], head, unreadable)
        if not cleanup_contract.reviewed(record, comments, author, head, unchanged):
            why = "item changed since the accepted head"
            if unreadable:
                why += f", or fetch the accepted {', '.join(sorted(unreadable))}"
            missing.append(f"{name}: {why}")
    if missing:
        raise ValueError(
            f"PR #{pr} at {head} lacks {len(missing)} cleanup acceptance(s); have "
            "each accepted, or delete a record nothing uses:\n"
            + "\n".join(f"  {line}" for line in missing)
        )


def enqueue(args: argparse.Namespace) -> str:
    """Queue a PR whose head has green Quick and requested runs; report the entry."""
    repository = repository_of(args.repo)
    pull = api(repository, f"pulls/{args.pr}")
    head = enqueue_checks(repository, args.pr, pull)
    requeue_gate(repository, args.pr, head, args.retry)
    merge = ("pr", "merge", str(args.pr), "--repo", repository, "--merge")
    try:
        gh(*merge, "--match-head-commit", head)
    except subprocess.CalledProcessError as error:
        # The first call may have taken effect before its answer was lost.
        if not transient(error):
            raise
        state = queue_state(repository, args.pr)
        if not (state["mergeQueueEntry"] or state["autoMergeRequest"]):
            print("gh pr merge: transient GitHub error; trying once more", flush=True)
            gh(*merge, "--match-head-commit", head)
    state = queue_state(repository, args.pr)
    if entry := state["mergeQueueEntry"]:
        return entry_text(entry)
    if state["autoMergeRequest"]:
        return (
            "auto-merge armed: GitHub queues the PR once its pending required "
            "checks pass"
        )
    raise ValueError(f"PR #{args.pr} is neither queued nor armed for auto-merge")


def enqueue_assessment(
    repository: str, pr: int, pull: dict, read: PrRuns | Exception
) -> str:
    """What enqueue's current checks say of the PR, without queueing or logging:
    `pass` or `blocked: <reason>`. Passing them is not permission to merge."""
    try:
        if isinstance(read, Exception):
            raise read
        head = enqueue_checks(repository, pr, pull, read)
        requeue_gate(repository, pr, head, None)
    except (ValueError, subprocess.CalledProcessError) as error:
        return f"blocked: {error}"
    return "pass"


def listing(repository: str, path: str) -> list:
    """A paginated REST array, its items as the API gives them."""
    pages = json.loads(gh("api", "--paginate", "--slurp", f"repos/{repository}/{path}"))
    return [item for page in pages for item in page]


@dataclass
class RunFact:
    """What `state` shows of a run or check run."""

    status: str
    conclusion: str | None
    url: str
    created_at: str | None


@dataclass
class QueueFact:
    queued: bool
    text: str


@dataclass
class NewestRequest:
    """The PR's newest requested or bootstrap run on any of its commits, which
    `enqueue` reads, and how many of the PR's commits follow the one it tested."""

    profile: str
    commit: str
    behind: int
    run: RunFact


@dataclass
class PullState:
    """One PR's state as `state` shows it, all read at one head."""

    repository: str
    number: int
    title: str
    state: str
    draft: bool
    head: str
    mergeable: str
    quick: RunFact | None
    requests: dict[str, RunFact]
    newest_request: NewestRequest | None
    queue: QueueFact
    reports: list[review_report.ReviewReport]
    enqueue_checks: str


def run_fact(run: Mapping[str, Any]) -> RunFact:
    return RunFact(
        run["status"],
        run["conclusion"],
        run["html_url"],
        run.get("created_at") or run.get("started_at"),
    )


def requests_at_head(runs: list[TestedRun], head: str) -> dict[str, RunFact]:
    """The newest requested run at the head for each profile, bootstrap apart."""
    newest = {tested.profile: tested.run for tested in runs if tested.commit == head}
    return {profile: run_fact(run) for profile, run in sorted(newest.items())}


def newest_on_commits(
    runs: list[TestedRun], commits: list[str]
) -> NewestRequest | None:
    if not runs:
        return None
    newest = runs[-1]
    behind = len(commits) - 1 - commits.index(newest.commit)
    return NewestRequest(newest.profile, newest.commit, behind, run_fact(newest.run))


def queue_fact(repository: str, pr: int) -> QueueFact:
    pull = queue_state(repository, pr)
    if pull["state"] == "MERGED":
        return QueueFact(False, "merged")
    if position := queue_position(pull):
        return QueueFact(True, position)
    last = queue_run(repository, pr)
    text = "not queued"
    if last and last["conclusion"] in REMOVED:
        text += f"; last merge-group run {last['conclusion']}: {last['html_url']}"
    return QueueFact(False, text)


def mergeable_word(pull: dict) -> str:
    words = {True: "yes", False: "conflicts", None: "pending"}
    return f"{words[pull.get('mergeable')]} ({pull.get('mergeable_state', 'unknown')})"


def pull_state(repository: str, pr: int) -> PullState:
    """One PR's state as the tools read it, from one head; refuse a view read
    across a push."""
    pull = api(repository, f"pulls/{pr}")
    head = pull["head"]["sha"]
    comments = listing(repository, f"issues/{pr}/comments?per_page=100")
    reviews = listing(repository, f"pulls/{pr}/reviews?per_page=100")
    reports = review_report.review_reports(comments, reviews, head)
    reports.sort(key=lambda report: report.reported_at, reverse=True)
    quick = latest_quick(repository, head)
    # One read for the view and the checks; a failed read blocks only the checks.
    read: PrRuns | Exception
    try:
        read = read_pr_runs(repository, pr, pull["created_at"], head)
    except (ValueError, subprocess.CalledProcessError) as error:
        read = error
    shown = read if isinstance(read, PrRuns) else PrRuns([head], [])
    facts = PullState(
        repository=repository,
        number=pr,
        title=pull["title"],
        state="merged" if pull.get("merged_at") else pull["state"],
        draft=bool(pull.get("draft")),
        head=head,
        mergeable=mergeable_word(pull),
        quick=run_fact(quick) if quick else None,
        requests=requests_at_head(shown.runs, head),
        newest_request=newest_on_commits(shown.runs, shown.commits),
        queue=queue_fact(repository, pr),
        reports=reports,
        enqueue_checks=enqueue_assessment(repository, pr, pull, read),
    )
    end = api(repository, f"pulls/{pr}")["head"]["sha"]
    if end != head:
        raise ValueError(
            f"PR #{pr} head moved from {head} to {end} while its state was read; "
            f"run python3 tools/ci.py state {pr} again"
        )
    return facts


def run_line(label: str, fact: RunFact | None) -> str:
    if fact is None:
        return f"{label}: none"
    return f"{label}: {fact.conclusion or fact.status} {fact.url}"


def report_line(report: review_report.ReviewReport) -> str:
    verdict = report.disposition
    if report.scope:
        verdict += f" ({report.scope})"
    where = "at head" if report.matches_head else f"at {report.sha[:10]}, not head"
    return f"  {report.agent}: {verdict} {where} {report.url}"


SHOWN_REPORTS = 5


def state_lines(facts: PullState) -> list[str]:
    ready = "draft" if facts.draft else "ready"
    lines = [
        f"PR #{facts.number} {facts.state}, {ready}: {facts.title}",
        f"head {facts.head}, mergeable: {facts.mergeable}",
        run_line("Quick at head", facts.quick),
    ]
    requests = facts.requests
    lines += [run_line(f"{p} at head", fact) for p, fact in requests.items()]
    if not requests:
        lines.append("Requested runs at head: none")
    if (newest := facts.newest_request) and newest.behind:
        lines.append(
            run_line(
                f"Newest requested run, {newest.profile} at {newest.commit[:10]}, "
                f"{newest.behind} commit(s) below head",
                newest.run,
            )
        )
    lines.append(f"Queue: {facts.queue.text}")
    reports = facts.reports
    lines.append(
        f"Review reports: {len(reports)}" + (", newest first" if reports else "")
    )
    lines += [report_line(report) for report in reports[:SHOWN_REPORTS]]
    if older := reports[SHOWN_REPORTS:]:
        links = ", ".join(report.url for report in older)
        lines.append(f"  {len(older)} older: {links}")
    lines.append(f"enqueue checks: {facts.enqueue_checks}")
    return lines


def show_state(repository: str, pr: int, as_json: bool) -> None:
    facts = pull_state(repository, pr)
    text = (
        json.dumps(asdict(facts), indent=2)
        if as_json
        else "\n".join(state_lines(facts))
    )
    print(text)


@dataclass
class PrRun:
    """A run `log --pr` may read: a requested run, or a merge-group run whose
    group commit names the head it tested."""

    kind: str
    run_id: int
    attempt: int
    created_at: str
    url: str
    group: QueueRun | None = None


def pr_runs(repository: str, pr: int, profile: str | None) -> tuple[str, list[PrRun]]:
    """The PR's head and its candidate runs: requested runs at that head and
    merge-group runs."""
    pull = api(repository, f"pulls/{pr}")
    head = pull["head"]["sha"]
    runs = []
    if profile != "queue":
        runs += [
            PrRun(
                f"requested {d.profile}",
                d.run["id"],
                d.run.get("run_attempt", 1),
                d.run["created_at"],
                d.run["html_url"],
            )
            for d in dispatched_runs(repository, pr, pull["created_at"], head)
            if profile in (None, d.profile)
        ]
    if profile in (None, "queue"):
        runs += [
            PrRun(
                "merge-group",
                run["id"],
                run.get("run_attempt", 1),
                run["created_at"],
                run["html_url"],
                run,
            )
            for run in queue_runs(repository, pr)
        ]
    return head, runs


def pr_run(repository: str, pr: int, profile: str | None) -> int:
    """The newest of the PR's candidate runs, saying which it chose and why."""
    head, runs = pr_runs(repository, pr, profile)
    if not runs:
        raise ValueError(
            f"PR #{pr} has no {profile or 'requested'} run at its head {head} and no "
            "merge-group run; pass the run ID or URL: python3 tools/ci.py log RUN"
        )
    runs.sort(key=lambda run: (run.created_at, run.run_id))
    chosen = runs[-1]
    group = chosen.group
    tested = head if group is None else ejected_head(repository, group)
    print(
        f"Selected the newest {chosen.kind} run for PR #{pr}: run {chosen.run_id} "
        f"attempt {chosen.attempt}, tested head {tested} "
        f"(PR head {head}): {chosen.url}"
    )
    others = {run.kind: run for run in runs if run.kind != chosen.kind}
    if others:
        listed = ", ".join(f"{kind} run {run.run_id}" for kind, run in others.items())
        print(f"Also found: {listed}; pass --profile or the run ID to read another")
    return chosen.run_id


def request_run(path: Path) -> tuple[str, int]:
    record = read_request(path)
    run = recorded_run(path, record)
    if run is None:
        raise ValueError(
            f"The request in {path} has no run yet; wait for it: "
            f"python3 tools/ci.py wait {path}"
        )
    return record["repository"], run["id"]


@dataclass(frozen=True)
class LogTarget:
    """The run `log` reads: its attempt (None for the latest) and one job."""

    repository: str
    run_id: int
    attempt: int | None = None
    job_id: int | None = None


def log_target(args: argparse.Namespace) -> LogTarget:
    """The run, attempt and job that `log`'s arguments name."""
    if (args.pr is None) == (args.target is None):
        raise ValueError(
            "name one run: python3 tools/ci.py log RUN_ID|RUN_URL|JOB_URL|"
            "REQUEST_FILE, or python3 tools/ci.py log --pr N"
        )
    if args.pr is not None:
        repository = repository_of(args.repo)
        return LogTarget(repository, pr_run(repository, args.pr, args.profile))
    if match := RUN_URL.fullmatch(args.target):
        attempt, job = (int(n) if n else None for n in (match[3], match[4]))
        return LogTarget(match[1], int(match[2]), attempt, job)
    if args.target.isdigit():
        return LogTarget(repository_of(args.repo), int(args.target))
    return LogTarget(*request_run(Path(args.target)))


def selected_jobs(
    repository: str, jobs: list[ci_log.Job], job_id: int | None, name: str | None
) -> list[ci_log.Job]:
    if job_id is not None:
        return [cast(ci_log.Job, api(repository, f"actions/jobs/{job_id}"))]
    if name:
        return [job for job in jobs if name.lower() in job["name"].lower()]
    return [job for job in jobs if job["conclusion"] in JOB_FAILED]


def no_jobs(run: ci_log.Run, jobs: list[ci_log.Job], name: str | None) -> str:
    if name:
        names = ", ".join(job["name"] for job in jobs)
        return f"No job named like {name!r}; the run's jobs: {names}"
    if ci_log.unfinished(jobs) or run["status"] != "completed":
        return "No failed job yet"
    return f"No failed job: the run concluded {run['conclusion']}"


def log_lines(target: LogTarget, args: argparse.Namespace) -> list[str]:
    """The run's heading, then each selected job's log as `args` asks."""
    grep: str | None = args.grep
    pattern = re.compile(grep) if grep else None
    repository = target.repository
    attempt = f"/attempts/{target.attempt}" if target.attempt else ""
    run = cast(ci_log.Run, api(repository, f"actions/runs/{target.run_id}{attempt}"))
    jobs = run_jobs(repository, target.run_id, target.attempt)
    lines = ci_log.run_heading(run, jobs)
    chosen = selected_jobs(repository, jobs, target.job_id, args.job)
    if not chosen:
        lines.append(no_jobs(run, jobs, args.job))
    for job in chosen:
        lines += ci_log.job_report(repository, job, pattern, args.full)
    return lines


def show_log(args: argparse.Namespace) -> None:
    print("\n".join(log_lines(log_target(args), args)))


def failure_logs(
    repository: str, run_id: int | None, attempt: int | None = None
) -> None:
    """Best effort: the failed jobs' summary after a failed wait, of the attempt
    the wait pinned. A failure to read them prints one line and changes no exit
    status."""
    if run_id is None:
        return
    summary = argparse.Namespace(job=None, grep=None, full=False)
    try:
        lines = log_lines(LogTarget(repository, run_id, attempt), summary)
        print("\n".join(lines), flush=True)
    except Exception as error:  # Any failure here must keep the wait's status.
        target = (
            f"https://github.com/{repository}/actions/runs/{run_id}/attempts/{attempt}"
            if attempt
            else f"{run_id} --repo {repository}"
        )
        print(
            f"Could not read the failed jobs' logs ({error}); read them with: "
            f"python3 tools/ci.py log {target}",
            flush=True,
        )


# CI administration's macOS fallback: the paid runner, set by hand before.
MACOS_VARIABLE = "CI_MACOS_HEAVY"
MACOS_RUNNER = "blacksmith-6vcpu-macos-26"
MACOS_JOB = "Mac OS"
# GitHub's failure annotation on a job no hosted runner ever took; a cancel by
# hand also leaves no runner and no step, but says who cancelled.
UNACQUIRED = "was not acquired by Runner of type hosted"


class Terminated(BaseException):
    """SIGTERM, as signal-witness.py forwards it to a background job."""


def capacity_cancelled(repository: str, job: ci_log.Job) -> bool:
    """A macOS job GitHub cancelled because no hosted runner took it. A job
    cancelled after a runner started it, or cancelled by hand, does not count."""
    if not (
        MACOS_JOB in job["name"]
        and job["conclusion"] == "cancelled"
        and not job.get("runner_id")
        and not job.get("steps")
    ):
        return False
    notes = json.loads(
        gh("api", f"repos/{repository}/check-runs/{job['id']}/annotations")
    )
    return any(UNACQUIRED in (note.get("message") or "") for note in notes)


def run_id_of(text: str) -> int:
    if match := RUN_URL.fullmatch(text):
        return int(match[2])
    if text.isdigit():
        return int(text)
    raise ValueError(f"a run is an ID or URL, not {text!r}")


def capacity_cancellations(repository: str, runs: list[int]) -> list[str]:
    """Each distinct run attempt whose macOS jobs no hosted runner took; the
    jobs of one attempt are one cancellation, and a run named twice is one run."""
    found = []
    for run_id in dict.fromkeys(runs):
        latest = api(repository, f"actions/runs/{run_id}").get("run_attempt", 1)
        for attempt in range(1, latest + 1):
            jobs = run_jobs(repository, run_id, attempt)
            if any(capacity_cancelled(repository, job) for job in jobs):
                found.append(f"run {run_id} attempt {attempt}")
    return found


def macos_attempt(
    rerun: RunTarget, expected: set[str], deadline: float, interval: int
) -> list[ci_log.Job]:
    """Wait for an attempt after `rerun`'s to appear, then for every expected
    macOS job, by name, to complete in it; return them. Other jobs are not
    waited on."""
    repository, run_id, after = rerun.repository, rerun.run_id, rerun.attempt
    attempt = None
    while True:
        if attempt is None:
            latest = api(repository, f"actions/runs/{run_id}").get("run_attempt", 1)
            if latest > after:
                attempt = latest
                print(f"Rerun is attempt {attempt} of run {run_id}", flush=True)
        if attempt is not None:
            jobs = {
                job["name"]: job
                for job in run_jobs(repository, run_id, attempt)
                if job["name"] in expected
            }
            if len(jobs) == len(expected) and all(
                job["status"] == "completed" for job in jobs.values()
            ):
                return list(jobs.values())
        if time.monotonic() >= deadline:
            raise WaitTimeout(
                f"The macOS jobs of run {run_id} were still running at the timeout"
            )
        time.sleep(min(interval, max(0, deadline - time.monotonic())))


def stamp_now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def record_fallback(
    args: argparse.Namespace, repository: str, lines: list[str]
) -> None:
    """Record the variable's changes on the PR, or print them for the cadence."""
    body = (
        "**macOS fallback** "
        "([CI administration](docs/root-workflow.md#ci-administration))\n\n"
        + "\n".join(f"- {line}" for line in lines)
    )
    if args.pr is None:
        print(body)
        return
    body += (
        f"\n\nAgent: {args.agent} | Model: {args.model} | "
        f"Reasoning effort: {args.effort}"
    )
    try:
        gh("pr", "comment", str(args.pr), "--repo", repository, "--body", body)
    except subprocess.CalledProcessError as error:
        print(f"Could not comment on PR #{args.pr} ({error}); the record:\n{body}")


def acquire_fallback(repository: str, lines: list[str]) -> bool:
    """Create the variable, failing when it exists, so two invocations cannot
    both own it. Whether this invocation owns it; an unanswered create that
    left the variable set is reported and not owned."""
    try:
        detached_gh(
            "api",
            "-X",
            "POST",
            f"repos/{repository}/actions/variables",
            "-f",
            f"name={MACOS_VARIABLE}",
            "-f",
            f"value={MACOS_RUNNER}",
        )
    except subprocess.CalledProcessError as error:
        if "HTTP 409" in f"{error.stderr or ''}":
            raise ValueError(
                f"{MACOS_VARIABLE} already exists; another invocation or a person "
                "owns it. Nothing changed."
            ) from error
        try:
            observed = repr(variable(repository, MACOS_VARIABLE))
        except subprocess.CalledProcessError as unread:
            observed = f"unknown (reading it back failed too: {unread})"
        lines.append(
            f"{stamp_now()} creating {MACOS_VARIABLE} failed ({error}); it may be "
            f"set (it reads {observed}), owner unknown, so it was left for root "
            "to check"
        )
        raise ValueError(lines[-1]) from error
    lines.append(f"{stamp_now()} set {MACOS_VARIABLE}={MACOS_RUNNER}")
    return True


def terminate(_number: int, _frame: object) -> None:
    raise Terminated("SIGTERM")


def macos_fallback(args: argparse.Namespace) -> None:
    """Set the paid macOS runner, rerun the failed jobs, and delete the variable
    once the rerun's macOS jobs finish, after two hosted-capacity cancellations."""
    if args.pr is not None and not (args.agent and args.model and args.effort):
        raise ValueError("--pr records on the PR: pass --agent, --model and --effort")
    repository = repository_of(args.repo)
    runs = [run_id_of(text) for text in args.runs]
    counted = capacity_cancellations(repository, runs)
    if len(counted) < 2:
        raise ValueError(
            f"Counted {len(counted)} hosted-capacity cancellation(s) of macOS jobs "
            f"({', '.join(counted) or 'none'}); the fallback needs two. A job "
            "cancelled by hand or after a runner took it does not count. "
            "Nothing changed."
        )
    run_id = runs[-1]
    before = api(repository, f"actions/runs/{run_id}").get("run_attempt", 1)
    expected = {
        job["name"]
        for job in run_jobs(repository, run_id, before)
        if MACOS_JOB in job["name"] and job["conclusion"] in JOB_FAILED
    }
    if not expected:
        raise ValueError(
            f"Run {run_id} attempt {before} has no failed macOS job to rerun. "
            "Nothing changed."
        )
    url = f"https://github.com/{repository}/actions/runs/{run_id}"
    lines = [f"capacity cancellations: {', '.join(counted)}"]
    owned = left = False
    previous = signal.signal(signal.SIGTERM, terminate)
    try:
        # A SIGTERM or Ctrl-C during the create waits until the create's outcome
        # is known, so a variable this invocation set is always owned, and so
        # deleted.
        held = {signal.SIGTERM, signal.SIGINT}
        signal.pthread_sigmask(signal.SIG_BLOCK, held)
        try:
            owned = acquire_fallback(repository, lines)
        finally:
            signal.pthread_sigmask(signal.SIG_UNBLOCK, held)
        gh("run", "rerun", str(run_id), "--failed", "--repo", repository)
        lines.append(
            f"{stamp_now()} reran the failed jobs of {url} after attempt {before}"
        )
        deadline = time.monotonic() + args.timeout
        jobs = macos_attempt(
            RunTarget(repository, run_id, before), expected, deadline, args.interval
        )
        lines += [
            f"{job['name']}: {job['conclusion']} {job['html_url']}" for job in jobs
        ]
        if failed := [job["name"] for job in jobs if job["conclusion"] != "success"]:
            raise RunFailed(
                f"macOS jobs did not pass: {', '.join(failed)}", repository, run_id
            )
    except BaseException as error:
        if owned:
            lines.append(f"{stamp_now()} stopped: {str(error) or type(error).__name__}")
        raise
    finally:
        signal.signal(signal.SIGTERM, previous)
        if owned:
            left = delete_fallback(repository, lines)
        if owned or len(lines) > 1:
            record_fallback(args, repository, lines)
    if left:
        raise ValueError(f"{MACOS_VARIABLE} is still set; delete it by hand")
    print(f"macOS jobs of {url} passed on {MACOS_RUNNER}; {MACOS_VARIABLE} deleted")


def delete_fallback(repository: str, lines: list[str]) -> bool:
    """Delete the variable this invocation set and read it back; whether it is
    still set."""
    try:
        gh("variable", "delete", MACOS_VARIABLE, "--repo", repository)
    except subprocess.CalledProcessError as error:
        lines.append(f"{stamp_now()} deleting {MACOS_VARIABLE} failed: {error}")
    try:
        left = variable(repository, MACOS_VARIABLE) is not None
    except subprocess.CalledProcessError:
        left = True
    if left:
        lines.append(f"{stamp_now()} {MACOS_VARIABLE} is still set: delete it by hand")
        print(f"{MACOS_VARIABLE} is still set; delete it by hand", file=sys.stderr)
    else:
        lines.append(
            f"{stamp_now()} deleted {MACOS_VARIABLE}; the variable list no longer has it"
        )
    return left


def control(repository: str, run: dict, branches: dict[int, str]) -> bool:
    """Whether a run tested a negative control: a PR branch under CONTROL."""
    branch = run.get("head_branch") or ""
    queued = branch.removeprefix(f"gh-readonly-queue/{PROTECTED_REF}/pr-")
    title = (run.get("display_title") or "").split()
    if queued != branch:
        number = int(queued.split("-")[0])
    elif run.get("event") == "workflow_dispatch" and title[2:3] == ["PR"]:
        number = int(title[3])
    else:
        return branch.startswith(CONTROL)
    if number not in branches:
        branches[number] = api(repository, f"pulls/{number}")["head"]["ref"]
    return branches[number].startswith(CONTROL)


def failures(args: argparse.Namespace) -> None:
    """Count failed workflow runs since a date, leaving out negative controls."""
    repository = repository_of(args.repo)
    runs = [
        run
        for status in FAILED
        for page in json.loads(
            gh(
                "api",
                "--paginate",
                "--slurp",
                f"repos/{repository}/actions/runs?status={status}"
                f"&created=%3E%3D{args.since}&per_page=100",
            )
        )
        for run in page["workflow_runs"]
    ]
    branches: dict[int, str] = {}
    controls = [run for run in runs if control(repository, run, branches)]
    counted = runs if args.controls else [run for run in runs if run not in controls]
    for run in counted:
        print(f"{run['html_url']} {run['name']}: {run.get('display_title')}")
    left = "" if args.controls else f", leaving out {len(controls)} on {CONTROL}"
    print(f"{len(counted)} failed runs since {args.since}{left}")


def report_status(path: Path) -> None:
    record, run = status(path)
    print(json.dumps({"request": record, "run": run}, indent=2))
    if run:
        verify_children(record, run)
    if run and run["status"] == "completed":
        verify_completion(record, run)


def positive(text: str) -> int:
    value = int(text)
    if value <= 0:
        raise argparse.ArgumentTypeError("must be positive")
    return value


def sha(text: str) -> str:
    if not 7 <= len(text) <= 40 or any(c not in "0123456789abcdef" for c in text):
        raise argparse.ArgumentTypeError("must be 7 to 40 lowercase hex digits")
    return text


def complete_option(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--complete",
        action="store_true",
        help="report failed jobs as they appear but exit only when the run "
        "completes, so gh run rerun --failed can follow",
    )


def timing(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--timeout",
        type=positive,
        default=WAIT_TIMEOUT,
        help=f"seconds to wait before exit 2 (default {WAIT_TIMEOUT})",
    )
    parser.add_argument("--interval", type=positive, default=15)


def wait_command(args: argparse.Namespace) -> None:
    """`wait PR|REQUEST_FILE`, `wait PR --quick` or `wait --run RUN`."""
    if args.run is not None:
        if args.target or args.profile or args.quick:
            raise ValueError("--run names the run; give no PR, request or --profile")
        target = run_target(args.run, args.repo)
        wait_run(target, args.timeout, args.interval, args.complete)
    elif args.target is None:
        raise ValueError("name a PR, a request file or --run RUN")
    elif args.quick:
        if not args.target.isdigit() or args.profile or args.complete:
            raise ValueError("--quick takes a PR number and no other option")
        quick_wait(
            repository_of(args.repo), int(args.target), args.timeout, args.interval
        )
    else:
        wait(wait_target(args), args.timeout, args.interval, args.complete)


def wait_target(args: argparse.Namespace) -> Path:
    """A wait's request file: the one named, or the one a PR number resolves to."""
    if args.target.isdigit():
        return resolve_request(repository_of(args.repo), int(args.target), args.profile)
    if args.profile or args.repo:
        raise ValueError(
            "--profile and --repo apply to a PR number; a request file names both"
        )
    return Path(args.target)


def queue_wait_command(args: argparse.Namespace) -> None:
    merge = queue_wait(repository_of(args.repo), args.pr, args.timeout, args.interval)
    print(f"PR #{args.pr} merged: {merge}")


def request_command(args: argparse.Namespace) -> None:
    path = request(args)
    if path and args.wait:
        wait(path, args.timeout, args.interval, args.complete)


HANDLERS = {
    "log": show_log,
    "logs": show_log,
    "comment": lambda args: status_comment.run(args, repository_of(args.repo)),
    "queue-wait": queue_wait_command,
    "ready": ready_command,
    "enqueue": lambda args: print(f"PR #{args.pr}: {enqueue(args)}"),
    "failures": failures,
    "request": request_command,
    "wait": lambda args: wait_command(args),
    "macos-fallback": macos_fallback,
}
# The commands whose failure prints the failed jobs' log summary.
WAITS = ("wait", "request", "queue-wait", "ready")


def dispatch(args: argparse.Namespace) -> None:
    if handler := HANDLERS.get(args.command):
        handler(args)
    elif str(args.target).isdigit():
        show_state(repository_of(args.repo), int(args.target), args.json)
    elif args.repo or args.json:
        raise ValueError(
            "--repo and --json apply to a PR number; a request file names its repository"
        )
    else:
        report_status(Path(args.target))


def log_parser(commands: argparse._SubParsersAction) -> None:
    logs = commands.add_parser(
        "log",
        aliases=["logs"],
        help="print each failed job's failing step, error lines and tail, from "
        "logs cached under .local/ci-logs/; works while other jobs still run",
    )
    logs.add_argument(
        "target",
        nargs="?",
        metavar="RUN_ID|RUN_URL|JOB_URL|REQUEST_FILE",
        help="the run (or one job) to read",
    )
    logs.add_argument(
        "--pr", type=int, help="the PR's newest requested or merge-group run"
    )
    logs.add_argument(
        "--profile",
        choices=(*PROFILES, "queue"),
        help="with --pr: that profile's requests, or queue for merge-group runs",
    )
    logs.add_argument("--repo", help="with a run ID or --pr")
    logs.add_argument(
        "--job", metavar="NAME", help="read the jobs whose name contains NAME"
    )
    logs.add_argument("--grep", metavar="RE", help="print the log lines matching RE")
    logs.add_argument("--full", action="store_true", help="print the raw cached log")


def request_parser(commands: argparse._SubParsersAction) -> None:
    submit = commands.add_parser("request", help="request CI (default: Rust checks)")
    submit.add_argument("pr", type=int)
    submit.add_argument(
        "--profile",
        choices=PROFILES,
        default="rust",
        help="full gates Windows x64, GCC SDL2 and Emscripten; platform diagnoses the whole matrix",
    )
    submit.add_argument("--repo")
    submit.add_argument("--request-file", type=Path)
    submit.add_argument(
        "--bootstrap",
        action="store_true",
        help="ci:full label request for reviewed workflow changes with CI_ON_DEMAND unset",
    )
    submit.add_argument(
        "--wait", action="store_true", help="then wait for the run, like wait"
    )
    complete_option(submit)
    submit.add_argument(
        "--head",
        metavar="SHA",
        type=sha,
        help="refuse unless the PR's head on GitHub is SHA (or starts with it)",
    )
    timing(submit)
    submit.add_argument(
        "--unstamped",
        metavar="REASON",
        help="request a head without a preflight stamp, such as a seeded control; logged",
    )


def pr_parsers(commands: argparse._SubParsersAction) -> None:
    marked = commands.add_parser(
        "ready", help="mark a draft PR ready once its head has a preflight stamp"
    )
    marked.add_argument("pr", type=int)
    marked.add_argument("--repo")
    marked.add_argument(
        "--wait",
        action="store_true",
        help="then wait for Quick at the head: exit 0 passed, 1 failed, 2 still "
        "running at the timeout",
    )
    timing(marked)
    for command in ("state", "status"):
        shown = commands.add_parser(
            command,
            help="print a PR's head, checks, requested runs, queue, review reports "
            "and enqueue checks"
            + (", or a request file's state once" if command == "status" else ""),
        )
        shown.add_argument(
            "target",
            metavar="PR" if command == "state" else "PR|REQUEST_FILE",
            type=int if command == "state" else str,
        )
        shown.add_argument("--repo")
        shown.add_argument("--json", action="store_true", help="the same facts as JSON")
    waited = commands.add_parser(
        "wait",
        help="wait for a request: exit 0 passed, 1 failed, 2 still running at the "
        "timeout (resume with the command it prints)",
    )
    waited.add_argument(
        "target",
        nargs="?",
        metavar="PR|REQUEST_FILE",
        help="a request file, or a PR number for the newest request at its head",
    )
    waited.add_argument("--profile", choices=PROFILES, help="with a PR number")
    waited.add_argument("--repo", help="with a PR number or run ID")
    waited.add_argument(
        "--quick", action="store_true", help="with a PR number: wait for its Quick"
    )
    waited.add_argument(
        "--run",
        metavar="RUN_ID|RUN_URL",
        help="wait on any run with no request record; an /attempts/N URL pins "
        "that attempt, otherwise the latest attempt is pinned at the start",
    )
    complete_option(waited)
    timing(waited)
    entered = commands.add_parser(
        "enqueue", help="queue a PR whose head has green Quick and requested runs"
    )
    entered.add_argument("pr", type=int)
    entered.add_argument("--repo")
    entered.add_argument(
        "--retry",
        metavar="REASON",
        help="queue an unchanged head once more after a runner or network failure; logged",
    )
    counted = commands.add_parser(
        "failures", help=f"count failed runs, leaving out {CONTROL} negative controls"
    )
    counted.add_argument("--since", required=True, help="date, such as 2026-11-10")
    counted.add_argument("--controls", action="store_true", help="count them too")
    counted.add_argument("--repo")
    fallback_text = (
        f"after two hosted-capacity cancellations of macOS jobs: set "
        f"{MACOS_VARIABLE}, rerun the last run's failed jobs, delete the variable "
        "when its macOS jobs finish, and record both changes. A SIGKILL cannot be "
        f"caught and can leave {MACOS_VARIABLE} set; the command refuses to start "
        f"while it is set, and `gh variable list` shows it"
    )
    fallback = commands.add_parser(
        "macos-fallback", help=fallback_text, description=fallback_text
    )
    fallback.add_argument("runs", nargs="+", metavar="RUN", help="run IDs or URLs")
    fallback.add_argument("--pr", type=int, help="record the changes on this PR")
    fallback.add_argument("--agent", help="with --pr: your agent path")
    fallback.add_argument("--model", help="with --pr")
    fallback.add_argument("--effort", help="with --pr")
    fallback.add_argument("--repo")
    timing(fallback)
    queued = commands.add_parser(
        "queue-wait",
        help="wait until a queued PR merges (exit 0) or leaves the queue (exit 3)",
    )
    queued.add_argument("pr", type=int)
    queued.add_argument("--repo")
    queued.add_argument("--timeout", type=positive, default=QUEUE_TIMEOUT)
    queued.add_argument("--interval", type=positive, default=15)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    log_parser(commands)
    status_comment.register(commands)
    request_parser(commands)
    pr_parsers(commands)
    args = parser.parse_args()
    try:
        dispatch(args)
        return 0
    except Terminated:
        print(f"ci.py {args.command} stopped by SIGTERM", file=sys.stderr)
        return 128 + signal.SIGTERM
    except WaitTimeout as timeout:
        print(timeout)
        return 2
    except QueueRemoved as removed:
        print(removed)
        failure_logs(removed.repository, removed.run_id)
        return 3
    except RunFailed as failed:
        print(f"ci.py {args.command} failed: {failed}", file=sys.stderr)
        if args.command in WAITS:
            failure_logs(failed.repository, failed.run_id, failed.attempt)
        return 1
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f"ci.py {args.command} failed: {error}", file=sys.stderr)
        if isinstance(error, subprocess.CalledProcessError) and error.stderr:
            print(error.stderr.strip(), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
