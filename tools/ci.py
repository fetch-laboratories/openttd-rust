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

PROFILES = ("rust", "native", "platform", "full")
WORKFLOW = "ci-request.yml"
PROTECTED_REF = "rust-migration"
CONTEXT = "Full validation"
# Under the agent shell's 10-minute command cap, so a foreground wait returns.
WAIT_TIMEOUT = 540


def gh(*args: str) -> str:
    return subprocess.run(
        ["gh", *args], check=True, capture_output=True, text=True
    ).stdout.strip()


def api(repository: str, path: str) -> dict:
    return json.loads(gh("api", f"repos/{repository}/{path}"))


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


def request(args: argparse.Namespace) -> Path:
    repository = (
        args.repo
        or json.loads(gh("repo", "view", "--json", "nameWithOwner"))["nameWithOwner"]
    )
    pull = api(repository, f"pulls/{args.pr}")
    head = validate_pull(pull)
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
    path = args.request_file or Path(".local/ci-requests") / f"{request_id}.json"
    write_request(path, record)
    if args.bootstrap:
        variables = json.loads(
            gh("variable", "list", "--repo", repository, "--json", "name,value")
        )
        if any(
            item["name"] == "CI_ON_DEMAND" and item["value"] == "true"
            for item in variables
        ):
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
        pages = json.loads(
            gh(
                "api",
                "--paginate",
                "--slurp",
                f"repos/{repository}/actions/runs/{run['id']}/jobs?filter=latest&per_page=100",
            )
        )
        run["failed_jobs"] = [
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
    return record, run


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
    for command in ("status", "wait"):
        check = commands.add_parser(command)
        check.add_argument("request_file", type=Path)
        check.add_argument("--timeout", type=int, default=WAIT_TIMEOUT)
        check.add_argument("--interval", type=int, default=15)
    args = parser.parse_args()
    try:
        if args.command == "request":
            path = request(args)
            if args.wait:
                wait(path, WAIT_TIMEOUT, 15)
        elif args.command == "wait":
            if args.timeout <= 0 or args.interval <= 0:
                raise ValueError("Timeout and interval must be positive")
            wait(args.request_file, args.timeout, args.interval)
        else:
            record, run = status(args.request_file)
            print(json.dumps({"request": record, "run": run}, indent=2))
            if run:
                verify_children(run)
            if run and run["status"] == "completed":
                verify_completion(record, run)
        return 0
    except WaitTimeout as timeout:
        print(timeout)
        return 2
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f"CI request failed: {error}", file=sys.stderr)
        if isinstance(error, subprocess.CalledProcessError) and error.stderr:
            print(error.stderr.strip(), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
