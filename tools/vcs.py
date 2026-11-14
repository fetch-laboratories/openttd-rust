"""Text output from repository commands, preserving each caller's whitespace policy."""

import subprocess
import sys
from collections.abc import Callable
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def git(*arguments: str, cwd: Path = ROOT, errors: str = "strict") -> str:
    return subprocess.check_output(
        ["git", *arguments], cwd=cwd, text=True, errors=errors
    )


def gh(*arguments: str, cwd: Path | None = None, capture_stderr: bool = False) -> str:
    if not capture_stderr:
        return subprocess.check_output(["gh", *arguments], cwd=cwd, text=True)
    return subprocess.run(
        ["gh", *arguments], cwd=cwd, check=True, capture_output=True, text=True
    ).stdout


def merge_base(
    base_ref: str,
    head: str = "HEAD",
    *,
    run: Callable[..., str] = git,
    announce: bool = False,
) -> str:
    """The commit a change started from, not a base tip that moved since.

    A check given the base branch's current tip would read every later base
    commit as lines the change removed. The merge base of an ancestor is that
    ancestor, so passing a fork point is unchanged. Without a common ancestor
    there is nothing to compare against, so the check stops. With announce, the
    commit used is printed to standard error when it is not the given one. A
    caller passes its own `git` as run, so a test that points it at a scratch
    repository moves this lookup too.
    """
    try:
        base = run("merge-base", head, base_ref).strip()
    except subprocess.CalledProcessError as error:
        raise SystemExit(
            f"no common ancestor of {head} and {base_ref}; fetch full history"
            " (git fetch --unshallow) or pass the branch's real base"
        ) from error
    if announce and base != run("rev-parse", base_ref).strip():
        print(
            f"comparing with the merge base {base[:12]} of {base_ref}", file=sys.stderr
        )
    return base
