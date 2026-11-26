#!/usr/bin/env python3
"""Keep the tables C++ fills for Rust out of the headers core headers include."""

import importlib.util
import unittest
from pathlib import Path, PurePosixPath

spec = importlib.util.spec_from_file_location(
    "table_headers", Path(__file__).with_name("table-headers.py")
)
assert spec and spec.loader
checker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checker)

TABLE = "src/rust/shared_services_table.h"
WIDE = "src/rust/services_ffi.h"


def tree(files: dict[str, str]) -> dict[PurePosixPath, str]:
    sources = {WIDE: "#pragma once\n", TABLE: '#include "services_ffi.h"\n', **files}
    return {PurePosixPath(path): text for path, text in sources.items()}


class TableHeaderTests(unittest.TestCase):
    def hits(self, files: dict[PurePosixPath, str]) -> list[str]:
        return checker.violations(files)

    def test_direct_include_from_a_core_header_is_refused(self) -> None:
        hits = self.hits(
            tree({"src/vehicle_base.h": '#include "rust/shared_services_table.h"\n'})
        )
        self.assertEqual(hits, [f"src/vehicle_base.h -> {TABLE}"])

    def test_reach_through_a_fork_header_names_the_chain(self) -> None:
        files = tree(
            {
                "src/station_base.h": '#include "rust/station_ffi.h"\n',
                "src/rust/station_ffi.h": '#include "shared_services_table.h"\n',
            }
        )
        self.assertEqual(
            self.hits(files),
            [f"src/station_base.h -> src/rust/station_ffi.h -> {TABLE}"],
        )

    def test_angle_includes_resolve_through_each_search_path(self) -> None:
        for spelling in ("rust/shared_services_table.h", "shared_services_table.h"):
            with self.subTest(spelling=spelling):
                files = tree({"src/core/pool_type.hpp": f"#include <{spelling}>\n"})
                self.assertEqual(len(self.hits(files)), 1)

    def test_quoted_relative_include_from_a_subdirectory(self) -> None:
        files = tree(
            {
                "src/script/api/script_x.hpp": '# include "../../rust/shared_services_table.h"\n'
            }
        )
        self.assertEqual(len(self.hits(files)), 1)

    def test_splices_and_comments_inside_a_directive_are_read_as_the_compiler_does(self) -> None:
        for text in (
            '#include \\\n"rust/shared_services_table.h"\n',
            '#in\\\nclude "rust/shared_services_table.h"\n',
            '# /* note */ include "rust/shared_services_table.h"\n',
            '/* a */ #include <rust/shared_services_table.h> // b\n',
        ):
            with self.subTest(text=text):
                self.assertEqual(len(self.hits(tree({"src/map_func.h": text}))), 1)

    def test_commented_out_includes_are_ignored(self) -> None:
        for text in (
            '/*\n#include "rust/shared_services_table.h"\n*/\n',
            '// #include "rust/shared_services_table.h"\n',
            '// note \\\n#include "rust/shared_services_table.h"\n',
            'const char *s = "/*";\n#include "rust/services_ffi.h"\nconst char *t = "*/";\n',
        ):
            with self.subTest(text=text):
                self.assertEqual(self.hits(tree({"src/map_func.h": text})), [])

    def test_generated_setting_addresses_record_is_a_table(self) -> None:
        files = tree(
            {
                "src/settings_type.h": '#include "abi-record-settings.h"\n#include <abi-record-setting_addresses.h>\n',
            }
        )
        self.assertEqual(
            self.hits(files),
            ["src/settings_type.h -> generated-abi/abi-record-setting_addresses.h"],
        )

    def test_fillers_tests_and_wide_headers_pass(self) -> None:
        files = tree(
            {
                "src/vehicle_base.h": '#include "rust/services_ffi.h"\n#include <vector>\n',
                "src/rust/services.cpp": '#include "shared_services_table.h"\n',
                "src/tests/fixture.hpp": '#include "../rust/shared_services_table.h"\n',
                "src/rust/session_ffi.h": '#include "shared_services_table.h"\n',
                "src/station_cmd.cpp": '#include "rust/shared_services_table.h"\n',
            }
        )
        self.assertEqual(self.hits(files), [])

    def test_tree_passes(self) -> None:
        self.assertEqual(checker.violations(checker.tree(checker.ROOT)), [])


if __name__ == "__main__":
    unittest.main()
