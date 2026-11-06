#!/usr/bin/env python3
"""Reject a C++ callee bound into more than one Rust service table (#582)."""

import re
from collections import Counter, defaultdict
from collections.abc import Iterator
from pathlib import Path

import baseline_ratchet
from code_lines import without_comments

ROOT = Path(__file__).resolve().parents[1]
BASELINE = "tools/rebound-callee-baseline.txt"
DESIGNATOR = re.compile(r"[{,](?>\s+|/\*.*?\*/|//[^\n]*|#[^\n]*)*\.", re.S)
FIELD = re.compile(r"\s*\.\s*(\w+)\s*=\s*(.*?)\s*", re.S)
# `Callee`, `&Callee`, `ns::Callee` or `Callee<T>`; fork callees are CamelCase or openttd_*.
CALLEE = re.compile(r"&?\s*(?:::)?((?:\w+::)*(?:[A-Z]|openttd_)\w*)(?:\s*<[^<>]*>)?")
# A field read from another table, such as `GetRustSharedServices().snow_line`.
FORWARDED = re.compile(r"&?\s*(?:\w+::)*\w*Services\(\)(?:\s*(?:\.|->)\s*\w+)+", re.S)
NOEXCEPT = re.compile(r"\b(\w+)\s*\([^;{}]*?\)\s*(?:const\s*)?noexcept\b")


def tables(source: str) -> Iterator[tuple[int, list[str]]]:
    """Top-level entries of every braced initializer, keyed by its opening brace."""
    stack: list[tuple[int, list[str], int]] = []
    for index, char in enumerate(source):
        if char in "{([":
            stack.append((index, [], index + 1))
        elif char in "})]" and stack:
            start, entries, entry = stack.pop()
            if source[start] == "{":
                yield start, [*entries, source[entry:index]]
        elif char == "," and stack and source[stack[-1][0]] == "{":
            start, entries, entry = stack[-1]
            entries.append(source[entry:index])
            stack[-1] = (start, entries, index + 1)


def violations(files: dict[str, str]) -> set[str]:
    production = {
        path: text
        for path, text in files.items()
        if path.startswith("src/") and not path.startswith("src/tests/")
    }
    # A commented-out declaration only widens the callee set; lexing is the slow part.
    functions = {
        name for text in production.values() for name in NOEXCEPT.findall(text)
    }
    masked = {
        path: re.sub(r"(?m)^[ \t]*#.*$", "", without_comments(text, path, True))
        for path, text in production.items()
        if DESIGNATOR.search(text)
    }
    sites: defaultdict[str, list[tuple[str, int, str]]] = defaultdict(list)
    hits: set[str] = set()
    for path, text in masked.items():
        for start, entries in tables(text):
            for entry in entries:
                field = FIELD.fullmatch(entry)
                if not field:
                    continue
                callee = CALLEE.fullmatch(field[2])
                if callee and callee[1].rsplit("::", 1)[-1] in functions:
                    sites[callee[1]].append((path, start, field[1]))
                elif FORWARDED.fullmatch(field[2]):
                    value = " ".join(field[2].split())
                    hits.add(f"forwarded | {path} | {field[1]} = {value}")
    for callee, bound in sites.items():
        if len({(path, start) for path, start, _ in bound}) > 1:
            seen: Counter[str] = Counter()
            for path, _, name in bound:
                key = f"rebound | {callee} | {path} | {name}"
                seen[key] += 1
                hits.add(key if seen[key] == 1 else f"{key} | {seen[key]}")
    return hits


def main() -> int:
    return baseline_ratchet.main(
        __doc__,
        ROOT,
        BASELINE,
        ("src",),
        {".cpp", ".h", ".hpp", ".inc"},
        violations,
        new_label="new rebinding",
        summary="Rebound callees: {count} baselined bindings",
    )


if __name__ == "__main__":
    raise SystemExit(main())
