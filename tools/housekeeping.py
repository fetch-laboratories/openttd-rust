#!/usr/bin/env python3
"""List, and with `--apply` remove, stale worktrees and ad hoc `.local` output (#532).

A dry run prints each candidate with its size and reason; `--apply` removes exactly the
`remove` lines unless a live output handle appears before deletion. Worktrees go through `git worktree remove` without `--force`, so a dirty
or locked one is kept. Live output handles are checked again before removal. Output in a
live worktree's own `.local` stays with that worktree.

A worktree that is kept still gives up its git-ignored `build-rust` (#824) once its work is
merged or closed, or, for a T3 task's checkout, once it has been idle for `--idle-hours`. Locked, referenced, evidence
and profile worktrees, a live cwd inside, a symlinked `build-rust` and one holding tracked
or staged files keep the build. Only
that directory is deleted, never the checkout or its staged or uncommitted files.
"""

import argparse
import errno
import fcntl
import json
import os
import re
import shutil
import signal
import stat
import subprocess
import sys
import time
from collections.abc import Callable
from dataclasses import dataclass, field, replace
from datetime import UTC, datetime
from pathlib import Path

import vcs

PROC_ROOT = Path("/proc")
BASE = "origin/rust-migration"
# Per-run output folders; every other `.local` entry is kept unless listed below.
OUTPUT = ("simulation", "verification", "coverage", "deep")
KEPT = {
    "reference": "pinned reference checkout",
    "compiler-cache": "cache",
    "instruction-cache": "cache",
    "cargo": "toolchain",
    "rustup": "toolchain",
    "valgrind": "toolchain",
    "deps": "bootstrapped dependencies",
    "downloads": "bootstrap downloads",
    "tooling": "commit hooks",
    "speed-reference-profiles": "retained speed reference profiles",
    "worktrees": "worktree root",
    "ci-requests": "CI request records read by tools/ci.py wait",
}
# A run without report.json touched this recently may still be running.
RUNNING_SECONDS = 12 * 3600
# A kept worktree's build-rust is reclaimed once its activity is this old (#824).
IDLE_HOURS = 2.0
BUILD = "build-rust"
# Retained profile checkouts; their builds are evidence, not cache. Under the main
# checkout's .local/worktrees, git lists them by their resolved HDD path.
PROFILES = "root-profile-"
# Session managers hide their cwd from their own user and never run agent work.
SESSION_MANAGERS = {"systemd", "(sd-pam)"}
REFERENCE = re.compile(r"\.local/(" + "|".join(OUTPUT) + r")/([^/\"\s]+)")


@dataclass(frozen=True)
class Item:
    path: Path
    remove: bool
    reason: str
    worktree: bool = False
    branch: str | None = None  # local branch deleted after its worktree
    unlock: bool = False  # stale lock released by --apply before removal
    prune: bool = False  # missing folder; --apply prunes its record
    completed: bool = False  # pushed completion bypasses only the activity fallback
    owner: Path | None = None  # kept worktree whose build-rust this item reclaims
    idle_seconds: float | None = None  # idle reclaim rechecks activity at removal
    identity: tuple[int, int] | None = field(init=False, default=None)

    def __post_init__(self) -> None:
        try:
            info = self.path.lstat()
            object.__setattr__(self, "identity", (info.st_dev, info.st_ino))
        except OSError:
            pass


@dataclass(frozen=True)
class Worktree:
    path: Path
    head: str
    branch: str | None
    locked: bool
    prunable: bool
    lock_note: str = ""


PrState = Callable[[Path, str], str | None]
IssueState = Callable[[Path, int], str]
# Issue and PR numbers a lock note names: #123 or an issues/pull URL.
NAMED = re.compile(r"(?:#|/(?:issues|pull)/)(\d+)\b")


def git(repo: Path, *arguments: str) -> str:
    return vcs.git(*arguments, cwd=repo)


