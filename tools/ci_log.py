"""Failed CI jobs' logs for ci.py log: fetch once, cache, print the failing step.

A job's log comes from the job-logs API, which serves a finished job's log while
other jobs of the run still go. Raw logs are cached under the checkout's
`.local/ci-logs/`, keyed by repository, run, attempt and job; a log GitHub has
not served (404 or 410), an empty log and a failed download are not cached.
"""

from __future__ import annotations

import os
import re
import subprocess
import tempfile
from collections.abc import Iterable
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path

import vcs

# The checkout's .local, unresolved: it is a symlink onto the output disk.
LOGS = vcs.ROOT / ".local" / "ci-logs"
# GitHub's envelope: an ISO timestamp with 7 fractional digits and one space.
STAMP = re.compile(r"^\ufeff?(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d)(?:\.\d+)?Z ")
ANSI = re.compile(
    r"\x1b(?:\[[0-?]*[ -/]*[@-~]|\][^\x07\x1b]*(?:\x07|\x1b\\)|[@-Z\\-_])"
)
ERROR = re.compile(
    r"##\[error\]|\berror(?:\[E\d+\])?:|FAILED|panicked at|Assertion|\*\*\*Failed"
    r"|Test +#\d+: .*Failed|exit code [1-9]"
)
WIDTH = 300
ERRORS = 30
TAIL = 40
MATCHES = 200
# The run states other than completed, in which jobs may still start or run.
UNFINISHED = ("queued", "in_progress", "waiting", "requested", "pending")
UNAVAILABLE = ("HTTP 404", "HTTP 410")


@dataclass
class Log:
    """A job's log: `text` is None while GitHub serves none; `note` says why."""

    text: str | None
    path: Path
    note: str = ""


def clean(line: str) -> str:
    """A log line without GitHub's timestamp envelope, ANSI codes or excess width."""
    line = ANSI.sub("", STAMP.sub("", line, count=1))
    return line if len(line) <= WIDTH else line[:WIDTH] + " [cut]"


def second(value: str | None) -> str | None:
    """An API timestamp as `YYYY-MM-DDTHH:MM:SS` in UTC, the log stamps' form."""
    if not value:
        return None
    moment = datetime.fromisoformat(value.replace("Z", "+00:00"))
    return moment.astimezone(timezone.utc).strftime("%Y-%m-%dT%H:%M:%S")


def numbered(text: str) -> list[tuple[int, str]]:
    return list(enumerate(text.splitlines(), 1))


def step_lines(text: str, step: dict | None) -> list[tuple[int, str]]:
    """The raw lines a step wrote, found by its start and end times against the
    lines' stamps; an unstamped line belongs with the line before it. Stamps
    count whole seconds, so a failed step ends at its last `##[error]` line, not
    in the next step's lines of the same second. The whole log when there is no
    step or no stamped line falls inside it."""
    lines = numbered(text)
    start, end = (
        second((step or {}).get("started_at")),
        second((step or {}).get("completed_at")),
    )
    if not start or not end:
        return lines
    inside, chosen = False, []
    for number, line in lines:
        stamp = STAMP.match(line)
        if stamp:
            inside = start <= stamp[1] <= end
        if inside:
            chosen.append((number, line))
    errors = [i for i, (_, line) in enumerate(chosen) if "##[error]" in line]
    return chosen[: errors[-1] + 1] if errors else chosen or lines


def failing_step(job: dict) -> dict | None:
    return next(
        (
            step
            for step in job.get("steps") or []
            if step.get("conclusion") in ("failure", "timed_out", "cancelled")
        ),
        None,
    )


def cache_path(repository: str, job: dict) -> Path:
    owner, name = repository.split("/", 1)
    attempt = f"attempt-{job.get('run_attempt') or 1}"
    return LOGS / owner / name / str(job["run_id"]) / attempt / f"{job['id']}.log"


def download(repository: str, job_id: int) -> str | None:
    """The job's raw log, or None when GitHub serves none (404 or 410)."""
    result = subprocess.run(
        [
            "gh",
            "api",
            "--allow-escape-sequences",
            f"repos/{repository}/actions/jobs/{job_id}/logs",
        ],
        capture_output=True,
        check=False,
    )
    stderr = result.stderr.decode(errors="replace")
    if result.returncode and any(code in stderr for code in UNAVAILABLE):
        return None
    if result.returncode:
        raise subprocess.CalledProcessError(
            result.returncode, result.args, result.stdout, stderr
        )
    return result.stdout.decode(errors="replace")


