#!/usr/bin/env python3
"""Pre-push, pre-commit and commit-msg hooks that enforce preflight locally (#920).

    python3 tools/git_hooks.py install

writes three small shims into the clone's shared hooks directory, beside
`post-checkout`, so they serve every worktree. Each shim runs this file from
the worktree that runs the hook, and does nothing in a worktree whose branch
predates that hook.

pre-push: a push of a branch whose change against origin/rust-migration is not
inert Markdown needs a preflight stamp for its tree. Every push first runs a
fast tier of a few seconds: the pinned commit checker on the branch's commits,
then, unless the change is inert Markdown, `cargo fmt --check` and Quick's Ruff
lines. A checkpoint push to a draft PR, or before its PR exists, may skip the
stamp with

    PREFLIGHT_CHECKPOINT="what is incomplete" git push ...

which is logged. It still runs the fast tier. A ready PR always needs the stamp.

pre-commit: the pinned checker's diff rules on the staged change.

commit-msg: the pinned checker's message rules, then what to do instead: the
accepted prefixes, each non-ASCII character or tab by line and column, the
exact command for a base-update merge, or why a fixup! title cannot stay.
"""

import json
import os
import re
import subprocess
import sys
import tempfile
import time
import unicodedata
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
# A branch whose git_hooks.py predates this hook does not list it in HOOKS.
grep -q '^HOOKS = .*"{hook}"' "$top/tools/git_hooks.py" 2>/dev/null || exit 0
exec python3 "$top/tools/git_hooks.py" {hook} "$@"
"""
# Each shim reads this line, on one line, to tell whether the worktree's copy of
# this file handles its hook.
HOOKS = ("pre-push", "pre-commit", "commit-msg")
BASE_BRANCH = "rust-migration"
# Titles git writes for `commit --fixup` and `--squash`; the checker refuses them.
FOLDED = ("fixup! ", "squash! ", "amend! ")
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


def run_checker(checker: Path, message: Path, quiet: bool = False) -> int:
    """The pinned check-message.py on a message file, as its own commit-msg runs it."""
    command = [sys.executable, str(checker), str(message), "client"]
    output = subprocess.DEVNULL if quiet else None
    return subprocess.run(command, stderr=output, check=False).returncode


def title_refused(checker: Path, title: str) -> bool:
    """Whether the checker refuses the title's prefix; characters are judged apart."""
    plain = "".join(c if 32 <= ord(c) <= 127 else "x" for c in title).rstrip()
    with tempfile.TemporaryDirectory() as directory:
        message = Path(directory) / "title"
        message.write_text(plain + "\n")
        return run_checker(checker, message, quiet=True) != 0


def git_text(*arguments: str) -> str | None:
    """Git in the repository the hook runs for, or None when it fails."""
    result = subprocess.run(
        ["git", *arguments], capture_output=True, text=True, check=False
    )
    return result.stdout.strip() if result.returncode == 0 else None


def prefix_remedy(checker: Path) -> list[str]:
    """The checker's own keyword list, so this hook keeps no second copy."""
    found = re.search(r'^KEYWORDS = "\(([^)]*)\)"', checker.read_text(), re.M)
    prefixes = ", ".join(found[1].split("|")) if found else "see the checker above"
    return [
        f"Accepted prefixes: {prefixes}.",
        "Start the title with one and ': ', for example `Fix: What it fixes` or "
        "`Fix #123: What it fixes`.",
    ]


def folded_remedy(title: str) -> list[str]:
    word = title.split()[0]
    return [
        f"A `{word}` title is refused here, and the commit checker refuses it at "
        "push. Give the commit a normal title: commit the change under its own "
        "`Fix: ...` title, or, when the commit it belongs to is HEAD, fold it in "
        "with `git commit --amend`."
    ]


def merge_heads() -> list[str]:
    path = git_text("rev-parse", "--git-path", "MERGE_HEAD")
    if path is None or not Path(path).is_file():
        return []
    return Path(path).read_text().split()


def on_base(commit: str) -> bool:
    base = f"origin/{BASE_BRANCH}"
    check = ["git", "merge-base", "--is-ancestor", commit, base]
    return subprocess.run(check, capture_output=True, check=False).returncode == 0


