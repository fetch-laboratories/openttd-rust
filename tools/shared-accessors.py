#!/usr/bin/env python3
"""Ratchet shared wrappers, orphan Rust exports and repeated service bindings."""

import argparse
import hashlib
import io
import json
import re
import tokenize
from collections import Counter
from collections.abc import Iterator
from pathlib import Path

import vcs

ROOT = Path(__file__).resolve().parents[1]
BASELINE = "tools/shared-accessor-baseline.txt"
SHARED = re.compile(
    r"^src/rust/(?:services[^/]*|vehicle_services[^/]*|"
    r"(?:environment|settings|company|company_identity|engine|presentation)_services\.h)$"
)
HEADER = re.compile(
    r"(?P<header>\b\w+(?:::\w+)*\s*\([^{};]*\)\s*(?:const\s*)?(?:noexcept\s*)?"
    r"|\[[^\]\n]*\]\s*(?:\([^{};]*\)\s*)?(?:(?:noexcept|mutable)\s*)*(?:->[^{};]+)?)\{"
)
OWNER = r"(?:Vehicle|BaseStation|Station|Company)"
WORLD = re.compile(
    r"\b(?:_settings_game|_settings_client|_game_mode|_current_company)\b|\bTimerGame\w*::\w+|\bMap::\w+"
)
# A value-initialized `OpenTTDXServices name{};` opens no initializer list.
TABLE = r"OpenTTD\w*(?:Services|Leaves)\s+\w+\s*(?:=\s*)?\{(?!\s*\})(.*?)\n\s*};"


def git(*args: str) -> str:
    return vcs.git(*args, cwd=ROOT)


def definitions(
    source: str, with_literals: bool = False
) -> Iterator[tuple[str, str, int]]:
    # Mask comments/strings without changing brace positions or line boundaries.
    literals: dict[int, str] = {}

    def mask(match: re.Match[str]) -> str:
        if with_literals and match[0][0] in "\"'":
            literals[match.start()] = (
                "LITERAL_" + hashlib.sha256(match[0].encode()).hexdigest().upper()
            )
        return re.sub(r"[^\n]", " ", match[0])

    source = re.sub(
        r'/\*.*?\*/|//[^\n]*|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'',
        mask,
        source,
        flags=re.S,
    )
    for match in HEADER.finditer(source):
        header = " ".join(match["header"].split())
        if header.split("(")[0].strip() in {"if", "while", "switch", "for", "catch"}:
            continue
        start, end = match.end(), source.find("}", match.end())
        body = source[start:end]
        for position, literal in reversed(literals.items()):
            if start <= position < end:
                offset = position - start
                body = body[:offset] + literal + body[offset:]
        yield header, body, source[: match.start()].count("\n") + 1


def single_read(
    header: str, body: str, helpers: set[str], writes: bool = False
) -> bool:
    statements = [s.strip() for s in body.split(";") if s.strip()]
    if not statements or any("{" in s or "}" in s for s in statements):
        return False
    aliases = []
    for statement in statements[:-1]:
        alias = re.fullmatch(r"(?:const )?(\w+)\s*[*&]?\s*(\w+)\s*=\s*(.+)", statement)
        if not alias or not re.search(rf"\b{OWNER}\b", alias[1] + " " + alias[3]):
            return False
        aliases.append(alias[2])
    expression = re.sub(r"^return\s+", "", statements[-1])
    if writes:
        assignment = re.fullmatch(r"([^=]+?)(?<![!<>])=(?!=).+", expression)
        if not assignment:
            return False
        expression = assignment[1]
    if re.search(r"(?<![=!<>])=(?!=)|\+\+|--|\b(?:new|delete|throw)\b", expression):
        return False
    reads = len(WORLD.findall(expression))
    owners = aliases + re.findall(rf"\b{OWNER}[ *&]+(?:const[ *&]+)?(\w+)", header)
    reads += sum(
        len(re.findall(rf"\b{owner}\s*(?:->|\.)\s*\w+(?!\w|\s*\()", expression))
        for owner in owners
    )
    reads += len(
        re.findall(rf"\b{OWNER}::(?:Get|GetIfValid)\([^;]*?->\s*\w+", expression)
    )
    reads += len(
        re.findall(
            rf"(?:static|reinterpret)_cast<[^>]*\b{OWNER}\s*\*>\([^)]*\)->\s*\w+",
            expression,
        )
    )
    if reads == 1:
        return True
    # Pass-through includes boundary casts/base() but no extra service calls.
    calls = set(re.findall(r"\b(\w+)\s*\(", expression))
    return bool(calls & helpers) and calls <= helpers | {"base", "to_underlying"}


