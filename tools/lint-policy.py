#!/usr/bin/env python3
"""Reject weaker lint configuration and new Rust/Python suppressions against base."""

import argparse
import ast
import io
import operator
import re
import subprocess
import tokenize
import tomllib
from collections import Counter
from collections.abc import Container
from functools import partial
from pathlib import Path
from typing import Any

from cleanup_contract import (
    ACCEPTED,
    MISSING,
    UNSAFE,
    Excepted,
    Ledger,
    carry_hints,
    checked_carries,
    context,
    default_event,
    pending_fails,
    review_state,
    rule_name,
    run,
    tracked,
    write,
)
from code_lines import without_comments
from rust_syntax import Node, nodes, syntax_trees
from vcs import git, merge_base

ROOT = Path(__file__).resolve().parents[1]
# --same-item's status for an item whose text differs between the two commits.
ITEM_DIFFERS = 3
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
# Original enumerator spellings and faithful native casts; see #806.
FIDELITY = {
    "non_upper_case_globals",
    "clippy::cast_possible_truncation",
    "clippy::cast_possible_wrap",
    "clippy::cast_sign_loss",
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


# Thresholds of Clippy lints the debt collector no longer runs (steering 33):
# a faithful port keeps the original's function length and nesting.
RETIRED_THRESHOLDS = {
    "too-many-lines-threshold",
    "cognitive-complexity-threshold",
    "excessive-nesting-threshold",
}


def permitted_change(key: str, before: object, after: object) -> bool:
    if key in RETIRED_THRESHOLDS and after is None:
        return True
    if key == "workspace" and before is None and after is True:
        # A new crate's `[lints] workspace = true` adopts the workspace policy.
        return True
    if key in ("select", "extend-select", "disallowed-macros") and isinstance(
        after, list
    ):
        # Adding a disallowed macro strengthens the policy (#891).
        return (before is None and key == "disallowed-macros") or (
            isinstance(before, list) and set(map(str, before)) <= set(map(str, after))
        )
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
            header = without_comments(current.text.decode(), "x.rs").split("{", 1)[0]
            parts.append(" ".join(header.split()))
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


def rust_attributes(path: str, text: str) -> list[tuple[str, str]]:
    """Each suppressed rule's site and its attribute's `reason` text."""
    rows = []
    masked = without_comments(text, path, strings=True)
    scopes = attribute_scopes(path, text)
    for match in re.finditer(r"#\s*!?\s*\[([^\]]*)\]", masked, re.S):
        for allow in re.finditer(r"\b(allow|expect)\s*\(([^)]*)\)", match[1]):
            rules = re.sub(r",?\s*reason\s*=.*", "", allow[2], flags=re.S)
            offset = match.start(1)
            raw = text[offset + allow.start(2) : offset + allow.end(2)]
            reason = re.search(r'\breason\s*=\s*"((?:[^"\\]|\\.)*)"', raw)
            prefix = text[: match.start()]
            point = (prefix.count("\n"), len(prefix.rsplit("\n", 1)[-1].encode()))
            scope = scopes.get(point, "module")
            if re.match(r"#\s*!", masked[match.start() :]):
                scope = "module"
            for rule in rules.split(","):
                if rule.strip():
                    site = f"{path} | {scope} | {allow[1]} {rule.strip()}"
                    rows.append((site, reason[1].strip() if reason else ""))
    return rows


def rust_suppressions(path: str, text: str) -> Counter[str]:
    return Counter(site for site, _ in rust_attributes(path, text))


def fidelity_sites(files: dict[str, str]) -> set[str]:
    """Item-scoped fidelity suppressions whose every attribute gives a reason."""
    reasons: dict[str, list[str]] = {}
    for path, text in files.items():
        if path.endswith(".rs"):
            for site, reason in rust_attributes(path, text):
                reasons.setdefault(site, []).append(reason)
    return {
        site
        for site, texts in reasons.items()
        if all(texts) and item_scoped(site) and rule_name(site) in FIDELITY
    }


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


def item_scoped(site: str) -> bool:
    """A named Rust item below module level, suppressing one lint, not a group."""
    path, scope, _ = site.rsplit(" | ", 2)
    return (
        path.endswith(".rs")
        and scope != "module"
        and not scope.rsplit(" > ", 1)[-1].startswith("mod ")
        and rule_name(site).rsplit("::", 1)[-1] not in LINT_GROUPS
    )


def carry_rule(site: str) -> tuple[str, bool]:
    """Carries keep the suppression and cannot broaden an item to a module."""
    return site.rsplit(" | ", 1)[1], item_scoped(site)


def unchanged_scope(site: str, revision: str, current: str | None = None) -> bool:
    """Whether the site's item code at `revision` equals `current`'s, comments aside.

    `current` is a commit, or None for the working tree.
    """
    path, scope, _ = site.rsplit(" | ", 2)
    try:
        old = git("show", f"{revision}:{path}")
        if current is None:
            new = (ROOT / path).read_text() if (ROOT / path).is_file() else None
        else:
            new = git("show", f"{current}:{path}")
    except subprocess.CalledProcessError:
        return False
    return new is not None and same_code(path, old, new, scope)


COMMENTS = ("line_comment", "block_comment")


def scope_code(path: str, text: str, scope: str) -> list[str] | None:
    """The tokens of the items the scope names, with their outer attributes,
    without comments: a reworded `SAFETY` line or `# Safety` section leaves the
    accepted body's argument where it was, and literals keep every byte."""
    tokens: list[str] = []
    found = False
    for node in nodes(syntax_trees({path: text})[path]):
        if not node.type.endswith("_item") or item_scope(node) != scope:
            continue
        found = True
        first = node
        while (previous := first.prev_named_sibling) is not None and previous.type in (
            "attribute_item",
            *COMMENTS,
        ):
            first = previous
        sibling: Node | None = first
        while sibling is not None:
            tokens += code_tokens(sibling)
            sibling = None if sibling == node else sibling.next_sibling
    return tokens if found and scope != "module" else None


def code_tokens(node: Node) -> list[str]:
    """Leaf token texts below `node`, skipping comment subtrees."""
    tokens, stack = [], [node]
    while stack:
        current = stack.pop()
        if current.type in COMMENTS:
            continue
        if current.child_count == 0:
            tokens.append(current.text.decode())
        else:
            stack.extend(reversed(current.children))
    return tokens


def same_code(path: str, old: str, new: str, scope: str) -> bool:
    """Whether the scope's items have the same code in both texts."""
    before = scope_code(path, old, scope)
    return before is not None and before == scope_code(path, new, scope)


def suppression_state(
    site: str,
    before: int,
    after: int,
    contract: tuple[str, list[dict[str, Any]], list[str], str],
) -> str:
    """Review state of a Rust item operation; never a module or lint group.

    An `unsafe_code` acceptance holds only while the item's code is unchanged.
    """
    if not item_scoped(site):
        return MISSING
    author, proposals, comments, head = contract
    matching = Excepted("suppression", "rust", site, before, after).proposals(proposals)
    unsafe = rule_name(site) == UNSAFE
    unchanged = partial(unchanged_scope, site) if unsafe else None
    state, _ = review_state(matching, comments, author, head, unchanged)
    if state == ACCEPTED:
        print(f"reviewed suppression at {head}: {site} ({before} -> {after})")
    return state


def accepted_suppression(
    site: str,
    before: int,
    after: int,
    contract: tuple[str, list[dict[str, Any]], list[str], str],
) -> bool:
    """Whether a reviewer accepted this Rust item operation."""
    return suppression_state(site, before, after, contract) == ACCEPTED


def suppression_errors(
    before: Counter[str],
    after: Counter[str],
    reasoned: Container[str] = frozenset(),
    base_ref: str = "origin/rust-migration",
    event: str = "pull_request",
) -> tuple[list[str], list[dict[str, str]]]:
    """New suppressions no record explains, and the ready carries for them.

    A proposed suppression awaiting its acceptance is pending review: it is
    printed, and is an error only on an event where pending fails.
    """
    added = {site: n for site, n in (after - before).items() if site not in reasoned}
    if not added:
        return [], []
    contract = context(base_ref)
    errors = []
    ready = []
    for tool, suffix in (("rust", ".rs"), ("python", ".py")):
        old, new = (
            {
                site: n
                for site, n in rows.items()
                if site.split(" | ")[0].endswith(suffix)
            }
            for rows in (before, after)
        )
        ledger = Ledger(tool, old, new, carry_rule, operator.le)
        valid = checked_carries(contract[1], ledger)
        unexplained = []
        for site in sorted(added):
            if not site.split(" | ")[0].endswith(suffix) or site in valid:
                continue
            state = suppression_state(site, before[site], after[site], contract)
            if state == ACCEPTED:
                continue
            unexplained.append(site)
            if state == MISSING:
                errors.append(f"new suppression: {site} ({added[site]})")
            elif pending_fails(event):
                errors.append(
                    f"new suppression: {site} ({added[site]})"
                    " lacks its reviewer's acceptance"
                )
            else:
                print(
                    f"pending review: suppression {site}"
                    " awaits its reviewer's acceptance"
                )
        ready += carry_hints(
            ledger,
            old.keys() - new.keys() - set(valid.values()),
            [site for site in unexplained if site not in old],
        )
    return errors, ready


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-ref", default="origin/rust-migration")
    parser.add_argument(
        "--write",
        action="store_true",
        help="record each ready carry in this branch's cleanup records file",
    )
    parser.add_argument(
        "--event",
        default=default_event(),
        help="the CI event (default: GITHUB_EVENT_NAME, else pull_request);"
        " a suppression pending review fails only in merge_group",
    )
    parser.add_argument(
        "--same-item",
        nargs=3,
        metavar=("SITE", "ACCEPTED", "HEAD"),
        help="exit 0 when the suppressed item's code, comments aside, is equal "
        "at both commits, 3 when it differs",
    )
    args = parser.parse_args()
    if args.same_item:
        # 0: the item's code is equal; ITEM_DIFFERS: it differs. An unreadable
        # file, a crash or a failed start exits otherwise, so a caller never
        # reads it as a change.
        site, accepted, head = args.same_item
        path, scope, _ = site.rsplit(" | ", 2)
        # Each blob is read once; a failed read raises, exiting 1, never 3.
        old, new = (git("show", f"{revision}:{path}") for revision in (accepted, head))
        return 0 if same_code(path, old, new, scope) else ITEM_DIFFERS
    args.base_ref = merge_base(args.base_ref, run=git, announce=True)
    # Quick runs lint policy on every change, so every record file is validated
    # here even when no gate needs its records.
    tracked(args.base_ref)
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
            expected = {"too-many-arguments-threshold": 7}
            if after != expected:
                errors.append("invalid initial Clippy thresholds")
        else:
            errors.extend(config_errors(before, after, path))
    head = inventory(None)
    check = partial(
        suppression_errors,
        suppressions(inventory(args.base_ref)),
        suppressions(head),
        fidelity_sites(head),
        args.base_ref,
        args.event,
    )
    found, ready = check()
    if args.write and ready:
        write(args.base_ref, ready)
        found, _ = check()
    elif ready:
        print("record the ready carries: python3 tools/lint-policy.py --write")
    errors.extend(found)
    for error in errors:
        print(error)
    print(f"lint policy: {len(errors)} errors")
    return int(bool(errors))


if __name__ == "__main__":
    run(main)
