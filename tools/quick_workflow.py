"""Commands read from Quick's workflow, so local tools run Quick's pinned versions.

This module imports no other tool module, so preflight, git_hooks and
migration can all use it without an import cycle.
"""

from pathlib import Path
from typing import Any, NotRequired, TypedDict

import yaml

WORKFLOW = Path(__file__).resolve().parents[1] / ".github/workflows/ci-quick.yml"


# A step's fields the tools read; `if` and `with` are keywords, hence this form.
Step = TypedDict(
    "Step",
    {
        "name": str,
        "run": str,
        "uses": str,
        "env": dict[str, str],
        "with": dict[str, Any],
        "if": str,
    },
    total=False,
)


class Job(TypedDict):
    """A job as the tools read it: its steps, and the jobs it needs."""

    steps: list[Step]
    needs: NotRequired[list[str]]


class Workflow(TypedDict):
    """A workflow file as the tools read it: its jobs, in file order."""

    jobs: dict[str, Job]


def steps(workflow: Workflow) -> list[Step]:
    """Quick's steps across its jobs, in file order (#1063)."""
    return [step for job in workflow["jobs"].values() for step in job["steps"]]


def run_lines(prefix: str) -> list[str]:
    """Each line of a Quick `run:` step that starts with prefix, in order."""
    quick = yaml.safe_load(WORKFLOW.read_text())
    lines = [
        line.strip()
        for step in steps(quick)
        for line in step.get("run", "").splitlines()
        if line.strip().startswith(prefix)
    ]
    if not lines:
        raise ValueError(f"Quick runs no {prefix!r} command; read {WORKFLOW}")
    return lines


def ruff_lines() -> list[str]:
    """Quick's own Ruff commands, so local checks cannot drift from it."""
    return run_lines("uvx --from ruff")


def ty_line() -> str:
    """Quick's own pinned ty command."""
    return run_lines("uvx ty@")[0]
