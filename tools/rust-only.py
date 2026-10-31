#!/usr/bin/env python3
"""Reject WITH_RUST conditionals in src; optionally rerun the #581 sweep."""

import argparse
import re
import subprocess
from pathlib import Path

from code_lines import Comment, tokens

ROOT = Path(__file__).resolve().parents[1]


def conditionals(text: str, path: str) -> list[int]:
    """Find conditional directives, excluding comments and string contents."""
    directives = "".join(
        value if kind in Comment.Preproc else "\n" * value.count("\n") + " "
        for kind, value in tokens(text, path)
    )
    directives = re.sub(r"\\\r?\n", " ", directives)
    return [
        directives.count("\n", 0, match.start()) + 1
        for match in re.finditer(
            r"(?m)^\s*#\s*(?:if|ifdef|ifndef|elif)\b[^\n]*\bWITH_RUST\b",
            directives,
        )
    ]


def source_files(paths: list[Path]) -> list[Path]:
    files = paths or [
        ROOT / name
        for name in subprocess.check_output(
            ["git", "ls-files", "-z", "src/"], cwd=ROOT, text=True
        ).split("\0")
        if name
    ]
    for path in files:
        if not path.resolve().is_relative_to(ROOT / "src") or not path.is_file():
            raise ValueError(f"Expected a file under src/: {path}")
    return files


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sweep", action="store_true")
    parser.add_argument("--unifdef", default="unifdef")
    parser.add_argument("files", nargs="*", type=Path)
    args = parser.parse_args()
    failed = False
    for path in source_files(args.files):
        text = path.read_text(errors="replace")
        if args.sweep and re.search(r"\bWITH_RUST\b", text):
            # unifdef 2.12 misparses C++ digit separators as character literals.
            flags = ["-t"] if path.name in ("strings.cpp", "station_cmd.cpp") else []
            result = subprocess.run([args.unifdef, "-DWITH_RUST", "-m", *flags, path])
            if result.returncode not in (0, 1):
                raise RuntimeError(f"unifdef failed for {path}: {result.returncode}")
            text = path.read_text()
            # unifdef retains mixed expressions with an unknown ABI-check macro.
            simplified = text.replace(
                "#if defined(WITH_RUST) && defined(WITH_RUST_ABI_CHECKS)",
                "#if defined(WITH_RUST_ABI_CHECKS)",
            )
            simplified = simplified.rstrip("\n") + "\n"
            if simplified != text:
                path.write_text(simplified)
                text = simplified
        if not re.search(r"\bWITH_RUST\b", text):
            continue
        for line in conditionals(text, str(path)):
            print(f"{path}:{line}: WITH_RUST conditional in src/")
            failed = True
    return int(failed)


if __name__ == "__main__":
    raise SystemExit(main())
