#!/usr/bin/env python3
"""Reject Rust exports/declarations without production C++ users (#564).

A reference inside an out-of-class member definition does not count when that
member has an explicit template instantiation and no caller: its definition,
declarations and instantiations are not callers (#856). --report-uncalled lists
exports whose only references sit in any uncalled member; script API members
count as called, through their generated Squirrel bindings. The check enforces
that list against tools/uncalled-export-baseline.txt, which only shrinks: each
row names the owner issue that retires the fixture calling those members (#737).

Free functions get the same treatment (#945): a file-scope definition with no
`Class::` qualifier is uncalled when its name appears nowhere outside its own
definition heads and declarations, in any tracked C++ file under src/ or tools/
(tests and consoles included), in a src/ `.ini` table, or anywhere in a tools/
Python file, which may embed a console's C++. --report-uncalled lists
those, and those that only tests and tools call. An export whose only production
references sit in uncalled free functions is a row in
tools/uncalled-function-baseline.txt, which follows the member baseline's rules.
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
UNCALLED_FUNCTIONS = "tools/uncalled-function-baseline.txt"
# Neither defines functions this fork calls by name: vendored code, and the
# script API, whose Squirrel bindings are generated.
NO_FREE_DEFINITIONS = ("src/3rdparty/", "src/script/api/", "src/tests/")
# What may stand before a free function's name: types, templates, pointers.
FREE_PREFIX = re.compile(r"[\w:<>,\s*&\[\]]*")
FREE_SUFFIX = re.compile(r"\s*(?:(?:const|noexcept|override|final)\b\s*|->[^;{}()=]+)*")
BRACES = re.compile(r"[;{}]")
# Control statements, and the program entry, which the runtime calls.
NOT_FUNCTIONS = {"if", "for", "while", "switch", "catch", "return", "sizeof", "main"}
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


def declares(statement: str, member: str, typed: bool = False) -> bool:
    """A declaration statement names a type before the member and calls nothing.

    With typed, a word must precede the name: whitespace alone satisfies the
    member form, which would read an unqualified call such as `\n\tName(x);`
    as a declaration.
    """
    words = statement.split()
    kind = r"[\w:<>,\s*&]*\w[\w:<>,\s*&]*?" if typed else r"[\w:<>,\s*&]+?"
    return (
        bool(words)
        and words[0] not in CALL_FREE
        and re.fullmatch(
            rf"\s*(?:template\s*<[^;{{}}]*>\s*)?{kind}[\s*&]{member}\s*"
            rf"\([^;{{}}]*\)[\w\s=]*;\s*",
            statement,
        )
        is not None
    )


def free_head(text: str) -> str | None:
    """The name a free function definition's head declares, or None.

    A type must precede the unqualified name, and only qualifiers may follow
    its parameter list; `Class::Member(`, macros, initializers and control
    statements are not free function heads.
    """
    if "(" not in text or "=" in text.split("(", 1)[0]:
        return None
    opening = text.index("(")
    named = re.search(r"(?<![\w:~])([A-Za-z_]\w*)\s*$", text[:opening])
    if named is None or named[1] in NOT_FUNCTIONS:
        return None
    prefix = text[: named.start()]
    if not re.search(r"\w", prefix) or not re.search(r"[\s*&]$", prefix):
        return None
    if FREE_PREFIX.fullmatch(prefix) is None:
        return None
    end = closing(text, opening, "(", ")")
    if end < 0 or FREE_SUFFIX.fullmatch(text, end) is None:
        return None
    return named[1]


def free_definitions(source: str) -> list[tuple[str, int, int, int]]:
    """Each file-scope free function definition: name, head start, body start, body end.

    Namespace and `extern` blocks are file scope; the bodies of classes,
    functions and initializers are skipped whole, so members and lambdas are
    not free functions. Preprocessor lines are not part of a head.
    """
    found = []
    head = 0
    index = 0
    while (match := BRACES.search(source, index)) is not None:
        index = match.end()
        if match[0] != "{":
            head = index
            continue
        text = re.sub(r"(?m)^[ \t]*#.*$", "", source[head : match.start()])
        if re.fullmatch(r"\s*(?:namespace\b[\w\s:]*|extern\s*\S*\s*)", text):
            head = index
            continue
        end = closing(source, match.start(), "{", "}")
        if end < 0:
            break
        name = free_head(text)
        if name is not None:
            start = head + len(text) - len(text.lstrip())
            found.append((name, start, match.start(), end))
        head = index = end
    return found


def free_uses(sources: dict[str, str], names: set[str]) -> dict[str, set[str]]:
    """For each name, the files that use it outside definition heads and declarations."""
    used: dict[str, set[str]] = {}
    for path, source in sources.items():
        raw = path.endswith((".ini", ".py"))
        if not raw:
            for _, start, body, _ in free_definitions(source):
                source = source[:start] + " " * (body - start) + source[body:]
        for statement in re.split(r"(?<=[;{}])", source):
            for name in names.intersection(re.findall(r"\b\w+\b", statement)):
                if raw or not declares(statement, name, typed=True):
                    used.setdefault(name, set()).add(path)
    return used


def uncalled_free_functions(
    files: dict[str, str],
) -> tuple[dict[str, str], dict[str, str], dict[str, set[str]]]:
    """Uncalled free functions, those only tests and tools call, and held exports.

    The first two map a function to its definition's `path:line`; the third maps
    each export whose production references all sit inside uncalled free
    functions to those functions.
    """
    sources = {
        path: (
            text
            if path.endswith((".ini", ".py"))
            else re.sub(
                r"\bextern\s+(?=\w)", "", without_comments(text, path, strings=True)
            )
        )
        for path, text in files.items()
        if path.startswith(("src/", "tools/"))
        and Path(path).suffix in {".cpp", ".h", ".hpp", ".inc", ".ini", ".py"}
        and not path.startswith("src/3rdparty/")
    }
    defined: dict[str, str] = {}
    for path, source in sources.items():
        if path.startswith("src/") and not path.startswith(NO_FREE_DEFINITIONS):
            if path.endswith((".ini", ".py")):
                continue
            for name, start, _, _ in free_definitions(source):
                line = source.count("\n", 0, start) + 1
                defined.setdefault(name, f"{path}:{line}")
    used = free_uses(sources, set(defined))
    production = {
        name
        for name, paths in used.items()
        if any(p.startswith("src/") and not p.startswith("src/tests/") for p in paths)
    }
    uncalled = {name: where for name, where in defined.items() if name not in used}
    test_only = {
        name: where
        for name, where in defined.items()
        if name in used and name not in production
    }
    held: dict[str, set[str]] = {}
    before, after = set(), set()
    for path, source in sources.items():
        if not path.startswith("src/") or path.startswith("src/tests/"):
            continue
        if path.endswith((".ini", ".py")):
            continue
        source = DECLARATION.sub("", source)
        before.update(re.findall(NAME, source))
        for name, start, _, end in reversed(free_definitions(source)):
            if name in uncalled:
                for export in re.findall(NAME, source[start:end]):
                    held.setdefault(export, set()).add(name)
                source = source[:start] + " " * (end - start) + source[end:]
        after.update(re.findall(NAME, source))
    exports = {name: held[name] for name in before - after if name in held}
    return uncalled, test_only, exports


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


def tracked_sources(*folders: str) -> dict[str, str]:
    """Tracked C++, Rust, `.ini` and Python sources under folders (default src/ and rust/)."""
    paths = subprocess.run(
        ["git", "ls-files", *(folders or ("src", "rust"))],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.split()
    return {
        path: (ROOT / path).read_text(errors="replace")
        for path in paths
        if Path(path).suffix in {".cpp", ".h", ".hpp", ".inc", ".rs", ".ini", ".py"}
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
    """List exports held only by uncalled members (#737 batch O) and free functions."""
    rows = uncalled(tracked_sources())
    for name, members in sorted(rows.items()):
        print(f"{name} | {members}")
    print(f"Exports held only by uncalled members: {len(rows)}")
    functions, test_only, exports = uncalled_free_functions(
        tracked_sources("src", "tools")
    )
    for name, where in sorted(functions.items()):
        print(f"uncalled free function | {name} | {where}")
    for name, where in sorted(test_only.items()):
        print(f"free function called only by tests or tools | {name} | {where}")
    for name, holders in sorted(exports.items()):
        print(f"{name} | {', '.join(sorted(holders))}")
    print(
        f"Uncalled free functions: {len(functions)}; called only by tests or tools: "
        f"{len(test_only)}; exports held only by uncalled free functions: "
        f"{len(exports)}"
    )
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


def baselines(base_ref: str, relative: str) -> tuple[str, str]:
    """A baseline's text in the tree and at base_ref; an absent base seeds once."""
    path = ROOT / relative
    baseline = path.read_text() if path.exists() else ""
    shown = subprocess.run(
        ["git", "show", f"{base_ref}:{relative}"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    return baseline, shown.stdout if shown.returncode == 0 else baseline


def check_uncalled(base_ref: str) -> int:
    baseline, previous = baselines(base_ref, UNCALLED)
    errors = uncalled_errors(uncalled(tracked_sources()), baseline, previous)
    for error in errors:
        print(error)
    print(
        f"Exports held only by uncalled members: {len(owners(baseline))} baselined; {len(errors)} errors"
    )
    functions, previous = baselines(base_ref, UNCALLED_FUNCTIONS)
    _, _, held = uncalled_free_functions(tracked_sources("src", "tools"))
    found = {name: ", ".join(sorted(holders)) for name, holders in held.items()}
    function_errors = [
        error.replace("uncalled members", "uncalled free functions").replace(
            "uncalled baseline", "uncalled-function baseline"
        )
        for error in uncalled_errors(found, functions, previous)
    ]
    for error in function_errors:
        print(error)
    print(
        f"Exports held only by uncalled free functions: {len(owners(functions))} "
        f"baselined; {len(function_errors)} errors"
    )
    return int(bool(errors or function_errors))


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
