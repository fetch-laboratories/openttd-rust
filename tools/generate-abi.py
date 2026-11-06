#!/usr/bin/env python3
"""Generate native ABI probes, typed service visitors and shared record declarations.

Fragments are rebuildable build outputs; Rust is the only field inventory.
"""

import argparse
import re
from collections import defaultdict
from collections.abc import Iterator
from pathlib import Path
from typing import NamedTuple

ROOT = Path(__file__).resolve().parents[1]
DECLARATION = re.compile(
    r'"([^"]+)"\s*\["([^"]+)"\]\s*=>\s*([^{}]+)\{([^{}]*)\}'
    r"\s*(?:width\s+(\w+)\s*:\s*([^;]+))?;"
)
RECORDS = re.compile(r"shared_records!\s*\{\s*fn\s+\w+\s*;(.*?)\n\}", re.S)
RECORD = re.compile(
    r'"([^"]+)"\s*\["([^"]+)"\]\s*=>\s*pub\s+struct\s+(\w+)\s*\{([^{}]*)\}'
)
PRIMITIVES = {
    "bool": "bool",
    "c_void": "void",
    "usize": "std::size_t",
    "isize": "std::ptrdiff_t",
    "u8": "uint8_t",
    "u16": "uint16_t",
    "u32": "uint32_t",
    "u64": "uint64_t",
    "i8": "int8_t",
    "i16": "int16_t",
    "i32": "int32_t",
    "i64": "int64_t",
}
TRANSPARENT = re.compile(
    r"#\[repr\(transparent\)\][^;{]*?pub\s+struct\s+(\w+)\s*\(\s*pub\s+(\w+)\s*\)\s*;"
)
OPAQUE = re.compile(r"pub\s+struct\s+(\w+)\s*\{\s*_private:\s*\[u8;\s*0\],?\s*\}")


class Field(NamedTuple):
    name: str
    cpp_type: str
    pointee_tag: str | None
    dimensions: str = ""


def split_fields(body: str) -> list[str]:
    """Split a field list at top-level commas, outside brackets and native type strings."""
    fields, depth, quoted, start = [], 0, False, 0
    for index, char in enumerate(body):
        if char == '"':
            quoted = not quoted
        elif not quoted and char in "<[(":
            depth += 1
        elif not quoted and char in ">])" and body[index - 1 : index + 1] != "=>":
            depth -= 1
        elif not quoted and depth == 0 and char == ",":
            fields.append(body[start:index])
            start = index + 1
    return [field for field in fields + [body[start:]] if field.strip()]


class SharedRecord(NamedTuple):
    cpp: str
    group: str
    rust: str
    fields: list[Field]


class AbiField(NamedTuple):
    rust: str
    cpp: str


class Declaration(NamedTuple):
    cpp: str
    group: str
    rust: str
    fields: list[AbiField]
    width: str
    width_type: str


def rust_sources(root: Path) -> list[tuple[Path, str]]:
    """Comment-stripped Rust sources in path order."""
    return [
        (path, re.sub(r"//[^\n]*", "", path.read_text(encoding="utf-8")))
        for path in sorted((root / "rust/openttd-kernels/src").rglob("*.rs"))
    ]


