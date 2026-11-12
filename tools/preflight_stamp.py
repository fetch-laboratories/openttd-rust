"""Preflight pass stamps, the inert-Markdown rule and the override log (#920).

A stamp records that `tools/preflight.py` passed every step on a clean tree
against a base. Stamps live in the main checkout's `.local`, so every worktree
on this host, and root's checkout, reads the same ones. A tree that differs
from a stamped tree only in inert Markdown counts as stamped.
"""

import json
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
# Markdown that no build reads: AGENTS.md and docs/, except the docs CMake
# packages for the in-game viewer. The workflows' `inert` rule; tools/test-ci.py
# checks both against CMake.
BUILT_DOCS = (
    "admin_network",
    "debugging_desyncs",
    "desync",
    "directory_structure",
    "eints",
    "fonts",
    "game_coordinator",
    "linkgraph",
    "logging_and_performance_metrics",
    "multiplayer",
    "savegame_format",
    "symbol_server",
)
# Stamps older than the newest this many are not compared for a Markdown-only match.
RECENT = 50


def git(*arguments: str) -> str:
    """Git in this checkout; tests point ROOT at a scratch repository."""
    return subprocess.check_output(["git", *arguments], cwd=ROOT, text=True).strip()


def common_local() -> Path:
    common = git("rev-parse", "--path-format=absolute", "--git-common-dir")
    return Path(common).parent / ".local/preflight"


def inert(path: str) -> bool:
    return path == "AGENTS.md" or (
        path.startswith("docs/")
        and path.endswith(".md")
        and path not in {f"docs/{name}.md" for name in BUILT_DOCS}
    )


def changed(old: str, new: str) -> list[str]:
    """Paths that differ between two commits or trees, both sides of a rename."""
    output = git("diff", "--no-renames", "--name-only", old, new, "--")
    return output.splitlines()


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
