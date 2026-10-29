#!/usr/bin/env python3
"""Reject Rust exports/declarations without production C++ users (#564)."""

import argparse
import re
import subprocess
from pathlib import Path

from code_lines import RUST_EXPORT, without_comments

ROOT = Path(__file__).resolve().parents[1]
BASELINE = "tools/orphan-export-baseline.txt"
NAME = r"openttd_rust_\w+"
DECLARATION = re.compile(
    rf'(?m)^[ \t]*(?:extern\s+"C"\s+)?(?!return\b|else\b)'
    rf"[\w:<>]+[\w:<> \t*&]*[\s*&]({NAME})\s*\([^;{{}}]*\)\s*(?:noexcept\s*)?;"
)


def violations(files: dict[str, str]) -> set[str]:
    exports, declarations, references = set(), set(), set()
    for path, text in files.items():
        if path.startswith("rust/") and path.endswith(".rs"):
            exports.update(
                RUST_EXPORT.findall(without_comments(text, path, strings=True))
            )
        elif path.startswith("src/") and not path.startswith("src/tests/"):
            source = without_comments(text, path, strings=True)
            # extern "C" was masked with other strings; it isn't needed to find
            # names, but leaving its prefix would hide a one-line declaration.
            source = re.sub(r"\bextern\s+(?=\w)", "", source)
            if path.startswith("src/rust/"):
                declarations.update(DECLARATION.findall(source))
            references.update(re.findall(NAME, DECLARATION.sub("", source)))
    return {f"no declaration | {name}" for name in exports - declarations} | {
        f"no production user | {name}"
        for name in exports | declarations
        if name not in references
    }


def git(*args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-ref", default="origin/rust-migration")
    args = parser.parse_args()
    files = {
        str(p.relative_to(ROOT)): p.read_text()
        for folder in ("src", "rust")
        for p in (ROOT / folder).rglob("*")
        if p.is_file() and p.suffix in {".cpp", ".h", ".hpp", ".rs"}
    }
    hits = violations(files)
    baseline = set((ROOT / BASELINE).read_text().splitlines())
    tracked = git("ls-tree", "--name-only", args.base_ref, BASELINE).strip()
    previous = (
        set(git("show", f"{args.base_ref}:{BASELINE}").splitlines())
        if tracked
        else baseline
    )
    errors = [
        ("new orphan", hits - baseline),
        ("remove retired", baseline - hits),
        ("baseline cannot grow", baseline - previous),
    ]
    for label, names in errors:
        for name in sorted(names):
            print(f"{label}: {name}")
    print(
        f"Orphan exports: {len(hits)} baselined violations; {sum(len(v) for _, v in errors)} errors"
    )
    return int(any(names for _, names in errors))


if __name__ == "__main__":
    raise SystemExit(main())
