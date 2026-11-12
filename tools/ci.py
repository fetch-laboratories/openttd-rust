#!/usr/bin/env python3
"""Request scoped GitHub CI for an immutable PR head, using ordinary gh CLI."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
import uuid
from datetime import datetime, timezone
from pathlib import Path
from typing import TypedDict
from urllib.parse import quote

import preflight_stamp
import vcs

PROFILES = ("rust", "native", "platform", "full")
WORKFLOW = "ci-request.yml"
PROTECTED_REF = "rust-migration"
CONTEXT = "Full validation"
# ci-invalidate.yml's merge-queue status on a PR head: no Full ran on that head.
QUEUED = "Full runs in the merge queue"
# Under the agent shell's 10-minute command cap, so a foreground wait returns.
WAIT_TIMEOUT = 540
# A queue-wait runs as a background job; the queue's Full takes about an hour.
QUEUE_TIMEOUT = 5400
# Quick's required check on a PR head, from its pull_request run.
QUICK = "Quick validation"
# The one branch prefix of deliberate negative controls; `failures` leaves them out.
CONTROL = "control/"
FAILED = ("failure", "timed_out", "startup_failure")
REQUESTS = Path(".local/ci-requests")


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
        raise ValueError(f"PR head changed: requested {head}, current {current}")
    return current


def validate_pull(pull: dict, head: str | None = None) -> str:
    # Mergeability selects the tested merge, so only requests require it. A base
    # push can leave it pending; the requested run and its result remain valid.
    current = current_head(pull, head)
    if pull.get("mergeable") is not True or not pull.get("merge_commit_sha"):
        raise ValueError(
            "PR is unmergeable or mergeability is pending; retry after resolving it"
        )
    return current


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
        # ci-request.yml names the requested head after the request since #922.
        titles = (title, f"{title} {record.get('head')}")
        matches = [
            run
            for run in runs
            if run.get("display_title") in titles
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


def request(args: argparse.Namespace) -> Path | None:
    repository = repository_of(args.repo)
    pull = api(repository, f"pulls/{args.pr}")
    head = current_head(pull)
    require_stamp(args.pr, head, args.unstamped)
    # A new request would reset a passing status, including one carried across a
    # docs-only base update, to pending and repeat the run.
    if args.profile == "full" and not args.bootstrap:
        if run := passed_run(repository, head):
            print(f"PR #{args.pr} at {head} already passed Full validation: {run}")
            return None
        try:
            queue = variable(repository, "CI_MERGE_QUEUE") == "true"
        except subprocess.CalledProcessError:
            # Reading variables needs its own token permission, which an ordinary
            # request never needed; the plan applies the same rule server-side.
            queue = False
        if queue and changes_definitions(repository, args.pr):
            raise ValueError(
                "In merge-queue mode a PR changing .github/workflows/ needs root's "
                "reviewed bootstrap Full"
            )
    validate_pull(pull, head)
    if args.bootstrap and args.profile != "full":
        raise ValueError("Bootstrap label requests only support --profile full")
    request_id = uuid.uuid4().hex
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
    if args.bootstrap:
        if variable(repository, "CI_ON_DEMAND") == "true":
            raise ValueError(
                "Label requests need CI_ON_DEMAND unset; root must review workflow changes first"
            )
        # Remove/add is intentional: only the explicit label event requests a run.
        labels = {label["name"] for label in pull["labels"]}
        if "ci:full" in labels:
            gh(
                "pr",
                "edit",
                str(args.pr),
                "--repo",
                repository,
                "--remove-label",
                "ci:full",
            )
        validate_pull(api(repository, f"pulls/{args.pr}"), head)
        gh("pr", "edit", str(args.pr), "--repo", repository, "--add-label", "ci:full")
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
    return path


def status(path: Path) -> tuple[dict, dict | None]:
    record = json.loads(path.read_text())
    if record.get("schema_version") != 1:
        raise ValueError("Unsupported CI request record schema")
    repository = record["repository"]
    current_head(api(repository, f"pulls/{record['pr']}"), record["head"])
    if record.get("run_id"):
        run = api(repository, f"actions/runs/{record['run_id']}")
    else:
        runs = api(repository, f"actions/workflows/{WORKFLOW}/runs?per_page=100")[
            "workflow_runs"
        ]
        run = find_run(record, runs)
        if run:
            record["run_id"] = run["id"]
            record["run_url"] = run["html_url"]
            write_request(path, record)
    if run and run["status"] != "completed":
        run["failed_jobs"] = failed_jobs(repository, run["id"])
    return record, run


def failed_jobs(repository: str, run_id: int) -> list[dict]:
    pages = json.loads(
        gh(
            "api",
            "--paginate",
            "--slurp",
            f"repos/{repository}/actions/runs/{run_id}/jobs?filter=latest&per_page=100",
        )
    )
    return [
        job
        for page in pages
        for job in page["jobs"]
        if job["conclusion"]
        in (
            "failure",
            "timed_out",
            "cancelled",
            "action_required",
            "startup_failure",
            "stale",
        )
    ]


def verify_children(run: dict) -> None:
    if run.get("failed_jobs"):
        failed = ", ".join(
            f"{job['name']} ({job['html_url']})" for job in run["failed_jobs"]
        )
        raise ValueError(
            f"Failed child jobs: {failed}. Other jobs continue; successful jobs remain available for retry."
        )


def verify_completion(record: dict, run: dict) -> None:
    if run["conclusion"] != "success":
        raise ValueError(
            f"Requested CI concluded {run['conclusion']}: {run['html_url']}"
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
            verify_children(run)
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
    """The PR left the merge queue without merging."""


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


def removal(repository: str, pr: int, state: str) -> str:
    run = queue_run(repository, pr)
    if run is None:
        return f"PR #{pr} left the merge queue ({state}); no merge-group run found"
    lines = [
        f"PR #{pr} left the merge queue ({state}); merge-group run "
        f"{run['status']}/{run['conclusion']}: {run['html_url']}"
    ]
    lines += [
        f"failed job: {job['name']} ({job['html_url']})"
        for job in failed_jobs(repository, run["id"])
    ]
    return "\n".join(lines)


def queue_wait(repository: str, pr: int, timeout: int, interval: int) -> str:
    """Block until the PR merges (return its merge commit) or leaves the queue."""
    deadline = time.monotonic() + timeout
    previous = None
    while True:
        pull = queue_state(repository, pr)
        commit = pull["mergeCommit"]
        if pull["state"] == "MERGED" and commit:
            return commit["oid"]
        entry = pull["mergeQueueEntry"]
        if pull["state"] != "OPEN" or not entry and not pull["autoMergeRequest"]:
            raise QueueRemoved(removal(repository, pr, pull["state"].lower()))
        state = (
            f"queued {entry['state'].lower()} at position {entry['position']}"
            if entry
            else "awaiting entry to the merge queue"
        )
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


def latest_request(repository: str, pr: int, head: str) -> dict | None:
    """The newest requested or bootstrap run of ci-request.yml for this PR head."""
    path = f"actions/workflows/{WORKFLOW}/runs?per_page=100&event="
    dispatched = api(repository, path + "workflow_dispatch")["workflow_runs"]
    named = [
        run
        for run in dispatched
        if (title := (run.get("display_title") or "").split())[2:4] == ["PR", str(pr)]
        and title[-1] == head
    ]
    # Every label starts a run; only a ci:full bootstrap runs its plan.
    labeled = [
        run
        for run in api(repository, path + f"pull_request&head_sha={head}")[
            "workflow_runs"
        ]
        if any(
            job["name"] == "plan" and job["conclusion"] != "skipped"
            for job in api(repository, f"actions/runs/{run['id']}/jobs")["jobs"]
        )
    ]
    # Requests made before #922 carry no head in their title; their records do.
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
    if not runs or runs[0]["conclusion"] not in FAILED:
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
        if run["conclusion"] in FAILED and ejected_head(repository, run) == head
    ]
    if len(failed) > 1:
        raise ValueError(
            f"PR #{pr} failed the queue {len(failed)} times at {head}; "
            "a second failure goes to root"
        )
    log = preflight_stamp.log_override("requeue", f"PR #{pr}", head, retry)
    print(f"Queueing PR #{pr} again unchanged after {url}; logged in {log}")


def enqueue(args: argparse.Namespace) -> str:
    """Queue a PR whose head has green Quick and requested runs; report the entry."""
    repository = repository_of(args.repo)
    pull = api(repository, f"pulls/{args.pr}")
    head = current_head(pull)
    if pull.get("draft"):
        raise ValueError(f"PR #{args.pr} is a draft; mark it ready with ci.py ready")
    quick = latest_quick(repository, head)
    if not quick or quick["conclusion"] != "success":
        state = (
            quick and f"{quick['status']}/{quick['conclusion']}: {quick['html_url']}"
        )
        raise ValueError(f"{QUICK} at {head} is not green: {state or 'no run'}")
    run = latest_request(repository, args.pr, head)
    if not run or run["conclusion"] != "success":
        state = run and f"{run['status']}/{run['conclusion']}: {run['html_url']}"
        raise ValueError(
            f"The latest requested run at {head} is not green: {state or 'none; '}"
            + ("" if run else f"run ci.py request {args.pr} --profile native")
        )
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
        return f"queued {entry['state'].lower()} at position {entry['position']}"
    if state["autoMergeRequest"]:
        return (
            "auto-merge armed: GitHub queues the PR once its pending required "
            "checks pass"
        )
    raise ValueError(f"PR #{args.pr} is neither queued nor armed for auto-merge")


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
        verify_children(run)
    if run and run["status"] == "completed":
        verify_completion(record, run)


def positive(text: str) -> int:
    value = int(text)
    if value <= 0:
        raise argparse.ArgumentTypeError("must be positive")
    return value


def dispatch(args: argparse.Namespace) -> None:
    if args.command == "queue-wait":
        merge = queue_wait(
            repository_of(args.repo), args.pr, args.timeout, args.interval
        )
        print(f"PR #{args.pr} merged: {merge}")
    elif args.command == "ready":
        ready(args)
    elif args.command == "enqueue":
        print(f"PR #{args.pr}: {enqueue(args)}")
    elif args.command == "failures":
        failures(args)
    elif args.command == "request":
        path = request(args)
        if path and args.wait:
            wait(path, WAIT_TIMEOUT, 15)
    elif args.command == "wait":
        wait(args.request_file, args.timeout, args.interval)
    else:
        report_status(args.request_file)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
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
    submit.add_argument("--wait", action="store_true")
    submit.add_argument(
        "--unstamped",
        metavar="REASON",
        help="request a head without a preflight stamp, such as a seeded control; logged",
    )
    marked = commands.add_parser(
        "ready", help="mark a draft PR ready once its head has a preflight stamp"
    )
    marked.add_argument("pr", type=int)
    marked.add_argument("--repo")
    for command in ("status", "wait"):
        check = commands.add_parser(command)
        check.add_argument("request_file", type=Path)
        check.add_argument("--timeout", type=positive, default=WAIT_TIMEOUT)
        check.add_argument("--interval", type=positive, default=15)
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
    args = parser.parse_args()
    try:
        dispatch(args)
        return 0
    except WaitTimeout as timeout:
        print(timeout)
        return 2
    except QueueRemoved as removed:
        print(removed)
        return 3
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f"CI request failed: {error}", file=sys.stderr)
        if isinstance(error, subprocess.CalledProcessError) and error.stderr:
            print(error.stderr.strip(), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
