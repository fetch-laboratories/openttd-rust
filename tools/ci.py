#!/usr/bin/env python3
"""Request scoped GitHub CI for an immutable PR head, using ordinary gh CLI."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import time
import uuid
from dataclasses import asdict
from datetime import datetime, timezone
from pathlib import Path
from typing import TypedDict, cast
from urllib.parse import quote

import ci_log
import preflight_stamp
import review_report
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
# GitHub computes mergeability for a few seconds after a push; a request waits.
MERGEABLE_WAIT = 60
MERGEABLE_INTERVAL = 3
# Quick's required check on a PR head, from its pull_request run.
QUICK = "Quick validation"
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
    r"(?:/attempts/\d+)?(?:/job/(\d+))?/?(?:[?#]\S*)?"
)


def gh(*args: str) -> str:
    return vcs.gh(*args, capture_stderr=True).strip()


def api(repository: str, path: str) -> dict:
    return json.loads(gh("api", f"repos/{repository}/{path}"))


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
        raise ValueError(
            f"PR head changed: requested {head}, current {current}; that request "
            "no longer covers the PR, so request CI for the new head"
        )
    return current


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
            "commit (just pushed: rerun in a few seconds), or pass the head you "
            "mean to test"
        )
    return head


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


def write_request(path: Path, record: dict) -> None:
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
            "`python3 tools/preflight.py --base origin/rust-migration` on it"
        )


def ready(args: argparse.Namespace) -> None:
    repository = repository_of(args.repo)
    head = current_head(api(repository, f"pulls/{args.pr}"))
    require_stamp(args.pr, head, None)
    gh("pr", "ready", str(args.pr), "--repo", repository)
    print(f"PR #{args.pr} at {head} marked ready")


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


def request(args: argparse.Namespace) -> Path | None:
    repository = repository_of(args.repo)
    pull = api(repository, f"pulls/{args.pr}")
    head = expected_head(pull, args.head)
    require_stamp(args.pr, head, args.unstamped)
    if (
        args.profile == "full"
        and not args.bootstrap
        and full_already_passed(repository, args.pr, head)
    ):
        return None
    if args.bootstrap and args.profile != "full":
        raise ValueError("Bootstrap label requests only support --profile full")
    pull = await_mergeable(repository, args.pr, pull, head)
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


def run_jobs(repository: str, run_id: int) -> list[dict]:
    """Every job of the run's latest attempt, through every page."""
    pages = json.loads(
        gh(
            "api",
            "--paginate",
            "--slurp",
            f"repos/{repository}/actions/runs/{run_id}/jobs?filter=latest&per_page=100",
        )
    )
    return [job for page in pages for job in page["jobs"]]


def failed_jobs(repository: str, run_id: int) -> list[dict]:
    return [
        job for job in run_jobs(repository, run_id) if job["conclusion"] in JOB_FAILED
    ]


class RunFailed(ValueError):
    """A requested run failed; `run_id` names it for the log summary."""

    def __init__(self, message: str, repository: str, run_id: int | None) -> None:
        super().__init__(message)
        self.repository = repository
        self.run_id = run_id


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


def wait(path: Path, timeout: int, interval: int) -> None:
    deadline = time.monotonic() + timeout
    previous = None
    while True:
        record, run = status(path)
        state = "awaiting workflow run" if run is None else run["status"]
        if state != previous:
            print(state + (f": {run['html_url']}" if run else ""), flush=True)
            previous = state
        if run:
            verify_children(record, run)
        if run and run["status"] == "completed":
            verify_completion(record, run)
            print(f"{record['profile']} CI passed for {record['head']}")
            return
        if time.monotonic() >= deadline:
            raise WaitTimeout(
                f"CI still running after {timeout} seconds; "
                f"resume with: python3 tools/ci.py wait {path}"
            )
        time.sleep(min(interval, max(0, deadline - time.monotonic())))


Candidate = tuple[dict, Path]


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
            found.append((record, path))
    return found


