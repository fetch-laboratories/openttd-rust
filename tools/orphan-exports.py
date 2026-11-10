#!/usr/bin/env python3
"""Reject Rust exports/declarations without production C++ users (#564).

A reference inside an out-of-class member definition does not count when that
member has an explicit template instantiation and no caller: its definition,
declarations and instantiations are not callers (#856). --report-uncalled lists,
without failing, exports whose only references sit in any uncalled member;
script API members count as called, through their generated Squirrel bindings.
"""

import re
import subprocess
import sys
from pathlib import Path

import baseline_ratchet
from code_lines import RUST_EXPORT, without_comments

ROOT = Path(__file__).resolve().parents[1]
BASELINE = "tools/orphan-export-baseline.txt"
NAME = r"openttd_rust_\w+"
DECLARATION = re.compile(
    rf'(?m)^[ \t]*(?:extern\s+"C"\s+)?(?!return\b|else\b)'
    rf"[\w:<>]+[\w:<> \t*&]*[\s*&]({NAME})\s*\([^;{{}}]*\)\s*(?:noexcept\s*)?;"
)
# `Class::Member(` heading a definition; the body's braces are matched below.
MEMBER_HEAD = re.compile(r"\b(\w+)::(\w+)\s*(?:<[^;(){}]*>\s*)?\(")
QUALIFIERS = re.compile(r"\s*(?:(?:const|noexcept|override|final)\b\s*)*")
INSTANTIATION = re.compile(r"(?<![\w<])template\s+(?!<)[^;{}]*;")
INSTANTIATED = re.compile(r"(?<![\w<])template\s+(?!<)[^;{}]*?\b(\w+)\s*<[^;{}]*>\s*\(")
CALL_FREE = ("return", "co_return", "throw", "delete", "else", "case", "goto")


def closing(source: str, start: int, opening: str, closing_char: str) -> int:
    """Index just past the bracket matching the one at start, or -1."""
    depth = 0
    for index in range(start, len(source)):
        if source[index] == opening:
            depth += 1
        elif source[index] == closing_char:
            depth -= 1
            if depth == 0:
                return index + 1
    return -1


def member_definitions(source: str) -> list[tuple[str, str, int, int, int]]:
    """Each out-of-class definition: owner, member, head start, body start, body end."""
    found = []
    for match in MEMBER_HEAD.finditer(source):
        owner, member = match.groups()
        if member == owner or member == "operator":
            continue
        end = closing(source, match.end() - 1, "(", ")")
        if end < 0:
            continue
        qualifiers = QUALIFIERS.match(source, end)
        body = qualifiers.end() if qualifiers else end
        if source.startswith("{", body):
            end = closing(source, body, "{", "}")
            found.append((owner, member, match.start(), body, end))
    return found


def declares(statement: str, member: str) -> bool:
    """A declaration statement names a type before the member and calls nothing."""
    words = statement.split()
    return (
        bool(words)
        and words[0] not in CALL_FREE
        and re.fullmatch(
            rf"\s*(?:template\s*<[^;{{}}]*>\s*)?[\w:<>,\s*&]+?[\s*&]{member}\s*"
            rf"\([^;{{}}]*\)[\w\s=]*;\s*",
            statement,
        )
        is not None
    )


def member_uses(sources: dict[str, str]) -> set[str]:
    """Identifiers outside definition heads, declarations and instantiations."""
    members = {
        definition[1]
        for source in sources.values()
        for definition in member_definitions(source)
    }
    used = set()
    for source in sources.values():
        source = INSTANTIATION.sub(";", source)
        for _, _, start, body, _ in member_definitions(source):
            source = source[:start] + " " * (body - start) + source[body:]
        for statement in re.split(r"(?<=[;{}])", source):
            names = members.intersection(re.findall(r"\b\w+\b", statement))
            used |= {name for name in names if not declares(statement, name)}
    return used


def without_uncalled_members(
    source: str, uncalled: set[str], owners: dict[str, set[str]]
) -> str:
    """Blank out definitions of uncalled members, noting each hidden reference."""
    for owner, member, start, _, end in reversed(member_definitions(source)):
        if member in uncalled and end > 0:
            for name in re.findall(NAME, source[start:end]):
                owners.setdefault(name, set()).add(f"{owner}::{member}")
            source = source[:start] + " " * (end - start) + source[end:]
    return source


def violations(files: dict[str, str], broad: bool = False) -> set[str]:
    return set(scan(files, broad))


def scan(files: dict[str, str], broad: bool = False) -> dict[str, set[str]]:
    """Each violation, with the uncalled members that hold its only references."""
    exports, declarations, boundary_declarations, references = (
        set(),
        set(),
        set(),
        set(),
    )
    sources = {}
    for path, text in files.items():
        if path.startswith("rust/") and path.endswith(".rs"):
            exports.update(
                RUST_EXPORT.findall(without_comments(text, path, strings=True))
            )
        elif path.startswith("src/") and not path.startswith("src/tests/"):
            source = without_comments(text, path, strings=True)
            # extern "C" was masked with other strings; it isn't needed to find
            # names, but leaving its prefix would hide a one-line declaration.
            sources[path] = re.sub(r"\bextern\s+(?=\w)", "", source)
    used = member_uses(sources)
    defined = {
        definition[1]
        for source in sources.values()
        for definition in member_definitions(source)
    }
    instantiated = {
        member for source in sources.values() for member in INSTANTIATED.findall(source)
    }
    uncalled = (defined if broad else defined & instantiated) - used
    owners: dict[str, set[str]] = {}
    for path, source in sources.items():
        found = set(DECLARATION.findall(source))
        declarations.update(found)
        if path.startswith("src/rust/"):
            boundary_declarations.update(found)
        source = DECLARATION.sub("", source)
        if not path.startswith("src/script/api/"):
            source = without_uncalled_members(source, uncalled, owners)
        references.update(re.findall(NAME, source))
    result = {f"no declaration | {name}": set() for name in exports - declarations}
    for name in exports | boundary_declarations:
        if name not in references:
            result[f"no production user | {name}"] = owners.get(name, set())
    return result


def report_uncalled() -> int:
    """List, without failing, exports held only by uncalled members (#737 batch O)."""
    paths = subprocess.run(
        ["git", "ls-files", "src", "rust"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.split()
    files = {
        path: (ROOT / path).read_text(errors="replace")
        for path in paths
        if Path(path).suffix in {".cpp", ".h", ".hpp", ".inc", ".rs"}
    }
    enforced = violations(files)
    rows = sorted(
        (row.split(" | ")[1], ", ".join(sorted(members)))
        for row, members in scan(files, broad=True).items()
        if row not in enforced
    )
    for name, members in rows:
        print(f"{name} | {members}")
    print(f"Exports held only by uncalled members: {len(rows)} (not enforced)")
    return 0


def main() -> int:
    if sys.argv[1:] == ["--report-uncalled"]:
        return report_uncalled()
    return baseline_ratchet.main(
        __doc__,
        ROOT,
        BASELINE,
        ("src", "rust"),
        {".cpp", ".h", ".hpp", ".inc", ".rs"},
        violations,
        new_label="new orphan",
        summary="Orphan exports: {count} baselined violations",
    )


if __name__ == "__main__":
    raise SystemExit(main())
