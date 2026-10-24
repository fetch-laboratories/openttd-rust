#!/usr/bin/env python3
"""Ratchet shared wrappers, orphan Rust exports and repeated service bindings."""

import argparse
import hashlib
import io
import json
import re
import subprocess
import tokenize
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BASELINE = "tools/shared-accessor-baseline.txt"
SHARED = re.compile(
    r"^src/rust/(?:services[^/]*|vehicle_services[^/]*|"
    r"(?:environment|settings|company|engine|presentation)_services\.h)$"
)
HEADER = re.compile(
    r"(?P<header>\b\w+(?:::\w+)*\s*\([^{};]*\)\s*(?:const\s*)?(?:noexcept\s*)?"
    r"|\[[^\]\n]*\]\s*(?:\([^{};]*\)\s*)?(?:(?:noexcept|mutable)\s*)*(?:->[^{};]+)?)\{"
)
OWNER = r"(?:Vehicle|BaseStation|Station|Company)"
WORLD = re.compile(
    r"\b(?:_settings_game|_settings_client|_game_mode|_current_company)\b|\bTimerGame\w*::\w+|\bMap::\w+"
)
TABLE = r"OpenTTD\w*(?:Services|Leaves)\s+\w+\s*(?:=\s*)?\{(.*?)\n\s*};"


def git(*args):
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True)


def definitions(source):
    # Mask comments/strings without changing brace positions or line boundaries.
    source = re.sub(
        r'/\*.*?\*/|//[^\n]*|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'',
        lambda m: re.sub(r"[^\n]", " ", m[0]),
        source,
        flags=re.S,
    )
    for match in HEADER.finditer(source):
        header = " ".join(match["header"].split())
        if header.split("(")[0].strip() in {"if", "while", "switch", "for", "catch"}:
            continue
        body = source[match.end() : source.find("}", match.end())]
        yield header, body, source[: match.start()].count("\n") + 1


def single_read(header, body, helpers, writes=False):
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


def collect(files, originals, writes=False):
    helpers = {
        h.split("(")[0].strip()
        for p, s in files.items()
        if SHARED.fullmatch(p)
        for h, _, _ in definitions(s)
    }
    hits = {}
    for path, source in files.items():
        if SHARED.fullmatch(path):
            continue

        def identity(h, b):
            return h, "".join(b.split()) if h.startswith("[") else ""

        inherited = Counter(
            identity(h, b) for h, b, _ in definitions(originals.get(path, ""))
        )
        occurrences = Counter()
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


def sources(revision=None):
    command = ("ls-tree", "-r", "--name-only", revision) if revision else ("ls-files",)
    return {
        p: git("show", f"{revision}:{p}") if revision else (ROOT / p).read_text()
        for p in git(*command, "--", "src", "rust", "tools").splitlines()
        if Path(p).suffix in {".rs", ".cpp", ".h", ".hpp", ".inc", ".py"}
        and (revision or (ROOT / p).is_file())
    }


def callee_copies(files):
    """Compare simple bound leaves; compound callbacks still require review."""
    native = {
        p: s for p, s in files.items() if Path(p).suffix in {".cpp", ".h", ".hpp"}
    }
    tables = [(p, t) for p, s in native.items() for t in re.findall(TABLE, s, re.S)]
    bound = {n for _, t in tables for n in re.findall(r"\.\w+\s*=\s*(\w+)\s*[,}]", t)}
    candidates = [
        (p, h, b, line)
        for p, s in native.items()
        for h, b, line in definitions(s)
        if h.split("(")[0].strip() in bound
    ] + [
        (p, h, b, line)
        for p, t in tables
        for h, b, line in definitions(t)
        if h.startswith("[]")
    ]
    groups, occurrences = {}, Counter()
    for path, header, body, line in candidates:
        if "noexcept" not in header:
            continue
        parameters = header[header.find("(") + 1 : header.rfind(")")].split(",")
        replacements = {}
        for i, parameter in enumerate(parameters):
            name = re.search(r"\b(\w+)\s*$", parameter)
            if name:
                replacements[name[1]] = f"arg{i}"
        body = re.sub(
            r"\b\w+\b", lambda m, names=replacements: names.get(m[0], m[0]), body
        )
        body = re.sub(
            r"(?:static|reinterpret|const)_cast<[^>]*\*[^>]*>\((arg\d+)(?:\.(?:shell|handle))?\)",
            r"\1",
            body,
        )
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
            groups.setdefault(signature, {})[
                f"{identity} | {occurrences[identity]}"
            ] = line
    return {
        k: v for group in groups.values() if len(group) > 1 for k, v in group.items()
    }


def boundary_hits(files, wrappers, originals):
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
        r"(?m)^\s*(?!return\b|else\b)\w[\w *&]*[\s*&](openttd_rust_\w+)\([^{};]*\);"
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
        if Path(p).suffix in {".cpp", ".h", ".hpp"} and not SHARED.fullmatch(p)
    )
    # Direct bindings may reuse one definition in the canonical shared layer.
    # A port-local definition with the same name must still be checked.
    functions = set(re.findall(r"\b(\w+)\s*\([^{};]*\)\s*noexcept\s*\{", native))
    bindings = Counter()
    for table in re.findall(
        TABLE,
        "\n".join(
            s for p, s in files.items() if Path(p).suffix in {".cpp", ".h", ".hpp"}
        ),
        re.S,
    ):
        bindings.update(
            n for n in re.findall(r"\.\w+\s*=\s*(\w+)\s*[,}]", table) if n in functions
        )
    for name, count in bindings.items():
        if count > 1:
            hits[f"binding | {name} | {count}"] = 1
    return hits


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-ref", help="ratchet against this Git revision")
    args = parser.parse_args()
    pinned = json.loads((ROOT / "migration/baseline.json").read_text())["commit"]
    paths = git("diff", "--name-only", pinned, "--", "src").splitlines()
    files = {
        p: (ROOT / p).read_text()
        for p in paths
        if (ROOT / p).is_file() and Path(p).suffix in {".h", ".hpp", ".cpp"}
    }
    tracked = set(git("ls-tree", "-r", "--name-only", pinned, "src").splitlines())
    originals = {p: git("show", f"{pinned}:{p}") for p in files if p in tracked}
    all_files = sources()
    hits = (
        collect(files, originals)
        | boundary_hits(all_files, files, originals)
        | callee_copies(all_files)
    )
    baseline = set((ROOT / BASELINE).read_text().splitlines())
    previous = baseline
    if args.base_ref:
        tracked_base = git("ls-tree", "-r", "--name-only", args.base_ref).splitlines()
        if BASELINE in tracked_base:
            previous = set(git("show", f"{args.base_ref}:{BASELINE}").splitlines())
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
                if p in base_paths and Path(p).suffix in {".h", ".hpp", ".cpp"}
            }
            reference = {
                p: git("show", f"{pinned}:{p}") for p in wrappers if p in tracked
            }
            previous |= set(boundary_hits(base_files, wrappers, reference))
    for key in previous.copy():
        if key.startswith("binding | "):
            name, count = key.rsplit(" | ", 1)
            previous.update(f"{name} | {n}" for n in range(2, int(count) + 1))
    inherited_callees = callee_copies(sources(args.base_ref or "origin/rust-migration"))
    new, growth = set(hits) - baseline - inherited_callees.keys(), baseline - previous
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
