#!/usr/bin/env python3
"""Pre-push and pre-commit hooks that enforce preflight locally (#920).

    python3 tools/git_hooks.py install

writes two small shims into the clone's shared hooks directory, beside
`post-checkout`, so they serve every worktree. Each shim runs this file from
the worktree that runs the hook, and does nothing in a worktree whose branch
predates it.

pre-push: a push of a branch whose change against origin/rust-migration is not
inert Markdown needs a preflight stamp for its tree. Every push first runs a
fast tier of a few seconds: the pinned commit checker on the branch's commits,
then, unless the change is inert Markdown, `cargo fmt --check` and Quick's Ruff
lines. A checkpoint push to a draft PR, or before its PR exists, may skip the
stamp with

    PREFLIGHT_CHECKPOINT="what is incomplete" git push ...

which is logged. It still runs the fast tier. A ready PR always needs the stamp.

pre-commit: the pinned checker's diff rules on the staged change.
"""

import json
import os
import subprocess
import sys
import time
from pathlib import Path

import preflight
import preflight_stamp
import vcs

import migration

ZERO = "0" * 40
OVERRIDE = "PREFLIGHT_CHECKPOINT"
MARKER = "Installed by tools/git_hooks.py"
SHIM = """#!/bin/sh
# {marker} (#920): runs this worktree's tracked hook.
top=$(git rev-parse --show-toplevel 2>/dev/null) || exit 0
[ -f "$top/tools/git_hooks.py" ] || exit 0
exec python3 "$top/tools/git_hooks.py" {hook} "$@"
"""
HOOKS = ("pre-push", "pre-commit")
PREFLIGHT = "python3 tools/preflight.py --base origin/rust-migration"


def fast_tier(base: str, head: str, inert: bool) -> list[preflight.Step]:
    return preflight.fast_tier(base, head, inert, preflight.hooks_checkout())


def ready_pr(branch: str) -> int | None:
    """The number of the open, ready PR from this branch, or None."""
    try:
        pull = json.loads(
            vcs.gh("pr", "view", branch, "--json", "number,isDraft,state")
        )
    except (OSError, subprocess.CalledProcessError, ValueError):
        return None
    if pull.get("state") == "OPEN" and not pull.get("isDraft"):
        return int(pull["number"])
    return None


def check_push(ref: str, sha: str, base: str, env: dict[str, str], logs: Path) -> str:
    """An empty string when the push may go ahead, otherwise the refusal."""
    branch = ref.removeprefix("refs/heads/")
    paths = preflight_stamp.changed(git_merge_base(base, sha), sha)
    inert = all(preflight_stamp.inert(path) for path in paths)
    failed = [
        step.name
        for number, step in enumerate(fast_tier(base, sha, inert), 1)
        if not preflight.run(
            step, env, logs / f"{branch.replace('/', '-')}-{number}.log"
        )
    ]
    if failed:
        return f"{branch}: fast tier failed: {', '.join(failed)}"
    if inert:
        print(f"{branch}: inert Markdown only; no preflight stamp needed")
        return ""
    if entry := preflight_stamp.stamp_for(sha):
        print(f"{branch}: preflight passed at {entry['passed_at']} on this tree")
        return ""
    reason = os.environ.get(OVERRIDE, "").strip()
    if not reason:
        return (
            f"{branch}: no preflight stamp for this tree. Run `{PREFLIGHT}` as a "
            f"background job, or for a draft checkpoint push with {OVERRIDE}='what "
            "is incomplete'"
        )
    if number := ready_pr(branch):
        return f"{branch}: PR #{number} is ready, so a push to it needs the stamp"
    log = preflight_stamp.log_override("checkpoint", branch, sha, reason)
    print(f"{branch}: checkpoint push without a stamp, logged in {log}")
    return ""


def git_merge_base(base: str, sha: str) -> str:
    return preflight_stamp.git("merge-base", base, sha)


def pre_push(lines: list[str]) -> int:
    # Git exports these to hooks; every command here names its worktree by cwd.
    for name in preflight.REPOSITORY_VARIABLES:
        os.environ.pop(name, None)
    updates = [
        line.split()
        for line in lines
        if line.strip() and line.split()[1] != ZERO
        if line.split()[2].startswith("refs/heads/")
    ]
    if not updates:
        return 0
    try:
        base = preflight.revision(preflight.PROTECTED)
        env = preflight.environment()
        logs = (
            migration.LOCAL / "preflight" / ("push-" + time.strftime("%Y%m%d-%H%M%S"))
        )
        logs.mkdir(parents=True, exist_ok=True)
        refusals = [
            refusal
            for _local, sha, remote_ref, _remote in updates
            if (refusal := check_push(remote_ref, sha, base, env, logs))
        ]
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as error:
        refusals = [f"pre-push check could not run: {error}"]
    for refusal in refusals:
        print(f"push refused: {refusal}", file=sys.stderr)
    return 1 if refusals else 0


def pre_commit() -> int:
    # An unavailable checker (no network for its first fetch, or an edited
    # checkout) lets the commit through: pre-push runs the same rules again.
    try:
        hooks = preflight.hooks_checkout() / "hooks"
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(
            f"pre-commit: pinned checker unavailable, not checked: {error}",
            file=sys.stderr,
        )
        return 0
    # The checker's diff runs in the committing repository, with git's variables.
    env = os.environ | {"HOOKS_DIR": str(hooks)}
    command = ["sh", str(hooks / "pre-commit")]
    return subprocess.run(command, cwd=migration.ROOT, env=env, check=False).returncode


def install(hooks: Path, force: bool) -> int:
    for hook in HOOKS:
        path = hooks / hook
        if (
            path.exists()
            and MARKER not in path.read_text(errors="replace")
            and not force
        ):
            print(
                f"{path} exists and was not written by this installer; --force replaces it"
            )
            return 1
        path.write_text(SHIM.format(marker=MARKER, hook=hook))
        path.chmod(0o755)
        print(f"installed {path}")
    return 0


def main(argv: list[str]) -> int:
    command = argv[1] if len(argv) > 1 else ""
    if command == "pre-push":
        return pre_push(sys.stdin.read().splitlines())
    if command == "pre-commit":
        return pre_commit()
    if command == "install":
        common = preflight_stamp.git(
            "rev-parse", "--path-format=absolute", "--git-common-dir"
        )
        return install(Path(common) / "hooks", "--force" in argv[2:])
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
