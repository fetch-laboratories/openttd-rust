#!/usr/bin/env python3
"""Reject bool register returns from Rust-defined C entries and callbacks (#644)."""

import argparse
import re
from pathlib import Path

from code_lines import without_comments

ROOT = Path(__file__).resolve().parents[1]
IDENT = r"(?:r#)?[A-Za-z_]\w*"
PATH = rf"(?:::)?{IDENT}(?:\s*::\s*{IDENT})*"
TYPE = rf"(?:\(\s*)*{PATH}(?:\s*\))*"
C_FN = r'\bextern\s+(?:"C"\s+)?fn'
ENTRY = re.compile(rf"{C_FN}\s+\$?({IDENT})\s*(?:<[^{{;]*?>\s*)?\(")
ALIAS = re.compile(rf"\btype\s+({IDENT})\s*=\s*({TYPE})\s*;")
IMPORT = re.compile(rf"\buse\s+({PATH})\s+as\s+({IDENT})\s*;")
TRANSPARENT = re.compile(
    rf"#\[\s*repr\s*\(\s*transparent\s*\)\s*\]\s*(?:#\[[^]]*\]\s*)*"
    rf"(?:pub(?:\([^)]*\))?\s+)?struct\s+({IDENT})\s*[({{]\s*"
    rf"(?:pub(?:\([^)]*\))?\s+)?(?:{IDENT}\s*:\s*)?({TYPE})\s*,?\s*[)}}]"
)


def type_name(spelling: str) -> str:
    """Ignore path qualification and parentheses around a scalar type."""
    return re.findall(IDENT, spelling)[-1]


def closing(source: str, start: int, left: str, right: str) -> int:
    """Return the matching delimiter, including nested parameter types/blocks."""
    depth = 0
    for offset in range(start, len(source)):
        char = source[offset]
        depth += int(char == left) - int(char == right)
        if depth == 0:
            return offset
    raise ValueError(f"Unclosed {left} at {start}")


def test_ranges(source: str) -> list[tuple[int, int]]:
    """Rust-only service mocks never return to C++; keep their native service types."""
    ranges = []
    for match in re.finditer(r"#\[\s*cfg\s*\(\s*test\s*\)\s*\]", source):
        start = source.find("{", match.end())
        semicolon = source.find(";", match.end())
        if start >= 0 and (semicolon < 0 or start < semicolon):
            ranges.append((match.start(), closing(source, start, "{", "}") + 1))
    return ranges


TEST_MODULE = re.compile(
    rf"#\[\s*cfg\s*\(\s*test\s*\)\s*\]\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+({IDENT})\s*;"
)


def test_files(files: dict[str, str]) -> set[Path]:
    """Files of `#[cfg(test)] mod name;` declarations, which are test mocks too."""
    out = set()
    for path, source in files.items():
        parent = Path(path)
        directory = (
            parent.parent
            if parent.name in ("lib.rs", "main.rs", "mod.rs")
            else parent.with_suffix("")
        )
        for name in TEST_MODULE.findall(source):
            out |= {directory / f"{name}.rs", directory / name / "mod.rs"}
    return out


def file_violations(path: str, source: str, aliases: set[str]) -> list[str]:
    """One non-mock file's bool-returning C definitions and macro return types."""
    hits = []
    tests = test_ranges(source)
    for entry in ENTRY.finditer(source):
        if any(start <= entry.start() < end for start, end in tests):
            continue
        end = closing(source, entry.end() - 1, "(", ")")
        result = re.match(
            rf"\s*->\s*({TYPE})\s*(?:where\b[^{{;]*)?\{{", source[end + 1 :]
        )
        if result and type_name(result[1]) in aliases:
            line = source.count("\n", 0, entry.start()) + 1
            hits.append(
                f"{path}:{line}: extern C definition {entry[1]} returns {result[1]}"
            )
    # A return type supplied to a C-entry macro must be checked at its invocation.
    # This also covers `=> bool` entry lists used by the ground-vehicle facade.
    if re.search(rf"{C_FN}[^{{}};]*\$ret", source):
        for result in re.finditer(rf"(?:->|=>)\s*({TYPE})\s*(?:=>|[);])", source):
            if type_name(result[1]) in aliases and not any(
                start <= result.start() < end for start, end in tests
            ):
                line = source.count("\n", 0, result.start()) + 1
                hits.append(
                    f"{path}:{line}: C-entry macro supplies {result[1]} as return type"
                )
    return hits


def violations(files: dict[str, str]) -> list[str]:
    """Scan definitions, macro templates and visible simple type-alias chains."""
    sources = {
        path: without_comments(text, path, strings=True) for path, text in files.items()
    }
    # Transparent single-field wrappers share their field's return ABI.
    aliases = {"bool"}
    bindings = [
        binding
        for source in sources.values()
        for pattern in (ALIAS, TRANSPARENT)
        for binding in pattern.findall(source)
    ] + [
        (name, target)
        for source in sources.values()
        for target, name in IMPORT.findall(source)
    ]
    while True:
        updated = aliases | {
            name for name, target in bindings if type_name(target) in aliases
        }
        if updated == aliases:
            break
        aliases = updated
    mocks = test_files(sources)
    return [
        hit
        for path, source in sources.items()
        if Path(path) not in mocks
        for hit in file_violations(path, source, aliases)
    ]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("files", nargs="*", type=Path)
    args = parser.parse_args()
    paths = args.files or sorted((ROOT / "rust/openttd-kernels/src").rglob("*.rs"))
    hits = violations({str(path): path.read_text() for path in paths})
    for hit in hits:
        print(hit)
    print(f"Rust C bool returns: {len(hits)} violations")
    return int(bool(hits))


if __name__ == "__main__":
    raise SystemExit(main())