def collect(
    files: dict[str, str], originals: dict[str, str], writes: bool = False
) -> dict[str, int]:
    helpers = {
        h.split("(")[0].strip()
        for p, s in files.items()
        if SHARED.fullmatch(p)
        for h, _, _ in definitions(s)
    }
    hits: dict[str, int] = {}
    for path, source in files.items():
        if SHARED.fullmatch(path):
            continue

        def identity(h: str, b: str) -> tuple[str, str]:
            return h, "".join(b.split()) if h.startswith("[") else ""

        inherited = Counter(
            identity(h, b) for h, b, _ in definitions(originals.get(path, ""))
        )
        occurrences: Counter[str] = Counter()
        for header, body, line in definitions(source):
            key = identity(header, body)
            if inherited[key]:
                inherited[key] -= 1
            elif single_read(header, body, helpers, writes):
                digest = hashlib.sha256("".join(body.split()).encode()).hexdigest()[:16]
                key = ("write | " if writes else "") + f"{path} | {header} | {digest}"
                occurrences[key] += 1
                hits[f"{key} | {occurrences[key]}"] = line
    return hits


# Compiled fork glue includes `.inc` table fragments, such as src/rust/newgrf_loading_services.inc.
NATIVE_SUFFIXES = {".cpp", ".h", ".hpp", ".inc"}


def sources(revision: str | None = None) -> dict[str, str]:
    command = ("ls-tree", "-r", "--name-only", revision) if revision else ("ls-files",)
    return {
        p: git("show", f"{revision}:{p}") if revision else (ROOT / p).read_text()
        for p in git(*command, "--", "src", "rust", "tools").splitlines()
        if Path(p).suffix in {".rs", ".cpp", ".h", ".hpp", ".inc", ".py"}
        and (revision or (ROOT / p).is_file())
    }


def callee_copies(files: dict[str, str]) -> dict[str, int]:
    """Compare simple bound leaves; compound callbacks still require review."""
    native = {p: s for p, s in files.items() if Path(p).suffix in NATIVE_SUFFIXES}
    tables = [(p, t) for p, s in native.items() for t in re.findall(TABLE, s, re.S)]
    bound = {n for _, t in tables for n in re.findall(r"\.\w+\s*=\s*(\w+)\s*[,}]", t)}
    candidates = [
        (p, h, b, line)
        for p, s in native.items()
        for h, b, line in definitions(s, with_literals=True)
        if h.split("(")[0].strip() in bound
    ] + [
        (p, h, b, line)
        for p, t in tables
        for h, b, line in definitions(t, with_literals=True)
        if h.startswith("[]")
    ]
    groups: dict[str, dict[str, int]]
    occurrences: Counter[str]
    groups, occurrences = {}, Counter()
    for path, header, body, line in candidates:
        if "noexcept" not in header:
            continue
        parameters = header[header.find("(") + 1 : header.rfind(")")].split(",")
        replacements: dict[str, str] = {}
        receiver_types: dict[str, str] = {}
        for i, parameter in enumerate(parameters):
            name = re.search(r"\b(\w+)\s*$", parameter)
            if name:
                replacements[name[1]] = f"arg{i}"
                receiver_types[f"arg{i}"] = parameter[: name.start()].strip()
        body = re.sub(
            r"\b\w+\b", lambda m, names=replacements: names.get(m[0], m[0]), body
        )
        for cast in re.finditer(
            r"(?:static|reinterpret|const)_cast<([^>]*\*[^>]*)>"
            r"\((arg\d+)(?:\.(?:shell|handle))?\)",
            body,
        ):
            receiver_types[cast[2]] = cast[1]
        body = re.sub(
            r"(?:static|reinterpret|const)_cast<[^>]*\*[^>]*>\((arg\d+)(?:\.(?:shell|handle))?\)",
            r"\1",
            body,
        )
        for vehicle in re.finditer(r"\bVehicleOf\((arg\d+)(?:\.handle)?\)", body):
            receiver_types[vehicle[1]] = "Vehicle *"
        body = re.sub(r"\bVehicleOf\((arg\d+)(?:\.handle)?\)", r"\1", body)
        statements = [s.strip() for s in body.split(";") if s.strip()]
        while len(statements) > 1:
            # Scalar declarations can narrow; only fold pointer/auto aliases.
            alias = re.fullmatch(
                r"(?:const )?(?:[\w:<>]+[ *]*\*\s*|auto[ *&]+)(\w+)\s*=\s*(arg\d+)",
                statements.pop(0),
            )
            if not alias:
                break
            statements = [re.sub(rf"\b{alias[1]}\b", alias[2], s) for s in statements]
        else:
            if not statements:
                continue
            expression = re.sub(r"^return\s+", "", statements[0])
            call = re.fullmatch(r"([\w:]+|arg\d+->\w+)\(([^()]*)\)", expression)
            if not call:
                continue
            arguments = [a.strip() for a in call[2].split(",") if a.strip()]
            if any(
                not re.fullmatch(
                    r"arg\d+|[A-Z][A-Z_0-9]*|\w+::\w+|\d+|true|false|nullptr", a
                )
                for a in arguments
            ):
                continue
            signature = f"{call[1]}({','.join(arguments)})"
            identity = f"callee | {signature} | {path} | {header}"
            occurrences[identity] += 1
            receiver = re.match(r"(arg\d+)->", call[1])
            receiver_type = receiver_types.get(receiver[1], "") if receiver else ""
            # Member names identify a callee only within the receiver's concrete type.
            receiver_type = re.sub(r"^(?:(?:const|volatile)\s+)+", "", receiver_type)
            receiver_type = re.sub(
                r"(?:\s*[*&]|\s+(?:const|volatile))+\s*$", "", receiver_type
            )
            receiver_type = "".join(receiver_type.split())
            group = f"{receiver_type}::{signature}" if receiver_type else signature
            groups.setdefault(group, {})[f"{identity} | {occurrences[identity]}"] = line
    return {
        k: v for group in groups.values() if len(group) > 1 for k, v in group.items()
    }


