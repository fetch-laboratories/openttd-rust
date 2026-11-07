#!/usr/bin/env python3
"""Reject weaker lint configuration and new Rust/Python suppressions against base."""

import argparse
import ast
import io
import re
import tokenize
import tomllib
from collections import Counter
from pathlib import Path
from typing import Any

from cleanup_contract import context, reviewed
from code_lines import without_comments
from rust_syntax import Node, nodes, syntax_trees
from vcs import git

ROOT = Path(__file__).resolve().parents[1]
RANK = {"allow": 0, "warn": 1, "deny": 2, "forbid": 3}
LINT_GROUPS = {
    "all",
    "cargo",
    "complexity",
    "correctness",
    "deprecated_safe",
    "future_incompatible",
    "keyword_idents",
    "let_underscore",
    "nonstandard_style",
    "nursery",
    "pedantic",
    "perf",
    "restriction",
    "refining_impl_trait",
    "rust_2018_compatibility",
    "rust_2018_idioms",
    "rust_2021_compatibility",
    "rust_2024_compatibility",
    "style",
    "suspicious",
    "unknown_or_malformed_diagnostic_attributes",
    "unused",
    "warnings",
}
CONFIG_NAMES = {
    "Cargo.toml",
    "ruff.toml",
    ".ruff.toml",
    "pyproject.toml",
    "clippy.toml",
    ".clippy.toml",
    "rust-toolchain.toml",
    "ty.toml",
}


def lint_config(path: str, data: dict[str, Any]) -> dict[str, Any]:
    """Include discovered overrides, excluding unrelated package metadata."""
    if Path(path).name == "Cargo.toml":
        return {
            "lints": data.get("lints", {}),
            "workspace": {"lints": data.get("workspace", {}).get("lints", {})},
        }
    if Path(path).name == "pyproject.toml":
        return {name: data.get("tool", {}).get(name, {}) for name in ("ruff", "ty")}
    return data


def permitted_change(key: str, before: object, after: object) -> bool:
    if key in ("select", "extend-select") and isinstance(after, list):
        return isinstance(before, list) and set(before) <= set(after)
    if key in ("ignore", "extend-ignore", "exclude", "extend-exclude"):
        return (
            isinstance(before, list)
            and isinstance(after, list)
            and set(after) <= set(before)
        )
    if isinstance(after, str) and after in RANK:
        return RANK[after] >= RANK.get(str(before), 1)
    thresholds = (
        "too-many-lines-threshold",
        "too-many-arguments-threshold",
        "cognitive-complexity-threshold",
        "excessive-nesting-threshold",
        "max-complexity",
        "max-returns",
        "max-branches",
        "max-args",
        "max-statements",
    )
    return (
        key in thresholds
        and isinstance(before, int)
        and isinstance(after, int)
        and after <= before
    )


def config_errors(
    old: dict[str, Any], new: dict[str, Any], path: str = ""
) -> list[str]:
    """Conservatively reject unclassified policy edits; strengthening is allowed."""
    errors = []
    for key in old.keys() | new.keys():
        before, after = old.get(key), new.get(key)
        name = path + "." + key
        if before == after:
            continue
        if isinstance(before, dict) and isinstance(after, dict):
            errors.extend(config_errors(before, after, name))
        elif not permitted_change(key, before, after):
            errors.append(
                f"weakened or unclassified lint policy: {name}: {before!r} -> {after!r}"
            )
    return errors


def item_scope(node: Node) -> str:
    """Name the actual attribute target and its enclosing named items."""
    kinds = {
        "function_item": "fn",
        "function_signature_item": "fn",
        "struct_item": "struct",
        "enum_item": "enum",
        "mod_item": "mod",
        "trait_item": "trait",
        "const_item": "const",
        "static_item": "static",
        "type_item": "type",
        "union_item": "union",
        "enum_variant": "variant",
        "field_declaration": "field",
    }
    parts = []
    current: Node | None = node
    while current:
        if current.type == "impl_item":
            parts.append(" ".join(current.text.decode().split("{", 1)[0].split()))
        elif current.type in kinds:
            name = current.child_by_field_name("name")
            if name:
                parts.append(f"{kinds[current.type]} {name.text.decode()}")
        elif current is node:
            return "module"  # Non-item/macro token targets have no reviewed scope.
        current = current.parent
    return " > ".join(reversed(parts)) or "module"


def attribute_scopes(path: str, text: str) -> dict[tuple[int, int], str]:
    """Associate outer attributes with siblings, never text in later strings/items."""
    scopes = {}
    for parent in nodes(syntax_trees({path: text})[path]):
        pending = []
        for child in parent.named_children:
            if child.type == "attribute_item":
                pending.append(child.start_point)
            elif pending and child.type not in ("line_comment", "block_comment"):
                scope = item_scope(child)
                scopes.update((point, scope) for point in pending)
                pending.clear()
    return scopes