def shared_records(
    root: Path = ROOT,
) -> list[SharedRecord]:
    """C++ name, group, Rust name and (field, C++ type) pairs of each shared record."""
    sources = rust_sources(root)
    scalars: dict[str, str] = {}
    opaque: set[str] = set()
    for _, source in sources:
        scalars.update(TRANSPARENT.findall(source))
        opaque.update(OPAQUE.findall(source))
    found: list[tuple[str, str, str, str]] = []
    for _, source in sources:
        for block in RECORDS.findall(source):
            block = re.sub(r"#\[[^\]]*\]", "", block)
            found += [match for match in RECORD.findall(block)]
    names = {rust: cpp for cpp, _, rust, _ in found}
    pointer_names = {
        declaration.rust: declaration.cpp for declaration in declarations(root)
    }
    pointer_names.update(names)

    def native(rust: str, pointee: str | None = None) -> str:
        rust = rust.strip()
        array = re.fullmatch(r"\[(.+);\s*([\w:]+)\s*\]", rust, re.S)
        if array:
            element = native(array[1])
            dimension = array[2].split("::")[-1]
            return re.sub(
                r"(\[.*\])?$", lambda m: f"[{dimension}]{m[0]}", element, count=1
            )
        pointer = re.fullmatch(r"\*(mut|const)\s+(.+)", rust)
        if pointer:
            path = pointer[2].strip()
            target = path.split("::")[-1]
            if pointee is not None:
                target = pointee
            elif path in pointer_names:
                target = pointer_names[path]
            elif target in pointer_names:
                target = pointer_names[target]
            elif target not in opaque:
                target = native(target)
            return f"{'const ' if pointer[1] == 'const' else ''}{target} *"
        rust = rust.split("::")[-1]
        if rust in names:
            return names[rust]
        seen = set()
        while rust in scalars and rust not in seen:
            seen.add(rust)
            rust = scalars[rust]
        if rust not in PRIMITIVES:
            raise ValueError(f"unsupported record field type: {rust}")
        return PRIMITIVES[rust]

    result = []
    opaque_tags: dict[str, str] = {}
    for cpp, group, rust, body in sorted(found):
        fields = []
        for field in split_fields(body):
            if field.count('"') % 2:
                raise ValueError(f"invalid record field: {cpp}: {field}")
            match = re.fullmatch(
                r'\s*pub\s+(?:r#)?(\w+)\s*:\s*(.+?)\s*(?:=>\s*(?:(\w+)(?:\s+(\w+))?|"([^"]+)"))?\s*',
                field,
                re.S,
            )
            if not match:
                raise ValueError(f"invalid record field: {cpp}: {field}")
            # A quoted native type names the C++ field type; the layout check compares sizes.
            cpp_type = match[5] or native(match[2], match[4])
            bounds = re.search(r"(?:\[\w+\])*$", cpp_type)
            dimensions = bounds[0] if bounds else ""
            cpp_type = cpp_type[: len(cpp_type) - len(dimensions)]
            if match[5]:
                fields.append(Field(match[1], cpp_type, None, dimensions))
                continue
            target = cpp_type.removeprefix("const ").removesuffix(" *")
            tag = match[3]
            if tag is not None and (
                not cpp_type.endswith(" *")
                or (target not in opaque and match[4] is None)
            ):
                raise ValueError(
                    f"pointee tag requires an opaque pointer: {cpp}.{match[1]}"
                )
            if cpp_type.endswith(" *") and (target in opaque or match[4] is not None):
                tag = tag or "struct"
                if tag not in ("class", "struct"):
                    raise ValueError(f"unsupported pointee tag: {tag}")
                if opaque_tags.setdefault(target, tag) != tag:
                    raise ValueError(f"conflicting pointee tags: {target}")
            if cpp_type.endswith(" *") and target in pointer_names.values():
                tag = "struct"
            fields.append(Field(match[1], cpp_type, tag, dimensions))
        result.append(SharedRecord(cpp, group, rust, fields))
    if len(names) != len(found):
        raise ValueError("duplicate shared record")
    return result


def record_header(group: str, entries: list[SharedRecord]) -> list[str]:
    """One self-contained C++ header declaring a group's records in Rust order."""
    defined = {record.cpp for record in entries}
    opaque = {
        field.cpp_type.removeprefix("const ").removesuffix(" *"): field.pointee_tag
        for record in entries
        for field in record.fields
        if field.pointee_tag is not None
        and field.cpp_type.removeprefix("const ").removesuffix(" *") not in defined
    }
    lines = [
        f"/* Generated by tools/generate-abi.py from Rust shared_records! ({group}). */",
        "#pragma once",
        "#include <array>",
        "#include <cstddef>",
        "#include <cstdint>",
    ]
    lines += [f"{tag} {name};" for name, tag in sorted(opaque.items())]
    lines += [f"struct {name};" for name in sorted(defined)]
    records = {record.cpp: record for record in entries}
    emitted: set[str] = set()

    def emit(record: SharedRecord, active: set[str]) -> None:
        if record.cpp in emitted:
            return
        if record.cpp in active:
            raise ValueError(f"recursive value record: {record.cpp}")
        for field in record.fields:
            if field.cpp_type in records:
                emit(records[field.cpp_type], active | {record.cpp})
        # Quoted native field types are declared by the includer.
        lines.append(f"struct {record.cpp} {{")
        lines.extend(
            f"\t{field.cpp_type}{'' if field.cpp_type.endswith('*') else ' '}{field.name}{field.dimensions};"
            for field in record.fields
        )
        lines.append("};")
        emitted.add(record.cpp)

    for record in entries:
        emit(record, set())
    return lines


def declarations(root: Path = ROOT) -> Iterator[Declaration]:
    """Yield C++ name, probe group, Rust type, field pairs and optional width."""
    seen = set()
    for path in sorted((root / "rust/openttd-kernels/src").rglob("*.rs")):
        source = path.read_text(encoding="utf-8")
        if "layouts! {" not in source:
            continue
        source = RECORDS.sub("", re.sub(r"//[^\n]*", "", source))
        for name, group, rust, body, width, width_type in DECLARATION.findall(source):
            if name in seen:
                raise ValueError(f"duplicate ABI type: {name}")
            seen.add(name)
            fields = []
            for field in filter(str.strip, body.split(",")):
                pair = [part.strip() for part in field.split("=>")]
                if not all(re.fullmatch(r"(?:r#)?\w+", part) for part in pair):
                    raise ValueError(f"invalid ABI field: {field}")
                fields.append(AbiField(pair[0], pair[-1]))
            if len(fields) != len({cpp for _, cpp in fields}):
                raise ValueError(f"duplicate C++ fields: {name}")
            if width and width not in dict(fields):
                raise ValueError(f"unknown width field: {name}.{width}")
            yield Declaration(name, group, rust.strip(), fields, width, width_type)
    if not seen:
        raise ValueError("no ABI declarations")


