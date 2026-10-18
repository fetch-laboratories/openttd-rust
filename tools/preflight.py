#!/usr/bin/env python3
"""Run the pinned inherited commit checker."""

import argparse
import os
import subprocess
import sys
from pathlib import Path

from validation_execution import file_lock

import migration

HOOKS_REVISION = "eeb3791aadf0aded5a7cd634c80823f17e87af9c"
HOOKS_URL = "https://github.com/OpenTTD/OpenTTD-git-hooks.git"


def git(repo, *arguments):
    return subprocess.check_output(
        ["git", "-C", str(repo), *arguments], text=True
    ).strip()


def hooks_checkout(supplied=None):
    destination = supplied or (
        migration.COMMON_LOCAL / "tooling/OpenTTD-git-hooks" / HOOKS_REVISION
    )
    with file_lock(
        migration.COMMON_LOCAL / "tooling/hooks.lock", label="commit-checker checkout"
    ):
        if not destination.exists() and supplied is None:
            destination.mkdir(parents=True)
            subprocess.run(["git", "init", "--quiet", str(destination)], check=True)
        if supplied is None and not (destination / "hooks/check-commits.sh").exists():
            for command in (
                ["fetch", "--depth=1", HOOKS_URL, HOOKS_REVISION],
                ["checkout", "--quiet", "--detach", HOOKS_REVISION],
            ):
                subprocess.run(["git", "-C", str(destination), *command], check=True)
        if git(destination, "rev-parse", "HEAD") != HOOKS_REVISION:
            raise ValueError("commit checker is not at the pinned revision")
        if git(destination, "status", "--porcelain", "--untracked-files=all"):
            raise ValueError("commit checker checkout is dirty")
    return destination


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", default="origin/rust-migration")
    parser.add_argument("--head", default="HEAD")
    parser.add_argument(
        "--hooks", type=Path, help="existing clean pinned hooks checkout (offline)"
    )
    arguments = parser.parse_args()
    code = 1
    try:
        base = git(
            migration.ROOT,
            "rev-parse",
            "--verify",
            "--end-of-options",
            f"{arguments.base}^{{commit}}",
        )
        head = git(
            migration.ROOT,
            "rev-parse",
            "--verify",
            "--end-of-options",
            f"{arguments.head}^{{commit}}",
        )
        hooks = hooks_checkout(arguments.hooks.resolve() if arguments.hooks else None)
        env = os.environ.copy()
        env.update(
            GIT_DIR=git(migration.ROOT, "rev-parse", "--absolute-git-dir"),
            HOOKS_DIR=str(hooks / "hooks"),
        )
        code = subprocess.run(
            ["sh", str(hooks / "hooks/check-commits.sh"), f"{base}..{head}"],
            cwd=migration.ROOT,
            env=env,
            check=False,
        ).returncode
    except (
        OSError,
        ValueError,
        KeyError,
        TypeError,
        subprocess.CalledProcessError,
    ) as error:
        print(error, file=sys.stderr)
    return code


if __name__ == "__main__":
    sys.exit(main())