def boundary_hits(
    files: dict[str, str], wrappers: dict[str, str], originals: dict[str, str]
) -> dict[str, int]:
    hits = collect(wrappers, originals, writes=True)
    combined = "\n".join(
        " ".join(
            t.string
            for t in tokenize.generate_tokens(io.StringIO(s).readline)
            if t.type != tokenize.COMMENT  # Python strings stay generated references.
        )
        if p.endswith(".py")
        else re.sub(r"//[^\n]*|/\*.*?\*/", "", s, flags=re.S)
        for p, s in files.items()
    )
    counts = Counter(re.findall(r"\bopenttd_rust_\w+\b", combined))
    prototype = (
        r'(?m)^\s*(?:extern\s+"C"\s+)?(?!return\b|else\b)\w[\w *&]*[\s*&]'
        r"(openttd_rust_\w+)\([^{};]*\);"
    )
    declarations = Counter(re.findall(prototype, combined))
    for path, source in files.items():
        if path.endswith(".rs"):
            for name in re.findall(
                r"#\[unsafe\(no_mangle\)\][^;{}]*?\bfn (openttd_rust_\w+)\(", source
            ):
                if counts[name] == 1 + declarations[name]:
                    hits[f"orphan | {name}"] = 1
    native = "\n".join(
        s
        for p, s in files.items()
        if Path(p).suffix in NATIVE_SUFFIXES and not SHARED.fullmatch(p)
    )
    # Direct bindings may reuse one definition in the canonical shared layer.
    # A port-local definition with the same name must still be checked.
    functions = set(re.findall(r"\b(\w+)\s*\([^{};]*\)\s*noexcept\s*\{", native))
    bindings: Counter[str] = Counter()
    for table in re.findall(
        TABLE,
        "\n".join(s for p, s in files.items() if Path(p).suffix in NATIVE_SUFFIXES),
        re.S,
    ):
        bindings.update(
            n for n in re.findall(r"\.\w+\s*=\s*(\w+)\s*[,}]", table) if n in functions
        )
    for name, count in bindings.items():
        if count > 1:
            hits[f"binding | {name} | {count}"] = 1
    return hits


def identity(key: str) -> str | None:
    """`path | header` of an accessor row, read or write; bindings and orphans have none."""
    parts = key.removeprefix("write | ").split(" | ")
    if parts[0] in {"binding", "orphan"} or len(parts) < 4:
        return None
    return " | ".join(parts[:-2])


def reclassified(
    growth: set[str],
    baseline: set[str],
    base_rows: int,
    base_files: dict[str, str],
    current_files: dict[str, str],
) -> set[str]:
    """Added rows for bodies whose path and header existed at the base revision.

    Accepted only while the baseline's total row count does not grow, so a
    changed body can take a retired row's place but never add an exemption.
    """
    if len(baseline) > base_rows:
        return set()
    headers: dict[str, Counter[str]] = {}
    current_headers: dict[str, Counter[str]] = {}
    rows = Counter(identity(key) for key in baseline)
    accepted = set()
    for key in growth:
        name = identity(key)
        if name is None:
            continue
        path, header = name.split(" | ", 1)
        if path not in headers:
            headers[path] = Counter(
                h for h, _, _ in definitions(base_files.get(path, ""))
            )
            current_headers[path] = Counter(
                h for h, _, _ in definitions(current_files.get(path, ""))
            )
        # Repeated lambda headers identify occurrences, not one shared body.
        # Read and write rows consume the same pre-existing occurrence budget.
        if (
            rows[name] <= headers[path][header]
            and current_headers[path][header] <= headers[path][header]
        ):
            accepted.add(key)
    return accepted


