#!/usr/bin/env python3
"""Measure pinned Clippy/Ruff JSON as shrinking item/site debt, never line keys."""

import argparse
import ast
import hashlib
import json
import re
import subprocess
from collections import defaultdict
from functools import lru_cache, partial
from pathlib import Path

from baseline_ratchet import debt_errors, exceeds
from cleanup_contract import Ledger, carry_hints, checked_carries, context, reviewed
from code_lines import without_comments
from rust_syntax import item_source, nodes, syntax_trees
from vcs import git

ROOT = Path(__file__).resolve().parents[1]
CLIPPY = (
    "too_many_lines",
    "too_many_arguments",
    "cognitive_complexity",
    "excessive_nesting",
    "missing_safety_doc",
)
RUFF = ("C901", "PLR0911", "PLR0912", "PLR0913", "PLR0915")
VERSIONS = {"clippy": "1.99.0", "ruff": "0.16.8"}


def compact(text: str) -> str:
    return " ".join(re.findall(r"\w+|[^\w\s]", text))


@lru_cache(maxsize=512)
def rust_items(path: str, text: str) -> tuple[str, list[tuple[int, int, str]]]:
    """Lexical named-item scopes, with balanced bodies and qualified impl names.

    This is an identity parser, not an ABI or Rust semantic parser. Macro sites
    retain their expansion-site token context and occurrence multiplicity.
    """
    clean = without_comments(text, path, strings=True)
    pairs: dict[int, int] = {}
    stack = []
    for match in re.finditer(r"[{}]", clean):
        if match[0] == "{":
            stack.append(match.start())
        elif stack:
            pairs[stack.pop()] = match.end()
    items = []
    pattern = r"\b(fn|mod|impl|trait|struct|enum|macro_rules!)\s*([^;{}]*?)(?=[{;])"
    for match in re.finditer(pattern, clean):
        opening = match.end()
        if opening not in pairs:
            continue
        kind, header = match.groups()
        name = compact(header if kind == "impl" else re.split(r"[\s(<:]", header)[0])
        parents = [name for start, end, name in items if start <= match.start() < end]
        qualified = (parents[-1] + "::" if parents else "") + kind + " " + name
        items.append((match.start(), pairs[opening], qualified))
    return clean, items


def rust_site(path: str, text: str, start: int, end: int, lint: str) -> tuple[str, int]:
    clean, items = rust_items(path, text)
    containing = [(a, b, n) for a, b, n in items if a <= start < b]
    item = containing[-1][2] if containing else "module"
    # Function-level metrics retain identity when the signature/body grows.
    if lint in CLIPPY[:3] or lint == "missing_safety_doc":
        site = "item"
    else:
        site = compact(clean[start:end])
        # Unsafe/nesting block size must not turn a magnitude increase into a
        # different allowance. The initial statement identifies the operation.
        if lint in ("excessive_nesting", "multiple_unsafe_ops_per_block"):
            site = site.split(";", 1)[0][:160]
        if lint == "undocumented_unsafe_blocks":
            site = compact(clean[start : start + 180]).split(";", 1)[0][:160]
    depth = 1
    if lint == "excessive_nesting" and containing:
        prefix = clean[containing[-1][0] : start]
        scope = clean[start:end]
        level = prefix.count("{") - prefix.count("}")
        depth = max(depth, level)
        for token in re.findall(r"[{}]", scope):
            level += 1 if token == "{" else -1
            depth = max(depth, level)
    fingerprint = hashlib.sha256(site.encode()).hexdigest()[:20]
    return f"{path} | {item} | {lint} | {fingerprint}", depth


def clippy_rows(raw: str, root: Path) -> dict[str, list[int]]:
    sites: dict[str, dict[tuple[str, int, int], int]] = defaultdict(dict)
    for line in raw.splitlines():
        message = json.loads(line)
        if message.get("reason") != "compiler-message":
            continue
        diagnostic = message["message"]
        lint = (diagnostic.get("code") or {}).get("code", "").removeprefix("clippy::")
        if lint not in CLIPPY:
            continue
        for span in diagnostic["spans"]:
            if not span["is_primary"]:
                continue
            path = span["file_name"]
            if not path.startswith("rust/") or not (root / path).is_file():
                raise ValueError(f"unclassified Clippy source: {path}")
            data = (root / path).read_bytes()
            start, end = span["byte_start"], span["byte_end"]
            # rustc byte offsets are UTF-8; lexer offsets are Unicode characters.
            a, b = len(data[:start].decode()), len(data[:end].decode())
            key, magnitude = rust_site(path, data.decode(), a, b, lint)
            match = re.search(
                r"\((\d+)/\d+\)|contains (\d+) unsafe operations", diagnostic["message"]
            )
            if match:
                magnitude = int(next(value for value in match.groups() if value))
            sites[key][(path, start, end)] = magnitude
    return {key: sorted(rows.values()) for key, rows in sorted(sites.items())}