def dispatched_runs(repository: str, pull: dict, head: str) -> list[dict]:
    """Requested runs for the PR head, found on GitHub by their identifier, which
    ends with the head (#922): every dispatched run since the PR opened, paginated.
    Each run gains `profile` and `request` from its name."""
    pr = pull["number"]
    since = quote(">=" + pull["created_at"])
    pages = json.loads(
        gh(
            "api",
            "--paginate",
            "--slurp",
            f"repos/{repository}/actions/workflows/{WORKFLOW}/runs"
            f"?event=workflow_dispatch&created={since}&per_page=100",
        )
    )
    found = []
    for run in (run for page in pages for run in page["workflow_runs"]):
        title = (run.get("display_title") or "").split()
        if (
            len(title) == 5
            and title[0] == "CI"
            and title[1] in PROFILES
            and title[2:4] == ["PR", str(pr)]
            and title[4].endswith(f"-{head}")
            and run.get("head_branch") == PROTECTED_REF
        ):
            found.append(dict(run, profile=title[1], request=title[4]))
    return found


def remote_requests(repository: str, pull: dict, head: str) -> list[Candidate]:
    """Request records rebuilt from the PR head's requested runs on GitHub."""
    found = []
    for run in dispatched_runs(repository, pull, head):
        record = {
            "schema_version": 1,
            "repository": repository,
            "pr": pull["number"],
            "head": head,
            "base": None,
            "profile": run["profile"],
            "request": run["request"],
            "bootstrap": False,
            "created_at": run["created_at"],
            "run_id": run["id"],
            "run_url": run["html_url"],
        }
        found.append((record, REQUESTS / f"{run['request']}.json"))
    return found


def newest_request(
    candidates: list[Candidate], pr: int, head: str, profile: str | None
) -> Candidate:
    """The newest candidate, refusing a choice between profiles nobody named."""
    newest: dict[str, Candidate] = {}
    for candidate in sorted(
        candidates, key=lambda c: (c[0]["created_at"], c[0]["request"])
    ):
        newest[candidate[0]["profile"]] = candidate
    if not newest:
        named = f"{profile} " if profile else ""
        raise ValueError(
            f"No {named}request for PR #{pr} at its head {head}; request one with: "
            f"python3 tools/ci.py request {pr} --profile {profile or 'native'} --wait"
        )
    if len(newest) > 1:
        raise ValueError(
            f"PR #{pr} has requests at {head} for {', '.join(sorted(newest))}; "
            f"name one: python3 tools/ci.py wait {pr} --profile PROFILE"
        )
    return next(iter(newest.values()))


def resolve_request(repository: str, pr: int, profile: str | None) -> Path:
    """Fix, once, the newest request for the PR's current head: from this
    checkout's records, else from GitHub. A later request is not followed."""
    pull = api(repository, f"pulls/{pr}")
    head = current_head(pull)

    def wanted(candidates: list[Candidate]) -> list[Candidate]:
        return [c for c in candidates if profile in (None, c[0]["profile"])]

    source = "local record"
    candidates = wanted(local_requests(repository, pr, head))
    if not candidates:
        source = "GitHub"
        candidates = wanted(remote_requests(repository, dict(pull, number=pr), head))
    record, path = newest_request(candidates, pr, head, profile)
    if source == "GitHub":
        write_request(path, record)
    print(
        f"Waiting on the newest {record['profile']} request for PR #{pr} at {head} "
        f"(from {source}): {record.get('run_url') or 'run not listed yet'}",
        flush=True,
    )
    return path


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


def latest_quick(repository: str, head: str) -> dict | None:
    """The head's latest Quick validation check run, from its pull_request run."""
    checks = api(
        repository, f"commits/{head}/check-runs?check_name={quote(QUICK)}&filter=latest"
    )
    runs = sorted(checks["check_runs"], key=lambda run: run["started_at"] or "")
    return runs[-1] if runs else None