def gh_pr_state(repo: Path, branch: str) -> str | None:
    """OPEN, MERGED or CLOSED across every PR from BRANCH, in that precedence."""
    command = ["gh", "pr", "list", "--state", "all", "--head", branch]
    command += ["--json", "state", "--jq", ".[].state"]
    states = vcs.gh(*command[1:], cwd=repo).split()
    return next((s for s in ("OPEN", "MERGED", "CLOSED") if s in states), None)


def gh_issue_state(repo: Path, number: int) -> str:
    """OPEN or CLOSED for an issue or PR; a merged PR is closed."""
    command = ["gh", "api", f"repos/{{owner}}/{{repo}}/issues/{number}"]
    return vcs.gh(*command[1:], "--jq", ".state", cwd=repo)


def stale_lock(main: Path, note: str, issue_state: IssueState) -> str | None:
    """The closed numbers when every issue or PR the note names is closed."""
    named = sorted(set(NAMED.findall(note)), key=int)
    states = [issue_state(main, int(number)).strip().upper() for number in named]
    if not named or "OPEN" in states:
        return None
    return ", ".join(f"#{number}" for number in named)


def lock_age(tree: Path) -> str:
    try:
        admin = Path(git(tree, "rev-parse", "--absolute-git-dir").strip())
        days = (time.time() - (admin / "locked").stat().st_mtime) / 86400
    except (OSError, subprocess.CalledProcessError):
        return "age unknown"
    return f"{days:.0f} days old"


def on_origin(main: Path, head: str) -> bool:
    refs = git(main, "for-each-ref", "--contains", head, "refs/remotes/origin")
    return bool(refs.strip())


def list_worktrees(repo: Path) -> list[Worktree]:
    found = []
    for block in git(repo, "worktree", "list", "--porcelain").strip().split("\n\n"):
        fields = dict(line.partition(" ")[::2] for line in block.splitlines())
        branch = fields.get("branch", "").removeprefix("refs/heads/") or None
        found.append(
            Worktree(
                Path(fields["worktree"]),
                fields.get("HEAD", ""),
                branch,
                "locked" in fields,
                "prunable" in fields,
                fields.get("locked", ""),
            )
        )
    return found


def origin_head(main: Path, branch: str) -> str:
    return subprocess.run(
        ["git", "-C", str(main), "rev-parse", "--verify", "--quiet"]
        + [f"refs/remotes/origin/{branch}"],
        capture_output=True,
        text=True,
    ).stdout.strip()


