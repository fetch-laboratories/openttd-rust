"""Preflight pass stamps, the docs-only rule and the override log (#920).

A stamp records that `tools/preflight.py` passed every step on a clean tree
against a base. Stamps live in the main checkout's `.local`, so every worktree
on this host, and root's checkout, reads the same ones. A tree that differs
from a stamped tree only in docs-only files counts as stamped.
"""

import json
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
# Docs-only files, which no build, test or check reads: AGENTS.md, GitHub's
# PR and issue forms, the Codex host's settings, and docs/ except scripts and
# what a build reads there: the files CMake packages and the images Doxygen
# takes. The workflows' `inert` rule; tools/test-preflight.py checks this list
# against CMake and the Doxygen image commands.
BUILT_DOCS = (
    "admin_network.md",
    "debugging_desyncs.md",
    "desync.md",
    "directory_structure.md",
    "eints.md",
    "fonts.md",
    "game_coordinator.md",
    "linkgraph.md",
    "logging_and_performance_metrics.md",
    "multiplayer.md",
    "savegame_format.md",
    "symbol_server.md",
    "obg_format.txt",
    "obm_format.txt",
    "obs_format.txt",
    "openttd.6",
    "elrail_tile.png",
    "elrail_track.png",
)
# Stamps older than the newest this many are not compared for a docs-only match.
RECENT = 50


def git(*arguments: str) -> str:
    """Git in this checkout; tests point ROOT at a scratch repository."""
    return subprocess.check_output(["git", *arguments], cwd=ROOT, text=True).strip()


def common_local() -> Path:
    common = git("rev-parse", "--path-format=absolute", "--git-common-dir")
    return Path(common).parent / ".local/preflight"


def inert(path: str) -> bool:
    return (
        path in ("AGENTS.md", ".github/PULL_REQUEST_TEMPLATE.md")
        or path.startswith((".github/ISSUE_TEMPLATE/", ".codex/"))
        or (
            path.startswith("docs/")
            and not path.endswith((".py", ".sh"))
            and path not in {f"docs/{name}" for name in BUILT_DOCS}
        )
    )


def changed(old: str, new: str) -> list[str]:
    """Paths that differ between two commits or trees, both sides of a rename."""
    # NUL-delimited: Git quotes a name with a non-ASCII byte in line output.
    output = subprocess.check_output(
        ["git", "diff", "--no-renames", "--name-only", "-z", old, new, "--"],
        cwd=ROOT,
        text=True,
    )
    return [name for name in output.split("\0") if name]


def tree(commit: str) -> str:
    return git("rev-parse", "--verify", f"{commit}^{{tree}}")


def record(base: str, head: str) -> Path:
    stamps = common_local() / "stamps"
    stamps.mkdir(parents=True, exist_ok=True)
    checked = tree(head)
    path = stamps / f"{checked}.json"
    when = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    entry = {"tree": checked, "base": base, "head": head, "passed_at": when}
    path.write_text(json.dumps(entry) + "\n")
    return path


def stamp_for(commit: str) -> dict[str, str] | None:
    """The stamp covering this commit's tree, or None."""
    wanted = tree(commit)
    stamps = common_local() / "stamps"
    exact = stamps / f"{wanted}.json"
    if exact.is_file():
        return json.loads(exact.read_text())
    recent = sorted(stamps.glob("*.json"), key=lambda p: p.stat().st_mtime)
    for path in reversed(recent[-RECENT:]):
        entry = json.loads(path.read_text())
        try:
            paths = changed(entry["tree"], wanted)
        except subprocess.CalledProcessError:
            continue
        if all(inert(name) for name in paths):
            return entry
    return None


def log_override(kind: str, subject: str, commit: str, reason: str) -> Path:
    """Append an override (a checkpoint push or an unstamped request) to the log."""
    log = common_local() / "overrides.log"
    log.parent.mkdir(parents=True, exist_ok=True)
    when = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    entry = {
        "at": when,
        "kind": kind,
        "subject": subject,
        "commit": commit,
        "reason": reason,
        "worktree": str(ROOT),
    }
    with log.open("a") as handle:
        handle.write(json.dumps(entry) + "\n")
    return log