def python_item(tree: ast.AST, row: int) -> str:
    names = []
    for node in ast.walk(tree):
        if isinstance(
            node, (ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef)
        ) and node.lineno <= row <= (node.end_lineno or node.lineno):
            names.append((node.lineno, node.name))
    return ".".join(name for _, name in sorted(names))


def ruff_rows(raw: str, root: Path) -> dict[str, list[int]]:
    result: dict[str, list[int]] = defaultdict(list)
    for diagnostic in json.loads(raw):
        path = str(Path(diagnostic["filename"]).relative_to(root))
        tree = ast.parse((root / path).read_text())
        item = python_item(tree, diagnostic["location"]["row"])
        message = diagnostic["message"]
        numbers = re.search(r"\((\d+)\s*>\s*\d+\)", message)
        if not numbers:
            raise ValueError(f"unmeasured Ruff diagnostic: {message}")
        key = f"{path} | {item} | {diagnostic['code']}"
        result[key].append(int(numbers[1]))
    return {key: sorted(values) for key, values in sorted(result.items())}


def collect(kind: str, root: Path) -> tuple[str, str]:
    if kind == "ruff":
        command = ["uvx", "--from", f"ruff=={VERSIONS[kind]}", "ruff"]
        version = subprocess.check_output([*command, "--version"], text=True).strip()
        run = subprocess.run(
            [
                *command,
                "check",
                "--no-cache",
                "--ignore-noqa",
                "--isolated",
                "--select",
                ",".join(RUFF),
                "--output-format",
                "json",
                "tools/",
            ],
            cwd=root,
            capture_output=True,
            text=True,
        )
    else:
        # Reuse the migration driver's host toolchain discovery unchanged.
        import migration

        env = migration.environment()
        env["CARGO_TARGET_DIR"] = str(root / "build-rust/cargo-lint-ratchet")
        env["CLIPPY_CONF_DIR"] = str(root / "migration/lints")
        version = subprocess.check_output(
            ["rustc", "--version"], env=env, text=True
        ).strip()
        run = subprocess.run(
            [
                "cargo",
                "clippy",
                "--workspace",
                "--all-targets",
                "--locked",
                "--message-format=json",
                "--",
                "-A",
                "warnings",
                *[
                    part
                    for lint in CLIPPY
                    for part in ("--force-warn", "clippy::" + lint)
                ],
            ],
            cwd=root,
            env=env,
            capture_output=True,
            text=True,
        )
    if VERSIONS[kind] not in version or run.returncode not in (
        (0, 1) if kind == "ruff" else (0,)
    ):
        raise ValueError(f"{kind} version/collection failed: {version}\n{run.stderr}")
    return run.stdout, version


def lint_of(key: str) -> list[str]:
    return key.split(" | ")[2:3]


def fits(values: list[int], limit: list[int]) -> bool:
    return not exceeds(values, limit)


def item_text(kind: str, path: str, text: str, item: str) -> str | None:
    """Source text of every item with this key identity, or None for modules."""
    if kind == "clippy":
        _, items = rust_items(path, text)
        tree = list(nodes(syntax_trees({path: text})[path]))
        parts = []
        for start, end, name in items:
            if name != item:
                continue
            a, b = len(text[:start].encode()), len(text[:end].encode())
            targets = [
                node
                for node in tree
                if node.type.endswith("_item")
                and node.start_byte <= a < node.end_byte == b
            ]
            if not targets:
                return None
            target = max(targets, key=lambda node: node.start_byte)
            parts.append(item_source(target, text))
    else:
        tree = ast.parse(text)
        parts = [
            "".join(
                text.splitlines(keepends=True)[
                    min(
                        (decorator.lineno for decorator in node.decorator_list),
                        default=node.lineno,
                    )
                    - 1 : node.lineno - 1
                ]
            )
            + (ast.get_source_segment(text, node) or "")
            for node in ast.walk(tree)
            if isinstance(node, (ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef))
            and python_item(tree, node.lineno) == item
        ]
    return "\n".join(parts) if parts and item not in ("module", "") else None