def bootstrap_runs(repository: str, head: str) -> list[dict]:
    """The head's ci:full bootstrap label runs of ci-request.yml."""
    path = f"actions/workflows/{WORKFLOW}/runs?per_page=100&event="
    # Every label starts a run; only a ci:full bootstrap runs its plan.
    return [
        run
        for run in api(repository, path + f"pull_request&head_sha={head}")[
            "workflow_runs"
        ]
        if any(
            job["name"] == "plan" and job["conclusion"] != "skipped"
            for job in api(repository, f"actions/runs/{run['id']}/jobs")["jobs"]
        )
    ]


def latest_request(repository: str, pr: int, head: str) -> dict | None:
    """The newest requested or bootstrap run of ci-request.yml for this PR head."""
    path = f"actions/workflows/{WORKFLOW}/runs?per_page=100&event="
    dispatched = api(repository, path + "workflow_dispatch")["workflow_runs"]
    named = [
        run
        for run in dispatched
        if (title := (run.get("display_title") or "").split())[2:4] == ["PR", str(pr)]
        and title[-1].endswith(f"-{head}")
    ]
    labeled = bootstrap_runs(repository, head)
    # Requests made before #922 carry no head in their identifier; their records do.
    for record_path in REQUESTS.glob("*.json"):
        record = json.loads(record_path.read_text())
        if (
            record.get("pr") == pr
            and record.get("head") == head
            and record.get("run_id")
        ):
            named.append(api(repository, f"actions/runs/{record['run_id']}"))
    runs = named + labeled
    return max(runs, key=lambda run: run["created_at"]) if runs else None