def plan_worktrees(
    main: Path,
    entries: list[Worktree],
    pr_state: PrState,
    references: str = "",
    issue_state: IssueState = gh_issue_state,
) -> list[Item]:
    # A head on the base's first-parent chain is a fresh branch, not merged work.
    first_parent = set(git(main, "rev-list", "--first-parent", BASE).split())
    items = []
    for tree in entries:
        if tree.path.is_relative_to(main / ".local/reference"):
            continue
        if tree.prunable or not tree.path.exists():
            reason = (
                f"folder missing; {'locked' if tree.locked else 'prune its record'}"
            )
            items.append(Item(tree.path, not tree.locked, reason, True, prune=True))
            continue
        task = ".t3/worktrees/" in tree.path.as_posix()
        if tree.branch is None and not task:
            items.append(Item(tree.path, False, "detached HEAD", True))
            continue
        state = pr_state(main, tree.branch) if tree.branch else None
        ancestor = subprocess.run(
            ["git", "-C", str(main), "merge-base", "--is-ancestor", tree.head, BASE]
        )
        merged = tree.head not in first_parent and ancestor.returncode == 0
        branch: str | None = tree.branch
        if state == "OPEN":
            items.append(Item(tree.path, False, "open PR", True))
            continue
        if merged:
            reason = f"merged into {BASE}"
        elif state == "MERGED":
            reason, branch = "PR merged; local branch has unmerged commits, kept", None
        elif (
            state == "CLOSED"
            and tree.branch
            and origin_head(main, tree.branch) == tree.head
        ):
            reason = "PR closed; origin keeps the branch"
        elif state == "CLOSED":
            reason, branch = "PR closed; origin lacks this head, branch kept", None
        else:
            fresh = tree.head in first_parent
            if task and fresh:
                reason = "finished T3 task; no commits beyond base"
                # A base-chain head holds no unpublished commits to delete.
            elif not fresh and on_origin(main, tree.head):
                # Review and update checkouts push to another branch's PR.
                reason = "no PR; head pushed to origin"
                if origin_head(main, str(tree.branch)) != tree.head:
                    branch = None
            else:
                reason = (
                    "no PR; branch at a base commit" if fresh else "no PR; unmerged"
                )
                items.append(Item(tree.path, False, reason, True))
                continue
        unlock = False
        if tree.locked:
            if closed := stale_lock(main, tree.lock_note, issue_state):
                reason, unlock = f"{reason}; stale lock: {closed} closed", True
            else:
                hold = f"locked ({tree.lock_note or 'no note'}; {lock_age(tree.path)})"
                items.append(Item(tree.path, False, f"{reason}, but {hold}", True))
                continue
        if git(tree.path, "status", "--porcelain").strip():
            dirty = f"{reason}, but dirty"
            items.append(Item(tree.path, False, dirty, True, None, unlock))
        else:
            completed = merged or (
                state in ("MERGED", "CLOSED") and on_origin(main, tree.head)
            )
            refusal = worktree_refusal(main, tree.path, references, completed)
            remove = not refusal
            items.append(
                Item(
                    tree.path,
                    remove,
                    refusal or reason,
                    True,
                    branch,
                    unlock,
                    completed=completed,
                )
            )
    return protect_outputs(items)


def with_builds(
    main: Path,
    items: list[Item],
    pr_state: PrState,
    references: str,
    idle_hours: float,
) -> list[Item]:
    """Follow each kept worktree with its build-rust row (#824)."""
    trees = {tree.path: tree for tree in list_worktrees(main)[1:]}
    kept = [
        trees[item.path]
        for item in items
        if item.worktree and not item.remove and item.path in trees
    ]
    builds = [plan_build(main, tree, pr_state, references, idle_hours) for tree in kept]
    protected = iter(protect_outputs([build for build in builds if build]))
    owners = {tree.path for tree, build in zip(kept, builds, strict=True) if build}
    result = []
    for item in items:
        result.append(item)
        if item.worktree and not item.remove and item.path in owners:
            result.append(next(protected))
    return result


def plan_build(
    main: Path, tree: Worktree, pr_state: PrState, references: str, idle_hours: float
) -> Item | None:
    """A kept worktree's build-rust: reclaimed when idle or its work is finished."""
    build = tree.path / BUILD
    if not build.is_dir() and not build.is_symlink():
        return None
    label = "build-rust of kept worktree"
    if exclusion := build_exclusion(main, tree, references):
        return Item(build, False, f"{label}: {exclusion}")
    newest = latest_activity(tree.path)
    shown = "unknown" if newest is None else stamp(newest)
    idle = 0.0 if newest is None else time.time() - newest
    task = ".t3/worktrees/" in tree.path.as_posix()
    # Ruling B (#824): only a T3 task's checkout is reclaimed for idleness.
    if task and newest is not None and idle >= idle_hours * 3600:
        why = f"idle {idle / 3600:.0f}h; newest activity {shown}"
        seconds = idle_hours * 3600
        return Item(
            build, True, f"{label}: {why}", owner=tree.path, idle_seconds=seconds
        )
    if work_done(main, tree, pr_state):
        why = f"merged/closed; newest activity {shown}"
        return Item(build, True, f"{label}: {why}", owner=tree.path)
    if not task:
        return Item(build, False, f"{label}: not a T3 task; work not merged or closed")
    return Item(build, False, f"{label}: active within {idle_hours:g}h ({shown})")


