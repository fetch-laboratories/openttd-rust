"""Text output from repository commands, preserving each caller's whitespace policy."""

import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def git(*arguments: str, cwd: Path = ROOT, errors: str = "strict") -> str:
    return subprocess.check_output(
        ["git", *arguments], cwd=cwd, text=True, errors=errors
    )


def gh(*arguments: str, cwd: Path | None = None) -> str:
    return subprocess.run(
        ["gh", *arguments], cwd=cwd, check=True, capture_output=True, text=True
    ).stdout