def ejected_head(repository: str, run: QueueRun) -> str:
    """The PR head a merge-group run tested: its group commit's second parent."""
    return api(repository, f"commits/{run['head_sha']}")["parents"][1]["sha"]


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
        if behind:
            raise ValueError(
                f"The queue removed PR #{pr} ({url}); merge {PROTECTED_REF} into it "
                f"first: its head is {behind} commits behind"
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


def enqueue_checks(repository: str, pr: int, pull: dict) -> str:
    """enqueue's checks before its requeue gate, read-only: return the head, or
    raise naming the first that fails."""
    head = current_head(pull)
    if pull.get("draft"):
        raise ValueError(f"PR #{pr} is a draft; mark it ready with ci.py ready")
    quick = latest_quick(repository, head)
    if not quick or quick["conclusion"] != "success":
        state = (
            quick and f"{quick['status']}/{quick['conclusion']}: {quick['html_url']}"
        )
        raise ValueError(f"{QUICK} at {head} is not green: {state or 'no run'}")
    run = latest_request(repository, pr, head)
    if not run or run["conclusion"] != "success":
        state = run and f"{run['status']}/{run['conclusion']}: {run['html_url']}"
        raise ValueError(
            f"The latest requested run at {head} is not green: {state or 'none; '}"
            + ("" if run else f"run ci.py request {pr} --profile native")
        )
    return head


def enqueue(args: argparse.Namespace) -> str:
    """Queue a PR whose head has green Quick and requested runs; report the entry."""
    repository = repository_of(args.repo)
    pull = api(repository, f"pulls/{args.pr}")
    head = enqueue_checks(repository, args.pr, pull)
    requeue_gate(repository, args.pr, head, args.retry)
    gh(
        "pr",
        "merge",
        str(args.pr),
        "--repo",
        repository,
        "--merge",
        "--match-head-commit",
        head,
    )
    state = queue_state(repository, args.pr)
    if entry := state["mergeQueueEntry"]:
        return entry_text(entry)
    if state["autoMergeRequest"]:
        return (
            "auto-merge armed: GitHub queues the PR once its pending required "
            "checks pass"
        )
    raise ValueError(f"PR #{args.pr} is neither queued nor armed for auto-merge")


def enqueue_assessment(repository: str, pr: int, pull: dict) -> str:
    """What enqueue's current checks say of the PR, without queueing or logging:
    `pass` or `blocked: <reason>`. Passing them is not permission to merge."""
    try:
        head = enqueue_checks(repository, pr, pull)
        requeue_gate(repository, pr, head, None)
    except (ValueError, subprocess.CalledProcessError) as error:
        return f"blocked: {error}"
    return "pass"


def listing(repository: str, path: str) -> list[dict]:
    """A paginated REST array."""
    pages = json.loads(gh("api", "--paginate", "--slurp", f"repos/{repository}/{path}"))
    return [item for page in pages for item in page]


def run_fact(run: dict | None) -> dict | None:
    if run is None:
        return None
    return {
        "status": run["status"],
        "conclusion": run["conclusion"],
        "url": run["html_url"],
        "created_at": run.get("created_at") or run.get("started_at"),
    }


def requests_at_head(repository: str, pull: dict, head: str) -> dict[str, dict]:
    """The newest requested run at the head for each profile, bootstrap apart."""
    runs = dispatched_runs(repository, pull, head)
    runs += [
        dict(run, profile="full bootstrap") for run in bootstrap_runs(repository, head)
    ]
    newest: dict[str, dict] = {}
    for run in sorted(runs, key=lambda run: (run["created_at"], run["id"])):
        newest[run["profile"]] = run
    return {
        profile: cast(dict, run_fact(run)) for profile, run in sorted(newest.items())
    }


def queue_fact(repository: str, pr: int) -> dict:
    pull = queue_state(repository, pr)
    if pull["state"] == "MERGED":
        return {"queued": False, "text": "merged"}
    if position := queue_position(pull):
        return {"queued": True, "text": position}
    last = queue_run(repository, pr)
    text = "not queued"
    if last and last["conclusion"] in REMOVED:
        text += f"; last merge-group run {last['conclusion']}: {last['html_url']}"
    return {"queued": False, "text": text}


def mergeable_word(pull: dict) -> str:
    words = {True: "yes", False: "conflicts", None: "pending"}
    return f"{words[pull.get('mergeable')]} ({pull.get('mergeable_state', 'unknown')})"


def pull_state(repository: str, pr: int) -> dict:
    """One PR's state as the tools read it, from one head; refuse a view read
    across a push."""
    pull: dict = {**api(repository, f"pulls/{pr}"), "number": pr}
    head = pull["head"]["sha"]
    comments = listing(repository, f"issues/{pr}/comments?per_page=100")
    reviews = listing(repository, f"pulls/{pr}/reviews?per_page=100")
    reports = review_report.review_reports(comments, reviews, head)
    reports.sort(key=lambda report: report.reported_at, reverse=True)
    facts = {
        "repository": repository,
        "number": pr,
        "title": pull["title"],
        "state": "merged" if pull.get("merged_at") else pull["state"],
        "draft": bool(pull.get("draft")),
        "head": head,
        "mergeable": mergeable_word(pull),
        "quick": run_fact(latest_quick(repository, head)),
        "requests": requests_at_head(repository, pull, head),
        "queue": queue_fact(repository, pr),
        "reports": [asdict(report) for report in reports],
        "enqueue_checks": enqueue_assessment(repository, pr, pull),
    }
    end = api(repository, f"pulls/{pr}")["head"]["sha"]
    if end != head:
        raise ValueError(
            f"PR #{pr} head moved from {head} to {end} while its state was read; "
            f"run python3 tools/ci.py state {pr} again"
        )
    return facts


def run_line(label: str, fact: dict | None) -> str:
    if fact is None:
        return f"{label}: none"
    return f"{label}: {fact['conclusion'] or fact['status']} {fact['url']}"


def report_line(report: dict) -> str:
    verdict = report["disposition"]
    if report["scope"]:
        verdict += f" ({report['scope']})"
    where = (
        "at head" if report["matches_head"] else f"at {report['sha'][:10]}, not head"
    )
    return f"  {report['agent']}: {verdict} {where} {report['url']}"


SHOWN_REPORTS = 5


def state_lines(facts: dict) -> list[str]:
    ready = "draft" if facts["draft"] else "ready"
    lines = [
        f"PR #{facts['number']} {facts['state']}, {ready}: {facts['title']}",
        f"head {facts['head']}, mergeable: {facts['mergeable']}",
        run_line("Quick at head", facts["quick"]),
    ]
    requests = facts["requests"]
    lines += [run_line(f"{p} at head", fact) for p, fact in requests.items()]
    if not requests:
        lines.append("Requested runs at head: none")
    lines.append(f"Queue: {facts['queue']['text']}")
    reports = facts["reports"]
    lines.append(
        f"Review reports: {len(reports)}" + (", newest first" if reports else "")
    )
    lines += [report_line(report) for report in reports[:SHOWN_REPORTS]]
    if older := reports[SHOWN_REPORTS:]:
        links = ", ".join(report["url"] for report in older)
        lines.append(f"  {len(older)} older: {links}")
    lines.append(f"enqueue checks: {facts['enqueue_checks']}")
    return lines


def show_state(repository: str, pr: int, as_json: bool) -> None:
    facts = pull_state(repository, pr)
    print(json.dumps(facts, indent=2) if as_json else "\n".join(state_lines(facts)))


def pr_runs(repository: str, pr: int, profile: str | None) -> tuple[str, list[dict]]:
    """The PR's head and its candidate runs: requested runs at that head and
    merge-group runs, each with a `kind`."""
    pull: dict = {**api(repository, f"pulls/{pr}"), "number": pr}
    head = pull["head"]["sha"]
    runs = []
    if profile != "queue":
        runs += [
            dict(run, kind=f"requested {run['profile']}")
            for run in dispatched_runs(repository, pull, head)
            if profile in (None, run["profile"])
        ]
    if profile in (None, "queue"):
        runs += [dict(run, kind="merge-group") for run in queue_runs(repository, pr)]
    return head, runs


def pr_run(repository: str, pr: int, profile: str | None) -> int:
    """The newest of the PR's candidate runs, saying which it chose and why."""
    head, runs = pr_runs(repository, pr, profile)
    if not runs:
        raise ValueError(
            f"PR #{pr} has no {profile or 'requested'} run at its head {head} and no "
            "merge-group run; pass the run ID or URL: python3 tools/ci.py log RUN"
        )
    runs.sort(key=lambda run: (run["created_at"], run["id"]))
    chosen = runs[-1]
    tested = (
        ejected_head(repository, cast(QueueRun, chosen))
        if chosen["kind"] == "merge-group"
        else head
    )
    print(
        f"Selected the newest {chosen['kind']} run for PR #{pr}: run {chosen['id']} "
        f"attempt {chosen.get('run_attempt', 1)}, tested head {tested} "
        f"(PR head {head}): {chosen['html_url']}"
    )
    others = {run["kind"]: run for run in runs if run["kind"] != chosen["kind"]}
    if others:
        listed = ", ".join(f"{kind} run {run['id']}" for kind, run in others.items())
        print(f"Also found: {listed}; pass --profile or the run ID to read another")
    return chosen["id"]


def request_run(path: Path) -> tuple[str, int]:
    record = read_request(path)
    run = recorded_run(path, record)
    if run is None:
        raise ValueError(
            f"The request in {path} has no run yet; wait for it: "
            f"python3 tools/ci.py wait {path}"
        )
    return record["repository"], run["id"]


def log_target(args: argparse.Namespace) -> tuple[str, int, int | None]:
    """The repository, run and job (or None) that `log`'s arguments name."""
    if (args.pr is None) == (args.target is None):
        raise ValueError(
            "name one run: python3 tools/ci.py log RUN_ID|RUN_URL|JOB_URL|"
            "REQUEST_FILE, or python3 tools/ci.py log --pr N"
        )
    if args.pr is not None:
        repository = repository_of(args.repo)
        return repository, pr_run(repository, args.pr, args.profile), None
    if match := RUN_URL.fullmatch(args.target):
        job = int(match[3]) if match[3] else None
        return match[1], int(match[2]), job
    if args.target.isdigit():
        return repository_of(args.repo), int(args.target), None
    return (*request_run(Path(args.target)), None)


def selected_jobs(
    repository: str, jobs: list[dict], job_id: int | None, name: str | None
) -> list[dict]:
    if job_id is not None:
        return [api(repository, f"actions/jobs/{job_id}")]
    if name:
        return [job for job in jobs if name.lower() in job["name"].lower()]
    return [job for job in jobs if job["conclusion"] in JOB_FAILED]


def no_jobs(run: dict, jobs: list[dict], name: str | None) -> str:
    if name:
        names = ", ".join(job["name"] for job in jobs)
        return f"No job named like {name!r}; the run's jobs: {names}"
    if ci_log.unfinished(jobs) or run["status"] != "completed":
        return "No failed job yet"
    return f"No failed job: the run concluded {run['conclusion']}"


def log_lines(
    repository: str, run_id: int, job_id: int | None, args: argparse.Namespace
) -> list[str]:
    """The run's heading, then each selected job's log as `args` asks."""
    grep: str | None = args.grep
    pattern = re.compile(grep) if grep else None
    run = api(repository, f"actions/runs/{run_id}")
    jobs = run_jobs(repository, run_id)
    lines = ci_log.run_heading(run, jobs)
    chosen = selected_jobs(repository, jobs, job_id, args.job)
    if not chosen:
        lines.append(no_jobs(run, jobs, args.job))
    for job in chosen:
        lines += ci_log.job_report(repository, job, pattern, args.full)
    return lines


def show_log(args: argparse.Namespace) -> None:
    repository, run_id, job_id = log_target(args)
    print("\n".join(log_lines(repository, run_id, job_id, args)))


def failure_logs(repository: str, run_id: int | None) -> None:
    """Best effort: the failed jobs' summary after a failed wait. A failure to
    read them prints one line and changes no exit status."""
    if run_id is None:
        return
    summary = argparse.Namespace(job=None, grep=None, full=False)
    try:
        print("\n".join(log_lines(repository, run_id, None, summary)), flush=True)
    except Exception as error:  # Any failure here must keep the wait's status.
        print(
            f"Could not read the failed jobs' logs ({error}); read them with: "
            f"python3 tools/ci.py log {run_id} --repo {repository}",
            flush=True,
        )


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


def timing(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--timeout",
        type=positive,
        default=WAIT_TIMEOUT,
        help=f"seconds to wait before exit 2 (default {WAIT_TIMEOUT})",
    )
    parser.add_argument("--interval", type=positive, default=15)


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
        wait(path, args.timeout, args.interval)


HANDLERS = {
    "log": show_log,
    "logs": show_log,
    "queue-wait": queue_wait_command,
    "ready": ready,
    "enqueue": lambda args: print(f"PR #{args.pr}: {enqueue(args)}"),
    "failures": failures,
    "request": request_command,
    "wait": lambda args: wait(wait_target(args), args.timeout, args.interval),
}
# The commands whose failure prints the failed jobs' log summary.
WAITS = ("wait", "request", "queue-wait")


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
        metavar="PR|REQUEST_FILE",
        help="a request file, or a PR number for the newest request at its head",
    )
    waited.add_argument("--profile", choices=PROFILES, help="with a PR number")
    waited.add_argument("--repo", help="with a PR number")
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
    request_parser(commands)
    pr_parsers(commands)
    args = parser.parse_args()
    try:
        dispatch(args)
        return 0
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
            failure_logs(failed.repository, failed.run_id)
        return 1
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f"ci.py {args.command} failed: {error}", file=sys.stderr)
        if isinstance(error, subprocess.CalledProcessError) and error.stderr:
            print(error.stderr.strip(), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
