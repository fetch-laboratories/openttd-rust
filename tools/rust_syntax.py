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
    start_byte: int
    end_byte: int
    text: bytes
    has_error: bool
    is_missing: bool
    named_children: list["Node"]
    prev_named_sibling: "Node | None"
    parent: "Node | None"

    def child_by_field_name(self, name: str) -> "Node": ...

    def children_by_field_name(self, name: str) -> list["Node"]: ...

    def descendant_for_byte_range(self, start: int, end: int) -> "Node | None": ...

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


def item_source(node: Node, text: str) -> str:
    """Raw item text including its outer attributes and leading comments."""
    first = node
    while (previous := first.prev_named_sibling) is not None and previous.type in (
        "attribute_item",
        "line_comment",
        "block_comment",
    ):
        first = previous
    return text.encode()[first.start_byte : node.end_byte].decode()


def parser_input(text: str) -> bytes:
    """Normalize rustc pretty-printer syntax outside the grammar's surface.

    Expression lint attributes and builtin # offset_of are compiler-generated
    expressions, not boundary declarations. Keep their operands and positions.
    """
    data = text.encode()
    for pattern in (
        rb'\bunsafe(?=\s+extern\s*"[^"\n]+"\s*\{)',
        rb"\#\[allow\(non_exhaustive_omitted_patterns\)\]",
    ):
        data = re.sub(pattern, lambda match: b" " * len(match[0]), data)
    return re.sub(
        rb"\bbuiltin\s*#\s*offset_of(?=\s*\()",
        lambda match: b"offset_of!" + b" " * (len(match[0]) - 10),
        data,
    )


def syntax_trees(files: dict[str, str]) -> dict[str, Node]:
    trees = {
        path: PARSER.parse(parser_input(text)).root_node for path, text in files.items()
    }
    for path, tree in trees.items():
        if tree.has_error:
            errors = [
                (node.start_point, node.text[:100].decode())
                for node in nodes(tree)
                if node.type == "ERROR" or node.is_missing
            ]
            raise ValueError(f"Rust boundary parse error: {path}: {errors[:8]}")
    return trees


def callback(
    node: Node | None,
    aliases: dict[str, list[Node]],
    seen: frozenset[str] = frozenset(),
) -> bool:
    if node is None:
        return False
    if node.type == "function_type":
        return bool(re.search(rb'\bextern\s*(?:"C(?:-unwind)?")?\s*fn\b', node.text))
    if node.type in (
        "type_identifier",
        "scoped_type_identifier",
        "identifier",
        "scoped_identifier",
    ):
        name = node.text.decode().split("::")[-1]
        if name in aliases and name not in seen:
            # Associated types may be callbacks in some implementations and ()
            # in others. Count their slots conservatively, without claiming a
            # monomorphized semantic ABI graph.
            return any(
                callback(alias, aliases, seen | {name}) for alias in aliases[name]
            )
    return any(callback(child, aliases, seen) for child in node.named_children)


def qualified(node: Node) -> str:
    parts = []
    parent = node.parent
    while parent:
        if parent.type in (
            "struct_item",
            "enum_item",
            "enum_variant",
            "union_item",
            "mod_item",
            "impl_item",
        ):
            parts.append(
                parent.child_by_field_name("name").text.decode()
                if parent.type != "impl_item"
                else parent.child_by_field_name("type").text.decode()
            )
        parent = parent.parent
    return "::".join(reversed(parts))


def export_name(node: Node) -> str | None:
    attrs = []
    prior = node.prev_named_sibling
    while prior and prior.type in ("attribute_item", "line_comment", "block_comment"):
        if prior.type == "attribute_item":
            attrs.append(prior.text.decode())
        prior = prior.prev_named_sibling
    attributes = " ".join(attrs)
    exported = re.search(r'export_name\s*=\s*"([^"]+)"', attributes)
    if exported:
        return exported[1]
    if re.search(r"\bno_mangle\b", attributes):
        return node.child_by_field_name("name").text.decode()
    if "export_name" in attributes:
        raise ValueError("unexpanded export_name attribute")
    return None


def boundary_ids(
    files: dict[str, str],
    *,
    expanded: bool = False,
    texts: dict[str, str] | None = None,
) -> dict[str, set[str]]:
    """Exports and callback slots, each keyed `path | identity`.

    With texts, also record each key's declaration: the whole function item
    for an export, the field declaration (name and type) for a named callback
    slot, and the field's type for a tuple slot.
    """
    trees = syntax_trees(files)
    aliases: dict[str, list[Node]] = {}
    for tree in trees.values():
        for node in nodes(tree):
            if node.type == "type_item":
                name = node.child_by_field_name("name").text.decode()
                aliases.setdefault(name, []).append(node.child_by_field_name("type"))
            elif node.type == "use_as_clause":
                name = node.child_by_field_name("alias").text.decode()
                aliases.setdefault(name, []).append(node.child_by_field_name("path"))
    result: dict[str, set[str]] = {"exports": set(), "callback_slots": set()}
    for path, tree in trees.items():
        for node in nodes(tree):
            for counter, key, text in collect_node(
                path, node, aliases, expanded=expanded
            ):
                result[counter].add(key)
                if texts is not None:
                    texts[key] = text
    return result


def tuple_callbacks(node: Node, aliases: dict[str, list[Node]]) -> dict[str, str]:
    """Each tuple callback slot, `Owner.index`, with its field's type text."""
    slots = {
        index: field.text.decode()
        for index, field in enumerate(node.children_by_field_name("type"))
        if callback(field, aliases)
    }
    owner = qualified(node)
    if slots and not owner:
        raise ValueError("unclassified tuple callback owner")
    return {f"{owner}.{index}": text for index, text in slots.items()}


def collect_node(
    path: str, node: Node, aliases: dict[str, list[Node]], *, expanded: bool
) -> list[tuple[str, str, str]]:
    """The node's boundary identities: (counter, `path | identity`, declaration)."""
    refuse_unclassified(path, node, expanded=expanded)
    if node.type == "function_item":
        name = export_name(node)
        return [("exports", f"{path} | {name}", node.text.decode())] if name else []
    if node.type == "field_declaration" and callback(
        node.child_by_field_name("type"), aliases
    ):
        owner = qualified(node)
        if not owner:
            raise ValueError(f"unclassified callback field in {path}")
        field = node.child_by_field_name("name").text.decode()
        return [("callback_slots", f"{path} | {owner}.{field}", node.text.decode())]
    if node.type == "ordered_field_declaration_list":
        return [
            ("callback_slots", f"{path} | {slot}", text)
            for slot, text in tuple_callbacks(node, aliases).items()
        ]
    return []


def refuse_unclassified(path: str, node: Node, *, expanded: bool) -> None:
    """Stop on a boundary this syntactic inventory cannot classify."""
    if node.type in ("macro_definition", "macro_invocation"):
        if expanded and node.type == "macro_definition":
            return  # The compiler emitted each expansion separately.
        if re.search(rb'\b(?:no_mangle|export_name)\b|\bextern\s*"C"', node.text):
            raise ValueError(
                f"macro boundary requires compiler expansion: {path}: {node.start_point[0] + 1}"
            )
    elif node.type in ("static_item", "const_item") and export_name(node):
        raise ValueError(
            f"exported data requires boundary classifier extension: {path}"
        )
