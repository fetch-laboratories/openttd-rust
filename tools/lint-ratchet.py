#!/usr/bin/env python3
"""Measure pinned Clippy/Ruff JSON as item debt that may not grow, never line keys.

Only growth fails: a new item over its threshold, or more or larger diagnostics
on an item than the merge base allows. Improvements and one-to-one renames that
do not grow pass without any record; `--write` records improvements.
"""

import argparse
import ast
import hashlib
import json
import re
import subprocess
from collections import defaultdict
from collections.abc import Callable
from functools import lru_cache, partial
from pathlib import Path

from baseline_ratchet import debt_errors, exceeds
from cleanup_contract import Ledger, carry_hints, checked_carries, context, reviewed
from code_lines import without_comments
from rust_syntax import item_source, nodes, syntax_trees
from vcs import git

ROOT = Path(__file__).resolve().parents[1]
# Function length, nesting and cognitive complexity are not collected: a
# faithful port keeps the original's function structure (steering 33).
CLIPPY = ("too_many_arguments", "missing_safety_doc")
ITEM = hashlib.sha256(b"item").hexdigest()[:20]
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


def rust_site(path: str, text: str, start: int, lint: str) -> str:
    """Key a diagnostic by its innermost named item; lines and bodies may change."""
    _, items = rust_items(path, text)
    containing = [name for a, b, name in items if a <= start < b]
    item = containing[-1] if containing else "module"
    return f"{path} | {item} | {lint} | {ITEM}"


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
            a = len(data[:start].decode())
            key = rust_site(path, data.decode(), a, lint)
            match = re.search(r"\((\d+)/\d+\)", diagnostic["message"])
            sites[key][(path, start, end)] = int(match[1]) if match else 1
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


@lru_cache(maxsize=4096)
def item_names(kind: str, path: str, revision: str | None) -> frozenset[str]:
    """Named items in a file at a revision, or in the working tree for None."""
    if revision is None:
        file = ROOT / path
        if not file.is_file():
            return frozenset()
        text = file.read_text()
    else:
        try:
            text = git("show", f"{revision}:{path}")
        except subprocess.CalledProcessError:
            return frozenset()
    if kind == "clippy":
        return frozenset(name for _, _, name in rust_items(path, text)[1])
    tree = ast.parse(text)
    return frozenset(
        python_item(tree, node.lineno)
        for node in ast.walk(tree)
        if isinstance(node, (ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef))
    )


def item_exists(kind: str, key: str, revision: str | None) -> bool:
    """Whether a row's item exists; module-level rows always do."""
    path, item = key.split(" | ")[:2]
    return item in ("module", "") or item in item_names(kind, path, revision)


def paired_renames(
    kind: str, ledger: Ledger, taken: dict[str, str], base: str | None
) -> dict[str, str]:
    """Pair a renamed or moved item's base row with its head row.

    A pair needs evidence of a rename, not just a retired and a new
    diagnostic: the old item existed at base and is gone at head, and the new
    item did not exist at base. Rows pair one-to-one within the same lint,
    first by item name (a move), then within one file (a rename). The head row
    may not grow over the base row. Returns `new -> old`; declared carries in
    `taken` keep precedence, and anything else needs a declared carry.
    """
    if base is None:
        return {}
    pairs = dict(taken)
    scopes: tuple[Callable[[str], tuple[str, ...]], ...] = (
        lambda key: (*lint_of(key), *key.split(" | ")[1:2]),
        lambda key: (*lint_of(key), *key.split(" | ")[:1]),
    )
    for scope in scopes:
        olds: dict[tuple[str, ...], list[str]] = defaultdict(list)
        news: dict[tuple[str, ...], list[str]] = defaultdict(list)
        used = set(pairs.values())
        for key in ledger.before:
            if (
                key not in ledger.after
                and key not in used
                and item_exists(kind, key, base)
                and not item_exists(kind, key, None)
            ):
                olds[scope(key)].append(key)
        for key in ledger.after:
            if (
                key not in ledger.before
                and key not in pairs
                and not item_exists(kind, key, base)
            ):
                news[scope(key)].append(key)
        for group, added in news.items():
            retired = olds.get(group, [])
            if len(added) == len(retired) == 1 and ledger.fits(
                ledger.after[added[0]], ledger.before[retired[0]]
            ):
                pairs[added[0]] = retired[0]
    return {new: old for new, old in pairs.items() if new not in taken}


