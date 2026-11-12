#!/usr/bin/env python3
"""Run every gate of Quick's `quick` job locally, plus the pinned commit checker.

The steps come from .github/workflows/ci-quick.yml itself, so the two cannot
drift. Unlike CI, every step runs even after a failure, and the result lists
each failed step.

--fast runs, in seconds, the pre-push hook's fast tier (the pinned commit
checker, cargo fmt --check and Quick's Ruff lines) and Quick's ty line, and
records no stamp.
"""

import argparse
import os
import re
import shutil
import subprocess
import sys
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import preflight_stamp
import quick_workflow
import yaml
from validation_execution import file_lock

import migration

HOOKS_REVISION = "eeb3791aadf0aded5a7cd634c80823f17e87af9c"
HOOKS_URL = "https://github.com/OpenTTD/OpenTTD-git-hooks.git"
WORKFLOW = quick_workflow.WORKFLOW
JOB = quick_workflow.JOB
COMMITS = "Check introduced commits with the pinned inherited checker"
# Preflight's own step after Quick's (#923): CI compiles these in its native jobs.
PROBES = "Compile comparison probes and runtime adapters without linking"
# Quick's `run:` steps that preflight does not run. Steps that `uses:` an action
# (checkouts, setup-uv) are skipped too: they only prepare the runner.
SKIPPED = {
    "Install pinned Rust compiler": "rustup installs the pinned toolchain on use",
    "Install nested collection probe dependency": "installs valgrind with apt",
    "Check every introduced commit against captured base": (
        f"preflight runs the same pinned checker as {COMMITS!r}"
    ),
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


def workflow_steps(workflow: dict[str, Any], base: str) -> list[Step]:
    """Quick's `run:` steps that preflight runs, in workflow order."""
    steps = []
    for entry in workflow["jobs"][JOB]["steps"]:
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
    """The pre-push hook's seconds-long checks; inert Markdown needs only the first."""
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


def run(step: Step, env: dict[str, str], log: Path) -> bool:
    print(f"--> {step.name}", flush=True)
    started = time.monotonic()
    with log.open("w") as output:
        code = subprocess.run(
            step.command,
            cwd=migration.ROOT,
            env=env | step.env,
            stdout=output,
            stderr=subprocess.STDOUT,
            check=False,
        ).returncode
    elapsed = time.monotonic() - started
    if code:
        tail = log.read_text(errors="replace").splitlines()[-TAIL_LINES:]
        print("\n".join(f"    {line}" for line in tail))
    print(f"{'FAIL' if code else 'pass'} {elapsed:.0f}s {step.name}", flush=True)
    if code:
        print(f"    exit {code}; full output: {log}", flush=True)
    return code == 0


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
    parser.add_argument("--list", action="store_true", help="list the steps and exit")
    return parser.parse_args()


def revision(name: str) -> str:
    return git(
        migration.ROOT,
        "rev-parse",
        "--verify",
        "--end-of-options",
        f"{name}^{{commit}}",
    )


def uncommitted() -> str:
    """A warning when tracked files differ from HEAD, or nothing."""
    if git(migration.ROOT, "status", "--porcelain", "--untracked-files=no"):
        return (
            "warning: tracked files differ from HEAD; this result describes the "
            "working tree, not the commit CI tests"
        )
    return ""


def summary(chosen: list[Step], failed: list[str], seconds: float, warning: str) -> int:
    print(f"{len(chosen) - len(failed)}/{len(chosen)} passed in {seconds:.0f}s")
    for name in failed:
        print(f"failed: {name}; rerun: python3 tools/preflight.py --step {name!r}")
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
    base, head = revision(options.base), revision(options.head)
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


def main() -> int:
    options = arguments()
    try:
        if options.fast:
            return fast(options)
        base, head = revision(options.base), revision(options.head)
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
        failed = [
            step.name
            for number, step in enumerate(chosen, 1)
            if not run(step, env, logs / f"{number:02}.log")
        ]
        # A step can change tracked files; the summary describes the end state too.
        warning = warning or uncommitted()
        if not failed:
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
    return summary(chosen, failed, time.monotonic() - started, warning)


if __name__ == "__main__":
    sys.exit(main())
