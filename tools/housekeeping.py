#!/usr/bin/env python3
"""List, and with `--apply` remove, stale worktrees and ad hoc `.local` output (#532).

A dry run prints each candidate with its size and reason; `--apply` removes exactly the
`remove` lines unless a live output handle appears before deletion. Worktrees go through `git worktree remove` without `--force`, so a dirty
or locked one is kept. Live output handles are checked again before removal. Output in a
live worktree's own `.local` stays with that worktree.
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
from dataclasses import dataclass, replace
from pathlib import Path

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
REFERENCE = re.compile(r"\.local/(" + "|".join(OUTPUT) + r")/([^/\"\s]+)")


@dataclass(frozen=True)
class Item:
    path: Path
    remove: bool
    reason: str
    worktree: bool = False
    branch: str | None = None  # local branch deleted after its worktree


@dataclass(frozen=True)
class Worktree:
    path: Path
    head: str
    branch: str | None
    locked: bool
    prunable: bool


PrState = Callable[[Path, str], str | None]


def git(repo: Path, *arguments: str) -> str:
    return subprocess.check_output(["git", "-C", str(repo), *arguments], text=True)


def gh_pr_state(repo: Path, branch: str) -> str | None:
    """OPEN, MERGED or CLOSED across every PR from BRANCH, in that precedence."""
    command = ["gh", "pr", "list", "--state", "all", "--head", branch]
    command += ["--json", "state", "--jq", ".[].state"]
    states = subprocess.check_output(command, cwd=repo, text=True).split()
    return next((s for s in ("OPEN", "MERGED", "CLOSED") if s in states), None)


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
    main: Path, entries: list[Worktree], pr_state: PrState, references: str = ""
) -> list[Item]:
    # A head on the base's first-parent chain is a fresh branch, not merged work.
    first_parent = set(git(main, "rev-list", "--first-parent", BASE).split())
    items = []
    for tree in entries:
        if tree.path.is_relative_to(main / ".local/reference"):
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
            if task and (tree.branch is None or fresh):
                reason = "finished T3 task; detached or no commits beyond base"
                # A fresh branch holds no work to delete; a detached head has none.
            else:
                reason = (
                    "no PR; branch at a base commit" if fresh else "no PR; unmerged"
                )
                items.append(Item(tree.path, False, reason, True))
                continue
        if tree.locked or tree.prunable:
            hold = "locked" if tree.locked else "missing; run git worktree prune"
            items.append(Item(tree.path, False, f"{reason}, but {hold}", True))
        elif git(tree.path, "status", "--porcelain").strip():
            items.append(Item(tree.path, False, f"{reason}, but dirty", True))
        else:
            refusal = worktree_refusal(main, tree.path, references)
            items.append(Item(tree.path, not refusal, refusal or reason, True, branch))
    return protect_outputs(items)


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


def worktree_refusal(main: Path, path: Path, references: str) -> str | None:
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
                if error.errno not in (errno.ENOENT, errno.ESRCH):
                    return f"cannot inspect process cwd: {error}"
    except OSError as error:
        return f"cannot inspect process cwd: {error}"
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
            paths.extend(path.rglob("*"))
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
        if item.remove and (reason := output_refusal(item.path, handles, error))
        else item
        for item in items
    ]


def touched(path: Path) -> float:
    return max([path.stat().st_mtime] + [c.lstat().st_mtime for c in path.iterdir()])


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


def plan_local(local: Path, now: float) -> list[Item]:
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


def plan(repo: Path, pr_state: PrState, now: float, references: str = "") -> list[Item]:
    main, *linked = list_worktrees(repo)
    trees = plan_worktrees(main.path, linked, pr_state, references)
    return trees + plan_local(main.path / ".local", now)


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
    if item.worktree and (refusal := worktree_refusal(main, item.path, references)):
        return refusal
    handles, error = open_outputs()
    if refusal := output_refusal(item.path, handles, error):
        return refusal
    if not item.worktree:
        writable(item.path)
        if item.path.is_dir() and not item.path.is_symlink():
            shutil.rmtree(item.path)
        else:
            item.path.unlink()
        return None
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


def main(
    argv: list[str] | None = None,
    pr_state: PrState = gh_pr_state,
    references_reader: Callable[[Path], str] = worktree_references,
) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--apply", action="store_true", help="remove the listed items")
    parser.add_argument(
        "--repo", type=Path, default=Path(__file__).resolve().parents[1]
    )
    args = parser.parse_args(argv)
    main_root = list_worktrees(args.repo)[0].path
    git(main_root, "fetch", "--prune", "--quiet", "origin")
    references = references_reader(main_root)
    items = plan(args.repo, pr_state, time.time(), references)
    total = 0
    for item in items:
        bytes_ = size(item.path) if item.remove and item.path.exists() else 0
        total += bytes_
        shown = human(bytes_) if item.remove else "-"
        label = "remove" if item.remove else "keep  "
        print(f"{label} {shown:>7} {item.path} ({item.reason})", flush=True)
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