def merge_remedy(heads: list[str], checker: Path) -> list[str]:
    """The command for a base update, after checking that it is one."""
    branch = git_text("symbolic-ref", "--quiet", "--short", "HEAD")
    merged = ", ".join(head[:10] for head in heads)
    if len(heads) != 1 or not on_base(heads[0]):
        return [
            f"This merge is not a base update: MERGE_HEAD {merged} is not a commit "
            f"on origin/{BASE_BRANCH}. Give it a title that says what it merges.",
            *prefix_remedy(checker),
        ]
    if branch in (None, BASE_BRANCH):
        return [
            f"This merge brings origin/{BASE_BRANCH} into {branch or 'a detached HEAD'}"
            ", which is not a base update of a PR branch; check what you meant to merge."
        ]
    describe = ["log", "-1", "--format=%h %s"]
    return [
        f"This is a base update: MERGE_HEAD is on origin/{BASE_BRANCH}.",
        f"  parent 1 (HEAD):       {git_text(*describe, 'HEAD')}",
        f"  parent 2 (MERGE_HEAD): {git_text(*describe, heads[0])}",
        "The merge is not committed, so a push now would leave it out. Commit it with:",
        f'  git commit -m "Update: Merge {BASE_BRANCH} into {branch}"',
    ]


def character_remedy(lines: list[tuple[int, str]]) -> list[str]:
    found = [
        f"  line {number} column {column}: "
        + ("a tab" if c == "\t" else f"U+{ord(c):04X} {unicodedata.name(c, 'control')}")
        for number, line in lines
        for column, c in enumerate(line, 1)
        if not 32 <= ord(c) <= 127
    ]
    if not found:
        return []
    return ["Replace each with ASCII (a tab with spaces):", *found]


def message_remedies(checker: Path, text: str) -> list[str]:
    """This fork's advice after the checker refused the message in text."""
    # The checker skips comment lines on the client side; so does this.
    lines = [
        (number, line)
        for number, line in enumerate(text.splitlines(), 1)
        if not line.startswith("#")
    ]
    title = lines[0][1] if lines else ""
    remedies = []
    if title.startswith(FOLDED):
        remedies += folded_remedy(title)
    elif title_refused(checker, title):
        heads = merge_heads()
        remedies += merge_remedy(heads, checker) if heads else prefix_remedy(checker)
    return remedies + character_remedy(lines)


def commit_msg(message: Path) -> int:
    # As in pre_commit, an unavailable checker lets the commit through: pre-push
    # runs the same message rules again.
    try:
        checker = preflight.hooks_checkout() / "hooks/check-message.py"
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(
            f"commit-msg: pinned checker unavailable, not checked: {error}",
            file=sys.stderr,
        )
        return 0
    code = run_checker(checker, message.absolute())
    if code:
        text = message.read_text(encoding="utf-8", errors="replace")
        for line in message_remedies(checker, text):
            print(f"commit-msg: {line}", file=sys.stderr)
    return code


def foreign_hook(path: Path) -> str:
    """Why install stops at a hook it did not write, and how to replace it."""
    lines = path.read_text(errors="replace").splitlines()[1:]
    opening = next((line.strip() for line in lines if line.strip()), "(empty)")
    return (
        f"{path} exists and was not written by this installer; after its first "
        f"line it reads: {opening[:100]!r}. This installer's {path.name} hook "
        "would replace it. To replace it deliberately, keep a copy if you want "
        f"one (mv {path} {path}.local), then run python3 tools/git_hooks.py "
        "install --force, which replaces each hook it did not write."
    )


def install(hooks: Path, force: bool) -> int:
    for hook in HOOKS:
        path = hooks / hook
        if (
            path.exists()
            and MARKER not in path.read_text(errors="replace")
            and not force
        ):
            print(foreign_hook(path))
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
    if command == "commit-msg" and len(argv) == 3:
        return commit_msg(Path(argv[2]))
    if command == "install":
        common = preflight_stamp.git(
            "rev-parse", "--path-format=absolute", "--git-common-dir"
        )
        return install(Path(common) / "hooks", "--force" in argv[2:])
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
