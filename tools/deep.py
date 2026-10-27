"""Deep checks: the host-side tier for runs too slow for the per-PR Full (#503).

`python3 tools/migration.py deep [selectors]` runs every member of `DEEP_SET` on the
checked-out commit at idle CPU and I/O priority and writes one `report.json` under
`.local/deep/`. Members are ordinary driver actions, so they build, hold the shared
build locks and yield to a speed pass's timed collection like any other harness game.
"""

import fcntl
import json
import os
import re
import shutil
import subprocess
import sys
import time
from collections.abc import Callable, Generator, Sequence
from contextlib import contextmanager
from dataclasses import asdict, dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import TypedDict

from validation_execution import run_logged

# One exclusive clone-wide lock: a deep run and a speed pass's timed collection never overlap.
LOCK_NAME = "deep.lock"
# Idle-priority games run slowly under load.
GAME_TIMEOUT = 7200
SHOWN_DIFFERENCES = 3


@dataclass(frozen=True)
class Member:
    name: str
    command: Callable[[Path, int, Sequence[str]], list[str]]  # root, jobs, selectors


def migration_command(*action: str) -> Callable[[Path, int, Sequence[str]], list[str]]:
    def command(root: Path, jobs: int, selectors: Sequence[str]) -> list[str]:
        driver = str(root / "tools/migration.py")
        return [sys.executable, driver, *action, "--jobs", str(jobs), *selectors]

    return command


DEEP_SET = (
    Member(
        "simulate-soak",
        migration_command("simulate", "--soak", "--timeout", str(GAME_TIMEOUT)),
    ),
    Member(
        "simulate-sanitize",
        migration_command("simulate", "--sanitize", "--timeout", str(GAME_TIMEOUT)),
    ),
)


class Failure(TypedDict):
    scenario: str
    problems: list[str]
    differences: list[dict[str, object]]


@dataclass
class Result:
    name: str
    command: list[str]
    exit: int
    seconds: float
    log: str
    passed: bool = False
    simulation_report: str | None = None
    scenarios: int = 0
    scenario_seconds: float = 0.0
    failures: list[Failure] = field(default_factory=list)
    problems: list[str] = field(default_factory=list)


def git(root: Path, *arguments: str) -> str:
    return subprocess.check_output(["git", *arguments], cwd=root, text=True).strip()


@contextmanager
def single_run(path: Path) -> Generator[bool]:
    """Yield False, without waiting, when another deep run holds the clone-wide lock."""
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a+b") as handle:
        try:
            fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except OSError:
            yield False
            return
        try:
            yield True
        finally:
            fcntl.flock(handle, fcntl.LOCK_UN)


def low_priority(command: list[str]) -> list[str]:
    prefix = ["nice", "-n", "19"]
    if shutil.which("ionice"):
        prefix += ["ionice", "-c", "3"]
    return [*prefix, *command]


def summarize(result: Result, commit: str) -> None:
    """Fill the outcome from the `Report:` line of the member's simulate log."""
    found = re.findall(r"Report: (\S+report\.json)", Path(result.log).read_text())
    if not found:
        return
    result.simulation_report = found[-1]
    simulation = json.loads(Path(found[-1]).read_text())
    if simulation["candidate_commit"] != commit:
        result.problems.append("member commit differs from the deep-check commit")
    if simulation["candidate_status"]:
        result.problems.append("member ran from a dirty checkout")
    rows = simulation["results"]
    result.scenarios = len(rows)
    result.scenario_seconds = round(sum(row["seconds"] for row in rows), 1)
    result.failures = [
        {
            "scenario": row["scenario"],
            "problems": row["problems"],
            "differences": row["differences"][:SHOWN_DIFFERENCES],
        }
        for row in rows
        if not row["passed"]
    ]
    result.passed = (
        result.exit == 0
        and simulation["passed"]
        and not result.failures
        and not result.problems
    )


def run_deep(
    root: Path,
    local: Path,
    common_local: Path,
    jobs: int,
    selectors: Sequence[str],
    members: Sequence[Member] = DEEP_SET,
) -> int:
    commit = git(root, "rev-parse", "HEAD")
    if git(root, "status", "--short"):
        sys.exit("deep requires a clean checkout: the report names its commit")
    with single_run(common_local / LOCK_NAME) as acquired:
        if not acquired:
            print("deep: another deep run is active; skipped", flush=True)
            return 0
        started = time.monotonic()
        stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
        out = local / "deep" / f"{stamp}-{commit[:12]}"
        out.mkdir(parents=True)
        env = dict(os.environ, PYTHONUNBUFFERED="1")
        results: list[Result] = []
        for member in members:
            command = member.command(root, jobs, selectors)
            log = out / f"{member.name}.log"
            begun = time.monotonic()
            code = run_logged(
                low_priority(command), cwd=root, env=env, log=log, phase=member.name
            )
            result = Result(
                member.name, command, code, round(time.monotonic() - begun, 1), str(log)
            )
            summarize(result, commit)
            results.append(result)
        report = {
            "schema_version": 1,
            "commit": commit,
            "started_at": stamp,
            "selectors": list(selectors),
            "partial": bool(selectors),
            "jobs": jobs,
            "seconds": round(time.monotonic() - started, 1),
            "results": [asdict(result) for result in results],
            "passed": all(result.passed for result in results),
        }
        (out / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    for result in results:
        status = "ok  " if result.passed else "FAIL"
        print(
            f"{status} {result.name}: {result.scenarios} scenarios, "
            f"{len(result.failures)} failed, exit {result.exit}, {result.seconds} s",
            flush=True,
        )
        for problem in result.problems:
            print(f"     {problem}", flush=True)
    print(f"Deep report: {out / 'report.json'}", flush=True)
    return 0 if report["passed"] else 1
