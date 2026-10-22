#!/usr/bin/env python3
"""Generate native ABI probes and typed service visitors from Rust declarations.

Fragments are rebuildable build outputs; Rust is the only field inventory.
"""

import argparse
import re
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DECLARATION = re.compile(
    r'"([^"]+)"\s*\["([^"]+)"\]\s*=>\s*([^{}]+)\{([^{}]*)\}'
    r"\s*(?:width\s+(\w+)\s*:\s*([^;]+))?;"
)


def declarations(root=ROOT):
    """Yield C++ name, probe group, Rust type, field pairs and optional width."""
    seen = set()
    for path in sorted((root / "rust/openttd-kernels/src").rglob("*.rs")):
        source = path.read_text()
        if "layouts! {" not in source:
            continue
        source = re.sub(r"//[^\n]*", "", source)
        for name, group, rust, body, width, width_type in DECLARATION.findall(source):
            if name in seen:
                raise ValueError(f"duplicate ABI type: {name}")
            seen.add(name)
            fields = []
            for field in filter(str.strip, body.split(",")):
                pair = [part.strip() for part in field.split("=>")]
                if not all(re.fullmatch(r"(?:r#)?\w+", part) for part in pair):
                    raise ValueError(f"invalid ABI field: {field}")
                fields.append((pair[0], pair[-1]))
            if len(fields) != len({cpp for _, cpp in fields}):
                raise ValueError(f"duplicate C++ fields: {name}")
            if width and width not in dict(fields):
                raise ValueError(f"unknown width field: {name}.{width}")
            yield name, group, rust.strip(), fields, width, width_type
    if not seen:
        raise ValueError("no ABI declarations")


def generate(root=ROOT):
    """Fragments live only in the build directory; no generated inventory is checked in."""
    output = defaultdict(list)
    headers = {}
    for header in sorted((root / "src/rust").glob("*.h")):
        text = re.sub(r"//[^\n]*|/\*.*?\*/", "", header.read_text(), flags=re.S)
        for tag, body, alias in re.findall(
            r"(?:typedef\s+)?struct\s*(\w+)?\s*\{([^{}]*)\}\s*(\w+)?\s*;", text, re.S
        ):
            headers[alias or tag] = (header.name, body)
    # Sorted by C++ name, so the output does not depend on the hosting module.
    records = sorted(declarations(root))
    for name, (_, body) in headers.items():
        if re.search(r"\(\s*(?:\w+\s+)?\*\w+\)\s*\(", body):
            if name not in {record[0] for record in records}:
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
        header, body = headers.get(name, ("", ""))
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
    # Layout types need their declaring headers; component fragments add their own.
    output["abi-includes.inc"] = sorted(
        f'#include "rust/{headers[name][0]}"' for name, *_ in records if name in headers
    )
    # Component probe fragments, in file-name order; each registers itself.
    probes = sorted((root / "tools/migration/windows-abi").glob("*.inc"))
    output["abi-probes.inc"] = [
        f'#include "windows-abi/{path.name}"' for path in probes
    ]
    return {name: "\n".join(lines) + "\n" for name, lines in output.items()}


def write_fragments(output, fragments):
    """Retire obsolete fragments too, so an old include cannot hide a removed list."""
    output.mkdir(parents=True, exist_ok=True)
    for path in output.glob("abi-*"):
        if path.is_file() and path.name not in fragments:
            path.unlink()
    for name, contents in fragments.items():
        path = output / name
        if not path.exists() or path.read_text() != contents:
            path.write_text(contents)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    write_fragments(args.output, generate())


if __name__ == "__main__":
    main()
