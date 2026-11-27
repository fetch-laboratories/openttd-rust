#!/usr/bin/env python3
"""Run every gate of Quick's `quick` job locally, plus the pinned commit checker.

The steps come from .github/workflows/ci-quick.yml itself, so the two cannot
drift. The steps read the tree and write nothing a later step reads, so --jobs
of them run at once: Quick's Rust checks first, then the longest of the last
run. A failure of the Rust checks stops the run and lists the steps it stopped;
any other failure lets every step run, and the result lists each failed step.
--keep-going runs every step after the Rust checks fail too.

--fast runs, in seconds, the pre-push hook's fast tier (the pinned commit
checker, cargo fmt --check and Quick's Ruff lines) and Quick's ty line, and
records no stamp.
"""

import argparse
import datetime
import json
import os
import re
import shutil
import signal
import statistics
import subprocess
import sys
import threading
import time
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass, field
from pathlib import Path
from typing import IO, Any, NamedTuple, TypedDict

import preflight_stamp
import quick_workflow
import yaml
from validation_execution import file_lock

import migration

HOOKS_REVISION = "eeb3791aadf0aded5a7cd634c80823f17e87af9c"
HOOKS_URL = "https://github.com/OpenTTD/OpenTTD-git-hooks.git"
WORKFLOW = quick_workflow.WORKFLOW
COMMITS = "Check introduced commits with the pinned inherited checker"
# Preflight's own step after Quick's (#923): CI compiles these in its native jobs.
PROBES = "Compile comparison probes, and compile and link runtime adapters"
# Quick's `run:` steps that preflight does not run. Steps that `uses:` an action
# (checkouts, setup-uv) are skipped too: they only prepare the runner.
SKIPPED = {
    "Install pinned Rust compiler": "rustup installs the pinned toolchain on use",
    "Install nested collection probe dependency": "installs valgrind with apt",
    "Check every introduced commit against captured base": (
        f"preflight runs the same pinned checker as {COMMITS!r}"
    ),
    "Require every Quick job to succeed": "reports the other jobs on GitHub",
}
# BASE_REF becomes --base only when its expression is an `||` chain of these
# base-commit sources; any other expression stops preflight.
BASE_SOURCES = {
    "inputs.base-ref",
    "github.event.pull_request.base.sha",
    "github.event.merge_group.base_sha",
    "github.event.before",
}
BASE_EXPRESSION = re.compile(r"\s*\$\{\{(.*?)\}\}\s*", re.DOTALL)
# Workflow tokens are left to the local `gh` login and environment.
TOKENS = {"GH_TOKEN", "GITHUB_TOKEN"}
EXPRESSION = re.compile(r"\$\{\{.*?\}\}", re.DOTALL)
TAIL_LINES = 40
# Quick's Rust checks: about a minute, started first, and their failure stops
# the run unless --keep-going, so it does not wait out the slower steps (#1103).
RUST_CHECKS = "Rust formatting, check, warnings and tests"
# Steps at once by default: half the host's threads, at most four. The probe
# compile, the longest step, sizes its own compiler pool to the free threads.
JOBS = min(4, max(1, (os.cpu_count() or 2) // 2))
# Each step's seconds in the last run on this host, shared by every worktree,
# so the longest steps start first.
TIMINGS = "preflight/timings.json"
# One line per full run on this host, read by --runs: what steering counts (#1069).
RUNS = "preflight/runs.jsonl"
# Guards printing and the set of running step processes.
PRINTING = threading.Lock()
ACTIVE: set[subprocess.Popen[bytes]] = set()
# Seconds a terminated step has to exit before it is killed.
STOP_SECONDS = 5
INF = float("inf")
STOPS = (signal.SIGTERM, signal.SIGHUP)
STOPS_AND_INT = {signal.SIGINT, *STOPS}
# Set when a parallel run is being stopped: a step launched after it stops at once.
STOPPING = threading.Event()
SHELL = ["bash", "--noprofile", "--norc", "-eo", "pipefail", "-c"]
RUST_FORMAT = "Rust formatting (cargo fmt --check)"
FORMAT = "python3 tools/migration.py fmt"
# The command that fixes a failed fast step, by text in the step's name.
FAST_FIXES = (
    ("cargo fmt", FORMAT),
    ("ruff format", FORMAT),
    ("ruff check", f"{FORMAT} --fix applies Ruff's own fixes; fix the rest by hand"),
)
# A stamp's base must be on this branch, so base-relative gates saw every PR change.
PROTECTED = "origin/rust-migration"
# Git exports these to a hook for the repository it runs in, and `git -C` does
# not override them (#933), so commands on any other checkout drop them.
REPOSITORY_VARIABLES = frozenset(
    "GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_COMMON_DIR GIT_OBJECT_DIRECTORY "
    "GIT_ALTERNATE_OBJECT_DIRECTORIES GIT_NAMESPACE GIT_PREFIX".split()
)


@dataclass
class Step:
    name: str
    command: list[str]
    env: dict[str, str] = field(default_factory=dict)


def git(repo: Path, *arguments: str) -> str:
    """Git on the repository at repo, which cwd alone names."""
    return subprocess.check_output(
        ["git", *arguments], cwd=repo, env=other_repository(), text=True
    ).strip()


def other_repository() -> dict[str, str]:
    """This process's environment without git's repository-locating variables."""
    return {k: v for k, v in os.environ.items() if k not in REPOSITORY_VARIABLES}


def pinned_hooks() -> Path:
    """Where the pinned commit checker is checked out, shared by every worktree."""
    return migration.COMMON_LOCAL / "tooling/OpenTTD-git-hooks" / HOOKS_REVISION


def hooks_checkout(supplied: Path | None = None) -> Path:
    destination = supplied or pinned_hooks()
    with file_lock(
        migration.COMMON_LOCAL / "tooling/hooks.lock", label="commit-checker checkout"
    ):
        if not destination.exists() and supplied is None:
            destination.mkdir(parents=True)
            git(destination, "init", "--quiet")
        if supplied is None and not (destination / "hooks/check-commits.sh").exists():
            git(destination, "fetch", "--quiet", "--depth=1", HOOKS_URL, HOOKS_REVISION)
            git(destination, "checkout", "--quiet", "--detach", HOOKS_REVISION)
        if git(destination, "rev-parse", "HEAD") != HOOKS_REVISION:
            raise ValueError("commit checker is not at the pinned revision")
        if git(destination, "status", "--porcelain", "--untracked-files=all"):
            raise ValueError("commit checker checkout is dirty")
    return destination


def is_base(text: str) -> bool:
    match = BASE_EXPRESSION.fullmatch(text)
    return (
        bool(match) and {term.strip() for term in match[1].split("||")} <= BASE_SOURCES
    )


def step_env(name: str, env: dict[str, Any], base: str) -> dict[str, str]:
    """A step's `env:` with BASE_REF bound to the base and tokens left local."""
    resolved = {}
    for key, value in env.items():
        text = str(value)
        if key == "BASE_REF" and is_base(text):
            resolved[key] = base
        elif key in TOKENS and text.strip() == "${{ github.token }}":
            continue
        elif EXPRESSION.search(text):
            raise ValueError(f"step {name!r}: unsupported expression in env {key}")
        else:
            resolved[key] = text
    return resolved


def workflow_steps(workflow: quick_workflow.Workflow, base: str) -> list[Step]:
    """Quick's `run:` steps that preflight runs, in workflow order."""
    steps = []
    for entry in quick_workflow.steps(workflow):
        name = entry.get("name", "")
        if "run" not in entry or name in SKIPPED:
            continue
        if "if" in entry:
            raise ValueError(f"step {name!r}: conditional steps need a decision")
        if EXPRESSION.search(entry["run"]):
            raise ValueError(f"step {name!r}: unsupported expression in run")
        steps.append(
            Step(
                name,
                [*SHELL, entry["run"]],
                step_env(name, entry.get("env", {}), base),
            )
        )
    return steps


def commit_step(base: str, head: str, hooks: Path) -> Step:
    return Step(
        COMMITS,
        ["sh", str(hooks / "hooks/check-commits.sh"), f"{base}..{head}"],
        {
            "GIT_DIR": git(migration.ROOT, "rev-parse", "--absolute-git-dir"),
            "HOOKS_DIR": str(hooks / "hooks"),
        },
    )


def fast_tier(base: str, head: str, inert: bool, hooks: Path) -> list[Step]:
    """The pre-push hook's seconds-long checks; a docs-only change needs only the first."""
    steps = [commit_step(base, head, hooks)]
    if not inert:
        steps.append(Step(RUST_FORMAT, ["cargo", "fmt", "--all", "--", "--check"]))
        steps += [Step(line, [*SHELL, line]) for line in quick_workflow.ruff_lines()]
    return steps


def environment() -> dict[str, str]:
    """The driver's Cargo and dependency environment, with uv's install folder."""
    env = migration.environment()
    local_bin = Path.home() / ".local/bin"
    if not shutil.which("uvx", path=env.get("PATH")) and local_bin.is_dir():
        env["PATH"] = f"{env.get('PATH', '')}{os.pathsep}{local_bin}"
    return env


class Outcome(NamedTuple):
    """A step's verdict and seconds; a stopped step has neither verdict nor length."""

    passed: bool
    seconds: float
    stopped: bool = False


def run(step: Step, env: dict[str, str], log: Path) -> bool:
    return timed(step, env, log).passed


def timed(step: Step, env: dict[str, str], log: Path) -> Outcome:
    """Run one step into its log; print its start, and its result in one block.

    A step that exits non-zero while the run is stopping was stopped, and is
    printed and counted so; one that exits zero then passed."""
    with PRINTING:
        print(f"--> {step.name}", flush=True)
    started = time.monotonic()
    with log.open("w") as output:
        process = launch(step, env, output)
        try:
            # A stop signal held back during the launch arrives here.
            unblock()
            code = process.wait()
        except BaseException:
            stop([process])
            raise
        finally:
            with PRINTING:
                ACTIVE.discard(process)
    elapsed = time.monotonic() - started
    if code and STOPPING.is_set():
        with PRINTING:
            print(f"stopped {elapsed:.0f}s {step.name}", flush=True)
        return Outcome(False, elapsed, stopped=True)
    lines = []
    if code:
        tail = log.read_text(errors="replace").splitlines()[-TAIL_LINES:]
        lines += [f"    {line}" for line in tail]
    lines.append(f"{'FAIL' if code else 'pass'} {elapsed:.0f}s {step.name}")
    if code:
        lines.append(f"    exit {code}; full output: {log}")
    with PRINTING:
        print("\n".join(lines), flush=True)
    return Outcome(code == 0, elapsed)


def launch(step: Step, env: dict[str, str], output: IO[str]) -> subprocess.Popen[bytes]:
    """Start a step in its own process group, so stop() reaches its children,
    and register it before any stop can miss it.

    On the main thread the stop signals wait until the caller unblocks them; on
    a worker, a stop that began during the launch stops the step at once.
    """
    if on_main_thread():
        signal.pthread_sigmask(signal.SIG_BLOCK, STOPS_AND_INT)
    try:
        process = subprocess.Popen(
            step.command,
            cwd=migration.ROOT,
            env=env | step.env,
            stdout=output,
            stderr=subprocess.STDOUT,
            start_new_session=True,
        )
    except BaseException:
        unblock()
        raise
    with PRINTING:
        ACTIVE.add(process)
        stopping = STOPPING.is_set() and not on_main_thread()
    if stopping:
        stop([process])
    return process


def on_main_thread() -> bool:
    return threading.current_thread() is threading.main_thread()


def unblock() -> None:
    if on_main_thread():
        signal.pthread_sigmask(signal.SIG_UNBLOCK, STOPS_AND_INT)


def signal_group(process: subprocess.Popen[bytes], number: int) -> None:
    # Process group 0 or below would be preflight's own, or every process.
    if process.pid <= 0:
        return
    try:
        os.killpg(process.pid, number)
    except ProcessLookupError:
        pass


def descendants(pid: int) -> list[int]:
    """Every process below pid, read from /proc: a step's tests can start
    processes in sessions of their own, which its process group misses."""
    children: dict[int, list[int]] = {}
    for entry in Path("/proc").glob("[0-9]*"):
        try:
            parent = int((entry / "stat").read_text().rsplit(")", 1)[1].split()[1])
        except (OSError, IndexError, ValueError):
            continue
        children.setdefault(parent, []).append(int(entry.name))
    found, unread = [], [pid]
    while unread:
        below = children.get(unread.pop(), [])
        found += below
        unread += below
    return found


def signal_each(pids: list[int], number: int) -> None:
    for pid in pids:
        try:
            os.kill(pid, number)
        except ProcessLookupError:
            pass


def stop(processes: list[subprocess.Popen[bytes]]) -> None:
    """Terminate the given steps' process groups and every process below them,
    and reap the steps, killing stragglers."""
    below = [pid for p in processes if p.pid > 0 for pid in descendants(p.pid)]
    for process in processes:
        signal_group(process, signal.SIGTERM)
    signal_each(below, signal.SIGTERM)
    deadline = time.monotonic() + STOP_SECONDS
    for process in processes:
        try:
            process.wait(max(0.0, deadline - time.monotonic()))
        except subprocess.TimeoutExpired:
            pass
        signal_group(process, signal.SIGKILL)
        process.wait()
    signal_each(below, signal.SIGKILL)


def timings_path() -> Path:
    return migration.COMMON_LOCAL / TIMINGS


def last_timings() -> dict[str, float]:
    """The last recorded seconds per step name; none when the file is unreadable."""
    try:
        recorded = json.loads(timings_path().read_text())
    except (OSError, ValueError):
        return {}
    return recorded if isinstance(recorded, dict) else {}


def save_timings(seconds: dict[str, float]) -> None:
    """Merge this run's seconds into the shared timings, under its lock."""
    path = timings_path()
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        with file_lock(path.with_name(f"{path.name}.lock"), label="preflight timings"):
            temporary = path.with_name(f"{path.name}.{os.getpid()}")
            merged = last_timings() | seconds
            temporary.write_text(json.dumps(merged, indent=1) + "\n")
            os.replace(temporary, path)
    except OSError as error:
        print(f"timings not saved: {error}", flush=True)


class Run(TypedDict):
    """One full run's line in the shared run log."""

    at: str
    head: str
    jobs: int
    seconds: int
    failed: list[str]


def record_run(head: str, jobs: int, seconds: float, failed: list[str]) -> None:
    entry = Run(
        at=time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        head=head,
        jobs=jobs,
        seconds=round(seconds),
        failed=failed,
    )
    path = migration.COMMON_LOCAL / RUNS
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        with file_lock(path.with_name(f"{path.name}.lock"), label="preflight runs"):
            with path.open("ab") as log:
                # A line an interrupted writer left unfinished ends here.
                if log.tell() and not path.read_bytes().endswith(b"\n"):
                    log.write(b"\n")
                log.write(json.dumps(entry).encode() + b"\n")
    except OSError as error:
        print(f"run not logged: {error}", flush=True)


def utc(text: str) -> datetime.datetime:
    """An ISO date or time, read as UTC when it names no zone."""
    when = datetime.datetime.fromisoformat(text)
    if when.tzinfo is None:
        when = when.replace(tzinfo=datetime.UTC)
    return when


def intact(entry: object) -> bool:
    """Whether a decoded line has every field of a Run, each of its type."""
    if not isinstance(entry, dict) or set(entry) != set(Run.__annotations__):
        return False
    try:
        utc(entry["at"])
    except (TypeError, ValueError):
        return False
    return (
        isinstance(entry["head"], str)
        and all(isinstance(entry[key], int) for key in ("jobs", "seconds"))
        and isinstance(entry["failed"], list)
        and all(isinstance(name, str) for name in entry["failed"])
    )


def read_runs(path: Path) -> tuple[list[Run], int]:
    """The run log's intact entries, and how many lines could not be read."""
    try:
        lines = path.read_bytes().splitlines()
    except FileNotFoundError:
        return [], 0
    runs, unreadable = [], 0
    for line in lines:
        try:
            entry = json.loads(line.decode())
        except ValueError:  # UnicodeDecodeError and JSONDecodeError
            entry = None
        if intact(entry):
            runs.append(entry)
        else:
            unreadable += 1
    return runs, unreadable


def report_runs(since: str) -> int:
    """Full runs on this host since a UTC date or time: count and minutes."""
    try:
        cutoff = utc(since)
    except ValueError:
        print(f"--runs: {since!r} is not an ISO date or time", file=sys.stderr)
        return 2
    try:
        entries, unreadable = read_runs(migration.COMMON_LOCAL / RUNS)
    except OSError as error:
        print(f"--runs: {error}", file=sys.stderr)
        return 1
    if unreadable:
        print(f"skipped {unreadable} unreadable lines in {RUNS}")
    runs = [entry for entry in entries if utc(entry["at"]) >= cutoff]
    if not runs:
        print(f"no full runs since {since}")
        return 0
    seconds = sorted(entry["seconds"] for entry in runs)
    failed = sum(bool(entry["failed"]) for entry in runs)
    heads = len({entry["head"] for entry in runs})
    print(
        f"{len(runs)} full runs since {since} on {heads} heads, {failed} failed; "
        f"median {statistics.median(seconds):.0f}s, longest {seconds[-1]}s, "
        f"{sum(seconds) / 60:.0f} minutes in all"
    )
    return 0


class Verdicts(NamedTuple):
    """The failed steps, and the steps a failure of the Rust checks stopped."""

    failed: list[str]
    stopped: list[str]


def halt(pool: ThreadPoolExecutor) -> None:
    """Cancel the queued steps and terminate the running ones."""
    with PRINTING:
        STOPPING.set()
        running = list(ACTIVE)
    pool.shutdown(wait=False, cancel_futures=True)
    stop(running)
    # A step still launching stops itself on STOPPING; wait for it.
    pool.shutdown()


def run_steps(
    steps: list[Step],
    env: dict[str, str],
    logs: Path,
    jobs: int,
    keep_going: bool = False,
) -> Verdicts:
    """Run every step, jobs at a time, the Rust checks and then the longest
    first; the failed and stopped names in step order.

    A verdict belongs to the step's position, not its name, which need not be
    unique. Each step keeps its workflow number for its log. A step with no
    recorded time starts first, since it may be the longest. On an interrupt,
    queued steps are cancelled and running ones terminated. So they are when
    the Rust checks fail, unless keep_going: their worker marks the run as
    stopping before it takes another step, so no queued step starts after.
    """
    last = last_timings()
    order = sorted(
        range(len(steps)),
        key=lambda i: (steps[i].name != RUST_CHECKS, -last.get(steps[i].name, INF)),
    )
    gate = bool(steps) and steps[order[0]].name == RUST_CHECKS and not keep_going

    def job(index: int) -> Outcome | None:
        if STOPPING.is_set():
            return None
        outcome = timed(steps[index], env, logs / f"{index + 1:02}.log")
        if gate and index == order[0] and not outcome.passed:
            with PRINTING:
                STOPPING.set()
        return outcome

    STOPPING.clear()
    pool = ThreadPoolExecutor(max(1, jobs))
    try:
        futures = {index: pool.submit(job, index) for index in order}
        rust = futures[order[0]].result() if gate else None
        if rust and not rust.passed:
            halt(pool)
        outcomes = {
            index: None if future.cancelled() else future.result()
            for index, future in futures.items()
        }
    except BaseException:
        halt(pool)
        raise
    finally:
        STOPPING.clear()
    pool.shutdown()
    ended = {i: o for i, o in outcomes.items() if o is not None and not o.stopped}
    # A stopped step's time is not its length, so its last full time stays.
    save_timings({steps[i].name: o.seconds for i, o in ended.items()})
    return Verdicts(
        [steps[i].name for i in sorted(ended) if not ended[i].passed],
        [step.name for i, step in enumerate(steps) if i not in ended],
    )


def selected(names: list[str], patterns: list[str] | None) -> list[str]:
    if not patterns:
        return names
    for pattern in patterns:
        if not any(pattern.lower() in name.lower() for name in names):
            raise ValueError(f"no step matches {pattern!r}")
    return [name for name in names if any(p.lower() in name.lower() for p in patterns)]


def arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", default="origin/rust-migration")
    parser.add_argument("--head", default="HEAD")
    parser.add_argument(
        "--hooks", type=Path, help="existing clean pinned hooks checkout (offline)"
    )
    narrow = parser.add_mutually_exclusive_group()
    narrow.add_argument(
        "--step",
        action="append",
        help="run only steps whose name contains this text (repeatable)",
    )
    narrow.add_argument(
        "--fast",
        action="store_true",
        help=(
            "run only the pre-push fast tier and Quick's ty line, in seconds, as "
            "a check before a commit; records no stamp"
        ),
    )
    parser.add_argument(
        "--jobs",
        type=int,
        default=JOBS,
        help=f"steps run at once (default {JOBS}); 1 runs them in workflow order",
    )
    parser.add_argument(
        "--keep-going",
        action="store_true",
        help=f"run every step after {RUST_CHECKS!r} fails, instead of stopping",
    )
    parser.add_argument("--list", action="store_true", help="list the steps and exit")
    parser.add_argument(
        "--runs",
        metavar="SINCE",
        help="count this host's full runs since a UTC date or time, and exit",
    )
    return parser.parse_args()


def revision(name: str) -> str:
    return git(
        migration.ROOT,
        "rev-parse",
        "--verify",
        "--end-of-options",
        f"{name}^{{commit}}",
    )


def bases(options: argparse.Namespace) -> tuple[str, str]:
    """The merge base of --base and --head, and --head, as commits.

    Preflight compares two trees, so against a base tip that has moved since
    the branch forked it would read each newer base commit as lines the branch
    removed. The merge base of an ancestor is that ancestor, so a fork point
    passed as --base is used unchanged.
    """
    tip, head = revision(options.base), revision(options.head)
    base = tip if tip == head else git(migration.ROOT, "merge-base", tip, head)
    if base != tip:
        print(f"base: {base[:10]}, the merge base of {options.base} and {options.head}")
    return base, head


def uncommitted() -> str:
    """A warning when tracked files differ from HEAD, or nothing."""
    if git(migration.ROOT, "status", "--porcelain", "--untracked-files=no"):
        return (
            "warning: tracked files differ from HEAD; this result describes the "
            "working tree, not the commit CI tests"
        )
    return ""


def summary(
    chosen: list[Step], verdicts: Verdicts, seconds: float, warning: str
) -> int:
    failed, stopped = verdicts
    passed = len(chosen) - len(failed) - len(stopped)
    print(f"{passed}/{len(chosen)} passed in {seconds:.0f}s")
    for name in failed:
        print(f"failed: {name}; rerun: python3 tools/preflight.py --step {name!r}")
    if stopped:
        print(
            f"stopped {len(stopped)} steps when {RUST_CHECKS!r} failed: fix it, "
            "then run preflight again; --keep-going runs every step after it fails"
        )
        for name in stopped:
            print(f"stopped: {name}")
    if warning:
        print(warning)
    return 1 if failed else 0


def stamp(options: argparse.Namespace, base: str, head: str, warning: str) -> str:
    """Record a pass stamp for a complete clean run, or say why there is none."""
    if options.step:
        return "no stamp: --step ran a subset"
    if warning:
        return "no stamp: tracked files differ from HEAD"
    if head != revision("HEAD"):
        return "no stamp: --head is not the checked-out commit"
    on_branch = subprocess.run(
        ["git", "merge-base", "--is-ancestor", base, PROTECTED],
        cwd=migration.ROOT,
        capture_output=True,
        check=False,
    )
    if on_branch.returncode:
        return f"no stamp: base {base[:10]} is not on {PROTECTED}"
    path = preflight_stamp.record(base, head)
    return f"stamp: recorded {path.name} for this tree against base {base[:10]}"


def fast(options: argparse.Namespace) -> int:
    """The fast tier on the working tree plus ty, each once, with no stamp."""
    base, head = bases(options)
    supplied = options.hooks.resolve() if options.hooks else None
    hooks = (supplied or pinned_hooks()) if options.list else hooks_checkout(supplied)
    ty = quick_workflow.ty_line()
    steps = [*fast_tier(base, head, False, hooks), Step(ty, [*SHELL, ty])]
    if options.list:
        print("\n".join(step.name for step in steps))
        return 0
    logs = migration.LOCAL / "preflight" / time.strftime("fast-%Y%m%d-%H%M%S")
    logs.mkdir(parents=True, exist_ok=True)
    env = environment()
    started = time.monotonic()
    failed = [
        step.name
        for number, step in enumerate(steps, 1)
        if not run(step, env, logs / f"{number:02}.log")
    ]
    seconds = time.monotonic() - started
    print(f"{len(steps) - len(failed)}/{len(steps)} passed in {seconds:.0f}s; no stamp")
    for name in failed:
        fix = next((fix for text, fix in FAST_FIXES if text in name), "")
        print(f"failed: {name}" + (f"; fix: {fix}" if fix else ""))
    return 1 if failed else 0


def execute(
    options: argparse.Namespace,
    chosen: list[Step],
    env: dict[str, str],
    logs: Path,
    head: str,
) -> Verdicts:
    """Run the chosen steps, in workflow order with --jobs 1; log a full run."""
    started = time.monotonic()
    if options.jobs == 1:
        verdicts = serial(chosen, env, logs, options.keep_going)
    else:
        verdicts = run_steps(chosen, env, logs, options.jobs, options.keep_going)
    if not options.step:
        record_run(head, options.jobs, time.monotonic() - started, verdicts.failed)
    return verdicts


def serial(
    chosen: list[Step], env: dict[str, str], logs: Path, keep_going: bool
) -> Verdicts:
    """Each step in workflow order; a failure of the Rust checks stops the rest."""
    failed: list[str] = []
    for number, step in enumerate(chosen, 1):
        if not run(step, env, logs / f"{number:02}.log"):
            failed.append(step.name)
            if step.name == RUST_CHECKS and not keep_going:
                return Verdicts(failed, [later.name for later in chosen[number:]])
    return Verdicts(failed, [])


def interrupt(number: int, _frame: object) -> None:
    raise KeyboardInterrupt(signal.Signals(number).name)


def main() -> int:
    options = arguments()
    if options.runs:
        return report_runs(options.runs)
    # Steps run in their own process groups, so a stop signal to preflight
    # must reach them through the same cleanup as Ctrl-C.
    previous = {number: signal.signal(number, interrupt) for number in STOPS}
    try:
        return check(options)
    except KeyboardInterrupt as stopped:
        print(f"preflight stopped ({stopped or 'SIGINT'}); no stamp", file=sys.stderr)
        return 130
    finally:
        for number, handler in previous.items():
            signal.signal(number, handler)


def check(options: argparse.Namespace) -> int:
    try:
        if options.fast:
            return fast(options)
        base, head = bases(options)
        steps = workflow_steps(yaml.safe_load(WORKFLOW.read_text()), base)
        probes = ["python3", "tools/compile-check.py", "--base", base]
        steps.append(Step(PROBES, probes))
        names = selected([COMMITS] + [step.name for step in steps], options.step)
        if options.list:
            print("\n".join(names))
            for name, reason in SKIPPED.items():
                print(f"skipped: {name} ({reason})")
            return 0
        if COMMITS in names:
            hooks = hooks_checkout(options.hooks.resolve() if options.hooks else None)
            steps.insert(0, commit_step(base, head, hooks))
        if warning := uncommitted():
            print(warning, flush=True)
        logs = migration.LOCAL / "preflight" / time.strftime("%Y%m%d-%H%M%S")
        logs.mkdir(parents=True, exist_ok=True)
        env = environment()
        started = time.monotonic()
        chosen = [step for step in steps if step.name in names]
        verdicts = execute(options, chosen, env, logs, head)
        # A step can change tracked files; the summary describes the end state too.
        warning = warning or uncommitted()
        if not verdicts.failed:
            print(stamp(options, base, head, warning), flush=True)
    except (
        OSError,
        ValueError,
        KeyError,
        TypeError,
        yaml.YAMLError,
        subprocess.CalledProcessError,
    ) as error:
        print(error, file=sys.stderr)
        return 1
    return summary(chosen, verdicts, time.monotonic() - started, warning)


if __name__ == "__main__":
    sys.exit(main())
