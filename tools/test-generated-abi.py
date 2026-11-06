#!/usr/bin/env python3
"""Check generated ABI probes against their Rust declarations."""

import os
import re
import runpy
import subprocess
import tempfile
import unittest
from pathlib import Path
from typing import cast

ROOT = Path(__file__).resolve().parents[1]
GENERATOR = runpy.run_path(str(ROOT / "tools/generate-abi.py"))
LAYOUT = re.compile(r'Layout\("([^"]+)", \{(.*)\}\);')
ITEM = re.compile(r"\w+\((?:[^()]|\([^()]*\))*\)")


def layouts(fragments: dict[str, str]) -> dict[str, tuple[str, list[str]]]:
    """Map each generated layout name to its fragment and ordered items."""
    result: dict[str, tuple[str, list[str]]] = {}
    for path, contents in fragments.items():
        if path.startswith("abi-layout-"):
            for line in contents.splitlines():
                name, values = cast(re.Match[str], LAYOUT.fullmatch(line)).groups()
                result[name] = (path, ITEM.findall(values))
    return result


class GeneratedAbiTests(unittest.TestCase):
    def test_embedded_record_precedes_container_and_void_stays_opaque(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            rust = root / "rust/openttd-kernels/src"
            rust.mkdir(parents=True)
            (rust / "record.rs").write_text(
                'layouts! { "Other" ["test"] => Other {}; }\n'
                "shared_records! {\n    fn abi_record;\n"
                '    "AContainer" ["test"] => pub struct Container {\n'
                "        pub value: Value,\n        pub context: *mut c_void,\n"
                "    }\n"
                '    "ZValue" ["test"] => pub struct Value {\n'
                "        pub number: u32,\n"
                '        pub bytes: [u8; 3] => "std::array<uint8_t, 3>",\n    }\n}\n'
            )
            header = "\n".join(
                GENERATOR["record_header"]("test", GENERATOR["shared_records"](root))
            )
            source = root / "record.cpp"
            (root / "record.h").write_text(header)
            source.write_text(
                '#include "record.h"\n#include <type_traits>\n'
                "static_assert(std::is_same_v<decltype(AContainer::value), ZValue>);\n"
                "static_assert(std::is_same_v<decltype(AContainer::context), void *>);\n"
                "static_assert(std::is_same_v<decltype(ZValue::bytes), std::array<uint8_t, 3>>);\n"
            )
            subprocess.run(
                ["c++", "-std=c++20", "-fsyntax-only", str(source)], check=True
            )

    def test_callback_record_uses_rust_signature_and_native_calling_convention(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            rust = root / "rust/openttd-kernels/src"
            rust.mkdir(parents=True)
            (rust / "record.rs").write_text(
                'layouts! { "Legacy" ["ops"] => Legacy {}; }\n'
                'shared_records! {\n fn abi_record;\n'
                '"Ops" ["ops"] => pub struct Ops {\n'
                'pub context: *mut c_void,\n'
                'pub allocate: unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void,\n'
                'pub destroy: unsafe extern "C" fn(*mut c_void, *mut c_void),\n'
                '    }\n}\n'
            )
            (root / "ops.h").write_text("\n".join(
                GENERATOR["record_header"]("ops", GENERATOR["shared_records"](root))
            ))
            source = root / "ops.cpp"
            source.write_text(
                '#include "ops.h"\n#include <type_traits>\n'
                'using Allocate = void *(OPENTTD_RUST_CALL *)(void *, std::size_t) noexcept;\n'
                'using Destroy = void (OPENTTD_RUST_CALL *)(void *, void *) noexcept;\n'
                'static_assert(std::is_same_v<decltype(Ops::allocate), Allocate>);\n'
                'static_assert(std::is_same_v<decltype(Ops::destroy), Destroy>);\n'
            )
            subprocess.run(["c++", "-std=c++20", "-fsyntax-only", "-I", str(ROOT / "src/rust"), str(source)], check=True)

    def test_each_layout_and_visitor_lists_exactly_its_declared_fields(self) -> None:
        fragments = GENERATOR["generate"]()
        generated = layouts(fragments)
        records = list(GENERATOR["declarations"]())
        shared = GENERATOR["shared_records"]()
        self.assertEqual(
            set(generated),
            {name for name, *_ in records} | {record.cpp for record in shared},
        )
        for name, group, _, fields in shared:
            with self.subTest(record=name):
                self.assertEqual(
                    generated[name],
                    (
                        f"abi-layout-{group}.inc",
                        [f"sizeof({name})", f"alignof({name})"]
                        + [
                            item
                            for field in fields
                            for item in (
                                f"offsetof({name}, {field.name})",
                                f"sizeof({name}::{field.name})",
                            )
                        ],
                    ),
                )
        for name, group, _, fields, width, _ in records:
            with self.subTest(name=name):
                path, items = generated[name]
                self.assertEqual(path, f"abi-layout-{group}.inc")
                self.assertEqual(items[:2], [f"sizeof({name})", f"alignof({name})"])
                offsets = [
                    re.fullmatch(rf"offsetof\({re.escape(name)}, (\w+)\)", item)
                    for item in items[2:]
                ]
                cpp = [cpp for _, cpp in fields]
                self.assertEqual(
                    [match and match[1] for match in offsets[: len(cpp)]], cpp
                )
                self.assertEqual(
                    items[2 + len(cpp) :],
                    [f"sizeof(decltype({name}::{dict(fields)[width]}))"]
                    if width
                    else [],
                )
                visitor = fragments.get(f"abi-visit-{name}.hpp", "")
                visits = re.findall(r'visit\("(\w+)", table\.(\w+)\);', visitor)
                self.assertIn(visits, ([], [(field, field) for field in cpp]))
                self.assertNotIn("reinterpret_cast", visitor)

    def test_generation_is_deterministic_and_build_directory_only(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            subprocess.run(
                [
                    "python3",
                    "-X",
                    "utf8=0",
                    str(ROOT / "tools/generate-abi.py"),
                    directory,
                ],
                env={**os.environ, "LC_ALL": "C", "PYTHONCOERCECLOCALE": "0"},
                check=True,
            )
            self.assertEqual(
                GENERATOR["generate"](),
                {path.name: path.read_text() for path in Path(directory).iterdir()},
            )
        self.assertFalse(list((ROOT / "src").rglob("abi-layout-*.inc")))

    def test_nested_shared_record_is_complete_before_its_value_use(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            rust = root / "rust/openttd-kernels/src"
            rust.mkdir(parents=True)
            (rust / "records.rs").write_text(
                'layouts! { "LegacyRecord" ["nested"] => Legacy {}; }\n'
                "shared_records! { fn abi_record;\n"
                '"AParent" ["nested"] => pub struct Parent {\n'
                "pub child: Child, pub tail: u16, pub link: *const External, pub legacy: *mut Legacy,\n    }\n"
                '"ZChild" ["nested"] => pub struct Child {\n'
                "pub field: u32,\n    }\n"
                '"External" ["external"] => pub struct External { pub value: u8, }\n}\n'
            )
            fragments = GENERATOR["generate"](root)
            header = root / "abi-record-nested.h"
            header.write_text(fragments[header.name])
            subprocess.run(
                [
                    os.environ.get("CXX", "c++"),
                    "-std=c++20",
                    "-x",
                    "c++",
                    "-fsyntax-only",
                    "-",
                ],
                input=(
                    f'#include "{header}"\n#include <type_traits>\n'
                    "static_assert(std::is_same_v<decltype(AParent::child), ZChild>);\n"
                    "static_assert(offsetof(AParent, tail) == sizeof(ZChild));\n"
                    "static_assert(std::is_same_v<decltype(AParent::link), const External *>);\n"
                    "static_assert(std::is_same_v<decltype(AParent::legacy), LegacyRecord *>);\n"
                ),
                text=True,
                check=True,
            )

    def test_shared_record_opaque_pointer_class_keys(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            rust = root / "rust/openttd-kernels/src"
            rust.mkdir(parents=True)
            source = rust / "probe.rs"
            prefix = (
                'layouts! { "Other" ["probe"] => Other {}; }\n'
                "pub struct NativeClass { _private: [u8; 0] }\n"
                "pub struct NativeStruct { _private: [u8; 0] }\n"
                "shared_records! { fn abi_record;\n"
                '"Probe" ["probe"] => pub struct Probe {\n'
            )
            source.write_text(
                prefix
                + "pub class_pointer: *mut NativeClass => class,\n"
                + "pub const_class: *const NativeClass => class,\n"
                + "pub struct_pointer: *mut NativeStruct => struct,\n"
                + "pub default_struct: *const NativeStruct,\n    }\n}\n"
            )
            fragments = GENERATOR["generate"](root)
            with self.subTest(behavior="native declarations agree with class keys"):
                compiler = os.environ.get("CXX", "c++")
                header = root / "abi-record-probe.h"
                header.write_text(fragments[header.name])
                subprocess.run(
                    [
                        compiler,
                        "-std=c++20",
                        "-Werror=mismatched-tags",
                        "-x",
                        "c++",
                        "-fsyntax-only",
                        "-",
                    ],
                    input=(
                        f'#include "{header}"\n'
                        "class NativeClass {};\nstruct NativeStruct {};\n"
                        "static_assert(sizeof(Probe) == 4 * sizeof(void *));\n"
                    ),
                    text=True,
                    check=True,
                )
            for fields, error in (
                (
                    "pub first: *mut NativeClass => class, pub second: *const NativeClass",
                    "conflicting pointee tags",
                ),
                ("pub value: u8 => class", "pointee tag requires an opaque pointer"),
                ("pub value: *mut NativeClass => union", "unsupported pointee tag"),
            ):
                with self.subTest(error=error):
                    source.write_text(prefix + fields + "\n    }\n}\n")
                    with self.assertRaisesRegex(ValueError, error):
                        GENERATOR["generate"](root)

    def test_shared_record_native_types_and_arrays(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            rust = root / "rust/openttd-kernels/src"
            rust.mkdir(parents=True)
            source = rust / "probe.rs"
            prefix = (
                'layouts! { "Other" ["probe"] => Other {}; }\n'
                "shared_records! { fn abi_record;\n"
                '"Probe" ["probe"] => pub struct Probe {\n'
            )
            source.write_text(
                prefix
                + 'pub list: [u8; 4] => "std::array<uint8_t, 4>",\n'
                + "pub plain: [u16; COUNT],\n"
                + "pub grid: [[u16; 3]; 2],\n"
                + 'pub kinds: [u8; 3] => "Kind[3]",\n'
                + 'pub flags: u32 => "Flags",\n'
                + "pub last: u8,\n    }\n}\n"
            )
            fragments = GENERATOR["generate"](root)
            header = fragments["abi-record-probe.h"]
            with self.subTest(behavior="native type and bounds follow the field name"):
                self.assertIn("\tstd::array<uint8_t, 4> list;\n", header)
                self.assertIn("\tuint16_t plain[COUNT];\n", header)
                self.assertIn("\tuint16_t grid[2][3];\n", header)
                self.assertIn("\tKind kinds[3];\n", header)
                self.assertIn("\tFlags flags;\n", header)
            with self.subTest(
                behavior="layout checks every field after a comma in a type"
            ):
                _, items = layouts(fragments)["Probe"]
                for field in ("list", "plain", "grid", "kinds", "flags", "last"):
                    self.assertIn(f"offsetof(Probe, {field})", items)
                    self.assertIn(f"sizeof(Probe::{field})", items)
            with self.subTest(
                behavior="the declaration compiles with the includer's types"
            ):
                path = root / "abi-record-probe.h"
                path.write_text(header)
                subprocess.run(
                    [
                        os.environ.get("CXX", "c++"),
                        "-std=c++20",
                        "-x",
                        "c++",
                        "-fsyntax-only",
                        "-",
                    ],
                    input=(
                        "#include <array>\n#include <cstdint>\n"
                        "using Kind = uint8_t;\nstruct Flags { uint32_t value; };\n"
                        "constexpr int COUNT = 5;\n"
                        f'#include "{path}"\n'
                        "static_assert(sizeof(Probe::grid) == 12 && sizeof(Probe::plain) == 10);\n"
                    ),
                    text=True,
                    check=True,
                )
            for fields, error in (
                ("pub value: [u8 3]", "unsupported record field type"),
                ('pub value: u8 => "Kind', "invalid record field"),
            ):
                with self.subTest(error=error):
                    source.write_text(prefix + fields + "\n    }\n}\n")
                    with self.assertRaisesRegex(ValueError, error):
                        GENERATOR["generate"](root)

    def test_synthetic_tree_aliases_widths_and_callback_retirement(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            rust = root / "rust/openttd-kernels/src"
            headers = root / "src/rust"
            rust.mkdir(parents=True)
            headers.mkdir(parents=True)
            (rust / "unrelated.rs").write_text('"Other" ["new"] => Local {};')
            source = rust / "nested" / "probe.rs"
            source.parent.mkdir()
            source.write_text(
                "layouts! {\n"
                '    "Probe" ["new"] => crate::nested::Local {\n'
                "        call => callback, // Native name differs.\n"
                "        r#type => kind,\n"
                "        size,\n"
                "    } width size: u16;\n"
                "}\n"
            )
            (headers / "probe.h").write_text(
                "struct Probe { void (*callback)() noexcept; int r; };"
            )
            self.assertEqual(
                list(GENERATOR["declarations"](root)),
                [
                    (
                        "Probe",
                        "new",
                        "crate::nested::Local",
                        [("call", "callback"), ("r#type", "kind"), ("size", "size")],
                        "size",
                        "u16",
                    )
                ],
            )
            source.write_text(
                'layouts! { "Probe" ["new"] => Local { call => callback }; }'
            )
            fragments = GENERATOR["generate"](root)
            self.assertEqual(
                layouts(fragments),
                {
                    "Probe": (
                        "abi-layout-new.inc",
                        [
                            "sizeof(Probe)",
                            "alignof(Probe)",
                            "offsetof(Probe, callback)",
                        ],
                    )
                },
            )
            self.assertIn(
                'visit("callback", table.callback)', fragments["abi-visit-Probe.hpp"]
            )
            output = root / "build"
            GENERATOR["write_fragments"](output, fragments)
            source.write_text('layouts! { "Replacement" ["new"] => Local {}; }')
            with self.assertRaisesRegex(ValueError, "callback table absent"):
                GENERATOR["generate"](root)
            (headers / "probe.h").rename(headers / "probe.retired")
            GENERATOR["write_fragments"](output, GENERATOR["generate"](root))
            self.assertFalse((output / "abi-visit-Probe.hpp").exists())
            (headers / "probe.retired").rename(headers / "probe.h")
            source.write_text('layouts! { "Probe" ["new"] => Local {}; }')
            with self.assertRaisesRegex(ValueError, "callbacks absent"):
                GENERATOR["generate"](root)


if __name__ == "__main__":
    unittest.main()