def unchanged_item(kind: str, site: str, revision: str) -> bool:
    """Whether the site's item text at the reviewed revision equals head's."""
    path, item = site.split(" | ")[:2]
    try:
        old = git("show", f"{revision}:{path}")
    except subprocess.CalledProcessError:
        return False
    current = ROOT / path
    before = item_text(kind, path, old, item)
    return (
        before is not None
        and current.is_file()
        and before == item_text(kind, path, current.read_text(), item)
    )


def reviewed_debt(
    kind: str,
    baseline: dict[str, list[int]],
    previous: dict[str, list[int]],
    rows: dict[str, list[int]] | None = None,
) -> dict[str, list[int]]:
    """Allowed head debt: base rows, valid carries and reviewed lint contracts."""
    measured = baseline if rows is None else rows
    growth = {
        key: values
        for key, values in baseline.items()
        if exceeds(values, previous.get(key, []))
    }
    if not growth and measured.keys() <= previous.keys():
        return previous
    author, proposals, comments, head = context()
    ledger = Ledger(kind, previous, baseline, lint_of, fits)
    carried = checked_carries(proposals, ledger)
    allowed = dict(previous)
    allowed.update({new: previous[old] for new, old in carried.items()})
    for site, values in growth.items():
        if site in carried:
            continue
        for proposal in proposals:
            expected = {
                "kind": "lint",
                "tool": kind,
                "site": site,
                "before": previous.get(site, []),
                "after": values,
            }
            if any(proposal.get(key) != value for key, value in expected.items()):
                continue
            required = (
                "operation",
                "source_contract",
                "reason",
                "retiring_owner",
                "retirement_condition",
            )
            if all(
                isinstance(proposal.get(key), str) and proposal[key].strip()
                for key in required
            ) and reviewed(
                proposal, comments, author, head, partial(unchanged_item, kind, site)
            ):
                allowed[site] = values
                print(
                    json.dumps(
                        {"accepted_at": head, "lint_exception": proposal},
                        sort_keys=True,
                    )
                )
    carry_hints(
        ledger._replace(after=measured),
        previous.keys() - measured.keys() - set(carried.values()),
        [key for key in measured if key not in allowed],
    )
    return allowed


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("kind", choices=VERSIONS)
    parser.add_argument("--base-ref", default="origin/rust-migration")
    parser.add_argument(
        "--initialize",
        action="store_true",
        help="explicit initial inventory; never used by CI",
    )
    parser.add_argument(
        "--input", type=Path, help="replay compiler JSON for diagnostic investigation"
    )
    args = parser.parse_args()
    kind = args.kind
    raw, version = (
        (args.input.read_text(), VERSIONS[kind]) if args.input else collect(kind, ROOT)
    )
    rows = clippy_rows(raw, ROOT) if kind == "clippy" else ruff_rows(raw, ROOT)
    path = f"migration/{kind}-debt.json"
    if args.initialize:
        (ROOT / path).write_text(
            json.dumps(
                {
                    "schema": 1,
                    "version": version,
                    "inventory_head": git("rev-parse", "HEAD").strip(),
                    "sites": rows,
                },
                indent=2,
            )
            + "\n"
        )
        print(
            f"Initialized {kind}: {sum(map(len, rows.values()))} distinct diagnostics"
        )
        return 0
    baseline = json.loads((ROOT / path).read_text())
    tracked = git("ls-tree", "--name-only", args.base_ref, path).strip()
    previous = (
        json.loads(git("show", f"{args.base_ref}:{path}")) if tracked else baseline
    )
    allowed = reviewed_debt(kind, baseline["sites"], previous["sites"], rows)
    errors = debt_errors(rows, baseline["sites"], allowed)
    if baseline["version"] != version or previous["version"] != version:
        errors.append("compiler/linter version differs from the recorded inventory")
    for error in errors:
        print(error)
    print(f"{kind}: {sum(map(len, rows.values()))} diagnostics; {len(errors)} errors")
    return int(bool(errors))


if __name__ == "__main__":
    raise SystemExit(main())
