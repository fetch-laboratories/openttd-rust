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

from code_lines import without_comments
from vcs import git

ROOT = Path(__file__).resolve().parents[1]
RANK = {"allow": 0, "warn": 1, "deny": 2, "forbid": 3}
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


def rust_suppressions(path: str, text: str) -> Counter[str]:
    rows: Counter[str] = Counter()
    clean = without_comments(text, path)
    masked = without_comments(text, path, strings=True)
    for match in re.finditer(r"#!?\[([^\]]*)\]", masked, re.S):
        for allow in re.finditer(r"\b(allow|expect)\s*\(([^)]*)\)", match[1]):
            rules = re.sub(r",?\s*reason\s*=.*", "", allow[2], flags=re.S)
            item = re.search(
                r"\b(fn|struct|enum|mod|trait|impl)\s+([^\n{;]+)", clean[match.end() :]
            )
            scope = (
                " ".join(item[0].split()[:2]).split("(")[0].split("<")[0]
                if item
                else "module"
            )
            if masked[match.start() : match.start() + 3] == "#![":
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
    added = suppressions(inventory(None)) - suppressions(inventory(args.base_ref))
    errors.extend(
        f"new suppression: {name} ({count})" for name, count in sorted(added.items())
    )
    for error in errors:
        print(error)
    print(f"lint policy: {len(errors)} errors")
    return int(bool(errors))


if __name__ == "__main__":
    raise SystemExit(main())