def build_exclusion(main: Path, tree: Worktree, references: str) -> str | None:
    """Builds that stay with their worktree: links, evidence, profiles, locks, live cwd."""
    checks: list[tuple[Callable[[], bool], str]] = [
        ((tree.path / BUILD).is_symlink, "symlinked"),
        (
            lambda: tree.path.is_relative_to(main / ".local/reference"),
            "pinned reference",
        ),
        (lambda: tree.path.name.startswith(PROFILES), "retained profile"),
        (lambda: tree.locked, "locked"),
    ]
    found = next((reason for check, reason in checks if check()), None)
    return (
        found
        or indexed_refusal(tree.path)
        or retained_refusal(main, tree.path, references)
        or cwd_refusal(tree.path)
    )


def indexed_refusal(tree: Path) -> str | None:
    """Ignore rules do not protect files already tracked or staged under build-rust."""
    try:
        listed = git(tree, "ls-files", "--cached", "-z", "--", BUILD)
    except subprocess.CalledProcessError as error:
        return f"cannot inspect index: {error}"
    return "tracked or staged files under build-rust" if listed.strip("\0") else None


def work_done(main: Path, tree: Worktree, pr_state: PrState) -> bool:
    """Merged into the base off its first-parent chain, or its branch's PR merged or closed."""
    if tree.branch and pr_state(main, tree.branch) in ("MERGED", "CLOSED"):
        return True
    first_parent = set(git(main, "rev-list", "--first-parent", BASE).split())
    ancestor = subprocess.run(
        ["git", "-C", str(main), "merge-base", "--is-ancestor", tree.head, BASE]
    )
    return tree.head not in first_parent and ancestor.returncode == 0


def stamp(when: float) -> str:
    return datetime.fromtimestamp(when, UTC).strftime("%Y-%m-%d %H:%M UTC")


def latest_activity(tree: Path) -> float | None:
    """Newest of the worktree's activity marks, every top-level file, tracked or not
    (a task's logs and scratch files), and every entry under its build-rust."""
    newest = activity(tree)
    for entry in tree.iterdir() if tree.is_dir() else []:
        try:
            info = entry.lstat()
        except OSError:
            continue
        if not stat.S_ISDIR(info.st_mode):
            newest = info.st_mtime if newest is None else max(newest, info.st_mtime)
    for folder, directories, files in os.walk(tree / BUILD):
        for name in directories + files:
            try:
                when = os.lstat(os.path.join(folder, name)).st_mtime
            except OSError:
                continue
            newest = when if newest is None else max(newest, when)
    return newest


def worktree_references(main: Path) -> str:
    """Open issues include PRs; paginate so no retained task is silently omitted."""
    bodies = subprocess.check_output(
        [
            "gh",
            "api",
            "--paginate",
            "repos/{owner}/{repo}/issues?state=open&per_page=100",
            "--jq",
            ".[].body",
        ],
        cwd=main,
        text=True,
    )
    roadmap = main / "docs/roadmap.md"
    # Keep all explicit roadmap references, including shortened T3 names. This
    # conservatively includes the resume checkpoint without depending on its title.
    return bodies + (roadmap.read_text() if roadmap.exists() else "")


def activity(tree: Path) -> float | None:
    """Newest git or top-level file write; Codex delegates hold no cwd between commands."""
    try:
        admin = tree / (tree / ".git").read_text().removeprefix("gitdir:").strip()
        marks = [admin / "HEAD", admin / "index", admin / "logs/HEAD"]
        marks += [
            tree / name
            for name in git(tree, "ls-files", "-z").split("\0")
            if name and "/" not in name
        ]
        return max((m.stat().st_mtime for m in marks if m.exists()), default=None)
    except OSError:
        return None


def recently_active(tree: Path) -> bool:
    newest = activity(tree)
    return newest is not None and time.time() - newest < RUNNING_SECONDS