def store(path: Path, text: str) -> None:
    """Write the whole log to this writer's own temporary file beside it, then
    rename it into place, so no partial file is read as a complete one and two
    writers of one log do not take each other's file. A failure removes it."""
    path.parent.mkdir(parents=True, exist_ok=True)
    partial = tempfile.NamedTemporaryFile(
        "w", dir=path.parent, prefix=path.name + ".", suffix=".partial", delete=False
    )
    try:
        with partial:
            partial.write(text)
        os.replace(partial.name, path)
    except BaseException:
        Path(partial.name).unlink(missing_ok=True)
        raise


def job_log(repository: str, job: dict) -> Log:
    """The job's log from the cache, else from GitHub, cached once complete."""
    path = cache_path(repository, job)
    if path.is_file():
        return Log(path.read_text(errors="replace"), path)
    if job.get("status") != "completed":
        return Log(None, path, "the job is still running; GitHub serves no log yet")
    text = download(repository, job["id"])
    if text is None:
        return Log(None, path, "GitHub has no log for it yet (404); try again later")
    if not text.strip():
        return Log(None, path, "GitHub served an empty log; try again later")
    store(path, text)
    return Log(text, path)


def bounded(lines: Iterable[tuple[int, str]], limit: int) -> list[str]:
    return [f"  {number}: {clean(line)}" for number, line in list(lines)[-limit:]]


def step_title(job: dict, step: dict | None) -> str:
    if step:
        return f"Failing step: {step.get('number')} {step.get('name')} ({step.get('conclusion')})"
    return (
        f"No failed step: the job ended {job.get('conclusion')}, as a cancel or a "
        "lost runner leaves it; the log's tail follows"
    )


def summary(log: Log, job: dict) -> list[str]:
    """The failing step, its error lines and its tail, bounded."""
    assert log.text is not None
    step = failing_step(job)
    section = step_lines(log.text, step)
    errors = [(n, line) for n, line in section if ERROR.search(clean(line))]
    lines = [
        step_title(job, step),
        f"Log: {log.path} ({len(numbered(log.text))} lines)",
    ]
    if errors:
        lines.append(f"Error lines ({min(len(errors), ERRORS)} of {len(errors)}):")
        lines += bounded(errors, ERRORS)
    lines.append(f"Step tail (last {min(len(section), TAIL)} lines):")
    return lines + bounded(section, TAIL)


def grep(log: Log, pattern: re.Pattern[str], full: bool) -> list[str]:
    """The log's lines matching `pattern`; raw and unbounded with `full`."""
    assert log.text is not None
    if full:
        return [line for line in log.text.splitlines() if pattern.search(line)]
    found = [(n, line) for n, line in numbered(log.text) if pattern.search(clean(line))]
    more = [f"  ({len(found) - MATCHES} more; --full prints all)"]
    return bounded(found[:MATCHES], MATCHES) + (more if len(found) > MATCHES else [])


def job_report(
    repository: str, job: dict, pattern: re.Pattern[str] | None, full: bool
) -> list[str]:
    """One job's heading and its log as the options ask."""
    state = job.get("conclusion") or job.get("status")
    lines = [f"== {job['name']} ({state}): {job['html_url']}"]
    log = job_log(repository, job)
    if log.text is None:
        return lines + [
            f"No log: {log.note}; it was not cached. Job: {job['html_url']}"
        ]
    if pattern:
        return lines + grep(log, pattern, full)
    if full:
        return lines + log.text.splitlines()
    return lines + summary(log, job)


def unfinished(jobs: list[dict]) -> list[dict]:
    return [job for job in jobs if job.get("status") in UNFINISHED]


def run_heading(run: dict, jobs: list[dict]) -> list[str]:
    """The run, and a warning when jobs still run: a later failure may follow."""
    state = run.get("conclusion") or run["status"]
    lines = [
        f"Run {run['id']} attempt {run.get('run_attempt', 1)} ({state}): {run['html_url']}"
    ]
    if running := unfinished(jobs):
        lines.append(
            f"{len(running)} jobs still running: the failures shown may not be the "
            "last. docs/workflow.md (Removal) asks for the run read to its last job "
            f"before a cause is recorded; read again when it ends: "
            f"python3 tools/ci.py log {run['id']}"
        )
    return lines