def generate(root: Path = ROOT) -> dict[str, str]:
    """Fragments live only in the build directory; no generated inventory is checked in."""
    output: defaultdict[str, list[str]] = defaultdict(list)
    headers: dict[str, tuple[str, str]] = {}
    aliases: dict[str, str] = {}
    for header in sorted((root / "src/rust").glob("*.h*")):
        source = header.read_text(encoding="utf-8")
        text = re.sub(r"//[^\n]*|/\*.*?\*/", "", source, flags=re.S)
        aliases.update(re.findall(r"using\s+(\w+)\s*=\s*([^;]+);", text))
        for tag, body, alias in re.findall(
            r"(?:typedef\s+)?struct\s*(\w+)?\s*\{([^{}]*)\}\s*(\w+)?\s*;", text, re.S
        ):
            headers[alias or tag] = (header.name, body)
    # Sorted by C++ name, so the output does not depend on the hosting module.
    records = sorted(declarations(root))
    for name, (_, body) in headers.items():
        if re.search(r"\(\s*(?:\w+\s+)?\*\w+\)\s*\(", body):
            if name not in {aliases.get(r[0], r[0]).split("<")[0] for r in records}:
                raise ValueError(f"callback table absent from Rust layouts: {name}")
    for name, group, _, fields, width, _ in records:
        values = [f"sizeof({name})", f"alignof({name})"]
        values += [f"offsetof({name}, {cpp})" for _, cpp in fields]
        if width:
            values.append(f"sizeof(decltype({name}::{dict(fields)[width]}))")
        output[f"abi-layout-{group}.inc"].append(
            f'Layout("{name}", {{{", ".join(values)}}});'
        )
        # Data pointers and scalar carriers never acquire callback defaults or checks.
        header, body = headers.get(aliases.get(name, name).split("<")[0], ("", ""))
        callbacks = re.findall(r"\(\s*(?:\w+\s+)?\*(\w+)\)\s*\(", body)
        if not callbacks:
            continue
        missing = set(callbacks) - {cpp for _, cpp in fields}
        if missing:
            raise ValueError(f"{name}: callbacks absent from Rust layout: {missing}")
        visitor = [
            "template <typename Visitor>",
            f"void AbiVisitMembers(const {name} &table, Visitor visit)",
            "{",
        ]
        visitor += [f'\tvisit("{cpp}", table.{cpp});' for _, cpp in fields]
        visitor.append("}")
        output[f"abi-visit-{name}.hpp"].extend(visitor)
    shared: defaultdict[str, list[SharedRecord]] = defaultdict(list)
    for record in shared_records(root):
        name, group, _, fields = record
        shared[group].append(record)
        values = [f"sizeof({name})", f"alignof({name})"]
        for field in fields:
            values += [
                f"offsetof({name}, {field.name})",
                f"sizeof({name}::{field.name})",
            ]
        output[f"abi-layout-{group}.inc"].append(
            f'Layout("{name}", {{{", ".join(values)}}});'
        )
    for group, entries in sorted(shared.items()):
        output[f"abi-record-{group}.h"] = record_header(group, entries)
    # Layout types need their declaring headers; component fragments add their own.
    output["abi-includes.inc"] = sorted(
        [
            f'#include "rust/{headers[name][0]}"'
            for name, *_ in records
            if name in headers
        ]
        + [f'#include "abi-record-{group}.h"' for group in shared]
    )
    # Component probe fragments, in file-name order; each registers itself.
    probes = sorted((root / "tools/migration/windows-abi").glob("*.inc"))
    output["abi-probes.inc"] = [
        f'#include "windows-abi/{path.name}"' for path in probes
    ]
    return {name: "\n".join(lines) + "\n" for name, lines in output.items()}


def write_fragments(output: Path, fragments: dict[str, str]) -> None:
    """Retire obsolete fragments too, so an old include cannot hide a removed list."""
    output.mkdir(parents=True, exist_ok=True)
    for path in output.glob("abi-*"):
        if path.is_file() and path.name not in fragments:
            path.unlink()
    for name, contents in fragments.items():
        path = output / name
        if not path.exists() or path.read_text(encoding="utf-8") != contents:
            path.write_text(contents, encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    write_fragments(args.output, generate())


if __name__ == "__main__":
    main()
