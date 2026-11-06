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
        elif key in ("select", "extend-select") and isinstance(after, list):
            if not set(before or []) <= set(after):
                errors.append(f"weakened lint selection: {name}")
        elif (
            key in ("ignore", "extend-ignore", "exclude", "extend-exclude")
            and isinstance(before, list)
            and isinstance(after, list)
        ):
            if not set(after) <= set(before):
                errors.append(f"broadened lint exclusion: {name}")
        elif (
            isinstance(before, str)
            and isinstance(after, str)
            and before in RANK
            and after in RANK
            and RANK[after] >= RANK[before]
        ):
            continue
        elif (
            before is None
            and isinstance(after, str)
            and after in ("warn", "deny", "forbid")
        ):
            continue
        elif (
            key
            in (
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
            and isinstance(before, int)
            and isinstance(after, int)
            and after <= before
        ):
            continue
        else:
            errors.append(
                f"unclassified lint-policy change: {name}: {before!r} -> {after!r}"
            )
    return errors


def suppressions(files: dict[str, str]) -> Counter[str]:
    rows: Counter[str] = Counter()
    for path, text in files.items():
        if path.endswith(".rs"):
            clean = without_comments(text, path)
            # Attributes are parsed as attribute token groups; strings/comments
            # containing allow(...) cannot create or hide inventory entries.
            masked = without_comments(text, path, strings=True)
            for match in re.finditer(r"#!?\[([^\]]*)\]", masked, re.S):
                for allow in re.finditer(r"\b(allow|expect)\s*\(([^)]*)\)", match[1]):
                    rules = re.sub(r",?\s*reason\s*=.*", "", allow[2], flags=re.S)
                    tail = clean[match.end() :]
                    item = re.search(
                        r"\b(fn|struct|enum|mod|trait|impl)\s+([^\n{;]+)", tail
                    )
                    scope = re.sub(r"\s+", " ", item[0]) if item else "module"
                    if masked[match.start() : match.start() + 3] == "#![":
                        scope = "module"
                    for rule in rules.split(","):
                        if rule.strip():
                            rows[f"{path} | {scope} | {allow[1]} {rule.strip()}"] += 1
        else:
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
        if not (
            path.endswith("Cargo.toml")
            or path in ("ruff.toml", "clippy.toml", "rust-toolchain.toml", "ty.toml")
        ):
            continue
        before = (
            tomllib.loads(git("show", f"{args.base_ref}:{path}"))
            if path in old_paths
            else {}
        )
        after = (
            tomllib.loads((ROOT / path).read_text()) if (ROOT / path).is_file() else {}
        )
        # Package identity and dependencies do not configure lint policy.
        if path.endswith("Cargo.toml"):
            before = {
                key: before[key] for key in ("lints", "workspace") if key in before
            }
            after = {key: after[key] for key in ("lints", "workspace") if key in after}
            for value in (before, after):
                if "workspace" in value:
                    value["workspace"] = {"lints": value["workspace"].get("lints", {})}
        if not before and path == "clippy.toml":
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
