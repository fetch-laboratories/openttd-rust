#!/usr/bin/env python3
"""Reject Rust exports/declarations without production C++ users (#564).

A reference inside an out-of-class member definition does not count when that
member has an explicit template instantiation and no caller: its definition,
declarations and instantiations are not callers (#856). --report-uncalled lists
exports whose only references sit in any uncalled member; script API members
count as called, through their generated Squirrel bindings. The check enforces
that list against tools/uncalled-export-baseline.txt, which only shrinks: each
row names the owner issue that retires the fixture calling those members (#737).
"""

import argparse
import re
import subprocess
from functools import partial
from pathlib import Path

import baseline_ratchet
import vcs
from code_lines import RUST_EXPORT, without_comments

ROOT = Path(__file__).resolve().parents[1]
BASELINE = "tools/orphan-export-baseline.txt"
UNCALLED = "tools/uncalled-export-baseline.txt"
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


def tracked_sources() -> dict[str, str]:
    paths = subprocess.run(
        ["git", "ls-files", "src", "rust"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.split()
    return {
        path: (ROOT / path).read_text(errors="replace")
        for path in paths
        if Path(path).suffix in {".cpp", ".h", ".hpp", ".inc", ".rs"}
    }


def uncalled(files: dict[str, str]) -> dict[str, str]:
    """Exports held only by uncalled members, each with those members."""
    enforced = violations(files)
    return {
        row.split(" | ")[1]: ", ".join(sorted(members))
        for row, members in scan(files, broad=True).items()
        if row not in enforced
    }


def report_uncalled() -> int:
    """List exports held only by uncalled members (#737 batch O)."""
    rows = uncalled(tracked_sources())
    for name, members in sorted(rows.items()):
        print(f"{name} | {members}")
    print(f"Exports held only by uncalled members: {len(rows)}")
    return 0


def owners(text: str) -> dict[str, str]:
    """Baseline rows: export name to the owner that retires its evidence caller."""
    rows = {}
    for line in text.splitlines():
        if line.strip() and not line.startswith("#"):
            name, _, owner = line.partition(" | ")
            rows[name.strip()] = owner.strip()
    return rows


def uncalled_errors(found: dict[str, str], baseline: str, previous: str) -> list[str]:
    """The uncalled-member baseline only shrinks, and each row names its owner (#737 O).

    A row is an export held only by uncalled members; the owner issue that retires
    the fixture calling those members removes it.
    """
    rows, before = owners(baseline), owners(previous)
    errors = [
        f"new export held only by uncalled members: {name} ({found[name]})"
        for name in sorted(found.keys() - rows.keys())
    ]
    errors += [
        f"remove retired row: {name}" for name in sorted(rows.keys() - found.keys())
    ]
    errors += [
        f"uncalled baseline cannot grow: {name}"
        for name in sorted(rows.keys() - before.keys())
    ]
    errors += [
        f"row names no owner issue: {name}"
        for name, owner in sorted(rows.items())
        if not re.search(r"#\d+", owner)
    ]
    return errors


def check_uncalled(base_ref: str) -> int:
    path = ROOT / UNCALLED
    baseline = path.read_text() if path.exists() else ""
    shown = subprocess.run(
        ["git", "show", f"{base_ref}:{UNCALLED}"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    previous = shown.stdout if shown.returncode == 0 else baseline
    errors = uncalled_errors(uncalled(tracked_sources()), baseline, previous)
    for error in errors:
        print(error)
    print(
        f"Exports held only by uncalled members: {len(owners(baseline))} baselined; {len(errors)} errors"
    )
    return int(bool(errors))


def main() -> int:
    # Parsed in full before any check runs, so --help and a mistyped option
    # cost nothing. baseline_ratchet.main parses sys.argv again: it accepts the
    # same --base-ref, the only option left once --report-uncalled has returned.
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument(
        "--base-ref",
        default="origin/rust-migration",
        help="the revision whose baselines may only shrink (default: %(default)s)",
    )
    parser.add_argument(
        "--report-uncalled",
        action="store_true",
        help="only list exports held by uncalled members, without the check",
    )
    args = parser.parse_args()
    if args.report_uncalled:
        return report_uncalled()
    base = vcs.merge_base(args.base_ref, run=partial(vcs.git, cwd=ROOT))
    return check_uncalled(base) | baseline_ratchet.main(
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