def reviewed_debt(
    kind: str,
    baseline: dict[str, list[int]],
    previous: dict[str, list[int]],
    rows: dict[str, list[int]] | None = None,
    base: str | None = None,
) -> dict[str, list[int]]:
    """Allowed head debt: base rows, renamed rows and reviewed lint contracts.

    A renamed row's allowance moves to its new key; the old key loses it, so
    one diagnostic never holds two allowances.
    """
    measured = baseline if rows is None else rows

    def grown(allowed: dict[str, list[int]]) -> dict[str, list[int]]:
        return {
            key: values
            for source in (baseline, measured)
            for key, values in source.items()
            if exceeds(values, allowed.get(key, []))
        }

    if not grown(previous):
        return previous
    ledger = Ledger(kind, previous, measured, lint_of, fits)
    author, proposals, comments, head = context()
    carried = checked_carries(proposals, ledger)
    renamed = paired_renames(kind, ledger, carried, base)
    allowed = dict(previous)
    for new, old in {**carried, **renamed}.items():
        allowed[new] = previous[old]
        print(f"carried: {old} -> {new}")
    for old in {**carried, **renamed}.values():
        allowed.pop(old, None)
    for site, values in grown(allowed).items():
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
    paired = {**carried, **renamed}
    carry_hints(
        ledger,
        previous.keys() - measured.keys() - set(paired.values()),
        [key for key in measured if key not in previous and key not in paired],
    )
    return allowed


def merge_base(base_ref: str) -> str:
    """The base the change started from, not a base tip that moved since.

    CI passes the base branch's current tip; comparing against it would count
    base's own later changes as this change's growth. Without a common
    ancestor there is no inventory to compare against, so the check stops.
    """
    try:
        return git("merge-base", "HEAD", base_ref).strip()
    except subprocess.CalledProcessError as error:
        raise SystemExit(
            f"no common ancestor of HEAD and {base_ref}; fetch full history"
            " (git fetch --unshallow) or pass the branch's real base"
        ) from error


def check(
    kind: str,
    rows: dict[str, list[int]],
    recorded: dict[str, list[int]],
    previous: dict[str, list[int]],
    base: str | None,
) -> tuple[list[str], list[str]]:
    """Return growth errors in the measured and in the recorded debt.

    Base rows whose item no longer exists at base are stale and allow nothing
    new. The recorded file may keep stale or improved base rows unchanged, but
    not a renamed item's old row: a rename is recorded in the same change.
    """
    live = (
        previous
        if base is None
        else {key: v for key, v in previous.items() if item_exists(kind, key, base)}
    )
    changed = {key: v for key, v in recorded.items() if previous.get(key) != v}
    allowed = reviewed_debt(kind, changed, live, rows, base)
    renamed_from = live.keys() - allowed.keys()
    kept = {key: v for key, v in previous.items() if key not in renamed_from}
    measured = debt_errors(rows, {}, allowed)
    stored = debt_errors({}, recorded, {**kept, **allowed})
    for key in sorted(renamed_from & recorded.keys()):
        stored.append(f"rename not recorded: {key}; run with --write")
    return measured, stored


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
        "--write",
        action="store_true",
        help="record measured debt, renames included, when nothing grew",
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
    families, parts = (CLIPPY, 4) if kind == "clippy" else (RUFF, 3)

    def collected(sites: dict[str, list[int]]) -> dict[str, list[int]]:
        malformed = [key for key in sites if len(key.split(" | ")) != parts]
        if malformed:
            raise SystemExit(f"malformed {path} keys: {malformed[:3]}")
        return {key: v for key, v in sites.items() if lint_of(key)[0] in families}

    recorded = json.loads((ROOT / path).read_text())
    base = merge_base(args.base_ref)
    tracked = git("ls-tree", "--name-only", base, path).strip()
    previous = json.loads(git("show", f"{base}:{path}")) if tracked else recorded
    measured, stored = check(
        kind, rows, collected(recorded["sites"]), collected(previous["sites"]), base
    )
    if args.write and not measured:
        recorded["sites"] = rows
        (ROOT / path).write_text(json.dumps(recorded, indent=2) + "\n")
        print(f"recorded the measured debt in {path}")
        stored = []
    errors = measured + stored
    if recorded["version"] != version or previous["version"] != version:
        errors.append("compiler/linter version differs from the recorded inventory")
    for error in errors:
        print(error)
    print(f"{kind}: {sum(map(len, rows.values()))} diagnostics; {len(errors)} errors")
    return int(bool(errors))


if __name__ == "__main__":
    raise SystemExit(main())