def worktree_refusal(
    main: Path, path: Path, references: str, completed: bool = False
) -> str | None:
    if refusal := retained_refusal(main, path, references) or cwd_refusal(path):
        return refusal
    if not completed and ".t3/worktrees/" in path.as_posix() and recently_active(path):
        return f"T3 task active within {RUNNING_SECONDS // 3600} hours"
    return None


def retained_refusal(main: Path, path: Path, references: str) -> str | None:
    """Roadmap or issue references and retained evidence keep a worktree whole."""
    if path.name in references or str(path) in references:
        return "referenced by roadmap or open issue/PR"
    local = main / ".local"
    deep = max(local.glob("deep/*/report.json"), default=None)
    evidence = deep.read_text() if deep else ""
    anchors = main / "tools/simulation/instructions.py"
    if anchors.exists():
        evidence += anchors.read_text()
    if (path / ".local").exists() and (str(path) in evidence or path.name in evidence):
        return "output named by current deep report or speed anchors"
    if any((path / ".local").rglob("provenance.json")):
        return "pinned runtime (provenance.json)"
    return None


def cwd_refusal(path: Path) -> str | None:
    try:
        for process in PROC_ROOT.iterdir():
            if not process.name.isdecimal():
                continue
            try:
                # Another user's process (init, system daemons) hides its cwd.
                if process.stat().st_uid != os.geteuid():
                    continue
                cwd = (process / "cwd").resolve(strict=True)
                if cwd == path.resolve() or cwd.is_relative_to(path.resolve()):
                    return "live process cwd inside worktree"
            except OSError as error:
                if error.errno in (errno.ENOENT, errno.ESRCH):
                    continue
                # A non-dumpable process (`(sd-pam)`) hides its cwd from its own user.
                start, creation = started(process), created(path)
                if isinstance(error, PermissionError) and command(process) in (
                    SESSION_MANAGERS
                ):
                    continue
                if (
                    isinstance(error, PermissionError)
                    and start is not None
                    and creation is not None
                    and start < creation
                ):
                    continue
                return f"cannot inspect process cwd: {error}"
    except OSError as error:
        return f"cannot inspect process cwd: {error}"
    return None


def command(process: Path) -> str | None:
    try:
        return (process / "comm").read_text().strip()
    except OSError:
        return None


def started(process: Path) -> float | None:
    """Upper bound on start time in epoch seconds; unknown prevents removal."""
    try:
        boot = re.search(r"^btime (\d+)$", (PROC_ROOT / "stat").read_text(), re.M)
        # Field 22 of /proc/PID/stat; the command name may contain spaces and parentheses.
        ticks = int((process / "stat").read_text().rsplit(")", 1)[1].split()[19])
        frequency = os.sysconf("SC_CLK_TCK")
        if boot is None or ticks < 0 or frequency <= 0:
            return None
        # btime truncates seconds and starttime truncates clock ticks.
        return int(boot[1]) + 1 + (ticks + 1) / frequency
    except (OSError, IndexError, ValueError):
        return None


def created(tree: Path) -> float | None:
    """`git worktree add` writes the admin directory's commondir once."""
    try:
        admin = (tree / ".git").read_text().removeprefix("gitdir:").strip()
        return (tree / admin / "commondir").stat().st_mtime
    except OSError:
        return None


@dataclass(frozen=True)
class OutputHandle:
    path: Path
    device: int
    inode: int


def open_outputs() -> tuple[list[OutputHandle], str | None]:
    """Snapshot writable regular-file FDs; an unreadable live process prevents removal."""
    handles: list[OutputHandle] = []
    try:
        for process in PROC_ROOT.iterdir():
            if not process.name.isdecimal():
                continue
            try:
                if process.stat().st_uid != os.geteuid():
                    continue
                for fd in (process / "fd").iterdir():
                    try:
                        info = fd.stat()
                        if not stat.S_ISREG(info.st_mode):
                            continue
                        fields = dict(
                            line.split(":", 1)
                            for line in (process / "fdinfo" / fd.name)
                            .read_text()
                            .splitlines()
                        )
                        if int(fields["flags"], 8) & os.O_ACCMODE == os.O_RDONLY:
                            continue
                        handles.append(
                            OutputHandle(fd.resolve(), info.st_dev, info.st_ino)
                        )
                    except OSError as error:
                        if error.errno not in (errno.ENOENT, errno.ESRCH):
                            raise
            except OSError as error:
                if error.errno not in (errno.ENOENT, errno.ESRCH):
                    raise
    except OSError as error:
        return handles, f"cannot inspect live output handles: {error}"
    return handles, None


