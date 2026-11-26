#!/usr/bin/env python3
"""Reject a core header that reaches a header of tables C++ fills for Rust (#1074).

A table of C++ functions or addresses that Rust reads lives in a `src/rust/*_table.h`
header, or in a generated record header named in TABLE_RECORDS, that only the C++ that
fills or calls it includes. A core header is any header under `src/` outside
`src/rust/` and `src/tests/`. Every project-local include, quoted or angled, is followed
as the compiler resolves it: a quoted name first against the including file's directory,
then each name against SEARCH_PATH. Names that resolve nowhere (system headers, other
generated headers) are not followed. The check needs no build.
"""

import argparse
import re
from collections import deque
from collections.abc import Mapping
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[1]
GENERATED = PurePosixPath("generated-abi")
# The build's include path for game objects: `src/` and, from cmake/Rust.cmake, the
# generated ABI directory and `src/rust/`.
SEARCH_PATH = (PurePosixPath("src"), PurePosixPath("src/rust"), GENERATED)
TABLE_RECORDS = frozenset({"abi-record-setting_addresses.h"})
INCLUDE = re.compile(r'^[ \t]*#[ \t]*include[ \t]*([<"])([^>"\n]+)[>"]', re.M)
HEADER_SUFFIXES = (".h", ".hpp")
# Translation phases 2 and 3: a comment becomes one space; literals are kept whole, so a
# quoted header name or a string holding "//" or "/*" is not taken for a comment.
LEXEME = re.compile(r'//[^\n]*|/\*.*?\*/|"(?:\\.|[^"\\\n])*"|\'(?:\\.|[^\'\\\n])*\'', re.S)


def directives(text: str) -> str:
    """The text as the preprocessor reads directives: splices joined, comments spaces."""
    joined = text.replace("\\\n", "")
    return LEXEME.sub(lambda m: " " if m.group(0)[0] == "/" else m.group(0), joined)


def normal(path: PurePosixPath) -> PurePosixPath:
    """Fold `..` and `.` parts, as the file system does for an existing directory."""
    parts: list[str] = []
    for part in path.parts:
        if part == "..":
            if parts:
                parts.pop()
        elif part != ".":
            parts.append(part)
    return PurePosixPath(*parts)


def is_table(path: PurePosixPath) -> bool:
    if path.parent == GENERATED:
        return path.name in TABLE_RECORDS
    return path.parent == PurePosixPath("src/rust") and path.name.endswith("_table.h")


def is_core(path: PurePosixPath) -> bool:
    return (
        path.suffix in HEADER_SUFFIXES
        and path.parts[:1] == ("src",)
        and path.parts[:2] not in (("src", "rust"), ("src", "tests"))
    )


def resolve(
    files: Mapping[PurePosixPath, str], includer: PurePosixPath, form: str, name: str
) -> PurePosixPath | None:
    directories = ((includer.parent,) if form == '"' else ()) + SEARCH_PATH
    for directory in directories:
        candidate = normal(directory / name)
        if candidate in files or is_table(candidate):
            return candidate
    return None


def includes(
    files: Mapping[PurePosixPath, str], path: PurePosixPath
) -> list[PurePosixPath]:
    found = []
    for form, name in INCLUDE.findall(directives(files.get(path, ""))):
        target = resolve(files, path, form, name.strip())
        if target is not None:
            found.append(target)
    return found


def violations(files: Mapping[PurePosixPath, str]) -> list[str]:
    """One line per core header that reaches a table header, with its first chain."""
    hits = []
    edges: dict[PurePosixPath, list[PurePosixPath]] = {}
    for root in sorted(filter(is_core, files)):
        chain = {root: root}
        queue = deque([root])
        while queue:
            current = queue.popleft()
            if is_table(current):
                steps = [current]
                while steps[-1] != root:
                    steps.append(chain[steps[-1]])
                hits.append(" -> ".join(str(step) for step in reversed(steps)))
                break
            if current not in edges:
                edges[current] = includes(files, current)
            for target in edges[current]:
                if target not in chain:
                    chain[target] = current
                    queue.append(target)
    return hits


def tree(root: Path) -> dict[PurePosixPath, str]:
    return {
        PurePosixPath(path.relative_to(root).as_posix()): path.read_text(
            errors="replace"
        )
        for path in (root / "src").rglob("*")
        if path.suffix in (*HEADER_SUFFIXES, ".inc") and path.is_file()
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("--root", type=Path, default=ROOT)
    args = parser.parse_args()
    hits = violations(tree(args.root))
    for hit in hits:
        print(f"core header reaches a table header: {hit}")
    print(f"Table headers reached from core headers: {len(hits)}")
    return int(bool(hits))


if __name__ == "__main__":
    raise SystemExit(main())
