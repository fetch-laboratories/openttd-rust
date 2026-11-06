"""Pinned Rust syntax tree for boundary inventory (no compiler/type-proof claim)."""

import importlib
import os
import re
import sys
from collections.abc import Iterator
from typing import Protocol


class Node(Protocol):
    """The syntax-tree node interface used by this inventory."""

    type: str
    text: bytes
    has_error: bool
    named_children: list["Node"]
    prev_named_sibling: "Node | None"
    parent: "Node | None"

    def child_by_field_name(self, name: str) -> "Node": ...

    @property
    def start_point(self) -> tuple[int, int]: ...


try:
    TS = importlib.import_module("tree_sitter")
    RUST = importlib.import_module("tree_sitter_rust")
except ImportError:
    os.execvp(
        "uv",
        [
            "uv",
            "run",
            "--no-project",
            "--with",
            "pygments==2.19.2",
            "--with",
            "tree-sitter==0.25.2",
            "--with",
            "tree-sitter-rust==0.24.0",
            "python3",
            *sys.argv,
        ],
    )
PARSER = TS.Parser(TS.Language(RUST.language()))


def nodes(node: "Node") -> Iterator["Node"]:
    yield node
    for child in node.named_children:
        yield from nodes(child)


def boundary_ids(files: dict[str, str]) -> dict[str, set[str]]:
    trees = {
        path: PARSER.parse(
            re.sub(rb'\bunsafe(?=\s+extern\s*"[^"\n]+"\s*\{)', b"      ", text.encode())
        ).root_node
        for path, text in files.items()
    }
    for path, tree in trees.items():
        if tree.has_error:
            raise ValueError(f"Rust boundary parse error: {path}")
    aliases: dict[str, list["Node"]] = {}
    for tree in trees.values():
        for node in nodes(tree):
            if node.type == "type_item":
                name = node.child_by_field_name("name").text.decode()
                aliases.setdefault(name, []).append(node.child_by_field_name("type"))

    def callback(node: "Node | None", seen: frozenset[str] = frozenset()) -> bool:
        if node is None:
            return False
        if node.type == "function_type":
            return bool(
                re.search(rb'\bextern\s*(?:"C(?:-unwind)?")?\s*fn\b', node.text)
            )
        if node.type in ("type_identifier", "scoped_type_identifier"):
            name = node.text.decode().split("::")[-1]
            if name in aliases and name not in seen:
                answers = {callback(alias, seen | {name}) for alias in aliases[name]}
                if len(answers) > 1:
                    raise ValueError(f"ambiguous callback alias: {name}")
                return answers == {True}
        return any(callback(child, seen) for child in node.named_children)

    result: dict[str, set[str]] = {"exports": set(), "callback_slots": set()}
    for path, tree in trees.items():
        for node in nodes(tree):
            if node.type == "function_item":
                attrs = []
                prior = node.prev_named_sibling
                while prior and prior.type in (
                    "attribute_item",
                    "line_comment",
                    "block_comment",
                ):
                    if prior.type == "attribute_item":
                        attrs.append(prior.text.decode())
                    prior = prior.prev_named_sibling
                attributes = " ".join(attrs)
                exported = re.search(r'export_name\s*=\s*"([^"]+)"', attributes)
                if exported or re.search(r"\bno_mangle\b", attributes):
                    name = (
                        exported[1]
                        if exported
                        else node.child_by_field_name("name").text.decode()
                    )
                    result["exports"].add(f"{path} | {name}")
            elif node.type == "field_declaration" and callback(
                node.child_by_field_name("type")
            ):
                parent = node.parent
                while parent and parent.type not in (
                    "struct_item",
                    "enum_item",
                    "union_item",
                ):
                    parent = parent.parent
                if parent is None:
                    raise ValueError(f"unclassified callback field in {path}")
                owner = parent.child_by_field_name("name").text.decode()
                field = node.child_by_field_name("name").text.decode()
                result["callback_slots"].add(f"{path} | {owner}.{field}")
            elif node.type in ("macro_definition", "macro_invocation"):
                if re.search(
                    rb'\b(?:no_mangle|export_name)\b|\bextern\s*"C"', node.text
                ):
                    raise ValueError(
                        f"macro boundary requires parser extension: {path}: {node.start_point[0] + 1}"
                    )
    return result
