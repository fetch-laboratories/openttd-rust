#!/usr/bin/env python3
"""Run every gate of Quick's `quick` job locally, plus the pinned commit checker.

The steps come from .github/workflows/ci-quick.yml itself, so the two cannot
drift. Unlike CI, every step runs even after a failure, and the result lists
each failed step.
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
import vcs
import yaml
from validation_execution import file_lock

import migration

HOOKS_REVISION = "eeb3791aadf0aded5a7cd634c80823f17e87af9c"
HOOKS_URL = "https://github.com/OpenTTD/OpenTTD-git-hooks.git"
WORKFLOW = migration.ROOT / ".github/workflows/ci-quick.yml"
JOB = "quick"
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
# A stamp's base must be on this branch, so base-relative gates saw every PR change.
PROTECTED = "origin/rust-migration"


@dataclass
class Step:
    name: str
    command: list[str]
    env: dict[str, str] = field(default_factory=dict)


def git(repo: Path, *arguments: str) -> str:
    return vcs.git(*arguments, cwd=repo).strip()


def hooks_checkout(supplied: Path | None = None) -> Path:
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
        command = ["bash", "--noprofile", "--norc", "-eo", "pipefail", "-c"]
        steps.append(
            Step(
                name,
                [*command, entry["run"]],
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
    parser.add_argument(
        "--step",
        action="append",
        help="run only steps whose name contains this text (repeatable)",
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


def main() -> int:
    options = arguments()
    try:
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