def output_info(path: Path) -> os.stat_result:
    """A dangling runtime link has no target whose open writers need protection."""
    try:
        return path.stat()
    except FileNotFoundError:
        info = path.lstat()
        if not stat.S_ISLNK(info.st_mode):
            raise
        return info


def lease_refusal(paths: list[Path]) -> str | None:
    """Check for writers through the kernel when proc hides a process's descriptors."""
    for path in paths:
        try:
            if not stat.S_ISREG(output_info(path).st_mode):
                continue
            descriptor = os.open(path, os.O_RDONLY | os.O_NONBLOCK | os.O_CLOEXEC)
            try:
                # Lease breaks use the normally ignored SIGURG instead of fatal SIGIO.
                fcntl.fcntl(descriptor, fcntl.F_SETOWN, os.getpid())
                fcntl.fcntl(descriptor, fcntl.F_SETSIG, signal.SIGURG)
                fcntl.fcntl(descriptor, fcntl.F_SETLEASE, fcntl.F_RDLCK)
            finally:
                os.close(descriptor)
        except OSError as error:
            if error.errno == errno.EAGAIN:
                return "live open output handle"
            return f"cannot inspect output candidate: {error}"
    return None


def output_refusal(
    path: Path, handles: list[OutputHandle], error: str | None
) -> str | None:
    target = path.resolve()
    if any(
        handle.path == target or handle.path.is_relative_to(target)
        for handle in handles
    ):
        return "live open output handle"
    identities = {(handle.device, handle.inode) for handle in handles}
    try:
        paths = [path]
        if path.is_dir() and not path.is_symlink():

            def unreadable(inspection_error: OSError) -> None:
                raise inspection_error

            for folder, directories, files in os.walk(path, onerror=unreadable):
                paths.extend(Path(folder) / name for name in directories + files)
        if any(
            (info.st_dev, info.st_ino) in identities
            for info in (output_info(p) for p in paths)
        ):
            return "live open output handle"
    except OSError as inspection_error:
        return f"cannot inspect output candidate: {inspection_error}"
    if error is not None:
        return lease_refusal(paths)
    return None


def protect_outputs(items: list[Item]) -> list[Item]:
    handles, error = open_outputs()
    return [
        replace(item, remove=False, reason=reason)
        if item.remove
        and not item.prune
        and (reason := output_refusal(item.path, handles, error))
        else item
        for item in items
    ]


def touched(path: Path) -> float:
    children = path.iterdir() if path.is_dir() else []
    return max([path.stat().st_mtime] + [c.lstat().st_mtime for c in children])


def finished(main: Path, report: Path) -> tuple[bool, str]:
    """A finished run is removable once the commit its report names is integrated."""
    try:
        value = json.loads(report.read_text())
    except (OSError, UnicodeDecodeError, json.JSONDecodeError):
        value = None
    # Simulation and verification reports name candidate_commit; deep reports, commit.
    fields = value if isinstance(value, dict) else {}
    commit = fields.get("candidate_commit", fields.get("commit"))
    if not isinstance(commit, str) or not commit:
        return False, "unclassified run output"
    ancestor = subprocess.run(
        ["git", "-C", str(main), "merge-base", "--is-ancestor", commit, BASE],
        capture_output=True,
    )
    if ancestor.returncode:
        return False, f"candidate {commit[:12]} not integrated; may be open-PR evidence"
    return True, f"finished run of integrated {commit[:12]}"