def scan(
    pinned: str, revision: str | None = None, all_files: dict[str, str] | None = None
) -> dict[str, int]:
    """Every hit of the working tree, or of `revision` (sources preloaded) under the same rules."""
    if revision:
        paths = git("diff", "--name-only", pinned, revision, "--", "src").splitlines()
        present = set(git("ls-tree", "-r", "--name-only", revision, "src").splitlines())
        files = {
            p: git("show", f"{revision}:{p}")
            for p in paths
            if p in present and Path(p).suffix in NATIVE_SUFFIXES
        }
    else:
        paths = git("diff", "--name-only", pinned, "--", "src").splitlines()
        files = {
            p: (ROOT / p).read_text()
            for p in paths
            if (ROOT / p).is_file() and Path(p).suffix in NATIVE_SUFFIXES
        }
    tracked = set(git("ls-tree", "-r", "--name-only", pinned, "src").splitlines())
    originals = {p: git("show", f"{pinned}:{p}") for p in files if p in tracked}
    if all_files is None:
        all_files = sources(revision)
    return (
        collect(files, originals)
        | boundary_hits(all_files, files, originals)
        | callee_copies(all_files)
    )


def fork_point(base_ref: str | None) -> str | None:
    """The merge base of base_ref and HEAD, when a base is given."""
    return base_ref and vcs.merge_base(base_ref, run=git, announce=True)


def main() -> bool:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-ref", help="ratchet against this Git revision")
    args = parser.parse_args()
    args.base_ref = fork_point(args.base_ref)
    pinned = json.loads((ROOT / "migration/baseline.json").read_text())["commit"]
    tracked = set(git("ls-tree", "-r", "--name-only", pinned, "src").splitlines())
    all_files = sources()
    hits = scan(pinned, all_files=all_files)
    baseline = set((ROOT / BASELINE).read_text().splitlines())
    previous = baseline
    base_rows = len(baseline)
    if args.base_ref:
        tracked_base = git("ls-tree", "-r", "--name-only", args.base_ref).splitlines()
        if BASELINE in tracked_base:
            previous = set(git("show", f"{args.base_ref}:{BASELINE}").splitlines())
            base_rows = len(previous)
        if not any(
            k.startswith(("binding |", "write |", "orphan |")) for k in previous
        ):
            base_files = sources(args.base_ref)
            base_paths = git(
                "diff", "--name-only", pinned, args.base_ref, "--", "src"
            ).splitlines()
            wrappers = {
                p: s
                for p, s in base_files.items()
                if p in base_paths and Path(p).suffix in NATIVE_SUFFIXES
            }
            reference = {
                p: git("show", f"{pinned}:{p}") for p in wrappers if p in tracked
            }
            previous |= set(boundary_hits(base_files, wrappers, reference))
    for key in previous.copy():
        if key.startswith("binding | "):
            name, count = key.rsplit(" | ", 1)
            previous.update(f"{name} | {n}" for n in range(2, int(count) + 1))
    base_sources = sources(args.base_ref or "origin/rust-migration")
    inherited_callees = callee_copies(base_sources)
    new, growth = set(hits) - baseline - inherited_callees.keys(), baseline - previous
    if growth and args.base_ref:
        # Rows the base already had under these rules are newly visible, not new glue.
        growth -= scan(pinned, args.base_ref, base_sources).keys()
    accepted = (
        reclassified(growth, baseline, base_rows, base_sources, all_files)
        if growth and args.base_ref
        else set()
    )
    growth -= accepted
    for key in sorted(accepted):
        print(f"{BASELINE}: reclassified pre-existing accessor: {key}")
    for key in new:
        print(f"{key.split(' | ')[0]}:{hits[key]}: new shared accessor: {key}")
    stale = baseline - set(hits)
    for label, keys in [("addition forbidden", growth), ("remove retired", stale)]:
        for key in sorted(keys):
            print(f"{BASELINE}: {label}: {key}")
    print("Shared accessors:", len(hits), "hits;", len(new | growth | stale), "errors")
    return bool(new or growth or stale)


if __name__ == "__main__":
    raise SystemExit(main())