def rust_suppressions(path: str, text: str) -> Counter[str]:
    rows: Counter[str] = Counter()
    masked = without_comments(text, path, strings=True)
    scopes = attribute_scopes(path, text)
    for match in re.finditer(r"#\s*!?\s*\[([^\]]*)\]", masked, re.S):
        for allow in re.finditer(r"\b(allow|expect)\s*\(([^)]*)\)", match[1]):
            rules = re.sub(r",?\s*reason\s*=.*", "", allow[2], flags=re.S)
            prefix = text[: match.start()]
            point = (prefix.count("\n"), len(prefix.rsplit("\n", 1)[-1].encode()))
            scope = scopes.get(point, "module")
            if re.match(r"#\s*!", masked[match.start() :]):
                scope = "module"
            for rule in rules.split(","):
                if rule.strip():
                    rows[f"{path} | {scope} | {allow[1]} {rule.strip()}"] += 1
    return rows


def python_suppressions(path: str, text: str) -> Counter[str]:
    rows: Counter[str] = Counter()
    tree = ast.parse(text)
    functions = [
        n
        for n in ast.walk(tree)
        if isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef))
    ]
    for token in tokenize.generate_tokens(io.StringIO(text).readline):
        if token.type != tokenize.COMMENT:
            continue
        match = re.search(r"#\s*(?:ruff:\s*)?noqa\b(.*)", token.string, re.I)
        if match:
            scope = ".".join(
                n.name
                for n in sorted(functions, key=lambda n: n.lineno)
                if n.lineno <= token.start[0] <= (n.end_lineno or n.lineno)
            )
            rows[f"{path} | {scope} | noqa{match[1].strip()}"] += 1
    return rows


def suppressions(files: dict[str, str]) -> Counter[str]:
    rows: Counter[str] = Counter()
    for path, text in files.items():
        scanner = rust_suppressions if path.endswith(".rs") else python_suppressions
        rows.update(scanner(path, text))
    return rows


def inventory(revision: str | None) -> dict[str, str]:
    paths = (
        git("ls-tree", "-r", "--name-only", revision, "rust/", "tools/").splitlines()
        if revision
        else git("ls-files", "rust/", "tools/").splitlines()
    )
    return {
        path: git("show", f"{revision}:{path}")
        if revision
        else (ROOT / path).read_text()
        for path in paths
        if path.endswith((".rs", ".py")) and (revision or (ROOT / path).is_file())
    }


def accepted_suppression(
    site: str,
    before: int,
    after: int,
    contract: tuple[str, list[dict[str, Any]], list[str], str],
) -> bool:
    """Permit a reviewed Rust item operation, never a module or lint group."""
    path, scope, suppression = site.rsplit(" | ", 2)
    if (
        not path.endswith(".rs")
        or scope == "module"
        or scope.rsplit(" > ", 1)[-1].startswith("mod ")
    ):
        return False
    rule = re.sub(r"\s+|r#", "", suppression.split(" ", 1)[1])
    if rule.rsplit("::", 1)[-1] in LINT_GROUPS:
        return False
    author, proposals, comments, head = contract
    expected = {
        "kind": "suppression",
        "tool": "rust",
        "site": site,
        "before": before,
        "after": after,
    }
    required = (
        "operation",
        "source_contract",
        "reason",
        "retiring_owner",
        "retirement_condition",
    )
    for proposal in proposals:
        if any(type(proposal.get(key)) is not int for key in ("before", "after")):
            continue
        if any(proposal.get(key) != value for key, value in expected.items()):
            continue
        if all(
            isinstance(proposal.get(key), str) and proposal[key].strip()
            for key in required
        ) and reviewed(proposal, comments, author, head):
            print(f"reviewed suppression at {head}: {site} ({before} -> {after})")
            return True
    return False


def suppression_errors(before: Counter[str], after: Counter[str]) -> list[str]:
    added = after - before
    if not added:
        return []
    contract = context()
    return [
        f"new suppression: {site} ({count})"
        for site, count in sorted(added.items())
        if not accepted_suppression(site, before[site], after[site], contract)
    ]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-ref", default="origin/rust-migration")
    args = parser.parse_args()
    errors = []
    old_paths = set(git("ls-tree", "-r", "--name-only", args.base_ref).splitlines())
    paths = set(git("ls-files").splitlines()) | old_paths
    for path in sorted(paths):
        if Path(path).name not in CONFIG_NAMES:
            continue
        before = (
            tomllib.loads(git("show", f"{args.base_ref}:{path}"))
            if path in old_paths
            else {}
        )
        after = (
            tomllib.loads((ROOT / path).read_text()) if (ROOT / path).is_file() else {}
        )
        before, after = lint_config(path, before), lint_config(path, after)
        if not before and path == "migration/lints/clippy.toml":
            # Bootstrap only these pinned thresholds, not an arbitrary config.
            expected = {
                "too-many-lines-threshold": 100,
                "too-many-arguments-threshold": 7,
                "cognitive-complexity-threshold": 25,
                "excessive-nesting-threshold": 5,
            }
            if after != expected:
                errors.append("invalid initial Clippy thresholds")
        else:
            errors.extend(config_errors(before, after, path))
    errors.extend(
        suppression_errors(
            suppressions(inventory(args.base_ref)), suppressions(inventory(None))
        )
    )
    for error in errors:
        print(error)
    print(f"lint policy: {len(errors)} errors")
    return int(bool(errors))


if __name__ == "__main__":
    raise SystemExit(main())