def plan_local(local: Path, now: float, references: str = "") -> list[Item]:
    keep: dict[Path, str] = {}
    deep = max((r.parent for r in local.glob("deep/*/report.json")), default=None)
    if deep is not None:
        keep[deep] = "current deep report"
        for folder, name in REFERENCE.findall((deep / "report.json").read_text()):
            keep.setdefault(local / folder / name, "referenced by the deep report")
    profiles: dict[Path, Path] = {}
    simulation = local / "simulation"
    for out in sorted(simulation.glob("*/**/candidate/callgrind.out")):
        relative = out.relative_to(simulation)
        profiles[Path(*relative.parts[1:])] = simulation / relative.parts[0]
    for run in profiles.values():
        keep.setdefault(run, "current candidate profile")
    items = []
    for entry in sorted(local.iterdir()) if local.is_dir() else []:
        name = entry.name
        if name in OUTPUT and entry.is_dir():
            for run in sorted(entry.iterdir()):
                # Bare names cover .local, HDD-target, inner-file and listed forms.
                if re.search(
                    r"(?<![\w.-])" + re.escape(run.name) + r"(?![\w.-])", references
                ):
                    keep.setdefault(run, "referenced by roadmap or open issue/PR")
                if run in keep:
                    items.append(Item(run, False, keep[run]))
                elif (run / "report.json").exists():
                    items.append(
                        Item(run, *finished(local.parent, run / "report.json"))
                    )
                elif now - touched(run) < RUNNING_SECONDS:
                    items.append(Item(run, False, "no report.json; may be running"))
                else:
                    items.append(
                        Item(
                            run,
                            True,
                            f"no report.json; idle {RUNNING_SECONDS // 3600} hours",
                        )
                    )
        elif name in KEPT:
            items.append(Item(entry, False, KEPT[name]))
        elif name.startswith("build-"):
            items.append(Item(entry, False, "shared build"))
        elif name.endswith((".lock", ".gate", ".holders")):
            items.append(Item(entry, False, "lock"))
        elif (entry / "provenance.json").exists():
            items.append(Item(entry, False, "pinned runtime (provenance.json)"))
        elif name == "comparison-logs" or (entry.is_file() and name.endswith(".log")):
            items.append(Item(entry, True, "ad hoc log"))
        else:
            items.append(Item(entry, False, "unknown; remove by hand if ad hoc output"))
    return protect_outputs(items)


def plan(
    repo: Path,
    pr_state: PrState,
    now: float,
    references: str = "",
    issue_state: IssueState = gh_issue_state,
) -> list[Item]:
    main, *linked = list_worktrees(repo)
    trees = plan_worktrees(main.path, linked, pr_state, references, issue_state)
    return trees + plan_local(main.path / ".local", now, references)


def size(path: Path) -> int:
    if not path.is_dir() or path.is_symlink():
        return path.lstat().st_size
    walk = os.walk(path)
    return sum(
        os.lstat(os.path.join(d, f)).st_size for d, _, files in walk for f in files
    )


def human(count: int) -> str:
    if count < 1024:
        return f"{count}B"
    scaled = float(count)
    for unit in "KMGT":
        scaled /= 1024
        if scaled < 1024 or unit == "T":
            break
    return f"{scaled:.1f}{unit}"


def writable(path: Path) -> None:
    """Extracted runtimes ship read-only folders that block removal."""
    for folder, subfolders, _ in os.walk(path):
        for name in subfolders:
            target = os.path.join(folder, name)
            if not os.path.islink(target):
                os.chmod(target, os.lstat(target).st_mode | stat.S_IWUSR)
    if path.is_dir() and not path.is_symlink():
        path.chmod(path.stat().st_mode | stat.S_IWUSR)


