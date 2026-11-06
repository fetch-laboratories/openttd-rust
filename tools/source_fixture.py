"""Temporary Git source trees used by scanner tests."""

from pathlib import Path

from vcs import git


def initialize(root: Path, files: dict[str, str]) -> None:
    for path, text in files.items():
        target = root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text)
    git("init", "-q", cwd=root)
    git("add", ".", cwd=root)
    git(
        "-c",
        "user.name=test",
        "-c",
        "user.email=test@example.invalid",
        "commit",
        "-qm",
        "Add: fixture",
        cwd=root,
    )