def remove(main: Path, item: Item, references: str = "") -> str | None:
    """Remove ITEM; return the refusal when git keeps it."""
    if item.prune:
        git(main, "worktree", "prune")
        listed = {tree.path for tree in list_worktrees(main)}
        return "record not pruned" if item.path in listed else None
    if refusal := ownership_refusal(main, item, references):
        return refusal
    handles, error = open_outputs()
    if refusal := output_refusal(item.path, handles, error):
        return refusal
    try:
        info = item.path.lstat()
        if item.identity is None or item.identity != (info.st_dev, info.st_ino):
            return "output candidate identity changed since planning"
    except OSError as error:
        return f"cannot inspect output candidate: {error}"
    if not item.worktree:
        return delete_output(item)
    command = ["git", "-C", str(main), "worktree", "remove", str(item.path)]
    if git(item.path, "status", "--porcelain").strip():
        return "worktree became dirty"
    writable(item.path)
    removed = subprocess.run(command, capture_output=True, text=True)
    if removed.returncode:
        return removed.stderr.strip()
    if item.branch is not None:
        # -d compares with the local HEAD or a pruned upstream; the plan checked origin.
        git(main, "branch", "-D", item.branch)
    return None


def delete_output(item: Item) -> str | None:
    """Delete a non-worktree item; a build first rechecks after the output scans."""
    if item.owner is not None and (
        refusal := indexed_refusal(item.owner) or idle_refusal(item)
    ):
        return refusal
    writable(item.path)
    if item.path.is_dir() and not item.path.is_symlink():
        shutil.rmtree(item.path)
    else:
        item.path.unlink()
    return None


def idle_refusal(item: Item) -> str | None:
    if item.idle_seconds is None or item.owner is None:
        return None
    newest = latest_activity(item.owner)
    if newest is None or time.time() - newest < item.idle_seconds:
        return "activity since planning"
    return None


def ownership_refusal(main: Path, item: Item, references: str) -> str | None:
    """Recheck a worktree, or a kept worktree's build, just before deletion."""
    if item.worktree:
        return worktree_refusal(main, item.path, references, item.completed)
    return build_refusal(main, item, references) if item.owner else None


def build_refusal(main: Path, item: Item, references: str) -> str | None:
    owner = item.owner
    assert owner is not None
    if item.path.is_symlink():
        return "build-rust became a symlink"
    if any(tree.locked for tree in list_worktrees(main) if tree.path == owner):
        return "worktree became locked"
    if refusal := (
        indexed_refusal(owner)
        or retained_refusal(main, owner, references)
        or cwd_refusal(owner)
    ):
        return refusal
    return idle_refusal(item)


def main(
    argv: list[str] | None = None,
    pr_state: PrState = gh_pr_state,
    references_reader: Callable[[Path], str] = worktree_references,
    issue_state: IssueState = gh_issue_state,
) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--apply", action="store_true", help="remove the listed items")
    parser.add_argument(
        "--repo", type=Path, default=Path(__file__).resolve().parents[1]
    )
    parser.add_argument(
        "--idle-hours",
        type=float,
        default=IDLE_HOURS,
        help="reclaim a kept worktree's build-rust after this long idle (default: %(default)g)",
    )
    args = parser.parse_args(argv)
    main_root = list_worktrees(args.repo)[0].path
    git(main_root, "fetch", "--prune", "--quiet", "origin")
    references = references_reader(main_root)
    items = plan(args.repo, pr_state, time.time(), references, issue_state)
    items = with_builds(main_root, items, pr_state, references, args.idle_hours)
    total = 0
    for item in items:
        bytes_ = size(item.path) if item.remove and item.path.exists() else 0
        total += bytes_
        shown = human(bytes_) if item.remove else "-"
        label = "remove" if item.remove else "keep  "
        print(f"{label} {shown:>7} {item.path} ({item.reason})", flush=True)
        if item.unlock and args.apply:
            git(main_root, "worktree", "unlock", str(item.path))
        if item.remove and args.apply:
            refusal = remove(main_root, item, references)
            if refusal:
                total -= bytes_
                print(f"       kept: {refusal}", flush=True)
    verb = "removed" if args.apply else "removable; rerun with --apply"
    print(f"{human(total)} {verb}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
